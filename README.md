# tgar-rs

**tgar** — Rust implementation of [TG Agent Relay](https://github.com/tzervas/tg-agent-relay) (iterative port; short name **tgar**).

The Python/shell product remains the shipping SoT until module parity; this repo is the strangler replacement behind optional `RELAY_IMPL=rust` (documented in P22d).

## Status

Phase **0** scaffold: workspace crates, `tgar version`, docs inventory (`docs/PORTING.md`, `docs/COMPAT.md`).

## Build

```bash
cargo build -p tgar
cargo test --workspace
./target/debug/tgar version   # 0.1.0-dev
```

MSRV is pinned in `rust-toolchain.toml` (1.96, aligned with tg-agent-relay).

## Crates

| Crate | Role |
|-------|------|
| `tgar` | CLI binary |
| `tgar-core` | Config, routing, metrics, stamps, goals, plans |
| `tgar-telegram` | Telegram Bot API client |
| `tgar-tts` | Prose strip + piper/espeak spawn (optional; Phase 5) |

## Porting accelerator

Pure-Python modules may be assisted with [py2rust](https://github.com/tzervas/py2rust) during porting. **py2rust is not required to build or run tgar-rs.**

See `docs/PORTING.md` for the module map and `docs/COMPAT.md` for `relay.toml` compatibility targets.

## License

MIT — same as tg-agent-relay.
