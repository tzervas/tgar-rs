# Telegram outbound (P22c)

## Environment

| Variable | Required for live send | Notes |
|----------|----------------------|-------|
| `BOT_TOKEN` | **Yes** | Same as Python relay `.env` / `tg-send.sh`. Missing token ⇒ CLI dry-run only. |
| `ALLOWED_CHAT_ID` | Yes (unless `--chat-id`) | Destination supergroup/channel/user id. |

Do not store tokens in this repository. Use `workstation-private-config` overlays per relay security posture.

## Implementation

- `crates/tgar-telegram`: `TelegramBot` + [`HttpClient`](../../crates/tgar-telegram/src/http.rs) trait (mockable in unit tests).
- `sendMessage` body: `application/x-www-form-urlencoded` (`chat_id`, `text`, optional `parse_mode`, `message_thread_id`, `reply_markup`).
- Pagination: `paginate` / `format_page_payloads` — parity with `tg_agent_relay.send.paginate` and `[k/n]` prefixes.

## Testing

```bash
cargo test -p tgar-telegram
```

No network: tests use `MockHttpClient`.