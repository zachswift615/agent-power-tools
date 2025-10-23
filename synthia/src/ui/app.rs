use crate::agent::messages::{Command, UIUpdate};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{Receiver, Sender};

// Event batching safety limits
const MAX_BATCH_SIZE: usize = 1000;
const BATCH_TIMEOUT_MS: u64 = 10;

/// Wrap text at word boundaries for a given terminal width
/// Markdown-aware: preserves newlines, code blocks, and indentation
fn wrap_text(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        // Check for code block markers
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // If in code block, preserve the line exactly
        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Empty lines are preserved
        if line.trim().is_empty() {
            result.push('\n');
            continue;
        }

        // Wrap regular lines
        let wrapped_line = wrap_single_line(line, width);
        result.push_str(&wrapped_line);
        result.push('\n');
    }

    result.trim_end().to_string()
}

/// Wrap a single line of text at word boundaries
fn wrap_single_line(text: &str, width: usize) -> String {
    let mut wrapped = String::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count(); // Unicode-aware

        if current_width > 0 && current_width + 1 + word_len > width {
            // Wrap to new line
            wrapped.push_str(&current_line);
            wrapped.push('\n');
            current_line.clear();
            current_width = 0;
        }

        if current_width > 0 {
            current_line.push(' ');
            current_width += 1;
        }

        // Handle very long words (URLs, hashes, etc.)
        if word_len > width {
            // Break at width boundary
            let chars: Vec<char> = word.chars().collect();
            let mut chunk_start = 0;

            while chunk_start < chars.len() {
                let remaining = width.saturating_sub(current_width);
                let chunk_end = (chunk_start + remaining).min(chars.len());
                let chunk: String = chars[chunk_start..chunk_end].iter().collect();

                if current_width > 0 {
                    wrapped.push_str(&current_line);
                    wrapped.push('\n');
                    current_line.clear();
                    current_width = 0;
                }

                current_line.push_str(&chunk);
                current_width = chunk_end - chunk_start;
                chunk_start = chunk_end;

                if chunk_start < chars.len() {
                    wrapped.push_str(&current_line);
                    wrapped.push('\n');
                    current_line.clear();
                    current_width = 0;
                }
            }
        } else {
            current_line.push_str(word);
            current_width += word_len;
        }
    }

    if !current_line.is_empty() {
        wrapped.push_str(&current_line);
    }

    wrapped
}

/// Sanitize text for terminal output by replacing tabs with spaces
/// Prevents terminal tab-stop issues that cause cascading indentation
fn sanitize_text(text: &str) -> String {
    text.replace('\t', "    ")
}

/// Print a line to terminal, ensuring cursor starts at column 0
/// Prevents cascading indentation by explicitly using \r (carriage return)
fn print_line(stdout: &mut impl Write, text: &str) -> io::Result<()> {
    execute!(stdout, Print(format!("\r{}\n", text)))
}

/// Print a colored line to terminal, ensuring cursor starts at column 0
fn print_colored_line(stdout: &mut impl Write, text: &str, color: Color) -> io::Result<()> {
    execute!(
        stdout,
        Print("\r"),
        SetForegroundColor(color),
        Print(text),
        ResetColor,
        Print("\n")
    )
}

/// Print a bordered line (for edit preview box)
/// Format: "│ <text>"
fn print_bordered_line(stdout: &mut impl Write, text: &str, color: Color) -> io::Result<()> {
    execute!(
        stdout,
        Print("\r│ "),
        SetForegroundColor(color),
        Print(text),
        ResetColor,
        Print("\n")
    )
}

#[derive(Debug)]
struct EditApprovalState {
    file_path: String,
    old_string: String,
    new_string: String,
    diff: String,
    response_tx: tokio::sync::oneshot::Sender<crate::agent::messages::ApprovalResponse>,
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
    is_rendering_input: bool, // Guard flag to prevent concurrent input renders
    pending_edit_approval: Option<EditApprovalState>,
    show_menu: bool,              // NEW: Menu display flag
    menu_selected: usize,         // NEW: Selected menu item index
    show_reasoning_submenu: bool, // NEW: Reasoning submenu display flag
    reasoning_submenu_selected: usize, // NEW: Selected reasoning level index
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
            is_rendering_input: false, // Not rendering initially
            pending_edit_approval: None,
            show_menu: false,         // NEW
            menu_selected: 0,         // NEW
            show_reasoning_submenu: false, // NEW
            reasoning_submenu_selected: 1, // NEW: default to "medium" (index 1)
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
            // Safety limits prevent infinite loops from paste bombs
            let mut had_input = false;
            let mut events_processed = 0;
            let batch_start = Instant::now();

            while event::poll(Duration::from_millis(0))?
                && events_processed < MAX_BATCH_SIZE
                && batch_start.elapsed() < Duration::from_millis(BATCH_TIMEOUT_MS)
            {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(&mut stdout, key).await?;
                    had_input = true;
                    events_processed += 1;
                }
            }

            if events_processed >= MAX_BATCH_SIZE {
                tracing::warn!("Hit max batch size ({}), possible paste bomb detected", MAX_BATCH_SIZE);
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

                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("Synthia: "),
                    ResetColor
                )?;

                // Print each line with explicit carriage return
                for line in wrapped.lines() {
                    execute!(stdout, Print(format!("\r{}\n", line)))?;
                }
                execute!(stdout, Print("\r\n"))?;
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

                // Write tool header line atomically (no queue/flush mixing)
                write!(stdout, "{}", SetForegroundColor(Color::Yellow))?;
                write!(stdout, "[Tool: {}] ", name)?;
                write!(stdout, "{}", SetForegroundColor(color))?;
                write!(stdout, "{} ", status_icon)?;
                write!(stdout, "{}", ResetColor)?;
                writeln!(stdout, "{}ms", duration_ms)?;

                // Show command if bash
                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                    let truncated = if command.len() > 60 {
                        format!("{}...", &command[..60])
                    } else {
                        command.to_string()
                    };
                    let sanitized = sanitize_text(&truncated);
                    execute!(stdout, Print(format!("\r  Command: {}\n", sanitized)))?;
                }

                // Show output preview
                let output_lines: Vec<&str> = output.lines().take(5).collect();
                let has_more = output.lines().count() > 5 || output.len() > 200;

                if !output_lines.is_empty() {
                    // Print Output label with explicit cursor reset
                    execute!(stdout, Print("\r\nOutput:\n"))?;

                    // Print each line with cursor reset to prevent cascading indentation
                    for line in output_lines {
                        let sanitized = sanitize_text(line);
                        execute!(stdout, Print(format!("\r  {}\n", sanitized.trim())))?;
                    }

                    if has_more {
                        execute!(stdout, Print("\r  ...\n"))?;
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
                    // Print line-by-line to ensure proper carriage returns
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Cyan),
                        Print("Synthia: "),
                        ResetColor
                    )?;

                    // Print each line with explicit carriage return
                    for line in wrapped.lines() {
                        execute!(stdout, Print(format!("\r{}\n", line)))?;
                    }
                    execute!(stdout, Print("\r\n"))?;

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
            UIUpdate::EditPreview {
                file_path,
                old_string,
                new_string,
                diff,
                response_tx,
            } => {
                self.clear_input_line(stdout)?;

                // Store approval state with channel
                self.pending_edit_approval = Some(EditApprovalState {
                    file_path: file_path.clone(),
                    old_string,
                    new_string,
                    diff: diff.clone(),
                    response_tx,
                });

                // Show diff preview
                self.render_edit_approval_prompt(stdout, &file_path, &diff)?;
            }
            UIUpdate::MenuDisplayRequested => {
                // Menu display is triggered by Ctrl+M in handle_input, not via UIUpdate
                // This variant is a no-op for now
            }
        }

        Ok(())
    }

    fn clear_input_line(&self, stdout: &mut impl Write) -> io::Result<()> {
        // Move to beginning of line and clear it
        // MUST flush immediately to prevent cascading indentation from queued operations
        execute!(
            stdout,
            Print("\r"),
            Clear(ClearType::CurrentLine)
        )
    }

    fn render_input_line(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        // Guard against concurrent renders (prevents duplication)
        if self.is_rendering_input {
            return Ok(());
        }

        self.is_rendering_input = true;

        // IMPORTANT: When input is long and wraps, we need to clear ALL lines it occupies
        // Calculate how many lines the current input takes
        let (term_width, _) = size()?;
        let prompt_len = 5; // "You: "
        let total_len = prompt_len + self.input.chars().count();
        let lines_needed = (total_len + term_width as usize - 1) / term_width as usize;

        // Get current cursor position
        let (_, mut cursor_y) = cursor::position()?;

        // Clear all lines that the input currently occupies
        // Move cursor up to the start of the input
        if lines_needed > 1 {
            // If we're on a wrapped line, we need to move up to clear from the start
            let lines_to_move_up = lines_needed.saturating_sub(1);
            if lines_to_move_up > 0 && cursor_y >= lines_to_move_up as u16 {
                cursor_y -= lines_to_move_up as u16;
                queue!(stdout, cursor::MoveTo(0, cursor_y))?;
            }
        }

        // Clear from current position to end of screen (clears all wrapped lines)
        queue!(
            stdout,
            cursor::MoveTo(0, cursor_y),
            Clear(ClearType::FromCursorDown)
        )?;
        stdout.flush()?;

        // Print prompt and input
        queue!(
            stdout,
            SetForegroundColor(Color::Green),
            Print("You: "),
            ResetColor,
            Print(&self.input),
        )?;

        // Calculate and move to correct cursor position
        let cursor_x = (prompt_len + self.cursor_position) % term_width as usize;
        let cursor_line_offset = (prompt_len + self.cursor_position) / term_width as usize;
        let final_cursor_y = cursor_y + cursor_line_offset as u16;

        queue!(stdout, cursor::MoveTo(cursor_x as u16, final_cursor_y))?;

        // Flush all queued operations at once
        let result = stdout.flush();

        // Release the guard
        self.is_rendering_input = false;

        result
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

    fn render_edit_approval_prompt(&self, stdout: &mut impl Write, file_path: &str, diff: &str) -> io::Result<()> {
        // Top border
        print_colored_line(stdout, "┌─ Edit Preview ────────────────────────────────────────┐", Color::Yellow)?;

        // Count changes
        let mut additions = 0;
        let mut deletions = 0;
        for line in diff.lines() {
            if line.starts_with('+') {
                additions += 1;
            } else if line.starts_with('-') {
                deletions += 1;
            }
        }

        // File path and change summary
        print_line(stdout, &format!("│ File: {}", file_path))?;
        print_line(stdout, &format!("│ Changes: +{} lines, -{} lines", additions, deletions))?;
        print_line(stdout, "│")?;

        // Focused diff: show only changed sections with context
        let context_lines = 2; // Show 2 lines of context around changes
        let max_consecutive_unchanged = 3; // Collapse if more than 3 unchanged lines in a row

        let all_lines: Vec<&str> = diff.lines().collect();
        let mut i = 0;
        let mut shown_lines = 0;
        let max_total_lines = 100; // Absolute max to prevent overwhelming output

        while i < all_lines.len() && shown_lines < max_total_lines {
            let line = all_lines[i];

            // Check if this line or nearby lines have changes
            let has_nearby_change = {
                let start = i.saturating_sub(context_lines);
                let end = (i + context_lines + 1).min(all_lines.len());
                all_lines[start..end].iter().any(|l| l.starts_with('+') || l.starts_with('-'))
            };

            if has_nearby_change {
                // Show this line (it's a change or near a change)
                let color = if line.starts_with('+') {
                    Color::Green
                } else if line.starts_with('-') {
                    Color::Red
                } else {
                    Color::DarkGrey // Context lines in grey
                };

                let sanitized = sanitize_text(line);
                print_bordered_line(stdout, &sanitized, color)?;
                shown_lines += 1;
                i += 1;
            } else {
                // Look ahead to find the next change
                let mut skip_count = 0;
                let mut next_change_idx = i;
                while next_change_idx < all_lines.len() {
                    if all_lines[next_change_idx].starts_with('+') || all_lines[next_change_idx].starts_with('-') {
                        break;
                    }
                    skip_count += 1;
                    next_change_idx += 1;
                }

                if skip_count > max_consecutive_unchanged {
                    // Collapse this section
                    print_line(stdout, &format!("│ ... ({} unchanged lines) ...", skip_count))?;
                    shown_lines += 1;
                    i = next_change_idx;
                } else {
                    // Just show this line
                    let sanitized = sanitize_text(line);
                    print_bordered_line(stdout, &sanitized, Color::DarkGrey)?;
                    shown_lines += 1;
                    i += 1;
                }
            }
        }

        if i < all_lines.len() {
            print_line(stdout, &format!("│ ... ({} more lines not shown) ...", all_lines.len() - i))?;
        }

        print_line(stdout, "│")?;

        // Accept/Reject prompt
        print_bordered_line(stdout, "[A]ccept  [R]eject", Color::Cyan)?;

        // Bottom border
        print_colored_line(stdout, "└───────────────────────────────────────────────────────┘", Color::Yellow)?;

        Ok(())
    }

    fn render_menu(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        writeln!(stdout, "\n=== Synthia Menu (↑/↓ navigate | Enter select | Esc cancel) ===")?;

        let menu_items = vec![
            "Set Session Name",
            "Save Session",
            "New Session",
            "Set Reasoning Level",
            "Context Management (Coming Soon)",
            "Toggle Mode (Coming Soon)",
        ];

        for (idx, item) in menu_items.iter().enumerate() {
            let selected = if idx == self.menu_selected { ">" } else { " " };

            if idx == self.menu_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            // Dim "Coming Soon" items
            if item.contains("Coming Soon") {
                queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
            }

            writeln!(stdout, "{} {}", selected, item)?;

            if idx == self.menu_selected || item.contains("Coming Soon") {
                queue!(stdout, ResetColor)?;
            }
        }

        writeln!(stdout)?;
        stdout.flush()
    }

    fn show_reasoning_submenu(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        self.show_menu = false;
        self.show_reasoning_submenu = true;
        self.reasoning_submenu_selected = 1; // Default to medium
        self.render_reasoning_submenu(stdout)
    }

    fn render_reasoning_submenu(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        writeln!(stdout, "\n=== Select Reasoning Level (↑/↓ navigate | Enter select | Esc cancel) ===")?;

        let levels = vec![
            ("Low", "Fast responses for general dialogue"),
            ("Medium", "Balanced speed and detail"),
            ("High", "Deep and detailed analysis"),
        ];

        for (idx, (level, desc)) in levels.iter().enumerate() {
            let selected = if idx == self.reasoning_submenu_selected { ">" } else { " " };

            if idx == self.reasoning_submenu_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            writeln!(stdout, "{} {} - {}", selected, level, desc)?;

            if idx == self.reasoning_submenu_selected {
                queue!(stdout, ResetColor)?;
            }
        }

        writeln!(stdout)?;
        stdout.flush()
    }

    async fn handle_menu_selection(&mut self, stdout: &mut impl Write) -> anyhow::Result<()> {
        match self.menu_selected {
            0 => {
                // Set Session Name
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;

                // Prompt for name
                print_colored_line(stdout, "Enter session name:", Color::Yellow)?;
                self.input.clear();
                self.cursor_position = 0;
                self.input_needs_render = true;

                // TODO: Capture input and send SetSessionName command
                // For now, this is a placeholder
            }
            1 => {
                // Save Session
                self.cmd_tx.send(Command::SaveSession).await?;
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            2 => {
                // New Session
                self.cmd_tx.send(Command::NewSession).await?;
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            3 => {
                // Set Reasoning Level - show submenu
                self.show_reasoning_submenu(stdout)?;
            }
            4 | 5 => {
                // Coming Soon items - do nothing
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_input(&mut self, stdout: &mut impl Write, key: event::KeyEvent) -> anyhow::Result<()> {
        // Handle edit approval input
        if let Some(approval_state) = self.pending_edit_approval.take() {
            match (key.code, key.modifiers) {
                (KeyCode::Char('a'), _) | (KeyCode::Char('A'), _) => {
                    let _ = approval_state.response_tx.send(crate::agent::messages::ApprovalResponse::Approve);
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                (KeyCode::Char('r'), _) | (KeyCode::Char('R'), _) => {
                    let _ = approval_state.response_tx.send(crate::agent::messages::ApprovalResponse::Reject);
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                _ => {
                    // Put it back if user didn't approve/reject
                    self.pending_edit_approval = Some(approval_state);
                    return Ok(());
                }
            }
        }

        // Handle menu navigation
        if self.show_menu {
            match key.code {
                KeyCode::Up => {
                    if self.menu_selected > 0 {
                        self.menu_selected -= 1;
                        self.render_menu(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    let menu_item_count = 6;  // Total menu items
                    if self.menu_selected < menu_item_count - 1 {
                        self.menu_selected += 1;
                        self.render_menu(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    self.handle_menu_selection(stdout).await?;
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.show_menu = false;
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

        // Handle reasoning submenu navigation
        if self.show_reasoning_submenu {
            match key.code {
                KeyCode::Up => {
                    if self.reasoning_submenu_selected > 0 {
                        self.reasoning_submenu_selected -= 1;
                        self.render_reasoning_submenu(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    if self.reasoning_submenu_selected < 2 {  // 3 options (0, 1, 2)
                        self.reasoning_submenu_selected += 1;
                        self.render_reasoning_submenu(stdout)?;
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    let level = match self.reasoning_submenu_selected {
                        0 => "low",
                        1 => "medium",
                        2 => "high",
                        _ => "medium",
                    };

                    self.cmd_tx.send(Command::SetReasoningLevel(level.to_string())).await?;
                    self.show_reasoning_submenu = false;
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;

                    print_colored_line(stdout, &format!("Reasoning level set to: {}", level), Color::Green)?;

                    return Ok(());
                }
                KeyCode::Esc => {
                    self.show_reasoning_submenu = false;
                    self.show_menu = true;
                    self.render_menu(stdout)?;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

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
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                self.show_menu = true;
                self.menu_selected = 0;
                self.render_menu(stdout)?;
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
                    let sanitized = sanitize_text(&msg);
                    writeln!(stdout, "{}", sanitized)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrapping_basic() {
        let text = "This is a very long line that should wrap properly at word boundaries";
        let wrapped = wrap_text(text, 20);

        // Check no mid-word breaks
        assert!(!wrapped.contains("boun daries"));
        assert!(!wrapped.contains("wor d"));

        // All lines should be <= 20 chars
        for line in wrapped.lines() {
            assert!(line.chars().count() <= 20, "Line too long: {}", line);
        }
    }

    #[test]
    fn test_word_wrapping_exact_fit() {
        let text = "Hello world test wrap";
        let wrapped = wrap_text(text, 11);

        // "Hello world" = 11 chars, should be on one line
        // "test wrap" should wrap to next lines
        let lines: Vec<&str> = wrapped.lines().collect();
        assert_eq!(lines[0], "Hello world");
    }

    #[test]
    fn test_long_word_wrapping() {
        let text = "Short https://verylongurlthatexceedsterminalwidthbyalot.com/path";
        let wrapped = wrap_text(text, 20);

        // Long URLs should break at width boundary
        for line in wrapped.lines() {
            assert!(line.chars().count() <= 20, "Line too long: '{}' ({})", line, line.chars().count());
        }

        // Should have multiple lines
        assert!(wrapped.lines().count() > 2);
    }

    #[test]
    fn test_unicode_wrapping() {
        // Emoji are multi-byte but count as 1 character
        let text = "Hello 🦀 Rust 🌟 is 💯 awesome";
        let wrapped = wrap_text(text, 15);

        for line in wrapped.lines() {
            assert!(line.chars().count() <= 15, "Line too long: '{}' ({})", line, line.chars().count());
        }

        // Text should wrap at word boundaries, preserving full words
        // "Hello 🦀 Rust 🌟" = 15 chars (fits exactly on one line)
        // "is 💯 awesome" = 13 chars (fits on second line)
        let lines: Vec<&str> = wrapped.lines().collect();
        assert_eq!(lines.len(), 2, "Expected 2 lines, got: {:?}", lines);
        assert_eq!(lines[0], "Hello 🦀 Rust 🌟");
        assert_eq!(lines[1], "is 💯 awesome");
    }

    #[test]
    fn test_unicode_long_word() {
        // Japanese text with long URL
        let text = "日本語 https://example.com/very/long/path/that/exceeds/width テスト";
        let wrapped = wrap_text(text, 20);

        for line in wrapped.lines() {
            assert!(line.chars().count() <= 20, "Line too long: '{}' ({})", line, line.chars().count());
        }
    }

    #[test]
    fn test_empty_text() {
        let wrapped = wrap_text("", 20);
        assert_eq!(wrapped, "");
    }

    #[test]
    fn test_single_word() {
        let wrapped = wrap_text("Hello", 20);
        assert_eq!(wrapped, "Hello");
    }

    #[test]
    fn test_single_long_word() {
        let word = "a".repeat(50);
        let wrapped = wrap_text(&word, 20);

        // Should break into chunks of 20
        for line in wrapped.lines() {
            assert!(line.chars().count() <= 20);
        }

        // Should have 3 lines (50 / 20 = 2.5, rounds to 3)
        assert_eq!(wrapped.lines().count(), 3);
    }

    #[test]
    fn test_hash_wrapping() {
        // Test with git commit hash
        let text = "Commit abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567 found";
        let wrapped = wrap_text(text, 20);

        for line in wrapped.lines() {
            assert!(line.chars().count() <= 20, "Line too long: '{}' ({})", line, line.chars().count());
        }
    }

    #[test]
    fn test_multiple_spaces() {
        // Multiple spaces should be treated as single separator
        let text = "Hello    world    test";
        let wrapped = wrap_text(text, 20);

        // Should not create empty words
        assert!(!wrapped.contains("  "));
    }

    #[test]
    fn test_width_one() {
        // Edge case: width of 1
        let text = "abc";
        let wrapped = wrap_text(text, 1);

        // Each character should be on its own line
        assert_eq!(wrapped.lines().count(), 3);
        assert_eq!(wrapped, "a\nb\nc");
    }

    #[test]
    fn test_real_world_flask_output() {
        // Simulate the user's Flask TODO example
        let text = "The application is a TODO list manager using HTML templates and in-memory storage.";
        let wrapped = wrap_text(text, 40);

        // Should wrap properly without breaking "using HTML"
        assert!(!wrapped.contains("usin g"));
        assert!(!wrapped.contains("HT ML"));

        for line in wrapped.lines() {
            assert!(line.chars().count() <= 40);
        }
    }

    #[test]
    fn test_batch_size_constant() {
        // Verify MAX_BATCH_SIZE is set to expected value
        assert_eq!(MAX_BATCH_SIZE, 1000);
    }

    #[test]
    fn test_batch_timeout_constant() {
        // Verify BATCH_TIMEOUT_MS is set to expected value
        assert_eq!(BATCH_TIMEOUT_MS, 10);
    }

    // Integration tests for event batching would require mocking crossterm events
    // These tests verify the constants are set correctly
    // End-to-end testing would be done manually or in integration tests
}
