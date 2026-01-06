use crate::config::{AppConfig, ProtectedNamespace, ProtectionLevel};
use crate::editor::ExternalEditor;
use crate::redis_client::{RedisClient, RedisType, RedisValue};
use crate::tree::{TreeBuilder, TreeNode};
use crate::ui::dialogs::Dialog;
use crate::ui::layout::AppLayout;
use crate::ui::theme::Theme;
use crate::ui::tree_view::{TreeView, TreeViewState};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct App {
    config: AppConfig,
    tree_nodes: Vec<TreeNode>,
    tree_state: TreeViewState,
    selected_value: Option<RedisValue>,
    selected_type: Option<RedisType>,
    selected_ttl: Option<i64>,
    theme: Theme,
    current_dialog: Option<Dialog>,
    value_scroll: u16,
    focus: Focus,
    should_quit: bool,
    status_message: String,
    redis_tx: mpsc::Sender<RedisCommand>,
    ui_rx: mpsc::Receiver<UiMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Value,
}

#[derive(Debug)]
pub enum RedisCommand {
    ScanKeys { pattern: String },
    GetValue { key: String },
    SetValue { key: String, value: Vec<u8> },
    DeleteKey { key: String },
}

#[derive(Debug)]
pub enum UiMessage {
    KeysLoaded(Vec<(String, RedisType)>),
    ValueLoaded {
        key: String,
        value: RedisValue,
        ttl: i64,
        redis_type: RedisType,
    },
    Error(String),
    WriteSuccess(String),
    DeleteSuccess(String),
}

impl App {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let (redis_tx, mut redis_rx) = mpsc::channel::<RedisCommand>(100);
        let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>(100);

        // Connect to Redis
        let mut client = RedisClient::connect(&config.connection.url).await?;

        // Spawn Redis task
        let _delimiters = config.ui.delimiters.clone();
        tokio::spawn(async move {
            while let Some(cmd) = redis_rx.recv().await {
                match cmd {
                    RedisCommand::ScanKeys { pattern } => {
                        match client.scan_keys(&pattern, 1000).await {
                            Ok(keys) => {
                                // Get types for all keys
                                let mut typed_keys = Vec::new();
                                for key in keys {
                                    let key_type =
                                        client.get_type(&key).await.unwrap_or(RedisType::Unknown);
                                    typed_keys.push((key, key_type));
                                }
                                let _ = ui_tx.send(UiMessage::KeysLoaded(typed_keys)).await;
                            }
                            Err(e) => {
                                let _ = ui_tx.send(UiMessage::Error(e.to_string())).await;
                            }
                        }
                    }
                    RedisCommand::GetValue { key } => {
                        let value_result = client.get_value(&key).await;
                        let ttl_result = client.get_ttl(&key).await;
                        let type_result = client.get_type(&key).await;

                        match (value_result, ttl_result, type_result) {
                            (Ok(value), Ok(ttl), Ok(redis_type)) => {
                                let _ = ui_tx
                                    .send(UiMessage::ValueLoaded {
                                        key,
                                        value,
                                        ttl,
                                        redis_type,
                                    })
                                    .await;
                            }
                            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                                let _ = ui_tx.send(UiMessage::Error(e.to_string())).await;
                            }
                        }
                    }
                    RedisCommand::SetValue { key, value } => {
                        let value_str = String::from_utf8_lossy(&value);
                        match client.set_string(&key, &value_str).await {
                            Ok(_) => {
                                let _ = ui_tx.send(UiMessage::WriteSuccess(key)).await;
                            }
                            Err(e) => {
                                let _ = ui_tx.send(UiMessage::Error(e.to_string())).await;
                            }
                        }
                    }
                    RedisCommand::DeleteKey { key } => match client.delete(&key).await {
                        Ok(_) => {
                            let _ = ui_tx.send(UiMessage::DeleteSuccess(key)).await;
                        }
                        Err(e) => {
                            let _ = ui_tx.send(UiMessage::Error(e.to_string())).await;
                        }
                    },
                }
            }
        });

        // Request initial scan
        redis_tx
            .send(RedisCommand::ScanKeys {
                pattern: "*".to_string(),
            })
            .await?;

        Ok(Self {
            config,
            tree_nodes: Vec::new(),
            tree_state: TreeViewState::new(),
            selected_value: None,
            selected_type: None,
            selected_ttl: None,
            theme: Theme::default(),
            current_dialog: None,
            value_scroll: 0,
            focus: Focus::Tree,
            should_quit: false,
            status_message: "Loading keys...".to_string(),
            redis_tx,
            ui_rx,
        })
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            // Process Redis messages
            while let Ok(msg) = self.ui_rx.try_recv() {
                self.handle_message(msg);
            }

            // Draw
            terminal.draw(|frame| self.render(frame))?;

            // Handle input
            if event::poll(Duration::from_millis(33))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await?;
                }
            }
        }

        Ok(())
    }

    fn handle_message(&mut self, msg: UiMessage) {
        match msg {
            UiMessage::KeysLoaded(keys) => {
                let builder = TreeBuilder::new(self.config.ui.delimiters.clone());
                self.tree_nodes = builder.build(&keys);
                self.tree_state.flatten(&self.tree_nodes);
                self.status_message = format!("Loaded {} keys", keys.len());
            }
            UiMessage::ValueLoaded {
                key,
                value,
                ttl,
                redis_type,
            } => {
                self.selected_value = Some(value);
                self.selected_ttl = Some(ttl);
                self.selected_type = Some(redis_type);
                self.value_scroll = 0;
                self.status_message = format!("Loaded {}", key);
            }
            UiMessage::Error(e) => {
                self.status_message = format!("Error: {}", e);
            }
            UiMessage::WriteSuccess(key) => {
                self.status_message = format!("Saved {}", key);
            }
            UiMessage::DeleteSuccess(key) => {
                self.status_message = format!("Deleted {}", key);
                // Trigger rescan
                let _ = self.redis_tx.try_send(RedisCommand::ScanKeys {
                    pattern: "*".to_string(),
                });
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        use crate::ui::info_bar::InfoBar;
        use crate::ui::value_view::ValueView;
        use ratatui::style::Style;
        use ratatui::widgets::Paragraph;

        let layout = AppLayout::new(frame.area());

        // Tree view
        let mut tree_view = TreeView::new(&self.tree_nodes, &mut self.tree_state, &self.theme);
        tree_view.render(frame, layout.tree_area);

        // Value view
        let selected_key = self.tree_state.selected_key();
        let value_view = ValueView::new(
            self.selected_value.as_ref(),
            selected_key,
            &self.theme,
            self.value_scroll,
        );
        value_view.render(frame, layout.value_area);

        // Info bar
        let size = match &self.selected_value {
            Some(RedisValue::String(s)) => Some(s.len()),
            _ => None,
        };
        let info_bar = InfoBar::new(
            self.selected_type,
            self.selected_ttl,
            size,
            &self.theme,
            self.config.connection.readonly,
        );
        info_bar.render(frame, layout.info_area);

        // Status bar
        let status = Paragraph::new(format!(
            " {} | {} | ? for help",
            self.config.connection.url, self.status_message
        ))
        .style(Style::default());
        frame.render_widget(status, layout.status_area);

        // Dialog
        if let Some(ref dialog) = self.current_dialog {
            crate::ui::dialogs::render_dialog(frame, dialog, &self.theme);
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle dialog first
        if self.current_dialog.is_some() {
            return self.handle_dialog_key(key).await;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.current_dialog = Some(Dialog::Help);
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Tree => Focus::Value,
                    Focus::Value => Focus::Tree,
                };
            }
            _ => match self.focus {
                Focus::Tree => self.handle_tree_key(key).await?,
                Focus::Value => self.handle_value_key(key),
            },
        }

        Ok(())
    }

    async fn handle_tree_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.tree_state.list_state.select_next();
                self.load_selected_value().await?;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tree_state.list_state.select_previous();
                self.load_selected_value().await?;
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                if let Some(idx) = self.tree_state.list_state.selected() {
                    if let Some(flat_node) = self.tree_state.flattened.get(idx) {
                        if flat_node.is_folder {
                            // Toggle expand
                            self.toggle_node_at_path(&flat_node.node_index.clone());
                            self.tree_state.flatten(&self.tree_nodes);
                        } else {
                            self.load_selected_value().await?;
                        }
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(idx) = self.tree_state.list_state.selected() {
                    if let Some(flat_node) = self.tree_state.flattened.get(idx) {
                        if flat_node.is_folder && flat_node.expanded {
                            self.toggle_node_at_path(&flat_node.node_index.clone());
                            self.tree_state.flatten(&self.tree_nodes);
                        }
                    }
                }
            }
            KeyCode::Char('g') => {
                self.tree_state.list_state.select_first();
                self.load_selected_value().await?;
            }
            KeyCode::Char('G') => {
                self.tree_state.list_state.select_last();
                self.load_selected_value().await?;
            }
            KeyCode::Char('r') => {
                self.load_selected_value().await?;
            }
            KeyCode::Char('R') => {
                self.status_message = "Rescanning...".to_string();
                self.redis_tx
                    .send(RedisCommand::ScanKeys {
                        pattern: "*".to_string(),
                    })
                    .await?;
            }
            KeyCode::Char('e') => {
                self.handle_edit().await?;
            }
            KeyCode::Char('d') => {
                self.handle_delete().await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_value_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.value_scroll = self.value_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.value_scroll = self.value_scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value_scroll = self.value_scroll.saturating_add(10);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value_scroll = self.value_scroll.saturating_sub(10);
            }
            KeyCode::Char('0') => {
                self.value_scroll = 0;
            }
            _ => {}
        }
    }

    async fn handle_dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.current_dialog = None;
            }
            KeyCode::Enter => {
                // Handle confirm actions based on dialog type
                if let Some(Dialog::DiffPreview { key, new_value, .. }) = &self.current_dialog {
                    if !self.config.connection.readonly {
                        self.redis_tx
                            .send(RedisCommand::SetValue {
                                key: key.clone(),
                                value: new_value.as_bytes().to_vec(),
                            })
                            .await?;
                    }
                }
                self.current_dialog = None;
            }
            _ => {}
        }

        Ok(())
    }

    fn toggle_node_at_path(&mut self, path: &[usize]) {
        fn toggle_recursive(nodes: &mut [TreeNode], path: &[usize]) {
            if path.is_empty() {
                return;
            }
            let idx = path[0];
            if path.len() == 1 {
                if let Some(node) = nodes.get_mut(idx) {
                    node.expanded = !node.expanded;
                }
            } else if let Some(node) = nodes.get_mut(idx) {
                toggle_recursive(&mut node.children, &path[1..]);
            }
        }
        toggle_recursive(&mut self.tree_nodes, path);
    }

    async fn load_selected_value(&mut self) -> Result<()> {
        if let Some(key) = self.tree_state.selected_key() {
            self.redis_tx
                .send(RedisCommand::GetValue {
                    key: key.to_string(),
                })
                .await?;
        }
        Ok(())
    }

    fn check_protection(&self, key: &str) -> Option<&ProtectedNamespace> {
        self.config
            .ui
            .protected_namespaces
            .iter()
            .find(|ns| key.starts_with(&ns.prefix))
    }

    async fn handle_edit(&mut self) -> Result<()> {
        if self.config.connection.readonly {
            self.status_message = "Read-only mode".to_string();
            return Ok(());
        }

        let Some(key) = self.tree_state.selected_key().map(|s| s.to_string()) else {
            return Ok(());
        };

        // Check protection
        if let Some(ns) = self.check_protection(&key) {
            match ns.level {
                ProtectionLevel::Block => {
                    self.current_dialog = Some(Dialog::Protection {
                        namespace: ns.prefix.clone(),
                        level: ns.level,
                    });
                    return Ok(());
                }
                ProtectionLevel::Confirm | ProtectionLevel::Warn => {
                    self.current_dialog = Some(Dialog::Protection {
                        namespace: ns.prefix.clone(),
                        level: ns.level,
                    });
                    // For simplicity, we'll skip edit in this case too
                    // A full implementation would handle the confirm flow
                    return Ok(());
                }
            }
        }

        // Get current value
        let Some(RedisValue::String(current_value)) = &self.selected_value else {
            self.status_message = "Only string values can be edited".to_string();
            return Ok(());
        };

        // Open editor
        let editor = ExternalEditor::new()?;
        match editor.edit(&key, current_value.as_bytes())? {
            Some(new_value) => {
                let new_str = String::from_utf8_lossy(&new_value).to_string();
                self.current_dialog = Some(Dialog::DiffPreview {
                    key,
                    old_value: current_value.clone(),
                    new_value: new_str,
                });
            }
            None => {
                self.status_message = "No changes made".to_string();
            }
        }

        Ok(())
    }

    async fn handle_delete(&mut self) -> Result<()> {
        if self.config.connection.readonly {
            self.status_message = "Read-only mode".to_string();
            return Ok(());
        }

        let Some(key) = self.tree_state.selected_key().map(|s| s.to_string()) else {
            return Ok(());
        };

        // Check protection
        if let Some(ns) = self.check_protection(&key) {
            self.current_dialog = Some(Dialog::Protection {
                namespace: ns.prefix.clone(),
                level: ns.level,
            });
            return Ok(());
        }

        self.current_dialog = Some(Dialog::Confirm {
            title: "Delete Key".to_string(),
            message: format!("Delete '{}'?", key),
            confirm_text: "yes".to_string(),
        });

        Ok(())
    }
}
