# CLAUDE.md — tgar-rs

Short operator notes for Claude / coding agents.

## Commands

```bash
# Tests
cargo test --workspace

# Build CLI
cargo build -p tgar

# Version
./target/debug/tgar version          # 0.1.0-dev

# Outbound send — offline dry-run (no BOT_TOKEN needed)
./target/debug/tgar send --text "hello" --dry-run
# or: BOT_TOKEN unset implies dry-run
./target/debug/tgar send --text "hello"

# Config parse stub
./target/debug/tgar config check --path fixtures/relay.minimal.toml

# Offline dual-run smoke
./scripts/dual-run-smoke.sh
```

## Layout

- `crates/tgar` — CLI (`version`, `config check`, `send`)
- `crates/tgar-core` — pure logic + config stub
- `crates/tgar-telegram` — Bot API + pagination (mock HTTP tests)
- `crates/tgar-tts` — Phase 5 placeholder

## Rules

1. Python `tg-agent-relay` is SoT until a phase is explicitly dual-run green.
2. No secrets in git; live send only with env credentials.
3. Prefer tests that need no network.
4. See `AGENTS.md` for product boundary and `docs/RELEASE.md` for 0.1.0 gates.
