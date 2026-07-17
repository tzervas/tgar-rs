# COMPAT — `relay.toml` keys (target support)

Keys listed from upstream `relay.toml.example`. **Phase** = earliest tgar-rs phase we aim to honor. Until that phase ships, Python/shell remains authoritative (`RELAY_IMPL=python`).

Legend: **0** foundation · **1** send · **2** poll · **3** routing · **4** threads/features · **5** TTS · **6** usage/media/dashboard · **7** hooks/providers

## `[general]`

| Key | Phase | Notes |
|-----|-------|-------|
| `page_size` | 1 | Telegram pagination |
| `page_delay` | 1 | |
| `reassemble_window` | 2 | inbound burst merge |
| `hook_max_pages` | 1 | |
| `send_interval_ms` | 1 | flock ordering gap |

## `[format]`

| Key | Phase | Notes |
|-----|-------|-------|
| `enabled` | 1 | default-on in Python |
| `parse_mode` | 1 | HTML / none |
| `wrap_width` | 1 | |
| `headers` | 1 | |
| `code_spans` | 1 | |
| `blockquotes` | 1 | |
| `soft_wrap` | 1 | |

## `[code_highlight]`

| Key | Phase | Notes |
|-----|-------|-------|
| `mode` | 6 | inline-only / html-doc |
| `theme` | 6 | |
| `line_numbers` | 6 | |
| `max_lines` | 6 | |
| `keep_text` | 6 | caption mode |
| `myc_inline_lang` | 6 | |

## `[generic]`

| Key | Phase | Notes |
|-----|-------|-------|
| `prefix` | 1 | relay-notify |
| `format` | 1 | template |

## `[claude_code.<Event>]`

| Keys per event | Phase | Notes |
|----------------|-------|-------|
| `enabled`, `prefix`, `format` | 7 | all documented Claude hook events |

## `[commands.<name>]`

| Key | Phase | Notes |
|-----|-------|-------|
| `keyword` | 2 | |
| `slash` | 2 | |
| `tag` | 2 | forward mode |
| `mode` | 2 | forward / relay |
| `handler` | 2 | relay mode script path |

Built-in command tables in example: `dashboard`, `stats`, `uptime`, `help`, `usage`, `project`, `thread`, `config`, `ext`, `example` — **2–6** depending on handler.

## `[tts]`

| Key | Phase | Notes |
|-----|-------|-------|
| `mode` | 5 | off / text+voice / voice-only |
| `engine` | 5 | auto / piper / espeak |
| `voice_model` | 5 | |
| `pitch` | 5 | |
| `length_scale` | 5 | piper |
| `max_chars` | 5 | |
| `hook_voice` | 5 | |
| `spoken_mode` | 5 | short / full |
| `spoken_max_chars` | 5 | |
| `clip_max_chars` | 5 | |
| `hook_voice_max_chars` | 5 | alias |
| `speak_code` | 5 | |
| `voice_code_ref` | 5 | |
| `voice_link_ref` | 5 | |
| `collapse_adjacent_refs` | 5 | |

## `[dashboard]`

| Key | Phase | Notes |
|-----|-------|-------|
| `window_hours` | 6 | |

## `[usage]`

| Key | Phase | Notes |
|-----|-------|-------|
| `enabled` | 6 | opt-in |
| `source` | 6 | claude-code / grok / multi |
| `projects_dir` | 6 | |
| `window` | 6 | |
| `providers` | 6 | display toggle |
| `models` | 6 | display toggle |
| `charts.default` | 6 | |

## `[usage.allotments]` / `[usage.allotments.<subject>]`

| Key | Phase | Notes |
|-----|-------|-------|
| `daily`, `weekly`, `monthly` | 6 | caps per subject |
| dotted keys `subject.period` | 6 | alternate form |

## `[threads]`

| Key | Phase | Notes |
|-----|-------|-------|
| `enabled` | 4 | |
| `auto_create` | 4 | |
| `max_creates_per_hour` | 4 | |

## `[threads.platform_chats]`

| Key | Phase | Notes |
|-----|-------|-------|
| `grok`, `claude`, `codex`, `self_hosted` | 4 | platform chat ids |

## `[config.remote]`

| Key | Phase | Notes |
|-----|-------|-------|
| `allow` | 3 | extra allowlisted keys |

## `[grok.<Event>]`

| Keys per event | Phase | Notes |
|----------------|-------|-------|
| `enabled`, `prefix`, `format`, `matcher` | 7 | Grok hook catalog |

## `[routing]`

| Key | Phase | Notes |
|-----|-------|-------|
| `default_backend` | 3 | |
| `require_prefix` | 3 | |
| `tag_style` | 3 | bracket / bare / none |

## `[backends.<name>]`

| Key | Phase | Notes |
|-----|-------|-------|
| `type` | 3 | claude-code, grok, ollama, openai, … |
| `delivery` | 3 | stdout / fifo / cmd |
| `tag` | 3 | |
| `prefixes` | 3 | |
| `project` | 3 | |
| `fifo` | 3 | |
| `model` | 3 | cmd backends |
| `cmd` | 3 | |

## `[sessions]`

| Key | Phase | Notes |
|-----|-------|-------|
| `dir` | 3 | `.sessions.d` |

## `[[chats]]`

| Key | Phase | Notes |
|-----|-------|-------|
| `chat_id` | 3 | |
| `backend` | 3 | |
| `project` | 3 | |
| `label` | 3 | |

## `[projects.<slug>]`

| Key | Phase | Notes |
|-----|-------|-------|
| `root` | 3 | |
| `allow_shared_worktree` | 3 | |
| `[projects.<slug>.worktrees]` | 3 | per-backend paths |

## Environment (not `relay.toml`)

| Var | Phase | Notes |
|-----|-------|-------|
| `BOT_TOKEN`, `ALLOWLIST_*` | 0–2 | secrets in `.env` only |
| `TG_*` overrides | 1–2 | mirror general keys |
| `RELAY_IMPL` | 0 | `python` (default) / `rust` — P22d |

Additive keys may appear in upstream before this table updates; unknown keys should be preserved on round-trip config load (Phase 0 goal).
