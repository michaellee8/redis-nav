use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Json,
    Xml,
    Html,
    Binary,
    PlainText,
}

pub fn detect_format(bytes: &[u8]) -> DetectedFormat {
    // Check for binary content (non-UTF8 or control chars)
    if !is_valid_text(bytes) {
        return DetectedFormat::Binary;
    }

    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.trim(),
        Err(_) => return DetectedFormat::Binary,
    };

    // Try JSON
    if (text.starts_with('{') && text.ends_with('}'))
        || (text.starts_with('[') && text.ends_with(']'))
    {
        if serde_json::from_str::<serde_json::Value>(text).is_ok() {
            return DetectedFormat::Json;
        }
    }

    // Check for XML/HTML
    if text.starts_with("<?xml") || text.starts_with("<!DOCTYPE") {
        return DetectedFormat::Xml;
    }

    if text.to_lowercase().contains("<html") {
        return DetectedFormat::Html;
    }

    if text.starts_with('<') && text.ends_with('>') {
        return DetectedFormat::Xml;
    }

    DetectedFormat::PlainText
}

fn is_valid_text(bytes: &[u8]) -> bool {
    // Check for common binary signatures
    if bytes.len() >= 4 {
        // PNG
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return false;
        }
        // JPEG
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return false;
        }
        // GIF
        if bytes.starts_with(b"GIF8") {
            return false;
        }
        // PDF
        if bytes.starts_with(b"%PDF") {
            return false;
        }
    }

    // Check for too many control characters
    let control_count = bytes
        .iter()
        .filter(|&&b| b < 32 && b != b'\n' && b != b'\r' && b != b'\t')
        .count();

    // Less than 10% control chars (use multiplication to avoid integer division truncation)
    control_count * 10 < bytes.len() || control_count == 0
}

pub fn format_as_hex(bytes: &[u8]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (offset, chunk) in bytes.chunks(16).enumerate() {
        let addr = format!("{:08x}  ", offset * 16);

        let hex_part: String = chunk
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if i == 8 {
                    format!(" {:02x}", b)
                } else {
                    format!("{:02x} ", b)
                }
            })
            .collect();

        let padding = " ".repeat((16 - chunk.len()) * 3 + if chunk.len() <= 8 { 1 } else { 0 });

        let ascii_part: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        let line = Line::from(vec![
            Span::styled(addr, Style::default().fg(Color::DarkGray)),
            Span::styled(hex_part, Style::default().fg(Color::Yellow)),
            Span::raw(padding),
            Span::styled(format!(" |{}|", ascii_part), Style::default().fg(Color::Cyan)),
        ]);

        lines.push(line);
    }

    lines
}

pub fn pretty_json(json_str: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(json_str)?;
    Ok(serde_json::to_string_pretty(&value)?)
}

pub fn highlight_json(json_str: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line in json_str.lines() {
        let spans = highlight_json_line(line);
        lines.push(Line::from(spans));
    }

    lines
}

fn highlight_json_line(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let mut in_string = false;
    let mut is_key = true;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_string => {
                if !current.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
                in_string = true;
                current.push(c);
            }
            '"' if in_string => {
                current.push(c);
                let color = if is_key { Color::Blue } else { Color::Green };
                spans.push(Span::styled(std::mem::take(&mut current), Style::default().fg(color)));
                in_string = false;
            }
            ':' if !in_string => {
                if !current.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
                spans.push(Span::raw(":"));
                is_key = false;
            }
            ',' if !in_string => {
                if !current.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
                spans.push(Span::raw(","));
                is_key = true;
            }
            '{' | '}' | '[' | ']' if !in_string => {
                if !current.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
                spans.push(Span::styled(c.to_string(), Style::default().fg(Color::White)));
                is_key = c == '{';
            }
            _ if !in_string && (c.is_numeric() || c == '-' || c == '.') => {
                if current.is_empty() || current.chars().all(|x| x.is_numeric() || x == '-' || x == '.') {
                    current.push(c);
                } else {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                    current.push(c);
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        if current == "true" || current == "false" {
            spans.push(Span::styled(current, Style::default().fg(Color::Magenta)));
        } else if current == "null" {
            spans.push(Span::styled(current, Style::default().fg(Color::DarkGray)));
        } else if current.chars().all(|c| c.is_numeric() || c == '-' || c == '.' || c.is_whitespace()) {
            // Check if it's a number (might have leading whitespace)
            let trimmed = current.trim();
            if !trimmed.is_empty() && trimmed.parse::<f64>().is_ok() {
                let leading: String = current.chars().take_while(|c| c.is_whitespace()).collect();
                if !leading.is_empty() {
                    spans.push(Span::raw(leading));
                }
                spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::Yellow)));
            } else {
                spans.push(Span::raw(current));
            }
        } else {
            spans.push(Span::raw(current));
        }
    }

    spans
}
