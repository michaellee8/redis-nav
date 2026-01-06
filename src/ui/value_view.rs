use crate::format::{detect_format, format_as_hex, highlight_json, pretty_json, DetectedFormat};
use crate::redis_client::RedisValue;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct ValueView<'a> {
    value: Option<&'a RedisValue>,
    key: Option<&'a str>,
    theme: &'a Theme,
    scroll: u16,
}

impl<'a> ValueView<'a> {
    pub fn new(
        value: Option<&'a RedisValue>,
        key: Option<&'a str>,
        theme: &'a Theme,
        scroll: u16,
    ) -> Self {
        Self {
            value,
            key,
            theme,
            scroll,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (lines, format_name) = match self.value {
            Some(RedisValue::String(s)) => {
                let format = detect_format(s.as_bytes());
                let lines = match format {
                    DetectedFormat::Json => {
                        if let Ok(pretty) = pretty_json(s) {
                            highlight_json(&pretty)
                        } else {
                            vec![Line::raw(s.clone())]
                        }
                    }
                    DetectedFormat::Binary => format_as_hex(s.as_bytes()),
                    _ => s.lines().map(|l| Line::raw(l.to_string())).collect(),
                };
                (lines, format_label(format))
            }
            Some(RedisValue::List(items)) => {
                let lines: Vec<Line> = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| Line::raw(format!("[{}] {}", i, item)))
                    .collect();
                (lines, "LIST")
            }
            Some(RedisValue::Set(items)) => {
                let lines: Vec<Line> = items.iter().map(|item| Line::raw(item.clone())).collect();
                (lines, "SET")
            }
            Some(RedisValue::ZSet(items)) => {
                let lines: Vec<Line> = items
                    .iter()
                    .map(|(member, score)| Line::raw(format!("{:.2}: {}", score, member)))
                    .collect();
                (lines, "ZSET")
            }
            Some(RedisValue::Hash(items)) => {
                let lines: Vec<Line> = items
                    .iter()
                    .map(|(k, v)| Line::raw(format!("{}: {}", k, v)))
                    .collect();
                (lines, "HASH")
            }
            _ => (vec![Line::raw("Select a key to view its value")], ""),
        };

        let title = match self.key {
            Some(k) if !format_name.is_empty() => format!(" {} ({}) ", k, format_name),
            Some(k) => format!(" {} ", k),
            None => " Value ".to_string(),
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border)
                    .title(title)
                    .title_style(self.theme.title),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

fn format_label(format: DetectedFormat) -> &'static str {
    match format {
        DetectedFormat::Json => "JSON",
        DetectedFormat::Xml => "XML",
        DetectedFormat::Html => "HTML",
        DetectedFormat::Binary => "BINARY",
        DetectedFormat::PlainText => "TEXT",
    }
}
