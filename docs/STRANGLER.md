# Strangler migration — Python relay → `tgar` (Rust)

**Upstream product (source of truth today):** [tg-agent-relay](https://github.com/tzervas/tg-agent-relay)  
**Rust implementation:** this repo (`tgar-rs`)  
**Design anchor:** [tg-agent-relay #22](https://github.com/tzervas/tg-agent-relay/issues/22) · [docs/TGAR_RS.md](https://github.com/tzervas/tg-agent-relay/blob/main/docs/TGAR_RS.md)

The strangler pattern ships **one vertical slice at a time**: each phase adds a
`tgar` subcommand that can stand in for the matching shell/Python entrypoint.
Operators keep running the same paths (`tg-send.sh`, `tg-poll.sh`, hooks) until
they opt into Rust per surface.

---

## `RELAY_IMPL` — implementation switch (planned)

| Value | Meaning |
|-------|---------|
| `python` | **Default** until a phase is marked stable. Shell wrappers exec the Python package (`tg_agent_relay`) when import succeeds; shell body remains recovery (see upstream `docs/DECISIONS.md` D1). |
| `rust` | Shell wrappers exec `tgar` for subcommands that have reached parity for that phase. Missing binary or unsupported subcommand → **fail loud** in CI/smoke; in production hooks, fall back to `python` with a metric line (never silent cutover). |

**Scope:** `RELAY_IMPL` is the **Rust vs Python** selector at the stable wrapper boundary.
It does **not** replace the existing Python vs shell toggles:

| Env | Today (Python path) |
|-----|---------------------|
| `RELAY_PYTHON_SEND=0` | Force shell send body in `tg-send.sh` |
| `RELAY_PYTHON_POLL=0` | Force shell poll body in `tg-poll.sh` |

When `RELAY_IMPL=rust`, wrappers skip Python import for wired subcommands and call
`tgar` directly. When `RELAY_IMPL=python` (default), behavior matches current
releases.

**Wiring timeline:** Phase 0–1 land `lib/relay-impl.sh` (upstream) + `tgar version` /
`tgar config check`. Send/poll wrappers gain the branch when Phase 1–2 exit criteria
pass (dual-run green on fixtures).

---

## Entrypoint map (target)

| Operator surface | Python / shell today | `tgar` subcommand (by phase) |
|------------------|----------------------|------------------------------|
| Outbound send | `tg-send.sh` → `tg_agent_relay.send` | **P1** `tgar send` |
| Inbound poll | `tg-poll.sh` → `tg_agent_relay.poll` | **P2** `tgar poll` |
| Hook notify | `relay-notify.sh` / adapters | **P7** `tgar hook <provider>` |
| Route resolve | `tg_agent_relay.cli route` | **P3** `tgar route` |
| Format HTML | `tg_agent_relay.cli format` | **P1** `tgar format` (parity with `format_api`) |
| Config validate | — | **P0** `tgar config check` |

Hook installs and `~/.claude/telegram-bridge/` paths **stay unchanged**; only the
process behind the wrapper swaps.

---

## Phase gates (when `RELAY_IMPL=rust` is allowed per surface)

| Phase | Stable `RELAY_IMPL=rust` for | Exit criteria |
|-------|------------------------------|---------------|
| 0 | `config check`, `version` only | `cargo test` green; config + metrics append |
| 1 | `send`, `format` | Mock HTTP send tests; dual-run format golden |
| 2 | `poll` | Fixture allowlist + command tags match Python |
| 3 | `route` | Pipe output parity with `cli route` |
| 4–7 | threads, TTS, media, hooks | Epic checklist in `docs/PORTING.md` |

Default remains **`RELAY_IMPL=python`** for all operator paths until the row for
that surface is explicitly checked in release notes.

---

## Dual-run smoke

`scripts/dual-run-smoke.sh` compares **Python CLI** vs **`tgar`** on the same stdin/fixtures
without calling Telegram (offline-safe).

```bash
# From tgar-rs repo root (expects tg-agent-relay checkout as sibling or TGAR_RELAY_ROOT)
export TGAR_RELAY_ROOT="${TGAR_RELAY_ROOT:-../tg-agent-relay}"
export RELAY_IMPL=python   # smoke always exercises both sides explicitly

./scripts/dual-run-smoke.sh
```

**Checks (expand as phases land):**

1. **`format`** — golden text → HTML body + `parse_mode` stderr line.
2. **`hook grok`** — synthetic JSON fixture → `OK:` / `SKIP:` prefix line.
3. **`route`** — test `relay.toml` JSON → pipe-delimited `RouteResult`.

On mismatch, the script prints a unified diff and exits non-zero. CI can run it
with `TGAR_SKIP_RUST=1` until the `tgar` binary exists (Python-only baseline).

---

## Rollback

1. Set `RELAY_IMPL=python` (or unset) in the bridge `.env` / systemd unit.
2. Restart poll daemon if running.
3. Confirm `.metrics.log` shows `python` path resumes (no `rust_impl` lines).

No config migration required — `relay.toml` and `.env` shapes stay compatible
(`docs/COMPAT.md` when published).

---

## Non-goals

- Big-bang rename of the product or hook paths.
- Replacing matplotlib dashboards in v0 (optional Python sidecar).
- Duplicating feature work in shell after Python default (#67); Rust supersedes
  shell bodies per phase, not a third implementation forever.