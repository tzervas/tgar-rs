//! Core types and pure logic for **tgar-rs** (P22b ports from `tg-agent-relay`).

/// Crate version (matches workspace `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod agent_handle;
pub mod agent_stamp;
pub mod goal_events;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!VERSION.is_empty());
    }
}