//! Mockable HTTP transport for Bot API POSTs.

use std::fmt;

/// Response from an HTTP POST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Transport error (network, TLS, timeout).
#[derive(Debug)]
pub struct HttpError {
    message: String,
}

impl HttpError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpError {}

/// POST `application/x-www-form-urlencoded` bodies (mockable in tests).
pub trait HttpClient: Send + Sync {
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse, HttpError>;
}

/// Default client backed by `ureq`.
#[derive(Debug, Clone, Default)]
pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse, HttpError> {
        let resp = ureq::post(url)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(body)
            .map_err(|e| HttpError::new(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| HttpError::new(e.to_string()))?;
        Ok(HttpResponse { status, body })
    }
}

/// In-memory mock: records the last request and returns a configured body.
#[derive(Debug, Default)]
pub struct MockHttpClient {
    pub last_url: std::sync::Mutex<Option<String>>,
    pub last_body: std::sync::Mutex<Option<String>>,
    pub response_body: std::sync::Mutex<String>,
}

impl MockHttpClient {
    pub fn with_ok_response() -> Self {
        Self {
            response_body: std::sync::Mutex::new(r#"{"ok":true,"result":{}}"#.to_string()),
            ..Default::default()
        }
    }
}

impl HttpClient for MockHttpClient {
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse, HttpError> {
        *self.last_url.lock().unwrap() = Some(url.to_string());
        *self.last_body.lock().unwrap() = Some(body.to_string());
        let body = self.response_body.lock().unwrap().clone();
        Ok(HttpResponse {
            status: 200,
            body,
        })
    }
}

impl<T: HttpClient + ?Sized> HttpClient for std::sync::Arc<T> {
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse, HttpError> {
        self.as_ref().post_form(url, body)
    }
}