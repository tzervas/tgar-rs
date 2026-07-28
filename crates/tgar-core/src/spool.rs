//! Durable inbound spool — lines survive when no agent reader is attached.
//!
//! Mirrors `tg_agent_relay/spool.py`. Layout:
//!
//! ```text
//! <bridge>/.run/spool/<backend>/<stamp>.msg
//! ```
//!
//! `stamp` is `<time_ns:019d>-<pid:07d>-<counter:04d>` so a plain lexical
//! sort is chronological. Files are written `.tmp` then atomically renamed,
//! so a partially written file is never drained.
//!
//! Drain protocol: claim via rename to `<name>.claimed.<pid>`; unlink only
//! after emit succeeds. Stale claims return to pending after
//! `RELAY_SPOOL_CLAIM_TTL` (default 300s). Bound by `RELAY_SPOOL_MAX`
//! (default 1000) with a `spool_overflow` metric on drop.
//!
//! When `bridge_dir` is `None`, paths resolve relative to the current working
//! directory (Python falls back to the tg-agent-relay package root). Callers
//! that care pass `bridge_dir`.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::metrics::emit_metric;

/// Default max pending lines per backend (`RELAY_SPOOL_MAX`).
pub const DEFAULT_SPOOL_MAX: u32 = 1000;
/// Default claim reclaim TTL in seconds (`RELAY_SPOOL_CLAIM_TTL`).
pub const DEFAULT_CLAIM_TTL: u64 = 300;

const SAFE: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Sanitize a backend id for use as a directory name.
///
/// Backend ids come from `relay.toml` (and routed tags); never let one escape
/// the spool root via `..` or `/`.
#[must_use]
pub fn safe_backend(backend: &str) -> String {
    let cleaned: String = backend
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

/// Directory holding pending lines for `backend`.
#[must_use]
pub fn spool_dir(backend: &str, bridge_dir: Option<&Path>) -> PathBuf {
    let root = bridge_dir.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    root.join(".run").join("spool").join(safe_backend(backend))
}

fn spool_max() -> u32 {
    match env::var("RELAY_SPOOL_MAX") {
        Ok(raw) => raw
            .parse::<u32>()
            .ok()
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_SPOOL_MAX),
        Err(_) => DEFAULT_SPOOL_MAX,
    }
}

fn claim_ttl() -> u64 {
    match env::var("RELAY_SPOOL_CLAIM_TTL") {
        Ok(raw) => raw
            .parse::<u64>()
            .ok()
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_CLAIM_TTL),
        Err(_) => DEFAULT_CLAIM_TTL,
    }
}

fn next_counter() -> u32 {
    // Match Python: _counter = (_counter + 1) % 10000; use _counter.
    COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1) % 10_000
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn unix_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

fn mtime_secs_f64(path: &Path) -> io::Result<f64> {
    let modified = path.metadata()?.modified()?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64()))
}

/// Pending spool files for `backend`, oldest first.
#[must_use]
pub fn pending_paths(backend: &str, bridge_dir: Option<&Path>) -> Vec<PathBuf> {
    let d = spool_dir(backend, bridge_dir);
    let mut out = match fs::read_dir(&d) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "msg"))
            .collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    out.sort();
    out
}

/// Number of lines waiting for an agent reader.
#[must_use]
pub fn pending_count(backend: &str, bridge_dir: Option<&Path>) -> usize {
    pending_paths(backend, bridge_dir).len()
}

fn enforce_cap(backend: &str, bridge_dir: Option<&Path>) {
    let cap = spool_max();
    let pending = pending_paths(backend, bridge_dir);
    let excess = pending.len().saturating_sub(cap as usize);
    if excess == 0 {
        return;
    }
    for victim in pending.into_iter().take(excess) {
        let name = victim
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if fs::remove_file(&victim).is_err() {
            continue;
        }
        emit_metric(
            "spool",
            "spool_overflow",
            &format!("backend={backend} dropped={name} cap={cap}"),
            bridge_dir,
        );
    }
}

/// Persist one inbound `line` for `backend`. Returns the spool path, or `None`
/// if the spool itself was unwritable.
///
/// `line` is stored without a trailing newline; [`drain`] re-adds it for FIFO
/// parity at the call site when needed. Returning `None` means the caller must
/// not report the message as delivered.
#[must_use]
pub fn spool_message(
    backend: &str,
    line: &str,
    bridge_dir: Option<&Path>,
    reason: &str,
) -> Option<PathBuf> {
    let d = spool_dir(backend, bridge_dir);
    if fs::create_dir_all(&d).is_err() {
        emit_metric(
            "spool",
            "spool_error",
            &format!("backend={backend} reason=mkdir_failed"),
            bridge_dir,
        );
        return None;
    }

    let stamp = format!(
        "{:019}-{:07}-{:04}",
        unix_nanos(),
        std::process::id(),
        next_counter()
    );
    let final_path = d.join(format!("{stamp}.msg"));
    let tmp = d.join(format!("{stamp}.tmp"));
    let payload = line.strip_suffix('\n').unwrap_or(line);

    if write_atomic(&tmp, &final_path, payload).is_err() {
        let _ = fs::remove_file(&tmp);
        emit_metric(
            "spool",
            "spool_error",
            &format!("backend={backend} reason=write_failed"),
            bridge_dir,
        );
        return None;
    }

    let mut detail = format!(
        "backend={backend} pending={}",
        pending_count(backend, bridge_dir)
    );
    if !reason.is_empty() {
        detail.push_str(" reason=");
        detail.push_str(reason);
    }
    emit_metric("spool", "message_spooled", &detail, bridge_dir);
    enforce_cap(backend, bridge_dir);
    Some(final_path)
}

fn write_atomic(tmp: &Path, final_path: &Path, payload: &str) -> io::Result<()> {
    {
        let mut fh = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(tmp)?;
        fh.write_all(payload.as_bytes())?;
        fh.flush()?;
        fh.sync_all()?;
    }
    fs::rename(tmp, final_path)?;
    // Best-effort directory sync so the rename is durable.
    if let Some(parent) = final_path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Return claims abandoned by dead drainers to pending. Returns count recovered.
#[must_use]
pub fn reclaim_stale(
    backend: &str,
    bridge_dir: Option<&Path>,
    ttl: Option<u64>,
    now: Option<f64>,
) -> usize {
    let d = spool_dir(backend, bridge_dir);
    let ttl_secs = ttl.unwrap_or_else(claim_ttl);
    let now_secs = now.unwrap_or_else(unix_secs_f64);
    // TTL values are small (seconds); f64 is fine for wall-clock compare.
    #[allow(clippy::cast_precision_loss)]
    let cutoff = now_secs - ttl_secs as f64;

    let entries = match fs::read_dir(&d) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect::<Vec<_>>(),
        Err(_) => return 0,
    };

    let mut recovered = 0usize;
    for path in entries {
        let Some(name) = path
            .file_name()
            .and_then(|s| s.to_str().map(str::to_string))
        else {
            continue;
        };
        if !name.contains(".claimed.") {
            continue;
        }
        let Ok(mtime) = mtime_secs_f64(&path) else {
            continue;
        };
        if mtime > cutoff {
            continue;
        }
        let Some((original, _)) = name.split_once(".claimed.") else {
            continue;
        };
        let dest = d.join(original);
        if fs::rename(&path, &dest).is_err() {
            continue;
        }
        recovered += 1;
    }

    if recovered > 0 {
        emit_metric(
            "spool",
            "spool_reclaimed",
            &format!("backend={backend} count={recovered}"),
            bridge_dir,
        );
    }
    recovered
}

/// Replay spooled lines for `backend` in order. Returns lines emitted.
///
/// `emit` is called once per line (without a trailing newline). A line is
/// unlinked only *after* `emit` returns `Ok`. If `emit` returns `Err`, the
/// claim is released immediately and the error propagates — the line stays
/// pending rather than being dropped.
///
/// `limit` of `0` means unlimited (Python parity).
///
/// # Errors
///
/// Returns whatever error `emit` produces. The failed line is restored to
/// pending before the error is returned.
pub fn drain<F, E>(
    backend: &str,
    mut emit: F,
    bridge_dir: Option<&Path>,
    limit: usize,
    reclaim: bool,
) -> Result<usize, E>
where
    F: FnMut(&str) -> Result<(), E>,
{
    if reclaim {
        let _ = reclaim_stale(backend, bridge_dir, None, None);
    }

    let d = spool_dir(backend, bridge_dir);
    let mut sent = 0usize;
    for path in pending_paths(backend, bridge_dir) {
        if limit > 0 && sent >= limit {
            break;
        }
        let claimed = d.join(format!(
            "{}.claimed.{}",
            path.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default(),
            std::process::id()
        ));
        // Atomic claim: the loser of a race gets NotFound and moves on.
        if fs::rename(&path, &claimed).is_err() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&claimed) else {
            let _ = fs::remove_file(&claimed);
            continue;
        };
        match emit(&text) {
            Ok(()) => {
                let _ = fs::remove_file(&claimed);
                sent += 1;
            }
            Err(e) => {
                // Put it back — an emit failure must not consume the message.
                let _ = fs::rename(&claimed, &path);
                return Err(e);
            }
        }
    }

    if sent > 0 {
        emit_metric(
            "spool",
            "spool_drained",
            &format!("backend={backend} count={sent}"),
            bridge_dir,
        );
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise all spool tests: `RELAY_SPOOL_MAX` is process-global and
    /// the bounded-cap test mutates it.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Hold env lock and clear spool env overrides for the duration of `f`.
    fn with_clean_env<T>(f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("env lock");
        // SAFETY: serialised by ENV_LOCK.
        env::remove_var("RELAY_SPOOL_MAX");
        env::remove_var("RELAY_SPOOL_CLAIM_TTL");
        f()
    }

    fn temp_bridge(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tgar-spool-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp bridge");
        dir
    }

    fn collect_drain(backend: &str, bridge: &Path, limit: usize) -> Vec<String> {
        let mut seen = Vec::new();
        drain(
            backend,
            |line| {
                seen.push(line.to_string());
                Ok::<(), ()>(())
            },
            Some(bridge),
            limit,
            true,
        )
        .expect("drain");
        seen
    }

    #[test]
    fn round_trip_preserves_content_and_order() {
        with_clean_env(|| {
            let b = temp_bridge("order");
            for i in 0..5 {
                let path =
                    spool_message("fleet", &format!("[telegram:fleet] line {i}"), Some(&b), "");
                assert!(path.is_some());
            }
            assert_eq!(pending_count("fleet", Some(&b)), 5);

            let seen = collect_drain("fleet", &b, 0);
            assert_eq!(
                seen,
                (0..5)
                    .map(|i| format!("[telegram:fleet] line {i}"))
                    .collect::<Vec<_>>()
            );
            assert_eq!(pending_count("fleet", Some(&b)), 0);
            assert_eq!(collect_drain("fleet", &b, 0), Vec::<String>::new());
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn trailing_newline_stripped_on_store() {
        with_clean_env(|| {
            let b = temp_bridge("newline");
            let _ = spool_message("fleet", "with newline\n", Some(&b), "");
            let got = collect_drain("fleet", &b, 0);
            assert_eq!(got, vec!["with newline".to_string()]);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn backends_are_isolated() {
        with_clean_env(|| {
            let b = temp_bridge("iso");
            let _ = spool_message("fleet", "for fleet", Some(&b), "");
            let _ = spool_message("cabal", "for cabal", Some(&b), "");
            assert_eq!(pending_count("fleet", Some(&b)), 1);
            assert_eq!(pending_count("cabal", Some(&b)), 1);
            let cabal = collect_drain("cabal", &b, 0);
            assert_eq!(cabal, vec!["for cabal".to_string()]);
            assert_eq!(pending_count("fleet", Some(&b)), 1);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn emit_failure_does_not_consume() {
        with_clean_env(|| {
            let b = temp_bridge("boom");
            let _ = spool_message("fleet", "must survive", Some(&b), "");

            let err = drain(
                "fleet",
                |_line| Err("agent went away mid-write"),
                Some(&b),
                0,
                true,
            );
            assert_eq!(err, Err("agent went away mid-write"));
            assert_eq!(pending_count("fleet", Some(&b)), 1);

            let recovered = collect_drain("fleet", &b, 0);
            assert_eq!(recovered, vec!["must survive".to_string()]);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn limit_leaves_remainder_pending() {
        with_clean_env(|| {
            let b = temp_bridge("limit");
            for i in 0..4 {
                let _ = spool_message("fleet", &format!("m{i}"), Some(&b), "");
            }
            let first = collect_drain("fleet", &b, 2);
            assert_eq!(first, vec!["m0".to_string(), "m1".to_string()]);
            assert_eq!(pending_count("fleet", Some(&b)), 2);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn stale_claim_reclaim() {
        with_clean_env(|| {
            let b = temp_bridge("reclaim");
            let _ = spool_message("fleet", "orphaned by a dead drainer", Some(&b), "");
            let victim = pending_paths("fleet", Some(&b))[0].clone();
            let claimed = victim.with_file_name(format!(
                "{}.claimed.999999",
                victim.file_name().unwrap().to_string_lossy()
            ));
            fs::rename(&victim, &claimed).expect("claim rename");
            assert_eq!(pending_count("fleet", Some(&b)), 0);

            // Fresh claim must NOT be reclaimed.
            assert_eq!(reclaim_stale("fleet", Some(&b), Some(300), None), 0);
            // Old claim (ttl=0) is returned to pending.
            assert_eq!(reclaim_stale("fleet", Some(&b), Some(0), None), 1);
            assert_eq!(pending_count("fleet", Some(&b)), 1);

            let back = collect_drain("fleet", &b, 0);
            assert_eq!(back, vec!["orphaned by a dead drainer".to_string()]);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn concurrent_drain_never_double_delivers() {
        with_clean_env(|| {
            let b = temp_bridge("race");
            for i in 0..6 {
                let _ = spool_message("fleet", &format!("once-{i}"), Some(&b), "");
            }
            let mut a_seen = Vec::new();
            let mut b_seen = Vec::new();

            // Interleave two drains: inner drain runs while outer holds a claim.
            {
                let b_ref = &b;
                let b_seen_ref = &mut b_seen;
                drain(
                    "fleet",
                    |line| -> Result<(), ()> {
                        a_seen.push(line.to_string());
                        drain(
                            "fleet",
                            |inner| {
                                b_seen_ref.push(inner.to_string());
                                Ok(())
                            },
                            Some(b_ref),
                            1,
                            false,
                        )?;
                        Ok(())
                    },
                    Some(&b),
                    2,
                    false,
                )
                .expect("outer drain");
            }

            let combined: Vec<&String> = a_seen.iter().chain(b_seen.iter()).collect();
            let set: std::collections::HashSet<&String> = combined.iter().copied().collect();
            assert_eq!(
                combined.len(),
                set.len(),
                "double delivery: a={a_seen:?} b={b_seen:?}"
            );
            assert!(
                !a_seen.is_empty() && !b_seen.is_empty(),
                "interleaved drains both made progress a={a_seen:?} b={b_seen:?}"
            );
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn bounded_spool_drops_oldest_and_records() {
        with_clean_env(|| {
            let b = temp_bridge("cap");
            // SAFETY: serialised by with_clean_env / ENV_LOCK; restored below.
            env::set_var("RELAY_SPOOL_MAX", "3");
            for i in 0..6 {
                let _ = spool_message("fleet", &format!("cap{i}"), Some(&b), "");
            }
            env::remove_var("RELAY_SPOOL_MAX");

            assert_eq!(pending_count("fleet", Some(&b)), 3);
            let kept = collect_drain("fleet", &b, 0);
            assert_eq!(
                kept,
                vec!["cap3".to_string(), "cap4".to_string(), "cap5".to_string()]
            );
            let metrics = fs::read_to_string(b.join(".metrics.log")).expect("metrics");
            assert!(
                metrics.contains("spool_overflow"),
                "overflow not recorded: {metrics}"
            );
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn backend_id_cannot_escape_spool_root() {
        with_clean_env(|| {
            let b = temp_bridge("escape");
            let escaped = spool_dir("../../etc", Some(&b));
            let spool_root = b.join(".run").join("spool");
            let escaped_canon = escaped.canonicalize().unwrap_or(escaped.clone());
            assert!(
                escaped.starts_with(&spool_root) || escaped_canon.starts_with(&spool_root),
                "escaped={} root={}",
                escaped.display(),
                spool_root.display()
            );
            let _ = spool_message("../../etc", "nice try", Some(&b), "");
            assert!(
                !b.parent().expect("parent").join("etc").exists(),
                "file escaped bridge dir"
            );
            assert_eq!(pending_count("../../etc", Some(&b)), 1);
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn half_written_tmp_is_never_drained() {
        with_clean_env(|| {
            let b = temp_bridge("tmp");
            let d = spool_dir("fleet", Some(&b));
            fs::create_dir_all(&d).expect("mkdir");
            fs::write(
                d.join("0000000000000000001-0000001-0001.tmp"),
                "half written",
            )
            .expect("write tmp");
            assert_eq!(pending_count("fleet", Some(&b)), 0);
            assert_eq!(collect_drain("fleet", &b, 0), Vec::<String>::new());
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn metrics_name_backend_and_reason() {
        with_clean_env(|| {
            let b = temp_bridge("metric");
            let _ = spool_message("cabal", "x", Some(&b), "no_agent_reader");
            let m = fs::read_to_string(b.join(".metrics.log")).expect("metrics");
            assert!(m.contains("backend=cabal"), "m={m}");
            assert!(m.contains("reason=no_agent_reader"), "m={m}");
            assert!(m.contains("pending=1"), "m={m}");
            let _ = fs::remove_dir_all(&b);
        });
    }

    #[test]
    fn safe_backend_strips_traversal() {
        // Python: replace unsafe chars, then strip(".") → "_.._etc"
        assert_eq!(safe_backend("../../etc"), "_.._etc");
        assert_eq!(safe_backend("fleet"), "fleet");
        assert_eq!(safe_backend("..."), "_");
        assert_eq!(safe_backend("a/b"), "a_b");
    }
}
