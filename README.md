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
redis-nav redis://localhost:6380

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
url = "redis://127.0.0.1:6380"

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
