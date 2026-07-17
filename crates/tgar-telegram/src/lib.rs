//! Telegram Bot API layer for **tgar-rs** (P22c: sendMessage + pagination).
//!
//! Live sends require `BOT_TOKEN` in the environment (see workspace `README.md` and `docs/TELEGRAM.md`).

mod bot;
mod http;
mod pagination;

pub use bot::{encode_send_message_form, SendMessageParams, TelegramBot};
pub use http::{HttpClient, HttpError, HttpResponse, MockHttpClient, UreqHttpClient};
pub use pagination::{format_page_payloads, paginate};