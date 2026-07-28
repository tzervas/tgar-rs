//! Bot identity — per-bot tokens, state isolation, and backend binding.
//!
//! Mirrors `tg_agent_relay/bots.py`. Multi-bot requires isolated state dirs so
//! Telegram getUpdates cursors (`.offset`) never clobber each other.
//!
//! Tokens are never stored in config — only env **names** (`token_env`).

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use toml::Value;

/// Default env var name for the single-bot / default token.
pub const DEFAULT_TOKEN_ENV: &str = "BOT_TOKEN";

const SAFE: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-";
const DEFAULT_ALIASES: &[&str] = &["", "default", "main"];

/// Sanitize a bot id for use as a directory name.
///
/// Bot ids come from `relay.toml`; never let one escape the state root via
/// `..` or a path separator.
#[must_use]
pub fn safe_bot(bot: &str) -> String {
    let cleaned: String = bot
        .chars()
        .map(|c| {
            if c.is_ascii() && SAFE.contains(&(c as u8)) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.');
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned.to_string()
    }
}

/// True when `bot` refers to the original single-bot identity.
#[must_use]
pub fn is_default_bot(bot: Option<&str>) -> bool {
    let s = bot.unwrap_or("").trim().to_ascii_lowercase();
    DEFAULT_ALIASES.contains(&s.as_str())
}

/// The `[bots.*]` table as `id → entry table`, or empty when unconfigured.
#[must_use]
pub fn bots_table(cfg: &Value) -> BTreeMap<String, Value> {
    let Some(bots) = cfg.get("bots").and_then(Value::as_table) else {
        return BTreeMap::new();
    };
    bots.iter()
        .filter(|(_, v)| v.is_table())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Configured bot ids, sorted. Empty when running single-bot.
#[must_use]
pub fn bot_ids(cfg: &Value) -> Vec<String> {
    bots_table(cfg).into_keys().collect()
}

/// True when any `[bots.*]` table exists (multi-bot features are active).
#[must_use]
pub fn bots_configured(cfg: &Value) -> bool {
    !bots_table(cfg).is_empty()
}

/// True when more than one bot is configured.
#[must_use]
pub fn has_multi_bot(cfg: &Value) -> bool {
    bots_table(cfg).len() > 1
}

/// Directory holding `bot`'s offset and reassembly buffers.
///
/// The default bot uses the bridge root unchanged so existing deployments
/// keep their exact `.offset` / `.tg-buffer*` paths.
#[must_use]
pub fn bot_state_dir(bridge_dir: &Path, bot: &str) -> PathBuf {
    if is_default_bot(Some(bot)) {
        bridge_dir.to_path_buf()
    } else {
        bridge_dir.join(".bots").join(safe_bot(bot))
    }
}

/// Name of the env var holding `bot`'s token (never the token itself).
#[must_use]
pub fn token_env_for_bot(cfg: &Value, bot: &str) -> String {
    if let Some(entry) = bots_table(cfg).get(bot) {
        if let Some(raw) = entry.get("token_env").and_then(Value::as_str) {
            if !raw.is_empty() {
                return raw.to_string();
            }
        }
    }
    DEFAULT_TOKEN_ENV.to_string()
}

/// Resolve `bot`'s token: process env first, then optional `env_map` (`.env`).
///
/// Falls back to `BOT_TOKEN` for the default bot so single-bot deployments
/// need no config change.
#[must_use]
pub fn bot_token(cfg: &Value, bot: &str, env_map: Option<&BTreeMap<String, String>>) -> String {
    let name = token_env_for_bot(cfg, bot);
    let from_process = env::var(&name).unwrap_or_default();
    let from_map = env_map
        .and_then(|m| m.get(&name))
        .cloned()
        .unwrap_or_default();
    let mut val = if from_process.is_empty() {
        from_map
    } else {
        from_process
    };
    if val.is_empty() && name != DEFAULT_TOKEN_ENV && is_default_bot(Some(bot)) {
        let fallback = env::var(DEFAULT_TOKEN_ENV).unwrap_or_default();
        let fallback_map = env_map
            .and_then(|m| m.get(DEFAULT_TOKEN_ENV))
            .cloned()
            .unwrap_or_default();
        val = if fallback.is_empty() {
            fallback_map
        } else {
            fallback
        };
    }
    val
}

/// Which bot serves `backend`. Empty means the default bot.
///
/// Reads `backends.<id>.bot`, falling back to `routing.default_bot`.
#[must_use]
pub fn bot_for_backend(cfg: &Value, backend: &str) -> String {
    if let Some(entry) = cfg
        .get("backends")
        .and_then(Value::as_table)
        .and_then(|t| t.get(backend))
        .and_then(Value::as_table)
    {
        if let Some(b) = entry.get("bot").and_then(Value::as_str) {
            if !b.is_empty() {
                return b.to_string();
            }
        }
    }
    if let Some(b) = cfg
        .get("routing")
        .and_then(Value::as_table)
        .and_then(|t| t.get("default_bot"))
        .and_then(Value::as_str)
    {
        if !b.is_empty() {
            return b.to_string();
        }
    }
    String::new()
}

/// Backend ids served by `bot`, sorted.
///
/// Backends with no explicit binding belong to the default bot.
#[must_use]
pub fn backends_for_bot(cfg: &Value, bot: &str) -> Vec<String> {
    let Some(backends) = cfg.get("backends").and_then(Value::as_table) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for bid in backends.keys() {
        let owner = bot_for_backend(cfg, bid);
        let matches = (is_default_bot(Some(bot)) && is_default_bot(Some(&owner))) || owner == bot;
        if matches {
            out.push(bid.clone());
        }
    }
    out.sort();
    out
}

/// Which bot this process serves: explicit arg, then `RELAY_BOT`, then default.
#[must_use]
pub fn selected_bot(cfg: &Value, explicit: Option<&str>) -> String {
    let _ = cfg; // parity with Python signature; reserved for future validation
    if let Some(e) = explicit {
        if !e.is_empty() {
            return e.to_string();
        }
    }
    env::var("RELAY_BOT")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serialize tests that touch process env.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn cfg_multi() -> Value {
        r#"
[bots.claude]
token_env = "BOT_TOKEN"
[bots.grok]
token_env = "BOT_TOKEN_GROK"
[backends.fleet]
delivery = "fifo"
bot = "grok"
[backends.cabal]
delivery = "fifo"
bot = "grok"
[backends.claude]
delivery = "fifo"
bot = "claude"
[backends.legacy]
delivery = "stdout"
"#
        .parse()
        .unwrap()
    }

    #[test]
    fn default_identity_aliases() {
        for alias in ["", "default", "main", "DEFAULT"] {
            assert!(is_default_bot(Some(alias)), "{alias:?}");
        }
        assert!(!is_default_bot(Some("grok")));
    }

    #[test]
    fn default_bot_keeps_legacy_layout() {
        let bridge = std::env::temp_dir().join(format!("tgar-bots-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&bridge);
        fs::create_dir_all(&bridge).unwrap();

        assert_eq!(bot_state_dir(&bridge, ""), bridge);
        assert_eq!(bot_state_dir(&bridge, "default"), bridge);
        assert!(!bots_configured(&Value::Table(toml::map::Map::new())));
        assert!(bot_ids(&Value::Table(toml::map::Map::new())).is_empty());
        assert_eq!(
            bot_for_backend(&Value::Table(toml::map::Map::new()), "anything"),
            ""
        );

        let _ = fs::remove_dir_all(&bridge);
    }

    #[test]
    fn named_bots_isolated_and_cannot_escape() {
        let bridge = std::env::temp_dir().join(format!("tgar-bots-iso-{}", std::process::id()));
        let _ = fs::remove_dir_all(&bridge);
        fs::create_dir_all(&bridge).unwrap();

        let grok_dir = bot_state_dir(&bridge, "grok");
        assert_eq!(grok_dir, bridge.join(".bots").join("grok"));
        assert!(grok_dir.starts_with(&bridge));

        let escaped = bot_state_dir(&bridge, "../../etc");
        assert!(escaped.starts_with(bridge.join(".bots")));

        // THE property: independent offset paths
        let d_default = bot_state_dir(&bridge, "");
        let d_grok = bot_state_dir(&bridge, "grok");
        let d_claude = bot_state_dir(&bridge, "claude");
        fs::create_dir_all(&d_grok).unwrap();
        fs::create_dir_all(&d_claude).unwrap();
        fs::write(d_default.join(".offset"), "100").unwrap();
        fs::write(d_grok.join(".offset"), "200").unwrap();
        fs::write(d_claude.join(".offset"), "300").unwrap();
        assert_eq!(
            fs::read_to_string(d_default.join(".offset")).unwrap(),
            "100"
        );
        assert_eq!(fs::read_to_string(d_grok.join(".offset")).unwrap(), "200");
        fs::write(d_grok.join(".offset"), "999").unwrap();
        assert_eq!(
            fs::read_to_string(d_default.join(".offset")).unwrap(),
            "100"
        );
        assert_eq!(fs::read_to_string(d_claude.join(".offset")).unwrap(), "300");
        assert_eq!(fs::read_to_string(d_grok.join(".offset")).unwrap(), "999");
        assert!(bridge.join(".offset").is_file());

        let _ = fs::remove_dir_all(&bridge);
    }

    #[test]
    fn token_env_resolution() {
        let _g = ENV_LOCK.lock().unwrap();
        let cfg = cfg_multi();
        assert_eq!(token_env_for_bot(&cfg, "grok"), "BOT_TOKEN_GROK");
        assert_eq!(token_env_for_bot(&cfg, "claude"), "BOT_TOKEN");
        assert_eq!(token_env_for_bot(&cfg, "nope"), "BOT_TOKEN");
        assert_eq!(
            token_env_for_bot(&Value::Table(toml::map::Map::new()), ""),
            "BOT_TOKEN"
        );

        let mut env_map = BTreeMap::new();
        env_map.insert("BOT_TOKEN".into(), "tok-default".into());
        env_map.insert("BOT_TOKEN_GROK".into(), "tok-grok".into());
        env::remove_var("BOT_TOKEN_GROK");
        assert_eq!(bot_token(&cfg, "grok", Some(&env_map)), "tok-grok");
        assert_eq!(bot_token(&cfg, "claude", Some(&env_map)), "tok-default");
        let mut only_default = BTreeMap::new();
        only_default.insert("BOT_TOKEN".into(), "x".into());
        assert_eq!(bot_token(&cfg, "grok", Some(&only_default)), "");
        assert_eq!(
            bot_token(&Value::Table(toml::map::Map::new()), "", Some(&env_map)),
            "tok-default"
        );

        env::set_var("BOT_TOKEN_GROK", "from-process-env");
        assert_eq!(bot_token(&cfg, "grok", Some(&env_map)), "from-process-env");
        env::remove_var("BOT_TOKEN_GROK");
    }

    #[test]
    fn backend_binding() {
        let cfg = cfg_multi();
        assert_eq!(bot_for_backend(&cfg, "fleet"), "grok");
        assert_eq!(bot_for_backend(&cfg, "claude"), "claude");
        assert_eq!(bot_for_backend(&cfg, "legacy"), "");
        assert_eq!(backends_for_bot(&cfg, "grok"), vec!["cabal", "fleet"]);
        assert_eq!(backends_for_bot(&cfg, "claude"), vec!["claude"]);
        assert_eq!(backends_for_bot(&cfg, ""), vec!["legacy"]);

        let cfg_dflt: Value = r#"
[bots.grok]
[routing]
default_bot = "grok"
[backends.x]
delivery = "fifo"
"#
        .parse()
        .unwrap();
        assert_eq!(bot_for_backend(&cfg_dflt, "x"), "grok");
        assert_eq!(backends_for_bot(&cfg_dflt, "grok"), vec!["x"]);
    }

    #[test]
    fn discovery_and_selection() {
        let _g = ENV_LOCK.lock().unwrap();
        let cfg = cfg_multi();
        assert_eq!(bot_ids(&cfg), vec!["claude", "grok"]);
        assert!(has_multi_bot(&cfg));
        let single: Value = r"[bots.only]".parse().unwrap();
        assert!(!has_multi_bot(&single));
        assert!(bots_configured(&cfg));

        assert_eq!(selected_bot(&cfg, Some("grok")), "grok");
        env::set_var("RELAY_BOT", "claude");
        assert_eq!(selected_bot(&cfg, None), "claude");
        assert_eq!(selected_bot(&cfg, Some("grok")), "grok");
        env::remove_var("RELAY_BOT");
        assert_eq!(selected_bot(&cfg, None), "");
    }
}
