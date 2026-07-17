//! Construct `@repo-branch` agent handles (pure functions, no I/O).
//!
//! Mirrors `tg_agent_relay/agent_handle.py`.

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

static BRANCH_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(feat|fix|chore|docs)/").unwrap());
static ORCH_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^@(?P<alias>cabal|orchestrator|main|fleet)(?:\s+|:|$)").unwrap()
});
static HANDLE_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@([A-Za-z0-9][A-Za-z0-9_-]{0,127})(?:\s+|:|$)").unwrap());

const RESERVED: &[&str] = &["cabal", "orchestrator", "main", "fleet"];

/// Last path segment of repo name, lowercased, alnum only, max 16 chars.
pub fn repo_short(repo: &str) -> String {
    let mut name = repo.trim();
    if let Some((_prefix, tail)) = name.rsplit_once('/') {
        name = tail;
    }
    let lower = name.to_lowercase();
    let out: String = lower.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    out.chars().take(16).collect()
}

/// Normalize branch for handle suffix: strip common prefixes, slug, max 20.
pub fn branch_short(branch: &str) -> String {
    let b = BRANCH_PREFIX_RE.replace(branch.trim(), "");
    let lower = b.to_lowercase();
    let slug: String = lower
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect();
    let collapsed = collapse_dashes(&slug);
    let trimmed = collapsed.trim_matches('-');
    trimmed.chars().take(20).collect()
}

fn collapse_dashes(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c == '-' {
            if !prev_dash {
                out.push('-');
            }
            prev_dash = true;
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    out
}

/// Full Telegram handle including leading `@`.
pub fn build_handle(repo: &str, branch: &str, repo_name: &str) -> String {
    let r = repo_short(if !repo.is_empty() { repo } else { repo_name });
    let br = branch_short(branch);
    if r.is_empty() && br.is_empty() {
        return String::new();
    }
    if r.is_empty() {
        return format!("@{br}");
    }
    if br.is_empty() {
        return format!("@{r}");
    }
    format!("@{r}-{br}")
}

/// Strip leading `@` for session filenames / `RELAY_BACKEND` derivation.
pub fn handle_id(handle: &str) -> String {
    let h = handle.trim();
    if let Some(stripped) = h.strip_prefix('@') {
        stripped.to_string()
    } else {
        h.to_string()
    }
}

/// True for orchestrator aliases (with or without `@`).
pub fn is_reserved_handle(handle: &str) -> bool {
    let h = handle_id(handle).to_lowercase();
    RESERVED.contains(&h.as_str())
}

/// If text starts with `@handle`, return `(handle_with_at, stripped_text)`.
pub fn parse_leading_handle(text: &str) -> Option<(String, String)> {
    let raw = text;
    let caps = HANDLE_PREFIX_RE.captures(raw)?;
    let hid = caps.get(1)?.as_str();
    let m = caps.get(0)?;
    let token = m.as_str();
    let at = format!("@{hid}");
    let rest = if token.ends_with(':') {
        raw[m.end()..].trim_start().to_string()
    } else {
        raw[token.len()..].trim_start().to_string()
    };
    Some((at, rest))
}

/// Reserved `@cabal` / `@orchestrator` / `@main` / `@fleet` → `(alias, stripped)`.
pub fn strip_orchestrator_prefix(text: &str) -> Option<(String, String)> {
    let raw = text;
    let caps = ORCH_PREFIX_RE.captures(raw)?;
    let alias = caps.name("alias")?.as_str().to_lowercase();
    let token = caps.get(0)?.as_str();
    let rest = if token.ends_with(':') {
        raw[caps.get(0)?.end()..].trim_start().to_string()
    } else {
        raw[token.len()..].trim_start().to_string()
    };
    Some((alias, rest))
}

/// Resolve configured orchestrator backend id for reserved aliases.
pub fn orchestrator_backend_id(cfg: &Value, alias: &str) -> String {
    let routing = cfg.get("routing").and_then(|v| v.as_object());
    if let Some(r) = routing {
        if let Some(ob) = r.get("orchestrator_backend").and_then(|v| v.as_str()) {
            let s = ob.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
        if let Some(default) = r.get("default_backend").and_then(|v| v.as_str()) {
            let s = default.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    let a = alias.trim().to_lowercase();
    if !a.is_empty() {
        if let Some(backends) = cfg.get("backends").and_then(|v| v.as_object()) {
            if backends.contains_key(&a) {
                return a;
            }
        }
    }
    String::new()
}

/// Construct handle from `RELAY_AGENT_HANDLE` or `RELAY_REPO` + `RELAY_BRANCH`.
pub fn build_handle_from_env(env: &HashMap<String, String>) -> String {
    if let Some(explicit) = env.get("RELAY_AGENT_HANDLE") {
        let e = explicit.trim();
        if !e.is_empty() {
            return if e.starts_with('@') {
                e.to_string()
            } else {
                format!("@{e}")
            };
        }
    }
    let repo = env.get("RELAY_REPO").map(|s| s.as_str()).unwrap_or("").trim();
    let branch = env.get("RELAY_BRANCH").map(|s| s.as_str()).unwrap_or("").trim();
    build_handle(repo, branch, "")
}

/// `RELAY_BACKEND` when multi-session: handle id without `@`.
pub fn backend_id_from_handle(handle: &str) -> String {
    handle_id(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_short() {
        assert_eq!(repo_short("tzervas/TG-Agent-Relay"), "tgagentrelay");
        assert_eq!(repo_short("tg-agent-relay"), "tgagentrelay");
        assert_eq!(repo_short(&"a".repeat(40)).len(), 16);
    }

    #[test]
    fn test_branch_short() {
        assert_eq!(
            branch_short("feat/agent-handles-bidirectional"),
            "agent-handles-bidire"
        );
        assert_eq!(branch_short("main"), "main");
        assert_eq!(branch_short("fix/foo_bar"), "foo-bar");
    }

    #[test]
    fn test_build_handle_examples() {
        assert_eq!(
            build_handle("tzervas/tg-agent-relay", "feat/agent-handles-bidirectional", ""),
            "@tgagentrelay-agent-handles-bidire"
        );
        assert_eq!(build_handle("foo", "main", ""), "@foo-main");
    }

    #[test]
    fn test_reserved_and_parse() {
        assert!(is_reserved_handle("@cabal"));
        assert!(is_reserved_handle("orchestrator"));
        assert!(!is_reserved_handle("@tgagentrelay-main"));
        let hit = parse_leading_handle("@tgagentrelay-main ship it").unwrap();
        assert_eq!(hit.0, "@tgagentrelay-main");
        assert_eq!(hit.1, "ship it");
        let orch = strip_orchestrator_prefix("@cabal /config").unwrap();
        assert_eq!(orch.0, "cabal");
        assert_eq!(orch.1, "/config");
    }

    #[test]
    fn test_orchestrator_backend_resolution() {
        let cfg: Value = serde_json::json!({
            "routing": {"orchestrator_backend": "cabal", "require_prefix": true},
            "backends": {"cabal": {"prefixes": ["@cabal"]}},
        });
        assert_eq!(orchestrator_backend_id(&cfg, "fleet"), "cabal");
    }

    #[test]
    fn test_env_and_backend_id() {
        let mut env = HashMap::new();
        env.insert("RELAY_REPO".into(), "o/r".into());
        env.insert("RELAY_BRANCH".into(), "feat/x".into());
        assert_eq!(build_handle_from_env(&env), "@r-x");
        assert_eq!(backend_id_from_handle("@cabal"), "cabal");
        assert_eq!(
            backend_id_from_handle("@tgagentrelay-main"),
            "tgagentrelay-main"
        );
    }
}