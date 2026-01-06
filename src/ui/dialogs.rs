use crate::config::ProtectionLevel;
use crate::ui::theme::Theme;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub enum Dialog {
    Help,
    Confirm {
        title: String,
        message: String,
        confirm_text: String,
    },
    Protection {
        namespace: String,
        level: ProtectionLevel,
    },
    DiffPreview {
        key: String,
        old_value: String,
        new_value: String,
    },
}

pub fn render_dialog(frame: &mut Frame, dialog: &Dialog, theme: &Theme) {
    let area = centered_rect(60, 50, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    match dialog {
        Dialog::Help => render_help(frame, area, theme),
        Dialog::Confirm {
            title,
            message,
            confirm_text,
        } => render_confirm(frame, area, title, message, confirm_text, theme),
        Dialog::Protection { namespace, level } => {
            render_protection(frame, area, namespace, *level, theme)
        }
        Dialog::DiffPreview {
            key,
            old_value,
            new_value,
        } => render_diff_preview(frame, area, key, old_value, new_value, theme),
    }
}

fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::raw("  j/Down    Move down"),
        Line::raw("  k/Up      Move up"),
        Line::raw("  h/Left    Collapse/parent"),
        Line::raw("  l/Right   Expand/select"),
        Line::raw("  Tab       Switch pane"),
        Line::raw("  /         Search"),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::raw("  e         Edit value"),
        Line::raw("  r         Refresh"),
        Line::raw("  d         Delete"),
        Line::raw("  y         Copy key"),
        Line::raw("  q         Quit"),
        Line::raw(""),
        Line::styled("Press Esc to close", Style::default().fg(Color::DarkGray)),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border)
                .title(" Help ")
                .title_style(theme.title),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

fn render_confirm(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    confirm_text: &str,
    _theme: &Theme,
) {
    let lines = vec![
        Line::raw(""),
        Line::raw(message),
        Line::raw(""),
        Line::styled(
            format!("Type '{}' to confirm, Esc to cancel", confirm_text),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(format!(" {} ", title))
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_protection(
    frame: &mut Frame,
    area: Rect,
    namespace: &str,
    level: ProtectionLevel,
    _theme: &Theme,
) {
    let (color, level_str, action) = match level {
        ProtectionLevel::Warn => (Color::Yellow, "WARN", "Press any key to continue, Esc to cancel"),
        ProtectionLevel::Confirm => (Color::Red, "CONFIRM", "Type 'yes' to confirm, Esc to cancel"),
        ProtectionLevel::Block => (Color::Red, "BLOCKED", "This operation is not allowed. Press Esc to close"),
    };

    let lines = vec![
        Line::raw(""),
        Line::styled(
            format!("Protected namespace: {}", namespace),
            Style::default().fg(color),
        ),
        Line::styled(
            format!("Protection level: {}", level_str),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::styled(action, Style::default().fg(Color::DarkGray)),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(" Protected Namespace ")
                .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_diff_preview(
    frame: &mut Frame,
    area: Rect,
    key: &str,
    old_value: &str,
    new_value: &str,
    theme: &Theme,
) {
    // Simple line-by-line diff
    let old_lines: Vec<&str> = old_value.lines().collect();
    let new_lines: Vec<&str> = new_value.lines().collect();

    let mut diff_lines = Vec::new();

    let max_len = old_lines.len().max(new_lines.len());
    for i in 0..max_len {
        let old_line = old_lines.get(i).copied();
        let new_line = new_lines.get(i).copied();

        match (old_line, new_line) {
            (Some(o), Some(n)) if o == n => {
                diff_lines.push(Line::raw(format!("  {}", o)));
            }
            (Some(o), Some(n)) => {
                diff_lines.push(Line::styled(
                    format!("- {}", o),
                    Style::default().fg(Color::Red),
                ));
                diff_lines.push(Line::styled(
                    format!("+ {}", n),
                    Style::default().fg(Color::Green),
                ));
            }
            (Some(o), None) => {
                diff_lines.push(Line::styled(
                    format!("- {}", o),
                    Style::default().fg(Color::Red),
                ));
            }
            (None, Some(n)) => {
                diff_lines.push(Line::styled(
                    format!("+ {}", n),
                    Style::default().fg(Color::Green),
                ));
            }
            (None, None) => {}
        }
    }

    diff_lines.push(Line::raw(""));
    diff_lines.push(Line::styled(
        "[Enter] Write to Redis    [Esc] Cancel",
        Style::default().fg(Color::DarkGray),
    ));

    let paragraph = Paragraph::new(diff_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border)
                .title(format!(" Confirm Changes to {} ", key))
                .title_style(theme.title),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);

    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);

    area
}
