//! Detect benign goal-mode tool noise and soften outbound hook spam.
//!
//! Mirrors `tg_agent_relay/goal_events.py`.

use regex::Regex;
use std::sync::LazyLock;

static GOAL_FAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(update_goal\s+failed|Goal\s+is\s+not\s+Active|cannot\s+mark\s+complete|goal\s+is\s+not\s+active)",
    )
    .unwrap()
});
static UPDATE_GOAL_TOOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)update_goal").unwrap());

const SOFT_LINE: &str = "ℹ️ Goal idle (no active goal to update).";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalNoiseAction {
    Skip,
    Soft,
    Pass,
}

/// True when *text* looks like a benign `update_goal` / goal-state failure.
pub fn is_goal_noise_text(text: &str) -> bool {
    let body = text.trim();
    if body.is_empty() {
        return false;
    }
    GOAL_FAIL.is_match(body)
}

pub fn is_update_goal_tool(tool_name: &str) -> bool {
    UPDATE_GOAL_TOOL.is_match(tool_name)
}

/// Return skip (suppress), soft (one-liner), or pass (unchanged).
pub fn classify_goal_noise(text: &str, tool_name: &str, hook_event: Option<&str>) -> GoalNoiseAction {
    let body = text.trim();
    if body.is_empty() && tool_name.is_empty() {
        return GoalNoiseAction::Pass;
    }

    let ev = hook_event.unwrap_or("").trim();
    if ev == "PostToolUseFailure"
        && is_update_goal_tool(tool_name)
        && (is_goal_noise_text(body) || body.is_empty())
    {
        return GoalNoiseAction::Skip;
    }

    if is_goal_noise_text(body) {
        if matches!(
            ev,
            "PostToolUseFailure" | "Notification" | "Stop" | "StopFailure"
        ) {
            return GoalNoiseAction::Skip;
        }
        if !ev.is_empty() || body.to_lowercase().contains("update_goal") {
            return GoalNoiseAction::Soft;
        }
    }
    GoalNoiseAction::Pass
}

/// Filter outbound text. `None` means do not send.
pub fn apply_goal_noise_policy(
    text: &str,
    tool_name: &str,
    hook_event: Option<&str>,
    is_hook: bool,
) -> Option<String> {
    let action = classify_goal_noise(text, tool_name, hook_event);
    match action {
        GoalNoiseAction::Skip => None,
        GoalNoiseAction::Soft => Some(SOFT_LINE.to_string()),
        GoalNoiseAction::Pass => {
            if is_hook && is_goal_noise_text(text) {
                None
            } else {
                Some(text.to_string())
            }
        }
    }
}

/// Adapter/provider hook one-liner filter (`None` → SKIP).
pub fn filter_hook_summary(
    summary: &str,
    tool_name: &str,
    hook_event: Option<&str>,
) -> Option<String> {
    apply_goal_noise_policy(summary, tool_name, hook_event, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_goal_noise() {
        assert!(is_goal_noise_text("update_goal failed: Goal is not Active"));
        assert!(is_goal_noise_text(
            "cannot mark complete without active goal"
        ));
        assert!(!is_goal_noise_text("PR merged"));
    }

    #[test]
    fn test_post_tool_failure_skips() {
        assert_eq!(
            classify_goal_noise(
                "Goal is not Active",
                "update_goal",
                Some("PostToolUseFailure"),
            ),
            GoalNoiseAction::Skip
        );
        assert_eq!(
            filter_hook_summary(
                "⚠️ update_goal failed: Goal is not Active",
                "",
                Some("PostToolUseFailure"),
            ),
            None
        );
    }

    #[test]
    fn test_apply_soft_on_generic() {
        let out = apply_goal_noise_policy("update_goal failed", "", None, false);
        assert!(out.is_some());
        assert!(out.unwrap().contains("Goal idle"));
    }
}