use crate::tree::TreeNode;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub struct TreeView<'a> {
    #[allow(dead_code)]
    nodes: &'a [TreeNode],
    state: &'a mut TreeViewState,
    theme: &'a Theme,
}

pub struct TreeViewState {
    pub list_state: ListState,
    pub flattened: Vec<FlatNode>,
}

#[derive(Debug, Clone)]
pub struct FlatNode {
    pub depth: usize,
    pub node_index: Vec<usize>, // Path to node in tree
    pub name: String,
    pub is_folder: bool,
    pub expanded: bool,
    pub child_count: usize,
    pub full_key: Option<String>,
}

impl TreeViewState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            flattened: Vec::new(),
        }
    }

    pub fn flatten(&mut self, nodes: &[TreeNode]) {
        self.flattened.clear();
        self.flatten_recursive(nodes, 0, &mut vec![]);

        if !self.flattened.is_empty() && self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    fn flatten_recursive(&mut self, nodes: &[TreeNode], depth: usize, path: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);

            self.flattened.push(FlatNode {
                depth,
                node_index: path.clone(),
                name: node.name.clone(),
                is_folder: node.is_folder(),
                expanded: node.expanded,
                child_count: node.child_count(),
                full_key: node.full_key.clone(),
            });

            if node.expanded {
                self.flatten_recursive(&node.children, depth + 1, path);
            }

            path.pop();
        }
    }

    pub fn selected_key(&self) -> Option<&str> {
        self.list_state
            .selected()
            .and_then(|i| self.flattened.get(i))
            .and_then(|n| n.full_key.as_deref())
    }
}

impl<'a> TreeView<'a> {
    pub fn new(nodes: &'a [TreeNode], state: &'a mut TreeViewState, theme: &'a Theme) -> Self {
        Self { nodes, state, theme }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .state
            .flattened
            .iter()
            .map(|node| {
                let indent = "  ".repeat(node.depth);
                let icon = if node.is_folder {
                    if node.expanded {
                        "[-] "
                    } else if node.child_count > 0 {
                        "[+] "
                    } else {
                        "[ ] "
                    }
                } else {
                    "    "
                };

                let suffix = if node.is_folder && node.child_count > 0 {
                    format!(" ({})", node.child_count)
                } else {
                    String::new()
                };

                let style = if node.is_folder {
                    self.theme.tree_folder
                } else {
                    self.theme.tree_key
                };

                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(icon, style),
                    Span::styled(node.name.clone(), style),
                    Span::styled(suffix, Style::default()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border)
                    .title(" Keys ")
                    .title_style(self.theme.title),
            )
            .highlight_style(self.theme.tree_selected)
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.state.list_state);
    }
}
