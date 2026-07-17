# Release path — tgar-rs → 0.1.0

**Current version:** `0.1.0-dev` (workspace `Cargo.toml`)  
**Upstream SoT:** [tg-agent-relay](https://github.com/tzervas/tg-agent-relay) (Python/shell, shipping 0.10.x)

This document is the honest checklist for the first **tagged** Rust release. Until every gate below is green, keep the `-dev` suffix and default operators on `RELAY_IMPL=python`.

## What 0.1.0 means

A **usable outbound slice**, not full product parity:

| Included in 0.1.0 | Explicitly **out** of 0.1.0 |
|-------------------|----------------------------|
| `tgar version` | Poll / getUpdates loop |
| `tgar send` dry-run + live `sendMessage` (paginated) | Route / backends / sessions |
| Mock HTTP unit tests for send | Hooks / TTS / media / dashboard |
| `tgar config check` (TOML parse + basic tables) | Full `relay.toml` schema validation |
| Offline dual-run smoke for version/send/config | Production cutover as default |

## Gates (all required to tag)

1. **Tests:** `cargo test --workspace` green on CI (`fleet-ci`) and locally.
2. **Send:** dry-run without `BOT_TOKEN`; live path covered by mock HTTP tests; optional manual live smoke with private token (not in CI).
3. **Config:** `tgar config check` succeeds on `fixtures/relay.minimal.toml` and at least one real `relay.toml.example` shape from upstream.
4. **Dual-run:** `./scripts/dual-run-smoke.sh` green offline; document `TGAR_RELAY_ROOT` when comparing Python.
5. **Docs:** README status table accurate; `docs/STRANGLER.md` + this file list what is / is not cutover-safe.
6. **Secrets:** no tokens in tree; `docs/TELEGRAM.md` credentials section current.
7. **Version bump:** set workspace version to `0.1.0` (drop `-dev`); `tgar version` prints `0.1.0`.
8. **Tag + notes:** `git tag -a v0.1.0 -m "tgar-rs 0.1.0 — send + config check"`; GitHub Release with changelog excerpt.

## Suggested changelog skeleton (fill at tag time)

```markdown
## 0.1.0 — YYYY-MM-DD

### Added
- `tgar send` with pagination and dry-run
- `tgar config check` TOML parse stub
- `tgar-telegram` mock HTTP tests
- Offline dual-run smoke script

### Not yet
- poll, route, hooks, TTS, media
- RELAY_IMPL=rust as default
```

## After 0.1.0

- Phase 2+: poll dual-run → 0.2.x line  
- Keep Python SoT until each surface’s strangler row is checked (see `docs/STRANGLER.md`).  
- crates.io publish is **optional**; GH release notes are the minimum.

## Rollback

Operators unset `RELAY_IMPL` / set `python` — no config migration. See `docs/STRANGLER.md` § Rollback.
