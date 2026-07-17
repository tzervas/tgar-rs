#!/usr/bin/env bash
# Offline dual-run smoke: exercise Rust `tgar` (and Python when available).
# WHAT: version + send dry-run + config check against a fixture.
# WHY: strangler exit criteria without Telegram network (P22d / P28f).
# WHY NOT: full format/route golden yet — land as phases add subcommands.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${TGAR_BIN:-$ROOT/target/debug/tgar}"
# Always rebuild default binary unless TGAR_BIN is overridden (keeps CLI surface current).
if [[ -z "${TGAR_BIN:-}" || "${TGAR_FORCE_BUILD:-}" == "1" ]]; then
  echo "dual-run-smoke: building tgar…"
  cargo build -p tgar --manifest-path "$ROOT/Cargo.toml" -q
  BIN="$ROOT/target/debug/tgar"
elif [[ ! -x "$BIN" ]]; then
  echo "dual-run-smoke: TGAR_BIN not executable: $BIN" >&2
  exit 1
fi

FIXTURE="${TGAR_CONFIG_FIXTURE:-$ROOT/fixtures/relay.minimal.toml}"
RELAY_ROOT="${TGAR_RELAY_ROOT:-}"
if [[ -z "$RELAY_ROOT" && -d "$ROOT/../tg-agent-relay" ]]; then
  RELAY_ROOT="$(cd "$ROOT/../tg-agent-relay" && pwd)"
fi

pass=0
fail=0

ok() { echo "  OK  $*"; pass=$((pass + 1)); }
bad() { echo "  FAIL $*"; fail=$((fail + 1)); }

echo "== Rust: tgar version =="
ver="$("$BIN" version)"
if [[ "$ver" == *"0.1.0"* ]]; then
  ok "version=$ver"
else
  bad "unexpected version: $ver"
fi

echo "== Rust: tgar send --dry-run =="
out="$("$BIN" send --text "dual-run smoke" --dry-run 2>/dev/null || true)"
if [[ "$out" == *"dual-run smoke"* ]]; then
  ok "send dry-run prints body"
else
  bad "send dry-run missing body (out=${out//$'\n'/\\n})"
fi

echo "== Rust: tgar config check =="
if [[ -f "$FIXTURE" ]]; then
  cfg_out="$("$BIN" config check --path "$FIXTURE")"
  if [[ "$cfg_out" == ok:* ]]; then
    ok "config check: $cfg_out"
  else
    bad "config check: $cfg_out"
  fi
else
  bad "fixture missing: $FIXTURE"
fi

if [[ -n "$RELAY_ROOT" && -d "$RELAY_ROOT" ]]; then
  echo "== Python side (optional): $RELAY_ROOT =="
  if command -v python3 >/dev/null 2>&1; then
    if python3 -c "import sys; sys.path.insert(0, '$RELAY_ROOT'); import tg_agent_relay" 2>/dev/null; then
      py_ver="$(python3 -c "import sys; sys.path.insert(0, '$RELAY_ROOT'); import tg_agent_relay as t; print(getattr(t, '__version__', 'unknown'))" 2>/dev/null || echo unknown)"
      ok "python package importable (version=$py_ver)"
    else
      echo "  SKIP python package not importable from $RELAY_ROOT"
    fi
  else
    echo "  SKIP no python3"
  fi
else
  echo "== Python side: SKIP (set TGAR_RELAY_ROOT to sibling tg-agent-relay) =="
fi

echo
echo "dual-run-smoke: pass=$pass fail=$fail"
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
echo "dual-run-smoke: green (offline Rust path)"
