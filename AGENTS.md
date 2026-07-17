# AGENTS.md — tgar-rs

Agent-facing rules for the Rust TG Agent Relay port.

## Product boundary

- **Python SoT:** [tg-agent-relay](https://github.com/tzervas/tg-agent-relay) remains the shipping product until strangler phase exit criteria pass.
- **This repo:** iterative Rust replacement (`tgar` CLI). Never invent poll/hook parity that is not in tree.
- **Do not touch mycelium** for this workstream.

## Local checks (must stay green)

```bash
cargo test --workspace
cargo build -p tgar
./target/debug/tgar version
./target/debug/tgar send --text "smoke" --dry-run
./target/debug/tgar config check --path fixtures/relay.minimal.toml
./scripts/dual-run-smoke.sh
```

Live Telegram sends need `BOT_TOKEN` + chat id — never commit secrets.

## Branch / PR

- Work on a feature/chore branch (not bare `main`).
- Prefer small strangler slices: one subcommand or pure module + tests.
- Document honest status in README (shipped vs Python SoT).

## Key docs

| Doc | Use |
|-----|-----|
| `README.md` | Product status + 5-minute path |
| `docs/PORTING.md` | Module → crate map by phase |
| `docs/STRANGLER.md` | `RELAY_IMPL` cutover |
| `docs/COMPAT.md` | `relay.toml` key phases |
| `docs/TELEGRAM.md` | send / credentials |
| `docs/RELEASE.md` | Path to tagged 0.1.0 |
| `CLAUDE.md` | Short CLI cheat-sheet for Claude |

## Next slices (small)

1. Expand `config` (overlays, defaults) without full Python parity claim.
2. Dual-run golden for format when `tgar format` lands.
3. Poll fixture allowlist (Phase 2) only after send dual-run is solid.
