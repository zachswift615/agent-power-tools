use crate::agent::messages::{Command, UIUpdate};
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

pub struct App {
    conversation: Vec<String>,
    input: String,
    cmd_tx: Sender<Command>,
    ui_rx: Receiver<UIUpdate>,
    should_quit: bool,
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            conversation: Vec::new(),
            input: String::new(),
            cmd_tx,
            ui_rx,
            should_quit: false,
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
                self.conversation.push(format!("Assistant: {}", text));
            }
            UIUpdate::ToolExecutionStarted { name, id } => {
                self.conversation
                    .push(format!("[Tool: {}] ⏳ Running...", name));
            }
            UIUpdate::ToolExecutionCompleted {
                name,
                id,
                duration_ms,
            } => {
                if let Some(last) = self.conversation.last_mut() {
                    *last = format!("[Tool: {}] ✓ {}ms", name, duration_ms);
                }
            }
            UIUpdate::Error(err) => {
                self.conversation.push(format!("Error: {}", err));
            }
            UIUpdate::Complete => {
                // Generation complete
            }
        }
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
                    self.conversation.push(format!("User: {}", msg));
                    self.cmd_tx.send(Command::SendMessage(msg)).await?;
                    self.input.clear();
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Status bar
        let status = Paragraph::new("Synthia v0.1.0")
            .style(Style::default().bg(Color::Blue).fg(Color::White))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[0]);

        // Conversation
        let conversation_text: Vec<Line> = self
            .conversation
            .iter()
            .map(|msg| Line::from(msg.as_str()))
            .collect();
        let conversation = Paragraph::new(conversation_text)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false });
        f.render_widget(conversation, chunks[1]);

        // Input
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[2]);
    }
}
