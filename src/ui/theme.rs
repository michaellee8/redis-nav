use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub tree_selected: Style,
    pub tree_folder: Style,
    pub tree_key: Style,
    pub ttl_normal: Style,
    pub ttl_warning: Style,
    pub ttl_critical: Style,
    pub border: Style,
    pub title: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            tree_selected: Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            tree_folder: Style::default().fg(Color::Blue),
            tree_key: Style::default().fg(Color::White),
            ttl_normal: Style::default().fg(Color::Green),
            ttl_warning: Style::default().fg(Color::Yellow),
            ttl_critical: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(Color::DarkGray),
            title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}
