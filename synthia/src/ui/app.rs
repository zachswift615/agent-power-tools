use crate::agent::messages::{Command, UIUpdate};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};
use std::io::{self, Write};
use tokio::sync::mpsc::{Receiver, Sender};

/// Wrap text at word boundaries for a given terminal width
fn wrap_text(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut current_line_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();

        // Check if adding this word would exceed width
        if current_line_len + word_len + 1 > width && current_line_len > 0 {
            result.push('\n');
            result.push_str(word);
            current_line_len = word_len;
        } else {
            if current_line_len > 0 {
                result.push(' ');
                current_line_len += 1;
            }
            result.push_str(word);
            current_line_len += word_len;
        }
    }

    result
}

pub struct App {
    input: String,
    cursor_position: usize,
    cmd_tx: Sender<Command>,
    ui_rx: Receiver<UIUpdate>,
    should_quit: bool,
    current_session_id: Option<String>,
    is_streaming: bool, // Track if currently receiving streaming text
    streaming_buffer: String, // Accumulate streaming text for final wrap
    session_list: Vec<crate::session::SessionInfo>,
    show_session_list: bool,
    session_list_selected: usize,
    input_needs_render: bool, // Track if input line needs re-rendering
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            cmd_tx,
            ui_rx,
            should_quit: false,
            current_session_id: None,
            is_streaming: false,
            streaming_buffer: String::new(),
            session_list: Vec::new(),
            show_session_list: false,
            session_list_selected: 0,
            input_needs_render: true, // Render on first loop
        }
    }

    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_pos)
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(self.input.len())
    }

    fn input_char_len(&self) -> usize {
        self.input.chars().count()
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();

        // Print welcome header
        self.print_header(&mut stdout)?;

        while !self.should_quit {
            // Handle UI updates from agent
            while let Ok(update) = self.ui_rx.try_recv() {
                self.handle_ui_update(&mut stdout, update)?;
            }

            // Process ALL pending key events before rendering
            // This prevents rendering after each character during paste operations
            let mut had_input = false;
            while event::poll(std::time::Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(&mut stdout, key).await?;
                    had_input = true;
                }
            }

            // Render input line only after all input processed
            if !self.show_session_list && self.input_needs_render {
                self.render_input_line(&mut stdout)?;
                self.input_needs_render = false;
            }

            // Wait a bit if no input (don't busy-loop)
            if !had_input {
                tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            }
        }

        // Cleanup
        disable_raw_mode()?;
        writeln!(stdout)?;
        stdout.flush()?;

        Ok(())
    }

    fn print_header(&self, stdout: &mut impl Write) -> io::Result<()> {
        queue!(
            stdout,
            SetForegroundColor(Color::Blue),
            Print("Synthia v0.1.0\n"),
            ResetColor,
            Print("\n")
        )?;
        stdout.flush()
    }

    fn handle_ui_update(&mut self, stdout: &mut impl Write, update: UIUpdate) -> io::Result<()> {
        match update {
            UIUpdate::AssistantText(text) => {
                self.clear_input_line(stdout)?;
                self.is_streaming = false;

                // Get terminal width and wrap text
                let (width, _) = size()?;
                let usable_width = (width as usize).saturating_sub(10); // -10 for "Synthia: " prefix
                let wrapped = wrap_text(&text, usable_width);

                queue!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("Synthia: "),
                    ResetColor
                )?;
                writeln!(stdout, "{}", wrapped)?;
                writeln!(stdout)?;
                stdout.flush()?;
                self.input_needs_render = true;
            }
            UIUpdate::AssistantThinking => {
                self.clear_input_line(stdout)?;
                self.is_streaming = false;

                queue!(
                    stdout,
                    SetForegroundColor(Color::DarkGrey),
                    Print("Synthia: Thinking...\n"),
                    ResetColor
                )?;
                stdout.flush()?;
            }
            UIUpdate::AssistantTextDelta(delta) => {
                if !self.is_streaming {
                    // First chunk - clear input line and print "Thinking..." indicator
                    self.clear_input_line(stdout)?;
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Cyan),
                        Print("Synthia: "),
                        SetForegroundColor(Color::DarkGrey),
                        Print("Thinking..."),
                        ResetColor
                    )?;
                    stdout.flush()?;
                    self.is_streaming = true;
                    self.streaming_buffer.clear();
                }

                // Just accumulate in buffer - don't print deltas
                // We'll print the wrapped version at the end
                self.streaming_buffer.push_str(&delta);
            }
            UIUpdate::ToolExecutionStarted { name, id: _ } => {
                self.clear_input_line(stdout)?;
                self.is_streaming = false;

                // Finalize streaming if needed
                if self.is_streaming {
                    writeln!(stdout)?;
                    writeln!(stdout)?;
                }

                queue!(
                    stdout,
                    SetForegroundColor(Color::Yellow),
                    Print(format!("[Tool: {}] ⏳ Running...\n", name)),
                    ResetColor
                )?;
                stdout.flush()?;
            }
            UIUpdate::ToolResult {
                name,
                id: _,
                input,
                output,
                is_error,
                duration_ms,
            } => {
                self.clear_input_line(stdout)?;

                let status_icon = if is_error { "✗" } else { "✓" };
                let color = if is_error { Color::Red } else { Color::Green };

                queue!(
                    stdout,
                    SetForegroundColor(Color::Yellow),
                    Print(format!("[Tool: {}] ", name)),
                    SetForegroundColor(color),
                    Print(format!("{} ", status_icon)),
                    ResetColor,
                    Print(format!("{}ms", duration_ms))
                )?;

                // Show command if bash
                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                    let truncated = if command.len() > 60 {
                        format!("{}...", &command[..60])
                    } else {
                        command.to_string()
                    };
                    writeln!(stdout, "\n  Command: {}", truncated)?;
                }

                // Show output preview
                let output_lines: Vec<&str> = output.lines().take(5).collect();
                let output_preview = output_lines.join("\n");
                let has_more = output.lines().count() > 5 || output.len() > 200;

                if !output_preview.is_empty() {
                    writeln!(stdout, "  Output: {}", output_preview.trim())?;
                    if has_more {
                        writeln!(stdout, "  ...")?;
                    }
                }

                writeln!(stdout)?;
                stdout.flush()?;
                self.input_needs_render = true;
            }
            UIUpdate::Error(err) => {
                self.clear_input_line(stdout)?;
                self.is_streaming = false;

                // Finalize streaming if needed
                if self.is_streaming {
                    writeln!(stdout)?;
                    writeln!(stdout)?;
                }

                queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print(format!("Error: {}\n", err)),
                    ResetColor
                )?;
                writeln!(stdout)?;
                stdout.flush()?;
                self.input_needs_render = true;
            }
            UIUpdate::Complete => {
                // Finalize streaming with proper word wrapping
                if self.is_streaming {
                    // Get terminal width
                    let (width, _) = size()?;
                    let usable_width = (width as usize).saturating_sub(10); // -10 for "Synthia: " prefix

                    // Wrap the accumulated text
                    let wrapped = wrap_text(&self.streaming_buffer, usable_width);

                    // Clear the unwrapped streaming output
                    self.clear_input_line(stdout)?;

                    // Re-print with proper wrapping
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Cyan),
                        Print("Synthia: "),
                        ResetColor
                    )?;
                    writeln!(stdout, "{}", wrapped)?;
                    writeln!(stdout)?;

                    self.is_streaming = false;
                    self.streaming_buffer.clear();
                    stdout.flush()?;
                    self.input_needs_render = true;
                }
            }
            UIUpdate::SessionSaved { session_id } => {
                self.current_session_id = Some(session_id.clone());
                self.clear_input_line(stdout)?;

                queue!(
                    stdout,
                    SetForegroundColor(Color::Yellow),
                    Print(format!("[Session saved: {}]\n", &session_id[..session_id.len().min(20)])),
                    ResetColor
                )?;
                stdout.flush()?;
            }
            UIUpdate::SessionLoaded { session_id } => {
                self.current_session_id = Some(session_id);
                self.show_session_list = false;
            }
            UIUpdate::SessionList { sessions } => {
                self.session_list = sessions;
                self.show_session_list = true;
                self.session_list_selected = 0;
                self.render_session_list(stdout)?;
            }
            UIUpdate::ConversationCleared => {
                // Clear screen
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    fn clear_input_line(&self, stdout: &mut impl Write) -> io::Result<()> {
        // Move to beginning of line and clear it
        queue!(
            stdout,
            Print("\r"),
            Clear(ClearType::CurrentLine)
        )?;
        Ok(())
    }

    fn render_input_line(&self, stdout: &mut impl Write) -> io::Result<()> {
        // Clear current line
        self.clear_input_line(stdout)?;

        // Print prompt and input
        queue!(
            stdout,
            SetForegroundColor(Color::Green),
            Print("You: "),
            ResetColor,
            Print(&self.input)
        )?;

        // Move cursor to correct position
        let prompt_len = 5; // "You: ".len()
        let cursor_x = prompt_len + self.cursor_position;
        queue!(stdout, Print("\r"), cursor::MoveRight(cursor_x as u16))?;

        stdout.flush()
    }

    fn render_session_list(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        writeln!(stdout, "\n=== Load Session (↑/↓ navigate | Enter select | Esc cancel) ===")?;

        for (idx, session) in self.session_list.iter().enumerate() {
            let selected = if idx == self.session_list_selected { ">" } else { " " };
            let timestamp = chrono::DateTime::from_timestamp(session.last_modified, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            if idx == self.session_list_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            writeln!(
                stdout,
                "{} {} - {} msgs - {}",
                selected,
                timestamp,
                session.message_count,
                &session.id[..session.id.len().min(30)]
            )?;

            if idx == self.session_list_selected {
                queue!(stdout, ResetColor)?;
            }
        }

        writeln!(stdout)?;
        stdout.flush()
    }

    async fn handle_input(&mut self, stdout: &mut impl Write, key: event::KeyEvent) -> anyhow::Result<()> {
        // Handle session list navigation
        if self.show_session_list {
            match key.code {
                KeyCode::Up => {
                    if self.session_list_selected > 0 {
                        self.session_list_selected -= 1;
                        self.render_session_list(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    if self.session_list_selected < self.session_list.len().saturating_sub(1) {
                        self.session_list_selected += 1;
                        self.render_session_list(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    if let Some(session) = self.session_list.get(self.session_list_selected) {
                        self.cmd_tx.send(Command::LoadSession(session.id.clone())).await?;
                        self.show_session_list = false;
                        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                        self.print_header(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.show_session_list = false;
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                _ => return Ok(()),
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
                self.cmd_tx.send(Command::SaveSession).await?;
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::NewSession).await?;
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::ListSessions).await?;
            }
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let msg = self.input.clone();

                    // Clear input line and print user message
                    self.clear_input_line(stdout)?;
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Green),
                        Print("You: "),
                        ResetColor
                    )?;
                    writeln!(stdout, "{}", msg)?;
                    writeln!(stdout)?;
                    stdout.flush()?;

                    self.cmd_tx.send(Command::SendMessage(msg)).await?;
                    self.input.clear();
                    self.cursor_position = 0;
                }
            }
            (KeyCode::Left, _) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.input_needs_render = true;
                }
            }
            (KeyCode::Right, _) => {
                if self.cursor_position < self.input_char_len() {
                    self.cursor_position += 1;
                    self.input_needs_render = true;
                }
            }
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.cursor_position = 0;
                self.input_needs_render = true;
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.cursor_position = self.input_char_len();
                self.input_needs_render = true;
            }
            (KeyCode::Char(c), _) => {
                let byte_pos = self.char_to_byte_pos(self.cursor_position);
                self.input.insert(byte_pos, c);
                self.cursor_position += 1;
                self.input_needs_render = true;
            }
            (KeyCode::Backspace, _) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    let byte_pos = self.char_to_byte_pos(self.cursor_position);
                    self.input.remove(byte_pos);
                    self.input_needs_render = true;
                }
            }
            (KeyCode::Delete, _) => {
                if self.cursor_position < self.input_char_len() {
                    let byte_pos = self.char_to_byte_pos(self.cursor_position);
                    self.input.remove(byte_pos);
                    self.input_needs_render = true;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
