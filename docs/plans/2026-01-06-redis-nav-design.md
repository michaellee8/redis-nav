# redis-nav Design Document

**Date:** 2026-01-06
**Status:** Approved
**Scope:** Phase 1 (Viewer MVP) + Phase 2 (Editor)

## Overview

redis-nav is a terminal UI application for browsing and editing Redis databases with a tree-based key hierarchy view, syntax highlighting, and safety features for production use.

## Tech Stack

| Dependency | Purpose | GitHub Stars |
|------------|---------|--------------|
| tokio | Async runtime | ~30.7k |
| ratatui | TUI framework | ~17k |
| clap | CLI argument parser | ~15.9k |
| serde | Serialization | ~10.3k |
| serde_json | JSON support | ~5.4k |
| redis-rs | Redis client | ~4.1k |
| crossterm | Terminal backend | ~3.8k |
| toml | Config file parsing | ~700+ |
| syntect | Syntax highlighting | ~1.6k |

## Project Structure

```
redis-nav/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing, app bootstrap
│   ├── app.rs               # Main App struct, event loop, state machine
│   ├── config/
│   │   ├── mod.rs
│   │   ├── cli.rs           # clap CLI argument definitions
│   │   └── file.rs          # TOML config file parsing
│   ├── redis/
│   │   ├── mod.rs
│   │   ├── client.rs        # Async Redis connection wrapper
│   │   ├── scanner.rs       # SCAN-based key iterator
│   │   └── types.rs         # Redis value type handling
│   ├── tree/
│   │   ├── mod.rs
│   │   ├── node.rs          # TreeNode struct with lazy loading
│   │   └── builder.rs       # Key-to-tree hierarchy builder
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs        # 3-pane layout (tree, value, info)
│   │   ├── tree_view.rs     # Tree widget with expand/collapse
│   │   ├── value_view.rs    # Syntax-highlighted value display
│   │   ├── info_bar.rs      # TTL, type, size info panel
│   │   ├── dialogs.rs       # Confirmation dialogs, editor preview
│   │   └── theme.rs         # Color scheme definitions
│   ├── format/
│   │   ├── mod.rs
│   │   ├── detect.rs        # Auto-detect JSON/XML/binary
│   │   └── highlight.rs     # Syntax highlighting spans
│   └── editor/
│       ├── mod.rs
│       └── external.rs      # $EDITOR integration with temp files
├── config.example.toml
└── tests/
    └── integration/
```

## Architecture

### Elm-inspired Unidirectional Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                         App                                  │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │  State  │───▶│  View   │───▶│ Terminal│───▶│ Screen  │  │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘  │
│       ▲                                                      │
│       │                                                      │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│  │ Update  │◀───│ Message │◀───│  Event  │                  │
│  └─────────┘    └─────────┘    └─────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

### Message Types

```rust
enum Message {
    // Navigation
    TreeNavigate(Direction),
    TreeExpand,
    TreeCollapse,

    // Data loading (async results)
    KeysLoaded(Vec<String>),
    ValueLoaded(String, RedisValue),
    LoadError(String),

    // Editing
    EditRequested,
    EditorClosed(Option<String>),  // None = cancelled
    ConfirmWrite(bool),

    // UI
    SwitchPane(Pane),
    Resize(u16, u16),
    Quit,
}
```

### Async Channel Design

- UI thread owns terminal, renders at 30fps
- Redis operations run on separate tokio tasks
- Communication via `mpsc` channels - UI never blocks on Redis
- Backpressure: pause SCAN if tree has >10k pending nodes

## Configuration

### CLI Arguments

```
redis-nav [OPTIONS] [CONNECTION_URL]

Arguments:
  [CONNECTION_URL]    Redis URL (redis://host:port) or profile name

Options:
  -h, --host <HOST>         Redis host [default: 127.0.0.1]
  -p, --port <PORT>         Redis port [default: 6379]
  -a, --password <PASS>     Redis password (or use REDIS_PASSWORD env)
  -n, --db <DB>             Database number [default: 0]
  -d, --delimiter <DELIM>   Key delimiters [default: ":"]  (repeatable)
  --profile <NAME>          Use named profile from config
  --readonly                Disable all write operations
  --config <PATH>           Config file path [default: ~/.config/redis-nav/config.toml]
```

### Config File (`~/.config/redis-nav/config.toml`)

```toml
[defaults]
delimiters = [":", "/"]
theme = "dark"

[profiles.local]
url = "redis://127.0.0.1:6379"
db = 0

[profiles.staging]
url = "redis://staging.internal:6379"
password_env = "STAGING_REDIS_PASSWORD"
delimiters = [":"]
readonly = true

[profiles.prod]
url = "rediss://prod.example.com:6380"  # TLS
password_env = "PROD_REDIS_PASSWORD"
readonly = true
protected_namespaces = [
    { prefix = "billing:", level = "block" },
    { prefix = "user:", level = "confirm" },
    { prefix = "cache:", level = "warn" },
]

[theme.dark]
tree_selected = { fg = "black", bg = "cyan" }
key_prefix = { fg = "blue" }
ttl_expiring = { fg = "red", bold = true }
```

### Protection Levels

- `warn` - Yellow banner, any key continues
- `confirm` - Red modal, must type "yes" to proceed
- `block` - Operation denied entirely, no override

## Tree Structure

### TreeNode

```rust
struct TreeNode {
    name: String,              // Display name (segment after last delimiter)
    full_key: Option<String>,  // Some = leaf (actual Redis key), None = virtual folder
    node_type: NodeType,
    children: Vec<TreeNode>,
    expanded: bool,
    loaded: bool,              // For lazy loading
}

enum NodeType {
    Folder,                    // Virtual grouping node
    Key(RedisType),            // Actual Redis key with type
}

enum RedisType {
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    Unknown,
}
```

### Tree Building

Given keys and delimiters `[":", "/"]`:

```
Input keys:
  user:123:profile
  user:123:settings
  cache/images/thumb

Output tree:
  user                    [Folder]
  ├── 123                 [Folder]
  │   ├── profile         [Key: String]
  │   └── settings        [Key: Hash]
  cache                   [Folder]
  └── images              [Folder]
      └── thumb           [Key: String]
```

### Lazy Loading Strategy

1. Initial SCAN loads first 1000 keys, builds partial tree
2. Tree nodes show `[+]` if potentially has more children
3. Expanding a folder triggers targeted `SCAN pattern:*` if needed
4. Background task continues full SCAN, merges results incrementally

## Value Viewer

### Format Detection

```rust
enum DetectedFormat {
    Json,
    Xml,
    Html,
    Binary,
    PlainText,
}

fn detect_format(bytes: &[u8]) -> DetectedFormat {
    // 1. Check for non-UTF8 or control chars -> Binary
    // 2. Try JSON parse (serde_json::from_slice)
    // 3. Check for XML/HTML markers (<?xml, <!DOCTYPE, <html)
    // 4. Default to PlainText
}
```

### Syntax Highlighting

- JSON: keys blue, strings green, numbers yellow, booleans magenta
- XML: tags cyan, attributes yellow, values green
- Binary: hex dump with ASCII sidebar

### TTL Display

- `TTL: -1 (no expiry)` - grey
- `TTL: 86400s (24h)` - green
- `TTL: 300s (5m)` - yellow
- `TTL: 30s` - red, blinking

## Editor Integration

### Edit Workflow

1. User presses 'e' on a key
2. Check protection level -> warn/confirm/block
3. Create temp file with current value
4. Spawn $EDITOR (fallback: vi -> nano -> notepad)
5. Wait for editor to close
6. Read temp file, detect if changed
7. Show diff preview modal
8. User confirms -> write to Redis
9. Cleanup temp file

### Diff Preview Modal

```
┌─ Confirm Changes to user:123:profile ────────────────────────┐
│                                                              │
│  - "theme": "dark",                                         │
│  + "theme": "light",                                        │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│         [Enter] Write to Redis    [Esc] Cancel              │
└──────────────────────────────────────────────────────────────┘
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down in tree |
| `k` / `Up` | Move up in tree |
| `h` / `Left` | Collapse node / go to parent |
| `l` / `Right` / `Enter` | Expand node / select key |
| `g` | Go to first item |
| `G` | Go to last item |
| `Tab` | Switch pane (tree / value) |
| `/` | Search/filter keys |
| `n` | Next search match |
| `N` | Previous search match |

### Actions

| Key | Action |
|-----|--------|
| `e` | Edit selected key (opens $EDITOR) |
| `r` | Refresh current key value |
| `R` | Refresh entire tree (re-scan) |
| `y` | Yank (copy) key name to clipboard |
| `Y` | Yank value to clipboard |
| `d` | Delete key (with confirmation) |
| `?` | Show help overlay |
| `q` / `Ctrl+C` | Quit |

### Value Pane

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Ctrl+D` | Page down |
| `Ctrl+U` | Page up |
| `0` | Scroll to top |
| `$` | Scroll to bottom |

## Testing Strategy

- Unit tests for tree building, format detection
- Integration tests with Redis container (podman)
- Mock mode for UI development without Redis
