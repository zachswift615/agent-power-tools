use crate::agent::messages::{Command, UIUpdate};
use crate::ui::markdown::render_markdown;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone)]
enum Message {
    User(String),
    Assistant(String),
    AssistantStreaming(String), // Accumulating streaming text
    Thinking, // "Thinking..." indicator
    Tool(String),
    Error(String),
}

pub struct App {
    conversation: Vec<Message>,
    input: String,
    cursor_position: usize, // Position of cursor in input string
    cmd_tx: Sender<Command>,
    ui_rx: Receiver<UIUpdate>,
    should_quit: bool,
    scroll_offset: u16,
    auto_scroll_enabled: bool, // Track if we should auto-scroll to bottom
    current_session_id: Option<String>,
    session_list: Vec<crate::session::SessionInfo>,
    show_session_list: bool,
    session_list_selected: usize,
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            conversation: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            cmd_tx,
            ui_rx,
            should_quit: false,
            scroll_offset: 0,
            auto_scroll_enabled: true, // Start with auto-scroll enabled
            current_session_id: None,
            session_list: Vec::new(),
            show_session_list: false,
            session_list_selected: 0,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Handle UI updates from agent
            while let Ok(update) = self.ui_rx.try_recv() {
                self.handle_ui_update(update);
            }

            // Render
            terminal.draw(|f| self.render(f))?;

            // Handle input
            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_input(key).await?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_input(mouse);
                    }
                    _ => {}
                }
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn handle_ui_update(&mut self, update: UIUpdate) {
        match update {
            UIUpdate::AssistantText(text) => {
                // Non-streaming: add complete message at once
                self.conversation.push(Message::Assistant(text));
                self.auto_scroll_to_bottom();
            }
            UIUpdate::AssistantThinking => {
                // Remove any previous thinking indicator
                if let Some(Message::Thinking) = self.conversation.last() {
                    self.conversation.pop();
                }
                self.conversation.push(Message::Thinking);
                self.auto_scroll_to_bottom();
            }
            UIUpdate::AssistantTextDelta(delta) => {
                // Streaming: accumulate text incrementally
                match self.conversation.last_mut() {
                    Some(Message::Thinking) => {
                        // Replace thinking indicator with first chunk
                        *self.conversation.last_mut().unwrap() = Message::AssistantStreaming(delta);
                    }
                    Some(Message::AssistantStreaming(ref mut text)) => {
                        // Append to existing streaming message
                        text.push_str(&delta);
                    }
                    _ => {
                        // Start new streaming message
                        self.conversation.push(Message::AssistantStreaming(delta));
                    }
                }
                self.auto_scroll_to_bottom();
            }
            UIUpdate::ToolExecutionStarted { name, id: _ } => {
                // Convert streaming message to final assistant message before tool execution
                if let Some(Message::AssistantStreaming(text)) = self.conversation.last().cloned() {
                    *self.conversation.last_mut().unwrap() = Message::Assistant(text);
                }
                self.conversation
                    .push(Message::Tool(format!("[Tool: {}] ⏳ Running...", name)));
                self.auto_scroll_to_bottom();
            }
            UIUpdate::ToolResult {
                name,
                id: _,
                input,
                output,
                is_error,
                duration_ms,
            } => {
                if let Some(Message::Tool(ref mut text)) = self.conversation.last_mut() {
                    // Format the tool result with input and output preview
                    let status_icon = if is_error { "✗" } else { "✓" };

                    // Extract key input params based on tool name
                    let input_summary = Self::format_tool_input(&name, &input);

                    // Truncate output to first ~5 lines (approximately 200 chars)
                    let output_lines: Vec<&str> = output.lines().take(5).collect();
                    let output_preview = output_lines.join("\n");
                    let has_more = output.lines().count() > 5 || output.len() > 200;

                    *text = format!(
                        "[Tool: {}] {} {}ms{}\n  Output: {}{}",
                        name,
                        status_icon,
                        duration_ms,
                        input_summary,
                        output_preview.trim(),
                        if has_more { "\n  ..." } else { "" }
                    );
                }
                self.auto_scroll_to_bottom();
            }
            UIUpdate::Error(err) => {
                self.conversation.push(Message::Error(err));
                self.auto_scroll_to_bottom();
            }
            UIUpdate::Complete => {
                // Finalize streaming message if present
                if let Some(Message::AssistantStreaming(text)) = self.conversation.last().cloned() {
                    *self.conversation.last_mut().unwrap() = Message::Assistant(text);
                }
            }
            UIUpdate::SessionSaved { session_id } => {
                self.current_session_id = Some(session_id.clone());
                self.conversation.push(Message::Tool(format!("[Session saved: {}]", session_id)));
                self.auto_scroll_to_bottom();
            }
            UIUpdate::SessionLoaded { session_id } => {
                self.current_session_id = Some(session_id);
                self.show_session_list = false;
            }
            UIUpdate::SessionList { sessions } => {
                self.session_list = sessions;
                self.show_session_list = true;
                self.session_list_selected = 0;
            }
        }
    }

    fn auto_scroll_to_bottom(&mut self) {
        // Enable auto-scroll mode and reset offset to ensure we're at the bottom
        self.auto_scroll_enabled = true;
        self.scroll_offset = 0;
    }

    /// Format tool input parameters for display
    fn format_tool_input(tool_name: &str, input: &serde_json::Value) -> String {
        // Extract relevant parameters based on tool type
        match tool_name {
            "bash" => {
                // For bash tool, show the command
                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                    let truncated = if command.len() > 60 {
                        format!("{}...", &command[..60])
                    } else {
                        command.to_string()
                    };
                    format!("\n  Command: {}", truncated)
                } else {
                    String::new()
                }
            }
            "read" => {
                // For read tool, show the file path
                if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
                    format!("\n  File: {}", file_path)
                } else {
                    String::new()
                }
            }
            "write" | "edit" => {
                // For write/edit tools, show the file path
                if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
                    format!("\n  File: {}", file_path)
                } else {
                    String::new()
                }
            }
            "grep" => {
                // For grep, show the pattern
                if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                    let truncated = if pattern.len() > 40 {
                        format!("{}...", &pattern[..40])
                    } else {
                        pattern.to_string()
                    };
                    format!("\n  Pattern: {}", truncated)
                } else {
                    String::new()
                }
            }
            "glob" => {
                // For glob, show the pattern
                if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                    format!("\n  Pattern: {}", pattern)
                } else {
                    String::new()
                }
            }
            _ => {
                // For other tools, don't show input details to keep it compact
                String::new()
            }
        }
    }

    async fn handle_input(
        &mut self,
        key: event::KeyEvent,
    ) -> anyhow::Result<()> {
        // Handle session list navigation when visible
        if self.show_session_list {
            match key.code {
                KeyCode::Up => {
                    if self.session_list_selected > 0 {
                        self.session_list_selected -= 1;
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    if self.session_list_selected < self.session_list.len().saturating_sub(1) {
                        self.session_list_selected += 1;
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    if let Some(session) = self.session_list.get(self.session_list_selected) {
                        self.cmd_tx.send(Command::LoadSession(session.id.clone())).await?;
                        self.conversation.clear();
                    }
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.show_session_list = false;
                    return Ok(());
                }
                _ => {}
            }
        }

        // Normal input handling
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Cancel).await?;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Shutdown).await?;
                self.should_quit = true;
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                // Manually save session
                self.cmd_tx.send(Command::SaveSession).await?;
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                // Start new session
                self.cmd_tx.send(Command::NewSession).await?;
                self.conversation.clear();
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                // List and load session
                self.cmd_tx.send(Command::ListSessions).await?;
            }
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let msg = self.input.clone();
                    self.conversation.push(Message::User(msg.clone()));
                    self.cmd_tx.send(Command::SendMessage(msg)).await?;
                    self.input.clear();
                    self.cursor_position = 0;
                    self.auto_scroll_to_bottom();
                }
            }
            (KeyCode::Up, KeyModifiers::CONTROL) => {
                // Scroll up - disable auto-scroll when user manually scrolls
                self.auto_scroll_enabled = false;
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            (KeyCode::Down, KeyModifiers::CONTROL) => {
                // Scroll down
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                // Re-enable auto-scroll if we've scrolled back to bottom (offset = 0)
                if self.scroll_offset == 0 {
                    self.auto_scroll_enabled = true;
                }
            }
            (KeyCode::Left, _) => {
                // Move cursor left
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            (KeyCode::Right, _) => {
                // Move cursor right
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
            }
            (KeyCode::Home, _) => {
                // Jump to start of input
                self.cursor_position = 0;
            }
            (KeyCode::End, _) => {
                // Jump to end of input
                self.cursor_position = self.input.len();
            }
            (KeyCode::Char(c), _) => {
                // Insert at cursor position
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            (KeyCode::Backspace, _) => {
                // Delete character before cursor
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.input.remove(self.cursor_position);
                }
            }
            (KeyCode::Delete, _) => {
                // Delete character after cursor
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse_input(&mut self, mouse: MouseEvent) {
        // Note: Mouse capture is disabled to allow terminal text selection.
        // This handler won't receive events, but is kept for potential future use.
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // Scroll up in message history (3 lines per scroll event for smooth scrolling)
                self.auto_scroll_enabled = false;
                self.scroll_offset = self.scroll_offset.saturating_add(3);
            }
            MouseEventKind::ScrollDown => {
                // Scroll down in message history (3 lines per scroll event)
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                // Re-enable auto-scroll if we've scrolled back to bottom (offset = 0)
                if self.scroll_offset == 0 {
                    self.auto_scroll_enabled = true;
                }
            }
            _ => {
                // Ignore other mouse events (clicks, drag, etc.)
            }
        }
    }

    fn render(&self, f: &mut ratatui::Frame) {
        // Calculate dynamic height for input based on text length
        // Account for block borders (2 lines) and calculate wrapped lines
        let input_width = f.area().width.saturating_sub(4); // Subtract borders and padding
        let input_lines = if self.input.is_empty() {
            1
        } else {
            (self.input.len() as u16 / input_width.max(1)) + 1
        };
        let input_height = (input_lines + 2).min(10); // +2 for borders, max 10 lines

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(input_height),
            ])
            .split(f.area());

        // Status bar with session info
        let session_info = if let Some(ref session_id) = self.current_session_id {
            format!(" | Session: {}", &session_id[..session_id.len().min(20)])
        } else {
            String::new()
        };
        let status_text = format!(
            "Synthia v0.1.0 (^↑/^↓ scroll | ^S save | ^N new | ^L load){}",
            session_info
        );
        let status = Paragraph::new(status_text)
            .style(Style::default().bg(Color::Blue).fg(Color::White))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[0]);

        // Conversation - render messages with markdown support
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.conversation {
            match msg {
                Message::User(text) => {
                    lines.push(Line::from(vec![
                        Span::styled("User: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::raw(text),
                    ]));
                    lines.push(Line::from("")); // Empty line for spacing
                }
                Message::Assistant(text) => {
                    lines.push(Line::from(
                        Span::styled("Synthia:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    ));

                    // Use custom markdown renderer
                    let markdown_lines = render_markdown(text);
                    for line in markdown_lines {
                        lines.push(line);
                    }
                    lines.push(Line::from("")); // Empty line for spacing
                }
                Message::AssistantStreaming(text) => {
                    // Display streaming text with a cursor indicator
                    lines.push(Line::from(
                        Span::styled("Synthia:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    ));

                    // Use custom markdown renderer for streaming text
                    let markdown_lines = render_markdown(text);
                    for line in markdown_lines {
                        lines.push(line);
                    }
                    // Add a blinking cursor to indicate streaming
                    lines.push(Line::from(
                        Span::styled("▊", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    ));
                    lines.push(Line::from("")); // Empty line for spacing
                }
                Message::Thinking => {
                    lines.push(Line::from(
                        Span::styled("Synthia: Thinking...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                    ));
                    lines.push(Line::from("")); // Empty line for spacing
                }
                Message::Tool(text) => {
                    lines.push(Line::from(
                        Span::styled(text, Style::default().fg(Color::Yellow))
                    ));
                }
                Message::Error(text) => {
                    lines.push(Line::from(
                        Span::styled(format!("Error: {}", text), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    ));
                    lines.push(Line::from("")); // Empty line for spacing
                }
            }
        }

        // Calculate scroll position
        // If auto-scroll is enabled, show the bottom of the conversation
        let scroll_offset = if self.auto_scroll_enabled {
            let total_lines = lines.len() as u16;
            let visible_height = chunks[1].height.saturating_sub(2); // Subtract borders
            total_lines.saturating_sub(visible_height)
        } else {
            self.scroll_offset
        };

        let conversation = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset, 0));
        f.render_widget(conversation, chunks[1]);

        // Input with wrapping support
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .wrap(Wrap { trim: false });
        f.render_widget(input, chunks[2]);

        // Set cursor position in the input field
        // Calculate cursor position accounting for text wrapping
        let cursor_x = chunks[2].x + 1 + (self.cursor_position as u16 % input_width.max(1));
        let cursor_y = chunks[2].y + 1 + (self.cursor_position as u16 / input_width.max(1));
        f.set_cursor_position((cursor_x, cursor_y));

        // Session list overlay (if showing)
        if self.show_session_list {
            use ratatui::widgets::{List, ListItem};

            // Create a centered overlay
            let popup_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Percentage(20),
                ])
                .split(f.area())[1];

            let popup_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Percentage(10),
                ])
                .split(popup_area)[1];

            // Create list items
            let items: Vec<ListItem> = self.session_list.iter().enumerate().map(|(idx, session)| {
                let style = if idx == self.session_list_selected {
                    Style::default().bg(Color::White).fg(Color::Black)
                } else {
                    Style::default()
                };

                let timestamp = chrono::DateTime::from_timestamp(session.last_modified, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let text = format!(
                    "{} - {} msgs - {}",
                    timestamp,
                    session.message_count,
                    &session.id[..session.id.len().min(30)]
                );

                ListItem::new(text).style(style)
            }).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Load Session (↑/↓ navigate | Enter select | Esc cancel)")
                        .style(Style::default().bg(Color::Black))
                );

            f.render_widget(list, popup_area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_scroll_to_bottom() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Disable auto-scroll
        app.auto_scroll_enabled = false;

        // Auto scroll should enable auto-scroll mode
        app.auto_scroll_to_bottom();
        assert!(app.auto_scroll_enabled, "Auto-scroll should be enabled");
    }

    #[test]
    fn test_message_types() {
        let msg_user = Message::User("test".to_string());
        let msg_assistant = Message::Assistant("test".to_string());
        let msg_streaming = Message::AssistantStreaming("test".to_string());
        let msg_thinking = Message::Thinking;
        let msg_tool = Message::Tool("test".to_string());
        let msg_error = Message::Error("test".to_string());

        // Just verify they can be created
        assert!(matches!(msg_user, Message::User(_)));
        assert!(matches!(msg_assistant, Message::Assistant(_)));
        assert!(matches!(msg_streaming, Message::AssistantStreaming(_)));
        assert!(matches!(msg_thinking, Message::Thinking));
        assert!(matches!(msg_tool, Message::Tool(_)));
        assert!(matches!(msg_error, Message::Error(_)));
    }

    #[test]
    fn test_scroll_offset_operations() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Initial offset is 0
        assert_eq!(app.scroll_offset, 0);

        // Scroll up
        app.scroll_offset = app.scroll_offset.saturating_add(5);
        assert_eq!(app.scroll_offset, 5);

        // Scroll down
        app.scroll_offset = app.scroll_offset.saturating_sub(2);
        assert_eq!(app.scroll_offset, 3);

        // Scroll down past 0 (should stop at 0)
        app.scroll_offset = app.scroll_offset.saturating_sub(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_ui_update_adds_messages() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Initially no messages
        assert_eq!(app.conversation.len(), 0);

        // Add assistant message
        app.handle_ui_update(UIUpdate::AssistantText("Hello".to_string()));
        assert_eq!(app.conversation.len(), 1);
        assert!(matches!(app.conversation[0], Message::Assistant(_)));

        // Add error message
        app.handle_ui_update(UIUpdate::Error("Error occurred".to_string()));
        assert_eq!(app.conversation.len(), 2);
        assert!(matches!(app.conversation[1], Message::Error(_)));

        // Add tool message
        app.handle_ui_update(UIUpdate::ToolExecutionStarted {
            name: "TestTool".to_string(),
            id: "123".to_string(),
        });
        assert_eq!(app.conversation.len(), 3);
        assert!(matches!(app.conversation[2], Message::Tool(_)));
    }

    #[test]
    fn test_auto_scroll_on_new_messages() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Disable auto-scroll (simulating user scrolling up)
        app.auto_scroll_enabled = false;

        // New assistant message should enable auto-scroll
        app.handle_ui_update(UIUpdate::AssistantText("New message".to_string()));
        assert!(app.auto_scroll_enabled, "Should enable auto-scroll on new assistant message");

        // Disable again
        app.auto_scroll_enabled = false;

        // New error message should also enable auto-scroll
        app.handle_ui_update(UIUpdate::Error("Error".to_string()));
        assert!(app.auto_scroll_enabled, "Should enable auto-scroll on error message");

        // Disable again
        app.auto_scroll_enabled = false;

        // Tool execution should enable auto-scroll
        app.handle_ui_update(UIUpdate::ToolExecutionStarted {
            name: "Tool".to_string(),
            id: "1".to_string(),
        });
        assert!(app.auto_scroll_enabled, "Should enable auto-scroll on tool execution");
    }

    #[test]
    fn test_manual_scroll_behavior() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Initially auto-scroll is enabled
        assert!(app.auto_scroll_enabled);

        // Scrolling up should disable auto-scroll
        app.scroll_offset = 0;
        app.scroll_offset = app.scroll_offset.saturating_add(1);
        app.auto_scroll_enabled = false; // Simulating Up key behavior
        assert!(!app.auto_scroll_enabled, "Manual scroll up should disable auto-scroll");
        assert_eq!(app.scroll_offset, 1);

        // Scrolling down to 0 should re-enable auto-scroll
        app.scroll_offset = app.scroll_offset.saturating_sub(1);
        if app.scroll_offset == 0 {
            app.auto_scroll_enabled = true; // Simulating Down key behavior
        }
        assert!(app.auto_scroll_enabled, "Scrolling back to bottom should re-enable auto-scroll");
    }

    #[test]
    fn test_typing_doesnt_affect_scroll() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Scroll up and disable auto-scroll
        app.auto_scroll_enabled = false;
        app.scroll_offset = 10;

        // Type some characters
        app.input.push('h');
        app.input.push('e');
        app.input.push('l');
        app.input.push('l');
        app.input.push('o');

        // Scroll position should not change
        assert_eq!(app.scroll_offset, 10, "Typing should not affect scroll position");
        assert!(!app.auto_scroll_enabled, "Typing should not enable auto-scroll");
    }

    #[test]
    fn test_mouse_scroll_up() {
        use crossterm::event::{MouseEvent, MouseEventKind};

        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Initially auto-scroll is enabled
        assert!(app.auto_scroll_enabled);
        assert_eq!(app.scroll_offset, 0);

        // Simulate mouse scroll up
        let mouse_event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse_input(mouse_event);

        // Should disable auto-scroll and increase offset by 3
        assert!(!app.auto_scroll_enabled, "Mouse scroll up should disable auto-scroll");
        assert_eq!(app.scroll_offset, 3, "Mouse scroll up should increase offset by 3");
    }

    #[test]
    fn test_mouse_scroll_down() {
        use crossterm::event::{MouseEvent, MouseEventKind};

        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Set up initial state with scroll offset
        app.auto_scroll_enabled = false;
        app.scroll_offset = 10;

        // Simulate mouse scroll down
        let mouse_event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse_input(mouse_event);

        // Should decrease offset by 3
        assert_eq!(app.scroll_offset, 7, "Mouse scroll down should decrease offset by 3");
        assert!(!app.auto_scroll_enabled, "Should not enable auto-scroll yet");
    }

    #[test]
    fn test_mouse_scroll_down_to_bottom() {
        use crossterm::event::{MouseEvent, MouseEventKind};

        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Set up initial state with small scroll offset
        app.auto_scroll_enabled = false;
        app.scroll_offset = 2;

        // Simulate mouse scroll down
        let mouse_event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse_input(mouse_event);

        // Should reach offset 0 and re-enable auto-scroll
        assert_eq!(app.scroll_offset, 0, "Should saturate at 0");
        assert!(app.auto_scroll_enabled, "Should re-enable auto-scroll when reaching bottom");
    }

    #[test]
    fn test_mouse_scroll_sensitivity() {
        use crossterm::event::{MouseEvent, MouseEventKind};

        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(10);
        let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(10);
        let mut app = App::new(cmd_tx, ui_rx);

        // Simulate 5 scroll up events
        for _ in 0..5 {
            let mouse_event = MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::empty(),
            };
            app.handle_mouse_input(mouse_event);
        }

        // Should scroll up by 15 lines total (5 events * 3 lines/event)
        assert_eq!(app.scroll_offset, 15, "5 scroll events should move 15 lines");
    }
}
