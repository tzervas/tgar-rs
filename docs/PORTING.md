# PORTING — tg-agent-relay → tgar-rs

Inventory map from upstream `tzervas/tg-agent-relay`. Phases match `plans/fractal/P22_TGAR_RS_DESIGN.md`.

**Transpile / gap honesty (P23):** see [TRANSPILE_RESEARCH.md](TRANSPILE_RESEARCH.md) for the py2rust assessment of Mycelium transpiler machinery (process only).

| Phase | Scope |
|-------|--------|
| **0** | Scaffold, CLI, config load, metrics append |
| **1** | Send path (Bot API outbound) |
| **2** | Poll path (getUpdates, allowlist, commands) |
| **3** | Routing, backends, sessions, chat/thread bind |
| **4** | Threads, plan approve, goal noise, stamps, comms format |
| **5** | TTS plain strip + engine spawn |
| **6** | Media download, usage ingest, dashboard (optional bridge) |
| **7** | Hook adapters → `tgar hook grok\|claude` |

---

## `tg_agent_relay/*.py`

| Source | Target crate / module | Phase | Notes |
|--------|----------------------|-------|-------|
| `__init__.py` | `tgar-core` (crate root re-exports) | 0 | Version surface; Python `0.10.x` ≠ Rust `0.1.0-dev` line |
| `config.py` | `tgar-core::config` | 0 | `relay.toml` + `.chats.d` overlay |
| `metrics.py` | `tgar-core::metrics` | 0 | TSV append to `.metrics.log` |
| `cli.py` | `tgar` binary subcommands | 0–7 | `version`, `hook`, `route`, `format`, … |
| `agent_handle.py` | `tgar-core::agent_handle` | 4 | py2rust candidate |
| `agent_stamp.py` | `tgar-core::agent_stamp` | 4 | py2rust candidate |
| `comms_format.py` | `tgar-core::comms_format` | 4 | partial py2rust |
| `goal_events.py` | `tgar-core::goal_events` | 4 | py2rust candidate |
| `plan_approve.py` | `tgar-core::plan_approve` | 4 | inline keyboards / approval flow |
| `format_api.py` | `tgar-core::format` | 1 | HTML parse_mode; ties to send |
| `protocols.py` | `tgar-core::protocols` | 1 | Send/format/usage protocol types |
| `send.py` | `tgar-telegram::send` + `tgar` `send` cmd | 1 | HTTP multipart, pagination, flock |
| `poll.py` | `tgar-telegram::poll` + `tgar` `poll` cmd | 2 | getUpdates loop, reassembly |
| `routing.py` | `tgar-core::routing` | 3 | `resolve()` pipe format |
| `threads.py` | `tgar-core::threads` | 4 | forum topics |
| `hooks.py` | `tgar-core::hooks` + `tgar hook` | 7 | provider hook dispatch |
| `tts.py` | `tgar-tts` + `tgar-core` config glue | 5 | spoken_mode short/full |
| `media_inbound.py` | `tgar-telegram::media_inbound` | 6 | download + routing |
| `highlight_docs.py` | `tgar-telegram` or sidecar | 6 | sendDocument HTML docs |
| `extensions.py` | `tgar-core::extensions` | 4 | `/ext` bus |
| `adk_bridge.py` | `tgar-core::adk` | 6 | optional ADK backend |
| `mcp_stub.py` | `tgar` MCP binary feature | 6 | stdio MCP tools |

---

## `lib/*.py` (companion to package)

| Source | Target | Phase | Notes |
|--------|--------|-------|-------|
| `toml_to_json.py` | `tgar-core::config::toml` | 0 | Replace with native TOML crate |
| `routing.py` | merge into `tgar-core::routing` | 3 | legacy duplicate of package routing |
| `sessions.py` | `tgar-core::sessions` | 3 | `.sessions.d` registry |
| `remote_config.py` | `tgar-core::remote_config` | 3 | allowlisted `/config` |
| `provider_catalog.py` | `tgar-core::providers::catalog` | 3 | preset backends |
| `provider_hook.py` | `tgar-core::providers::hook` | 7 | shared hook engine |
| `metrics_agg.py` | `tgar-core::metrics::agg` | 6 | dashboard input |
| `dashboard_render.py` | optional Python bridge or port | 6 | matplotlib |
| `usage_ingest.py` | `tgar-core::usage` | 6 | transcript walkers |
| `code_highlight.py` | `tgar-core::code_highlight` | 6 | pygments HTML doc |
| `context_render.py` | defer / sidecar | 6 | visual context experiment |
| `context_select.py` | defer / sidecar | 6 | |
| `tts_plain_text.py` | `tgar-tts::strip` | 5 | py2rust candidate |

---

## `providers/` (Python)

| Source | Target | Phase |
|--------|--------|-------|
| `base.py` | `tgar-core::providers::traits` | 7 |
| `claude/hooks.py`, `claude/usage.py` | `tgar-core::providers::claude` | 7 / 6 |
| `grok/hooks.py`, `grok/usage.py` | `tgar-core::providers::grok` | 7 / 6 |
| `ollama/usage.py` | `tgar-core::providers::ollama` | 6 |
| `openai/usage.py` | `tgar-core::providers::openai` | 6 |
| `adk/`, `aider/`, `gemini/`, `generic/` | same names under `providers` | 3–6 |

---

## Major shell entrypoints

| Script | Target | Phase | Notes |
|--------|--------|-------|-------|
| `tg-send.sh` | `tgar send` | 1 | format, TTS, flock, pagination |
| `tg-poll.sh` | `tgar poll` | 2 | allowlist, commands, routing emit |
| `relay-notify.sh` | `tgar notify` | 1 | generic prefix/format |
| `go-live.sh` | `tgar daemon` or compose script | 2 | poll supervisor |
| `watch-go-live.sh` | ops script (thin) | 2 | |
| `hook-notify.sh` | `relay-notify` wrapper | 7 | |
| `hook-notify-grok.sh` | grok-specific notify | 7 | |
| `install-hooks.sh` | `tgar hooks install claude` | 7 | settings.json sync |
| `install-grok-hooks.sh` | `tgar hooks install grok` | 7 | |
| `fetch-voices.sh` | `tgar tts fetch-voices` | 5 | piper ONNX download |

### `adapters/*.sh`

| Script | Target | Phase |
|--------|--------|-------|
| `claude-code.sh` | `tgar hook claude` (stdin JSON) | 7 |
| `grok.sh` | `tgar hook grok` | 7 |
| `generic-example.sh` | docs only | 7 |
| `backend-fifo-reader.sh` | `tgar fifo read` | 3 |

### `handlers/*.sh` (relay `mode = "relay"`)

| Script | Target | Phase |
|--------|--------|-------|
| `dashboard.sh` | `tgar cmd dashboard` | 6 |
| `stats.sh` | `tgar cmd stats` | 6 |
| `usage.sh` | `tgar cmd usage` | 6 |
| `help.sh` | `tgar cmd help` | 2 |
| `uptime.sh` | `tgar cmd uptime` | 2 |
| `config.sh` | `tgar cmd config` | 3 |
| `project.sh` | `tgar cmd project` | 3 |
| `thread.sh` | `tgar cmd thread` | 4 |
| `ext.sh` | `tgar cmd ext` | 4 |
| `example-echo.sh` | test fixture | — |

### `lib/*.sh` (shared shell)

| Script | Target | Phase |
|--------|--------|-------|
| `relay-config.sh` | absorbed into `tgar-core::config` | 0 |
| `relay-common.sh` | `tgar-core::template` | 1 |
| `format.sh` | `format_api` port | 1 |
| `code_highlight.sh` | code_highlight port | 6 |
| `tts.sh` | `tgar-tts` spawn | 5 |
| `routing.sh` | routing CLI glue | 3 |
| `python.sh` / `python_fallback.sh` | removed at cutover | 7 |
| `agent_handle.sh` / `agent_stamp.sh` | Rust core | 4 |
| `comms_format.sh` | comms_format | 4 |
| `claude-code-events.sh` / `grok-events.sh` | provider catalogs | 7 |
| `exec-env.sh` | `tgar-core::env` | 0 |

### `scripts/` (ops)

| Script | Target | Phase |
|--------|--------|-------|
| `register-session.sh` / `unregister-session.sh` | `tgar session register` | 3 |
| `list-sessions.sh` | `tgar session list` | 3 |
| `ensure-inbound.sh` | deploy helper | 2 |
| `deploy-local.sh` / `dev.sh` | dev ergonomics | 0 |
| `local-ci.sh` | mirror GitHub CI | 0 |
| `release.sh` | release automation | post-parity |

---

## Tests upstream

Python tests under `tests/test_*.py` become crate `#[cfg(test)]` + integration tests in `tgar-rs/tests/` per phase; golden fixtures copied from `tests/fixtures/` as needed (no secrets).
