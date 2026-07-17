//! Telegram Bot API layer (Phase 1–2 port target).

/// Placeholder until send/poll HTTP client lands.
pub const CRATE_MARKER: &str = "tgar-telegram";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_present() {
        assert!(!CRATE_MARKER.is_empty());
    }
}
