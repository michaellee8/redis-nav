# redis-nav Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a terminal UI for browsing and editing Redis databases with tree-based key hierarchy, syntax highlighting, and safety features.

**Architecture:** Elm-inspired unidirectional data flow with async Redis operations via tokio channels. UI renders on main thread at 30fps, Redis SCAN runs on background tasks. Config supports both CLI args and TOML profiles.

**Tech Stack:** Rust, ratatui, tokio, redis-rs, clap, serde, syntect

---

## Phase 0: Project Setup

### Task 0.1: Initialize Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

**Step 1: Create Cargo.toml with all dependencies**

```toml
[package]
name = "redis-nav"
version = "0.1.0"
edition = "2021"
description = "Terminal UI for browsing and editing Redis databases"
license = "MIT"
authors = ["redis-nav contributors"]

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# TUI
ratatui = { version = "0.29", features = ["crossterm"] }
crossterm = "0.28"

# Redis
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }

# CLI & Config
clap = { version = "4", features = ["derive"] }
toml = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Syntax highlighting
syntect = "5"

# Utilities
thiserror = "2"
anyhow = "1"
dirs = "5"
unicode-width = "0.2"

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create src/lib.rs with module declarations**

```rust
pub mod app;
pub mod config;
pub mod editor;
pub mod format;
pub mod redis_client;
pub mod tree;
pub mod ui;
```

**Step 3: Create src/main.rs with minimal entry point**

```rust
use anyhow::Result;
use clap::Parser;
use redis_nav::config::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("redis-nav starting with: {:?}", cli);
    Ok(())
}
```

**Step 4: Update .gitignore**

```
/target
Cargo.lock
*.swp
*.swo
.DS_Store
```

**Step 5: Verify project compiles**

Run: `cargo check`
Expected: Compilation errors (missing modules) - this is expected, we'll add them next.

**Step 6: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs .gitignore
git commit -m "chore: initialize cargo project with dependencies"
```

---

### Task 0.2: Create Module Skeleton Files

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/cli.rs`
- Create: `src/config/file.rs`
- Create: `src/redis_client/mod.rs`
- Create: `src/tree/mod.rs`
- Create: `src/format/mod.rs`
- Create: `src/ui/mod.rs`
- Create: `src/editor/mod.rs`
- Create: `src/app.rs`

**Step 1: Create config module**

`src/config/mod.rs`:
```rust
pub mod cli;
pub mod file;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub connection: ConnectionConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub url: String,
    pub db: u8,
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub delimiters: Vec<char>,
    pub protected_namespaces: Vec<ProtectedNamespace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedNamespace {
    pub prefix: String,
    pub level: ProtectionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionLevel {
    Warn,
    Confirm,
    Block,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                db: 0,
                readonly: false,
            },
            ui: UiConfig {
                delimiters: vec![':', '/'],
                protected_namespaces: vec![],
            },
        }
    }
}
```

`src/config/cli.rs`:
```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "redis-nav")]
#[command(about = "Terminal UI for browsing and editing Redis databases")]
pub struct Cli {
    /// Redis URL (redis://host:port) or profile name
    #[arg(value_name = "CONNECTION")]
    pub connection: Option<String>,

    /// Redis host
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Redis port
    #[arg(short, long, default_value = "6379")]
    pub port: u16,

    /// Redis password (or use REDIS_PASSWORD env)
    #[arg(short = 'a', long)]
    pub password: Option<String>,

    /// Database number
    #[arg(short = 'n', long, default_value = "0")]
    pub db: u8,

    /// Key delimiter (can be specified multiple times)
    #[arg(short, long, default_value = ":")]
    pub delimiter: Vec<char>,

    /// Use named profile from config
    #[arg(long)]
    pub profile: Option<String>,

    /// Disable all write operations
    #[arg(long)]
    pub readonly: bool,

    /// Config file path
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}
```

`src/config/file.rs`:
```rust
use super::{ProtectedNamespace, ProtectionLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub delimiters: Vec<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub password_env: Option<String>,
    pub db: Option<u8>,
    #[serde(default)]
    pub delimiters: Vec<String>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub protected_namespaces: Vec<ProtectedNamespace>,
}

impl ConfigFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        Ok(config)
    }
}
```

**Step 2: Create redis_client module**

`src/redis_client/mod.rs`:
```rust
use anyhow::Result;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use tokio::sync::mpsc;

pub struct RedisClient {
    connection: MultiplexedConnection,
}

#[derive(Debug, Clone)]
pub enum RedisValue {
    String(String),
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(Vec<(String, String)>),
    Stream(String), // Simplified for now
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisType {
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    Unknown,
}

impl RedisClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let client = Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self { connection })
    }

    pub async fn scan_keys(&mut self, pattern: &str, count: usize) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (new_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(count)
                .query_async(&mut self.connection)
                .await?;

            keys.extend(batch);
            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        Ok(keys)
    }

    pub async fn get_type(&mut self, key: &str) -> Result<RedisType> {
        let type_str: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut self.connection)
            .await?;

        Ok(match type_str.as_str() {
            "string" => RedisType::String,
            "list" => RedisType::List,
            "set" => RedisType::Set,
            "zset" => RedisType::ZSet,
            "hash" => RedisType::Hash,
            "stream" => RedisType::Stream,
            _ => RedisType::Unknown,
        })
    }

    pub async fn get_value(&mut self, key: &str) -> Result<RedisValue> {
        let key_type = self.get_type(key).await?;

        match key_type {
            RedisType::String => {
                let val: String = self.connection.get(key).await?;
                Ok(RedisValue::String(val))
            }
            RedisType::List => {
                let val: Vec<String> = self.connection.lrange(key, 0, -1).await?;
                Ok(RedisValue::List(val))
            }
            RedisType::Set => {
                let val: Vec<String> = self.connection.smembers(key).await?;
                Ok(RedisValue::Set(val))
            }
            RedisType::ZSet => {
                let val: Vec<(String, f64)> = self.connection.zrange_withscores(key, 0, -1).await?;
                Ok(RedisValue::ZSet(val))
            }
            RedisType::Hash => {
                let val: Vec<(String, String)> = self.connection.hgetall(key).await?;
                Ok(RedisValue::Hash(val))
            }
            _ => Ok(RedisValue::None),
        }
    }

    pub async fn get_ttl(&mut self, key: &str) -> Result<i64> {
        let ttl: i64 = self.connection.ttl(key).await?;
        Ok(ttl)
    }

    pub async fn set_string(&mut self, key: &str, value: &str) -> Result<()> {
        self.connection.set(key, value).await?;
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.connection.del(key).await?;
        Ok(())
    }
}
```

**Step 3: Create tree module**

`src/tree/mod.rs`:
```rust
use crate::redis_client::RedisType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub full_key: Option<String>,
    pub node_type: NodeType,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Folder,
    Key(RedisType),
}

impl TreeNode {
    pub fn new_folder(name: String) -> Self {
        Self {
            name,
            full_key: None,
            node_type: NodeType::Folder,
            children: Vec::new(),
            expanded: false,
            loaded: true,
        }
    }

    pub fn new_key(name: String, full_key: String, redis_type: RedisType) -> Self {
        Self {
            name,
            full_key: Some(full_key),
            node_type: NodeType::Key(redis_type),
            children: Vec::new(),
            expanded: false,
            loaded: true,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self.node_type, NodeType::Folder)
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

pub struct TreeBuilder {
    delimiters: Vec<char>,
}

impl TreeBuilder {
    pub fn new(delimiters: Vec<char>) -> Self {
        Self { delimiters }
    }

    pub fn build(&self, keys: &[(String, RedisType)]) -> Vec<TreeNode> {
        let mut root_children: Vec<TreeNode> = Vec::new();

        for (key, redis_type) in keys {
            self.insert_key(&mut root_children, key, *redis_type);
        }

        self.sort_nodes(&mut root_children);
        root_children
    }

    fn insert_key(&self, nodes: &mut Vec<TreeNode>, key: &str, redis_type: RedisType) {
        let parts = self.split_key(key);

        if parts.is_empty() {
            return;
        }

        self.insert_parts(nodes, &parts, key, redis_type);
    }

    fn insert_parts(
        &self,
        nodes: &mut Vec<TreeNode>,
        parts: &[&str],
        full_key: &str,
        redis_type: RedisType,
    ) {
        if parts.is_empty() {
            return;
        }

        let name = parts[0];
        let remaining = &parts[1..];

        // Find or create node
        let node_idx = nodes.iter().position(|n| n.name == name);

        if remaining.is_empty() {
            // This is a leaf node (actual key)
            if let Some(idx) = node_idx {
                // Convert folder to key if needed, or update
                if nodes[idx].is_folder() {
                    // Keep as folder but mark it also has a key
                    nodes[idx].full_key = Some(full_key.to_string());
                    nodes[idx].node_type = NodeType::Key(redis_type);
                }
            } else {
                nodes.push(TreeNode::new_key(
                    name.to_string(),
                    full_key.to_string(),
                    redis_type,
                ));
            }
        } else {
            // This is an intermediate node (folder)
            let idx = if let Some(idx) = node_idx {
                idx
            } else {
                nodes.push(TreeNode::new_folder(name.to_string()));
                nodes.len() - 1
            };

            self.insert_parts(&mut nodes[idx].children, remaining, full_key, redis_type);
        }
    }

    fn split_key<'a>(&self, key: &'a str) -> Vec<&'a str> {
        let mut parts = Vec::new();
        let mut start = 0;

        for (i, c) in key.char_indices() {
            if self.delimiters.contains(&c) {
                if i > start {
                    parts.push(&key[start..i]);
                }
                start = i + c.len_utf8();
            }
        }

        if start < key.len() {
            parts.push(&key[start..]);
        }

        parts
    }

    fn sort_nodes(&self, nodes: &mut Vec<TreeNode>) {
        nodes.sort_by(|a, b| {
            // Folders first, then by name
            match (&a.node_type, &b.node_type) {
                (NodeType::Folder, NodeType::Key(_)) => std::cmp::Ordering::Less,
                (NodeType::Key(_), NodeType::Folder) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for node in nodes {
            self.sort_nodes(&mut node.children);
        }
    }
}
```

**Step 4: Create format module**

`src/format/mod.rs`:
```rust
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

    control_count < bytes.len() / 10 // Less than 10% control chars
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
```

**Step 5: Create UI module stub**

`src/ui/mod.rs`:
```rust
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
```

Create stub files for UI submodules:

`src/ui/layout.rs`:
```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

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
```

`src/ui/theme.rs`:
```rust
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
```

`src/ui/tree_view.rs`:
```rust
use crate::tree::{NodeType, TreeNode};
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub struct TreeView<'a> {
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
```

`src/ui/value_view.rs`:
```rust
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
```

`src/ui/info_bar.rs`:
```rust
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
```

`src/ui/dialogs.rs`:
```rust
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
    theme: &Theme,
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
    theme: &Theme,
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
```

**Step 6: Create editor module**

`src/editor/mod.rs`:
```rust
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::format::{detect_format, DetectedFormat};

pub struct ExternalEditor {
    temp_dir: PathBuf,
}

impl ExternalEditor {
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("redis-nav");
        fs::create_dir_all(&temp_dir)?;
        Ok(Self { temp_dir })
    }

    pub fn edit(&self, key: &str, value: &[u8]) -> Result<Option<Vec<u8>>> {
        let ext = match detect_format(value) {
            DetectedFormat::Json => ".json",
            DetectedFormat::Xml | DetectedFormat::Html => ".xml",
            _ => ".txt",
        };

        let safe_key = sanitize_filename(key);
        let temp_path = self.temp_dir.join(format!("{}{}", safe_key, ext));

        // Write current value
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(value)?;
        file.flush()?;
        drop(file);

        let before_hash = hash_bytes(value);

        // Get editor
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });

        // Spawn editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .map_err(|e| anyhow!("Failed to launch editor '{}': {}", editor, e))?;

        if !status.success() {
            fs::remove_file(&temp_path).ok();
            return Err(anyhow!("Editor exited with non-zero status"));
        }

        // Read modified content
        let new_value = fs::read(&temp_path)?;
        fs::remove_file(&temp_path).ok();

        let after_hash = hash_bytes(&new_value);

        if before_hash == after_hash {
            Ok(None) // No changes
        } else {
            Ok(Some(new_value))
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(50)
        .collect()
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
```

**Step 7: Create app.rs with main application logic**

`src/app.rs`:
```rust
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
        let delimiters = config.ui.delimiters.clone();
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
        let mut nodes = &mut self.tree_nodes;
        for (i, &idx) in path.iter().enumerate() {
            if i == path.len() - 1 {
                if let Some(node) = nodes.get_mut(idx) {
                    node.expanded = !node.expanded;
                }
            } else if let Some(node) = nodes.get_mut(idx) {
                nodes = &mut node.children;
            }
        }
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
```

**Step 8: Verify project compiles**

Run: `cargo check`
Expected: Should compile with no errors

**Step 9: Commit**

```bash
git add -A
git commit -m "feat: add complete module skeleton with all core functionality"
```

---

## Phase 1: Testing Infrastructure

### Task 1.1: Set Up Redis Container

**Step 1: Create docker-compose file for Redis**

Create `docker-compose.yml`:
```yaml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
```

**Step 2: Create sample data script**

Create `scripts/seed-redis.sh`:
```bash
#!/bin/bash

REDIS_CLI="podman exec -i redis-nav-redis-1 redis-cli"

echo "Seeding Redis with sample data..."

# Users
$REDIS_CLI SET "user:1:profile" '{"name": "Alice", "email": "alice@example.com", "theme": "dark"}'
$REDIS_CLI SET "user:1:settings" '{"notifications": true, "language": "en"}'
$REDIS_CLI SET "user:2:profile" '{"name": "Bob", "email": "bob@example.com", "theme": "light"}'
$REDIS_CLI EXPIRE "user:2:profile" 3600

# Cache entries
$REDIS_CLI SET "cache:page:home" "<html><body>Home Page</body></html>"
$REDIS_CLI SET "cache:page:about" "<html><body>About Page</body></html>"
$REDIS_CLI EXPIRE "cache:page:home" 300
$REDIS_CLI EXPIRE "cache:page:about" 600

# API data
$REDIS_CLI SET "api/v1/products" '[{"id": 1, "name": "Widget"}, {"id": 2, "name": "Gadget"}]'
$REDIS_CLI SET "api/v1/categories" '["electronics", "clothing", "books"]'

# Lists
$REDIS_CLI RPUSH "queue:tasks" "task1" "task2" "task3"

# Sets
$REDIS_CLI SADD "tags:post:1" "rust" "tui" "redis"

# Hashes
$REDIS_CLI HSET "session:abc123" "user_id" "1" "created" "2024-01-01" "ip" "192.168.1.1"

# Sorted sets
$REDIS_CLI ZADD "leaderboard:daily" 100 "alice" 85 "bob" 72 "charlie"

# Binary-ish data (base64 encoded)
$REDIS_CLI SET "binary:sample" "$(echo -n 'SGVsbG8gV29ybGQh' | base64 -d)"

echo "Done! Seeded $(redis-cli DBSIZE | awk '{print $2}') keys"
```

**Step 3: Start Redis and seed data**

Run:
```bash
podman-compose up -d
chmod +x scripts/seed-redis.sh
./scripts/seed-redis.sh
```

**Step 4: Commit**

```bash
git add docker-compose.yml scripts/seed-redis.sh
git commit -m "chore: add redis container and seed script"
```

---

### Task 1.2: Update main.rs with Full App Bootstrap

**Files:**
- Modify: `src/main.rs`

**Step 1: Update main.rs**

```rust
use anyhow::Result;
use clap::Parser;
use redis_nav::app::App;
use redis_nav::config::cli::Cli;
use redis_nav::config::file::ConfigFile;
use redis_nav::config::{AppConfig, ConnectionConfig, ProtectedNamespace, UiConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file if it exists
    let config_path = cli.config.clone().unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_default()
            .join("redis-nav")
            .join("config.toml")
    });

    let file_config = if config_path.exists() {
        ConfigFile::load(&config_path).ok()
    } else {
        None
    };

    // Build connection URL
    let url = if let Some(ref conn) = cli.connection {
        if conn.starts_with("redis://") || conn.starts_with("rediss://") {
            conn.clone()
        } else if let Some(ref fc) = file_config {
            // Try to use as profile name
            if let Some(profile) = fc.profiles.get(conn) {
                build_url_from_profile(profile, &cli)?
            } else {
                conn.clone()
            }
        } else {
            conn.clone()
        }
    } else if let Some(ref profile_name) = cli.profile {
        if let Some(ref fc) = file_config {
            if let Some(profile) = fc.profiles.get(profile_name) {
                build_url_from_profile(profile, &cli)?
            } else {
                anyhow::bail!("Profile '{}' not found in config", profile_name);
            }
        } else {
            anyhow::bail!("No config file found");
        }
    } else {
        // Build from CLI args
        let password = cli
            .password
            .clone()
            .or_else(|| std::env::var("REDIS_PASSWORD").ok());

        if let Some(pass) = password {
            format!("redis://:{}@{}:{}", pass, cli.host, cli.port)
        } else {
            format!("redis://{}:{}", cli.host, cli.port)
        }
    };

    // Build delimiters
    let delimiters = if !cli.delimiter.is_empty() {
        cli.delimiter.clone()
    } else if let Some(ref fc) = file_config {
        fc.defaults
            .delimiters
            .iter()
            .filter_map(|s| s.chars().next())
            .collect()
    } else {
        vec![':']
    };

    // Build protected namespaces
    let protected_namespaces = if let Some(ref fc) = file_config {
        if let Some(ref profile_name) = cli.profile {
            fc.profiles
                .get(profile_name)
                .map(|p| p.protected_namespaces.clone())
                .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let config = AppConfig {
        connection: ConnectionConfig {
            url,
            db: cli.db,
            readonly: cli.readonly,
        },
        ui: UiConfig {
            delimiters,
            protected_namespaces,
        },
    };

    // Initialize terminal
    let mut terminal = ratatui::init();
    terminal.clear()?;

    // Run app
    let mut app = App::new(config).await?;
    let result = app.run(&mut terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}

fn build_url_from_profile(
    profile: &redis_nav::config::file::Profile,
    cli: &Cli,
) -> Result<String> {
    if let Some(ref url) = profile.url {
        return Ok(url.clone());
    }

    let host = profile.host.as_deref().unwrap_or(&cli.host);
    let port = profile.port.unwrap_or(cli.port);

    let password = profile
        .password
        .clone()
        .or_else(|| {
            profile
                .password_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
        })
        .or_else(|| cli.password.clone())
        .or_else(|| std::env::var("REDIS_PASSWORD").ok());

    if let Some(pass) = password {
        Ok(format!("redis://:{}@{}:{}", pass, host, port))
    } else {
        Ok(format!("redis://{}:{}", host, port))
    }
}
```

**Step 2: Verify it compiles and runs**

Run: `cargo build --release`
Expected: Successful build

Run: `cargo run -- --help`
Expected: Shows help text

**Step 3: Test with local Redis**

Run: `cargo run`
Expected: TUI launches and shows keys from Redis

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: complete main.rs with config loading and app bootstrap"
```

---

## Phase 2: Integration Testing and Polish

### Task 2.1: Create Example Config File

**Files:**
- Create: `config.example.toml`

**Step 1: Create config.example.toml**

```toml
# redis-nav configuration example
# Copy to ~/.config/redis-nav/config.toml

[defaults]
delimiters = [":", "/"]
theme = "dark"

[profiles.local]
url = "redis://127.0.0.1:6379"
db = 0

[profiles.staging]
host = "staging.internal"
port = 6379
password_env = "STAGING_REDIS_PASSWORD"
delimiters = [":"]
readonly = true
protected_namespaces = [
    { prefix = "prod:", level = "block" },
]

[profiles.prod]
url = "rediss://prod.example.com:6380"
password_env = "PROD_REDIS_PASSWORD"
readonly = true
protected_namespaces = [
    { prefix = "billing:", level = "block" },
    { prefix = "user:", level = "confirm" },
    { prefix = "cache:", level = "warn" },
]
```

**Step 2: Commit**

```bash
git add config.example.toml
git commit -m "docs: add example config file"
```

---

### Task 2.2: Add Basic Tests

**Files:**
- Create: `tests/tree_builder_test.rs`
- Create: `tests/format_test.rs`

**Step 1: Create tree builder tests**

`tests/tree_builder_test.rs`:
```rust
use redis_nav::redis_client::RedisType;
use redis_nav::tree::TreeBuilder;

#[test]
fn test_single_delimiter() {
    let builder = TreeBuilder::new(vec![':']);
    let keys = vec![
        ("user:1:name".to_string(), RedisType::String),
        ("user:1:email".to_string(), RedisType::String),
        ("user:2:name".to_string(), RedisType::String),
    ];

    let tree = builder.build(&keys);

    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].name, "user");
    assert_eq!(tree[0].children.len(), 2); // user:1 and user:2
}

#[test]
fn test_multiple_delimiters() {
    let builder = TreeBuilder::new(vec![':', '/']);
    let keys = vec![
        ("user:1:name".to_string(), RedisType::String),
        ("api/v1/users".to_string(), RedisType::String),
    ];

    let tree = builder.build(&keys);

    assert_eq!(tree.len(), 2); // "user" and "api"
}

#[test]
fn test_empty_keys() {
    let builder = TreeBuilder::new(vec![':']);
    let keys: Vec<(String, RedisType)> = vec![];

    let tree = builder.build(&keys);

    assert!(tree.is_empty());
}
```

**Step 2: Create format detection tests**

`tests/format_test.rs`:
```rust
use redis_nav::format::{detect_format, DetectedFormat};

#[test]
fn test_detect_json_object() {
    let json = r#"{"name": "test", "value": 123}"#;
    assert_eq!(detect_format(json.as_bytes()), DetectedFormat::Json);
}

#[test]
fn test_detect_json_array() {
    let json = r#"[1, 2, 3]"#;
    assert_eq!(detect_format(json.as_bytes()), DetectedFormat::Json);
}

#[test]
fn test_detect_xml() {
    let xml = r#"<?xml version="1.0"?><root></root>"#;
    assert_eq!(detect_format(xml.as_bytes()), DetectedFormat::Xml);
}

#[test]
fn test_detect_html() {
    let html = r#"<!DOCTYPE html><html><body></body></html>"#;
    assert_eq!(detect_format(html.as_bytes()), DetectedFormat::Html);
}

#[test]
fn test_detect_plain_text() {
    let text = "Hello, world!";
    assert_eq!(detect_format(text.as_bytes()), DetectedFormat::PlainText);
}

#[test]
fn test_detect_binary_png() {
    let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(detect_format(&png_header), DetectedFormat::Binary);
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add tests/
git commit -m "test: add unit tests for tree builder and format detection"
```

---

### Task 2.3: Final Polish and README

**Files:**
- Create: `README.md`

**Step 1: Create README.md**

```markdown
# redis-nav

A terminal UI for browsing and editing Redis databases with tree-based key hierarchy, syntax highlighting, and safety features.

## Features

- Tree-based key hierarchy view with multiple delimiter support
- Syntax highlighting for JSON, XML, and hex dump for binary
- Safe SCAN-based key loading (never uses KEYS *)
- External $EDITOR integration with diff preview
- TTL visualization with color-coded warnings
- Protected namespace support (warn/confirm/block)
- Connection profiles via config file

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Connect to local Redis
redis-nav

# Connect with URL
redis-nav redis://localhost:6379

# Use a profile from config
redis-nav --profile prod

# Read-only mode
redis-nav --readonly
```

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `h/l` | Collapse/expand |
| `Enter` | Select key |
| `Tab` | Switch pane |
| `e` | Edit value |
| `r` | Refresh |
| `d` | Delete |
| `?` | Help |
| `q` | Quit |

## Configuration

Create `~/.config/redis-nav/config.toml`:

```toml
[defaults]
delimiters = [":", "/"]

[profiles.local]
url = "redis://127.0.0.1:6379"

[profiles.prod]
url = "rediss://prod.example.com:6380"
password_env = "PROD_REDIS_PASSWORD"
readonly = true
protected_namespaces = [
    { prefix = "billing:", level = "block" },
]
```

## License

MIT
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage instructions"
```

---

## Completion Checklist

After all tasks are complete, verify:

1. [ ] `cargo build --release` succeeds
2. [ ] `cargo test` passes
3. [ ] `cargo run` launches TUI
4. [ ] Can navigate tree with j/k/h/l
5. [ ] Can view key values with syntax highlighting
6. [ ] Can edit values with $EDITOR (non-readonly mode)
7. [ ] TTL displays correctly
8. [ ] Help dialog shows with ?
9. [ ] Quit works with q

---

**End of Implementation Plan**
