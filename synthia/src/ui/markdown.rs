use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Simple markdown renderer for TUI
/// Supports: **bold**, *italic*, `code`, # headers, - lists, and code blocks
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lines = Vec::new();

    for line in text.lines() {
        // Handle code blocks
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End of code block - render it
                for code_line in &code_block_lines {
                    lines.push(Line::from(
                        Span::styled(
                            format!("  {}", code_line),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::DIM),
                        ),
                    ));
                }
                code_block_lines.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_lines.push(line.to_string());
            continue;
        }

        // Handle headers
        if line.starts_with("# ") {
            lines.push(Line::from(
                Span::styled(
                    line[2..].to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ));
            continue;
        } else if line.starts_with("## ") {
            lines.push(Line::from(
                Span::styled(
                    line[3..].to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ));
            continue;
        } else if line.starts_with("### ") {
            lines.push(Line::from(
                Span::styled(
                    line[4..].to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ));
            continue;
        }

        // Handle list items
        if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
            let indent = line.len() - line.trim_start().len();
            let content = line.trim_start()[2..].trim();
            let styled_line = parse_inline_formatting(content);
            lines.push(Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled("• ", Style::default().fg(Color::Green)),
            ].into_iter().chain(styled_line).collect::<Vec<_>>()));
            continue;
        }

        // Handle numbered lists
        if let Some(stripped) = line.trim_start().strip_prefix(|c: char| c.is_numeric()) {
            if stripped.starts_with(". ") {
                let indent = line.len() - line.trim_start().len();
                let content = stripped[2..].trim();
                let styled_line = parse_inline_formatting(content);
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(indent)),
                    Span::styled(
                        format!("{}. ", line.trim_start().chars().take_while(|c| c.is_numeric()).collect::<String>()),
                        Style::default().fg(Color::Green)
                    ),
                ].into_iter().chain(styled_line).collect::<Vec<_>>()));
                continue;
            }
        }

        // Regular line with inline formatting
        let styled_line = parse_inline_formatting(line);
        lines.push(Line::from(styled_line));
    }

    lines
}

fn parse_inline_formatting(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Bold: **text**
            '*' if chars.peek() == Some(&'*') => {
                // Push any accumulated text
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }

                chars.next(); // consume second *
                let mut bold_text = String::new();

                // Collect until closing **
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'*') {
                        chars.next(); // consume second *
                        break;
                    }
                    bold_text.push(c);
                }

                if !bold_text.is_empty() {
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                }
            }
            // Italic: *text*
            '*' => {
                // Push any accumulated text
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }

                let mut italic_text = String::new();

                // Collect until closing *
                while let Some(c) = chars.next() {
                    if c == '*' {
                        break;
                    }
                    italic_text.push(c);
                }

                if !italic_text.is_empty() {
                    spans.push(Span::styled(
                        italic_text,
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                }
            }
            // Inline code: `code`
            '`' => {
                // Push any accumulated text
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }

                let mut code_text = String::new();

                // Collect until closing `
                while let Some(c) = chars.next() {
                    if c == '`' {
                        break;
                    }
                    code_text.push(c);
                }

                if !code_text.is_empty() {
                    spans.push(Span::styled(
                        code_text,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::DIM),
                    ));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Push any remaining text
    if !current.is_empty() {
        spans.push(Span::raw(current));
    }

    // If no spans were created, return a single span with the original text
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_text() {
        let lines = render_markdown("This is **bold** text");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
    }

    #[test]
    fn test_italic_text() {
        let lines = render_markdown("This is *italic* text");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
    }

    #[test]
    fn test_inline_code() {
        let lines = render_markdown("Use `cargo build` to compile");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
    }

    #[test]
    fn test_headers() {
        let lines = render_markdown("# Header 1\n## Header 2\n### Header 3");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_list_items() {
        let lines = render_markdown("- Item 1\n- Item 2\n* Item 3");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_code_block() {
        let text = "```\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let lines = render_markdown(text);
        assert_eq!(lines.len(), 3); // 3 lines of code
    }

    #[test]
    fn test_mixed_formatting() {
        let lines = render_markdown("**Bold** and *italic* with `code`");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.len() >= 5);
    }
}
