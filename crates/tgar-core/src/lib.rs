//! Core types and pure logic for **tgar-rs** (P22b ports from `tg-agent-relay`).

/// Crate version (matches workspace `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod agent_handle;
pub mod agent_stamp;
pub mod bots;
pub mod config;
pub mod goal_events;
pub mod metrics;
pub mod spool;

pub use bots::{
    backends_for_bot, bot_for_backend, bot_ids, bot_state_dir, bot_token, bots_configured,
    has_multi_bot, is_default_bot, selected_bot, token_env_for_bot, DEFAULT_TOKEN_ENV,
};
pub use config::{default_config_path, load_config, RelayConfig};
pub use metrics::{emit_metric, metrics_path};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!VERSION.is_empty());
    }
}
