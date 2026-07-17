//! Build agent/repo stamp lines from env (no network / git by default).
//!
//! Mirrors env-driven paths in `tg_agent_relay/agent_stamp.py`. Git/`gh` lookup is
//! intentionally omitted so stamps are deterministic in tests and offline hooks.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::agent_handle::build_handle_from_env;

static GH_SSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^git@github\.com:(?P<owner>[^/]+)/(?P<repo>[^/.]+)(?:\.git)?$").unwrap()
});
static GH_HTTPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/.]+)(?:\.git)?/?$").unwrap()
});
static MERGED_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(merged|MERGE)\b").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampInfo {
    pub repo: String,
    pub branch: String,
    pub branch_url: String,
    pub pr_url: String,
    pub pr_state: String,
    pub handle: String,
}

impl StampInfo {
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.handle.is_empty() {
            out.push(format!("🤖 handle={}", self.handle));
        }
        if !self.repo.is_empty() || !self.branch.is_empty() {
            out.push(format!("🏷 repo={} branch={}", self.repo, self.branch));
        }
        if !self.branch_url.is_empty() {
            out.push(format!("🔗 branch: {}", self.branch_url));
        }
        if !self.pr_url.is_empty() {
            out.push(format!("🔗 pr: {}", self.pr_url));
        }
        if !self.pr_state.is_empty() && !self.pr_url.is_empty() {
            out.push(format!("📌 status={}", self.pr_state));
        }
        out
    }

    pub fn text(&self) -> String {
        self.lines().join("\n")
    }
}

/// Parse GitHub `owner` and `repo` from a remote URL (for tests / optional callers).
pub fn parse_github_remote(url: &str) -> (String, String) {
    let url = url.trim();
    if url.is_empty() {
        return (String::new(), String::new());
    }
    if let Some(caps) = GH_SSH.captures(url) {
        return (
            caps.name("owner").map(|m| m.as_str().to_string()).unwrap_or_default(),
            caps.name("repo").map(|m| m.as_str().to_string()).unwrap_or_default(),
        );
    }
    if let Some(caps) = GH_HTTPS.captures(url) {
        return (
            caps.name("owner").map(|m| m.as_str().to_string()).unwrap_or_default(),
            caps.name("repo").map(|m| m.as_str().to_string()).unwrap_or_default(),
        );
    }
    (String::new(), String::new())
}

fn pr_from_env(env: &HashMap<String, String>) -> (String, String) {
    let mut url = env
        .get("RELAY_PR_URL")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let mut state = env
        .get("RELAY_PR_STATE")
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    let num = env
        .get("RELAY_PR_NUMBER")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let repo = env
        .get("RELAY_REPO")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if url.is_empty() && !num.is_empty() && !repo.is_empty() && repo.contains('/') {
        url = format!("https://github.com/{repo}/pull/{num}");
    }
    if !matches!(state.as_str(), "open" | "merged" | "closed") {
        state.clear();
    }
    (url, state)
}

fn branch_url_from_repo_branch(repo: &str, branch: &str) -> String {
    if repo.contains('/') && !branch.is_empty() {
        let parts: Vec<&str> = repo.splitn(2, '/').collect();
        if parts.len() == 2 {
            let owner = parts[0];
            let name = parts[1];
            return format!("https://github.com/{owner}/{name}/tree/{branch}");
        }
    }
    String::new()
}

/// Build stamp metadata from environment only (no git subprocess, no `gh`).
pub fn build_stamp_info(env: &HashMap<String, String>, force_merged: bool) -> StampInfo {
    let repo_env = env
        .get("RELAY_REPO")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let branch_env = env
        .get("RELAY_BRANCH")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let (pr_url, mut pr_state) = pr_from_env(env);

    let repo = repo_env;
    let branch = branch_env;
    let branch_url = branch_url_from_repo_branch(&repo, &branch);

    if force_merged && !pr_url.is_empty() {
        pr_state = "merged".to_string();
    }

    let handle = build_handle_from_env(env);

    StampInfo {
        repo,
        branch,
        branch_url,
        pr_url,
        pr_state,
        handle,
    }
}

pub fn build_stamp(env: &HashMap<String, String>, body_for_merge_hint: &str) -> String {
    let merged_hint = !body_for_merge_hint.is_empty() && MERGED_RE.is_match(body_for_merge_hint);
    build_stamp_info(env, merged_hint).text()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_remotes() {
        assert_eq!(
            parse_github_remote("git@github.com:tzervas/foo.git"),
            ("tzervas".into(), "foo".into())
        );
        assert_eq!(
            parse_github_remote("https://github.com/tzervas/bar"),
            ("tzervas".into(), "bar".into())
        );
    }

    #[test]
    fn test_stamp_from_env() {
        let mut env = HashMap::new();
        env.insert("RELAY_REPO".into(), "tzervas/demo".into());
        env.insert("RELAY_BRANCH".into(), "feat/x".into());
        env.insert("RELAY_PR_NUMBER".into(), "12".into());
        env.insert("RELAY_PR_STATE".into(), "open".into());
        let info = build_stamp_info(&env, false);
        assert_eq!(info.repo, "tzervas/demo");
        assert_eq!(info.branch, "feat/x");
        assert_eq!(info.pr_url, "https://github.com/tzervas/demo/pull/12");
        assert_eq!(info.pr_state, "open");
        let text = info.text();
        assert!(text.contains("🏷 repo=tzervas/demo branch=feat/x"));
        assert!(text.contains("🔗 pr: https://github.com/tzervas/demo/pull/12"));
        assert!(text.contains("📌 status=open"));
    }

    #[test]
    fn test_merged_hint_forces_status() {
        let mut env = HashMap::new();
        env.insert("RELAY_REPO".into(), "o/r".into());
        env.insert("RELAY_BRANCH".into(), "main".into());
        env.insert(
            "RELAY_PR_URL".into(),
            "https://github.com/o/r/pull/1".into(),
        );
        let out = build_stamp(&env, "PR merged into main");
        assert!(out.contains("status=merged"));
    }

    #[test]
    fn test_stamp_info_lines_empty_pr_state() {
        let info = StampInfo {
            repo: "a/b".into(),
            branch: "main".into(),
            branch_url: "https://github.com/a/b/tree/main".into(),
            pr_url: String::new(),
            pr_state: String::new(),
            handle: String::new(),
        };
        assert!(!info.text().contains("📌"));
    }
}