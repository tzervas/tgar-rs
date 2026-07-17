//! `sendMessage` via url-encoded POST (parity with `tg_agent_relay.send.send_message`).

use crate::http::{HttpClient, HttpError};

/// Parameters for [`TelegramBot::send_message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMessageParams<'a> {
    pub chat_id: &'a str,
    pub text: &'a str,
    pub parse_mode: Option<&'a str>,
    pub message_thread_id: Option<&'a str>,
    pub reply_markup: Option<&'a str>,
}

/// Telegram Bot API client using a pluggable [`HttpClient`].
pub struct TelegramBot<C> {
    token: String,
    client: C,
}

impl<C: HttpClient> TelegramBot<C> {
    pub fn new(token: impl Into<String>, client: C) -> Self {
        Self {
            token: token.into(),
            client,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.token)
    }

    /// POST `sendMessage`. Returns `true` when the JSON body contains `"ok":true`.
    pub fn send_message(&self, params: SendMessageParams<'_>) -> Result<bool, HttpError> {
        let body = encode_send_message_form(&params);
        let url = self.api_url("sendMessage");
        let resp = self.client.post_form(&url, &body)?;
        Ok(response_ok(&resp.body))
    }
}

fn response_ok(raw: &str) -> bool {
    raw.replace(' ', "").contains("\"ok\":true")
}

/// Percent-encode form fields for Telegram Bot API (same shape as Python `urllib.parse.urlencode`).
pub fn encode_send_message_form(params: &SendMessageParams<'_>) -> String {
    let mut pairs: Vec<(&str, &str)> = vec![
        ("chat_id", params.chat_id),
        ("text", params.text),
    ];
    if let Some(pm) = params.parse_mode.filter(|s| !s.is_empty()) {
        pairs.push(("parse_mode", pm));
    }
    if let Some(tid) = params.message_thread_id.filter(|s| !s.is_empty()) {
        pairs.push(("message_thread_id", tid));
    }
    if let Some(rm) = params.reply_markup.filter(|s| !s.is_empty()) {
        pairs.push(("reply_markup", rm));
    }
    pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::MockHttpClient;

    #[test]
    fn encode_basic_fields() {
        let body = encode_send_message_form(&SendMessageParams {
            chat_id: "-1001",
            text: "hi there",
            parse_mode: None,
            message_thread_id: None,
            reply_markup: None,
        });
        assert!(body.contains("chat_id=-1001"));
        assert!(body.contains("text=hi%20there"));
    }

    #[test]
    fn encode_optional_fields() {
        let body = encode_send_message_form(&SendMessageParams {
            chat_id: "1",
            text: "x",
            parse_mode: Some("HTML"),
            message_thread_id: Some("42"),
            reply_markup: Some(r#"{"inline_keyboard":[]}"#),
        });
        assert!(body.contains("parse_mode=HTML"));
        assert!(body.contains("message_thread_id=42"));
        assert!(body.contains("reply_markup="));
    }

    #[test]
    fn send_message_posts_to_bot_url() {
        let mock = MockHttpClient::with_ok_response();
        let bot = TelegramBot::new("tok123", &mock);
        let ok = bot
            .send_message(SendMessageParams {
                chat_id: "-100",
                text: "ping",
                parse_mode: None,
                message_thread_id: None,
                reply_markup: None,
            })
            .unwrap();
        assert!(ok);
        let url = mock.last_url.lock().unwrap().clone().unwrap();
        assert!(url.contains("bottok123/sendMessage"));
        let body = mock.last_body.lock().unwrap().clone().unwrap();
        assert!(body.contains("chat_id=-100"));
        assert!(body.contains("text=ping"));
    }

    #[test]
    fn send_message_false_on_not_ok() {
        let mock = MockHttpClient {
            response_body: std::sync::Mutex::new(r#"{"ok":false}"#.to_string()),
            ..Default::default()
        };
        let bot = TelegramBot::new("t", &mock);
        let ok = bot
            .send_message(SendMessageParams {
                chat_id: "1",
                text: "x",
                parse_mode: None,
                message_thread_id: None,
                reply_markup: None,
            })
            .unwrap();
        assert!(!ok);
    }
}