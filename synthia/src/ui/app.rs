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
                self.conversation.push(Message::Assistant(text));
                self.auto_scroll_to_bottom();
            }
            UIUpdate::ToolExecutionStarted { name, id: _ } => {
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
                // Generation complete
            }
        }
    }

    fn auto_scroll_to_bottom(&mut self) {
        // Reset scroll to 0, which will show the bottom of the conversation
        self.scroll_offset = 0;
    }

    async fn handle_input(
        &mut self,
        key: event::KeyEvent,
    ) -> anyhow::Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Cancel).await?;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Shutdown).await?;
                self.should_quit = true;
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
                // Scroll up
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            (KeyCode::Down, _) => {
                // Scroll down
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Status bar
        let status = Paragraph::new("Synthia v0.1.0 (↑/↓ to scroll)")
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

        let conversation = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        f.render_widget(conversation, chunks[1]);

        // Input
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[2]);
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

        // Set scroll offset to some value
        app.scroll_offset = 10;

        // Auto scroll should reset to 0
        app.auto_scroll_to_bottom();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_message_types() {
        let msg_user = Message::User("test".to_string());
        let msg_assistant = Message::Assistant("test".to_string());
        let msg_tool = Message::Tool("test".to_string());
        let msg_error = Message::Error("test".to_string());

        // Just verify they can be created
        assert!(matches!(msg_user, Message::User(_)));
        assert!(matches!(msg_assistant, Message::Assistant(_)));
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

        // Scroll up manually
        app.scroll_offset = 10;

        // New assistant message should auto-scroll to bottom
        app.handle_ui_update(UIUpdate::AssistantText("New message".to_string()));
        assert_eq!(app.scroll_offset, 0, "Should auto-scroll to bottom on new assistant message");

        // Scroll up again
        app.scroll_offset = 5;

        // New error message should also auto-scroll
        app.handle_ui_update(UIUpdate::Error("Error".to_string()));
        assert_eq!(app.scroll_offset, 0, "Should auto-scroll to bottom on error message");

        // Scroll up again
        app.scroll_offset = 7;

        // Tool execution should auto-scroll
        app.handle_ui_update(UIUpdate::ToolExecutionStarted {
            name: "Tool".to_string(),
            id: "1".to_string(),
        });
        assert_eq!(app.scroll_offset, 0, "Should auto-scroll to bottom on tool execution");
    }
}
