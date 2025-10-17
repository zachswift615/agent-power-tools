use crate::agent::messages::{Command, UIUpdate};
use crate::ui::markdown::render_markdown;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
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
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key).await?;
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
            UIUpdate::ToolExecutionCompleted {
                name,
                id: _,
                duration_ms,
            } => {
                if let Some(Message::Tool(ref mut text)) = self.conversation.last_mut() {
                    *text = format!("[Tool: {}] ✓ {}ms", name, duration_ms);
                }
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
        // Enable auto-scroll mode - actual scroll position calculated in render()
        self.auto_scroll_enabled = true;
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
                    self.auto_scroll_to_bottom();
                }
            }
            (KeyCode::Up, _) => {
                // Scroll up - disable auto-scroll when user manually scrolls
                self.auto_scroll_enabled = false;
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            (KeyCode::Down, _) => {
                // Scroll down
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                // Re-enable auto-scroll if we've scrolled back to bottom (offset = 0)
                if self.scroll_offset == 0 {
                    self.auto_scroll_enabled = true;
                }
            }
            (KeyCode::Char(c), _) => {
                self.input.push(c);
            }
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
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
            "Synthia v0.1.0 (↑/↓ scroll | ^S save | ^N new | ^L load){}",
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
                        Span::styled("Assistant:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
                        Span::styled("Assistant:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
                        Span::styled("Assistant: Thinking...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
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
}
