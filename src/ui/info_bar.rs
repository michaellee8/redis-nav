use crate::redis_client::RedisType;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct InfoBar<'a> {
    key_type: Option<RedisType>,
    ttl: Option<i64>,
    size: Option<usize>,
    theme: &'a Theme,
    readonly: bool,
}

impl<'a> InfoBar<'a> {
    pub fn new(
        key_type: Option<RedisType>,
        ttl: Option<i64>,
        size: Option<usize>,
        theme: &'a Theme,
        readonly: bool,
    ) -> Self {
        Self {
            key_type,
            ttl,
            size,
            theme,
            readonly,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let type_str = match self.key_type {
            Some(RedisType::String) => "STRING",
            Some(RedisType::List) => "LIST",
            Some(RedisType::Set) => "SET",
            Some(RedisType::ZSet) => "ZSET",
            Some(RedisType::Hash) => "HASH",
            Some(RedisType::Stream) => "STREAM",
            Some(RedisType::Unknown) | None => "-",
        };

        let ttl_span = match self.ttl {
            Some(ttl) if ttl < 0 => Span::styled("no expiry", self.theme.ttl_normal),
            Some(ttl) if ttl < 60 => {
                Span::styled(format!("{}s", ttl), self.theme.ttl_critical)
            }
            Some(ttl) if ttl < 3600 => {
                Span::styled(format!("{}m", ttl / 60), self.theme.ttl_warning)
            }
            Some(ttl) => Span::styled(format!("{}h", ttl / 3600), self.theme.ttl_normal),
            None => Span::raw("-"),
        };

        let size_str = match self.size {
            Some(s) if s > 1024 * 1024 => format!("{:.1} MB", s as f64 / 1024.0 / 1024.0),
            Some(s) if s > 1024 => format!("{:.1} KB", s as f64 / 1024.0),
            Some(s) => format!("{} B", s),
            None => "-".to_string(),
        };

        let edit_hint = if self.readonly {
            Span::styled(" [readonly]", Style::default())
        } else {
            Span::styled(" [e]dit", Style::default())
        };

        let line = Line::from(vec![
            Span::raw(" Type: "),
            Span::styled(type_str, Style::default()),
            Span::raw(" | TTL: "),
            ttl_span,
            Span::raw(" | Size: "),
            Span::raw(size_str),
            Span::raw(" |"),
            edit_hint,
        ]);

        let paragraph = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border),
        );

        frame.render_widget(paragraph, area);
    }
}
