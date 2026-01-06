pub mod dialogs;
pub mod info_bar;
pub mod layout;
pub mod theme;
pub mod tree_view;
pub mod value_view;

use ratatui::Frame;

pub trait Component {
    fn render(&self, frame: &mut Frame, area: ratatui::layout::Rect);
}
