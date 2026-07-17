# tgar-rs

Rust implementation of [TG Agent Relay](https://github.com/tzervas/tg-agent-relay) (strangler port).

## Workspace

| Crate | Role |
|-------|------|
| `tgar` | CLI binary |
| `tgar-core` | Config, metrics, routing (P22b+) |
| `tgar-telegram` | Bot API client, pagination |

## Build & test

```bash
cargo test
cargo build -p tgar
```

## Telegram credentials

**Live** `sendMessage` calls require a bot token in the environment:

- `BOT_TOKEN` — Telegram Bot API token (never commit; use `.env` or private overlay per relay docs)
- `ALLOWED_CHAT_ID` — default destination chat for `tgar send` (or pass `--chat-id`)

Without `BOT_TOKEN`, `tgar send` runs in **dry-run** mode and prints paginated pages to stdout (no network).

## CLI

```bash
# Version
tgar version

# Dry-run pagination (no token required)
tgar send --text "Hello from tgar-rs"

# Live send (token + chat required)
export BOT_TOKEN=...
export ALLOWED_CHAT_ID=-100...
tgar send --text "Hello"
```

Options:

- `--page-size` — default `3500` (matches `relay.toml` `[general].page_size`)
- `--dry-run` — force print-only even if `BOT_TOKEN` is set

## Status

Phase P22c: `sendMessage` (url-encoded POST), pagination helpers, mock HTTP tests. See `docs/PORTING.md` and `plans/fractal/P22_TGAR_RS_DESIGN.md` in the homelab plans tree.