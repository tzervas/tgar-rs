//! Metrics append — TSV lines to `.metrics.log` (Phase 0).
//!
//! Mirrors `tg_agent_relay/metrics.py`. Best-effort: I/O errors are swallowed
//! so callers never fail closed on observability.
//!
//! Line format: `epoch\tsource\tevent\tdetail\n` (epoch = unix seconds).

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Resolve the metrics log path.
///
/// Order (matches Python):
/// 1. `bridge_dir` → `{bridge_dir}/.metrics.log`
/// 2. `RELAY_METRICS_LOG` env
/// 3. Fallback: `.metrics.log` in the current working directory
///
/// Python's package-relative fallback (`Path(__file__).parents[1]`) is not
/// portable into a library crate; cwd is the operator-facing equivalent when
/// the binary runs from a bridge directory. Prefer passing `bridge_dir`.
#[must_use]
pub fn metrics_path(bridge_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = bridge_dir {
        return dir.join(".metrics.log");
    }
    if let Some(env_path) = env::var_os("RELAY_METRICS_LOG") {
        return PathBuf::from(env_path);
    }
    PathBuf::from(".metrics.log")
}

/// Best-effort append: `epoch\tsource\tevent\tdetail\n`.
///
/// Never panics; I/O failures are ignored (parity with Python
/// `except OSError: pass`).
pub fn emit_metric(source: &str, event: &str, detail: &str, bridge_dir: Option<&Path>) {
    let path = metrics_path(bridge_dir);
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let line = format!("{epoch}\t{source}\t{event}\t{detail}\n");
    let _ = append_line(&path, &line);
}

fn append_line(path: &Path, line: &str) -> Result<(), ()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| ())?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| ())?;
    file.write_all(line.as_bytes()).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise tests that touch process env (`RELAY_METRICS_LOG`).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_bridge(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tgar-metrics-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp bridge");
        dir
    }

    #[test]
    fn path_uses_bridge_dir() {
        let bridge = Path::new("/tmp/bridge-example");
        assert_eq!(
            metrics_path(Some(bridge)),
            PathBuf::from("/tmp/bridge-example/.metrics.log")
        );
    }

    #[test]
    fn path_respects_env_when_no_bridge() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let custom = std::env::temp_dir().join(format!(
            "tgar-metrics-env-{}-{}.log",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        // SAFETY: serialised by ENV_LOCK; restored below.
        env::set_var("RELAY_METRICS_LOG", &custom);
        let got = metrics_path(None);
        env::remove_var("RELAY_METRICS_LOG");
        assert_eq!(got, custom);
    }

    #[test]
    fn path_falls_back_to_cwd_relative() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        env::remove_var("RELAY_METRICS_LOG");
        assert_eq!(metrics_path(None), PathBuf::from(".metrics.log"));
    }

    #[test]
    fn emit_appends_tsv_line() {
        let bridge = temp_bridge("append");
        emit_metric(
            "spool",
            "spool_drained",
            "backend=fleet count=1",
            Some(&bridge),
        );
        let text = fs::read_to_string(bridge.join(".metrics.log")).expect("read log");
        let line = text.lines().next().expect("one line");
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 4, "line={line}");
        assert!(
            parts[0].chars().all(|c| c.is_ascii_digit()),
            "epoch={}",
            parts[0]
        );
        assert_eq!(parts[1], "spool");
        assert_eq!(parts[2], "spool_drained");
        assert_eq!(parts[3], "backend=fleet count=1");
        let _ = fs::remove_dir_all(&bridge);
    }

    #[test]
    fn emit_creates_parent_dirs() {
        let bridge = temp_bridge("mkdir");
        let nested = bridge.join("a").join("b");
        // Point via RELAY_METRICS_LOG to a nested path (no bridge_dir).
        let log = nested.join("nested.log");
        let _guard = ENV_LOCK.lock().expect("env lock");
        env::set_var("RELAY_METRICS_LOG", &log);
        emit_metric("test", "created", "ok", None);
        env::remove_var("RELAY_METRICS_LOG");
        assert!(log.is_file(), "expected {}", log.display());
        let _ = fs::remove_dir_all(&bridge);
    }

    #[test]
    fn emit_second_line_appends() {
        let bridge = temp_bridge("twice");
        emit_metric("src", "e1", "d1", Some(&bridge));
        emit_metric("src", "e2", "d2", Some(&bridge));
        let text = fs::read_to_string(bridge.join(".metrics.log")).expect("read log");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "text={text}");
        assert!(lines[0].ends_with("\tsrc\te1\td1"));
        assert!(lines[1].ends_with("\tsrc\te2\td2"));
        let _ = fs::remove_dir_all(&bridge);
    }

    #[test]
    fn empty_detail_still_four_fields() {
        let bridge = temp_bridge("empty-detail");
        emit_metric("cli", "tick", "", Some(&bridge));
        let text = fs::read_to_string(bridge.join(".metrics.log")).expect("read log");
        let line = text.lines().next().expect("line");
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts, [parts[0], "cli", "tick", ""]);
        let _ = fs::remove_dir_all(&bridge);
    }
}
