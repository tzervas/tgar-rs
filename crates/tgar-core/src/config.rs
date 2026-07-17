//! Minimal `relay.toml` load + check (Phase 0 stub).
//!
//! WHAT: Parse TOML, surface top-level tables + `[general].page_size`.
//! WHY: Strangler Phase 0 exit criteria include `tgar config check` before
//! full schema parity with Python `tg_agent_relay.config`.
//! WHY NOT: Full overlay (`.chats.d`, sessions) — later phases; keep tests offline.

use std::path::{Path, PathBuf};

/// Subset of effective relay config loaded from one TOML file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelayConfig {
    /// Path that was loaded.
    pub path: PathBuf,
    /// `[general].page_size` when set (Telegram pagination chars).
    pub page_size: Option<u64>,
    /// Top-level table names present in the file (e.g. `general`, `format`).
    pub tables: Vec<String>,
}

/// Load and lightly validate a `relay.toml` path.
///
/// Errors when the path is missing, empty, or not valid TOML.
pub fn load_config(path: &Path) -> Result<RelayConfig, String> {
    if !path.is_file() {
        return Err(format!("config not found: {}", path.display()));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    if text.trim().is_empty() {
        return Err(format!("config empty: {}", path.display()));
    }
    let value: toml::Value = text
        .parse()
        .map_err(|e| format!("toml parse {}: {e}", path.display()))?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("toml root not a table: {}", path.display()))?;

    let mut tables: Vec<String> = table.keys().cloned().collect();
    tables.sort();

    let page_size = table
        .get("general")
        .and_then(|g| g.get("page_size"))
        .and_then(|v| v.as_integer())
        .and_then(|i| u64::try_from(i).ok());

    Ok(RelayConfig {
        path: path.to_path_buf(),
        page_size,
        tables,
    })
}

/// Resolve default config path: `RELAY_CONFIG` env, else `relay.toml` in cwd.
pub fn default_config_path() -> PathBuf {
    std::env::var_os("RELAY_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("relay.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_minimal_toml() {
        let dir = std::env::temp_dir().join(format!(
            "tgar-config-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("relay.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "[general]\npage_size = 3500\n\n[format]\nenabled = true\n"
        )
        .unwrap();

        let cfg = load_config(&path).expect("load");
        assert_eq!(cfg.page_size, Some(3500));
        assert!(cfg.tables.iter().any(|t| t == "general"));
        assert!(cfg.tables.iter().any(|t| t == "format"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_errors() {
        let err = load_config(Path::new("/no/such/relay.toml")).unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }
}
