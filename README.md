# tgar-rs

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/tgar-rs/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/tgar-rs/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/tgar-rs/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/tgar-rs/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

**tgar** — Rust implementation of [TG Agent Relay](https://github.com/tzervas/tg-agent-relay) (iterative port; short name **tgar**).

**Who / what / why:** operators who already run the Python/shell relay and want a single static binary path for outbound Telegram send (and later poll/hooks), without rewriting bridge install paths. This repo is the strangler replacement behind optional `RELAY_IMPL=rust` (see `docs/STRANGLER.md`).

## Honest product status

| Surface | Python SoT (`tg-agent-relay`) | Rust (`tgar-rs`) today |
|---------|-------------------------------|-------------------------|
| Version | shipping **0.10.x** | **0.1.0-dev** (`tgar version`) |
| Outbound send | `tg-send.sh` / `send.py` (live) | `tgar send` — dry-run without `BOT_TOKEN`; live `sendMessage` + pagination when token set |
| Config | full `relay.toml` + overlays | `tgar config check` — TOML parse + table list + `page_size` (schema stub) |
| Poll / route / hooks / TTS | full product | **not shipped** (see `docs/PORTING.md`) |
| Operator default | **`RELAY_IMPL=python`** | opt-in only after dual-run green per surface |

**Shipping SoT remains Python** until a surface’s Phase exit criteria pass. Do not claim production cutover for poll/hooks yet.

## 5-minute path (offline)

```bash
git clone https://github.com/tzervas/tgar-rs.git
cd tgar-rs
cargo build -p tgar
cargo test --workspace

./target/debug/tgar version
# → 0.1.0-dev

./target/debug/tgar send --text "Hello" --dry-run
# → Hello

./target/debug/tgar config check --path fixtures/relay.minimal.toml
# → ok: fixtures/relay.minimal.toml (tables=[format, general]; page_size=3500; …)

./scripts/dual-run-smoke.sh
# → dual-run-smoke: green (offline Rust path)
```

MSRV is pinned in `rust-toolchain.toml` (1.96, aligned with tg-agent-relay).

## Crates

| Crate | Role |
|-------|------|
| `tgar` | CLI binary (`version`, `config check`, `send`) |
| `tgar-core` | Config stub, stamps, goals, pure ports |
| `tgar-telegram` | Telegram Bot API client + pagination |
| `tgar-tts` | Prose strip + piper/espeak spawn (optional; Phase 5) |

## Porting accelerator

Pure-Python modules may be assisted with [py2rust](https://github.com/tzervas/py2rust) during porting. **py2rust is not required to build or run tgar-rs.**

See `docs/PORTING.md` for the module map, `docs/COMPAT.md` for `relay.toml` keys, `docs/STRANGLER.md` for cutover, and `docs/RELEASE.md` for the path to **0.1.0**.

## Telegram credentials

**Live** `sendMessage` requires:

- `BOT_TOKEN` — bot token (never commit; use `.env` or private overlay)
- `ALLOWED_CHAT_ID` or `tgar send --chat-id` — destination chat

Without `BOT_TOKEN`, `tgar send` **dry-runs**: prints paginated pages only (no network).

```bash
tgar send --text "Hello"
tgar send --text "..." --page-size 3500
tgar send --text "..." --dry-run
```

## License

MIT — same as tg-agent-relay.
