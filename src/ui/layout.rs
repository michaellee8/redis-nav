use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub tree_area: Rect,
    pub value_area: Rect,
    pub info_area: Rect,
    pub status_area: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let [main_area, status_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [tree_area, right_area] = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .areas(main_area);

        let [value_area, info_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(right_area);

        Self {
            tree_area,
            value_area,
            info_area,
            status_area,
        }
    }
}
