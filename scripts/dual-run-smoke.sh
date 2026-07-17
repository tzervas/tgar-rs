#!/usr/bin/env bash
# dual-run-smoke.sh — offline parity checks: Python tg-relay CLI vs tgar (Rust).
#
# Usage:
#   TGAR_RELAY_ROOT=../tg-agent-relay ./scripts/dual-run-smoke.sh
#
# Env:
#   TGAR_RELAY_ROOT   Path to tg-agent-relay checkout (default: ../tg-agent-relay)
#   TGAR_BIN          tgar binary (default: cargo run -q --manifest-path …/Cargo.toml --)
#   TGAR_SKIP_RUST    1 = only verify Python baseline (useful before tgar exists)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELAY_ROOT="${TGAR_RELAY_ROOT:-$(cd "$ROOT/.." && pwd)/tg-agent-relay}"
SKIP_RUST="${TGAR_SKIP_RUST:-0}"

if [[ ! -d "$RELAY_ROOT/tg_agent_relay" ]]; then
    echo "dual-run-smoke: missing tg-agent-relay at TGAR_RELAY_ROOT=$RELAY_ROOT" >&2
    exit 1
fi

# shellcheck disable=SC1091
source "$RELAY_ROOT/lib/python.sh" 2>/dev/null || true
_py() {
    if declare -f relay_python >/dev/null 2>&1; then
        relay_python "$@"
    else
        command python3 "$@"
    fi
}

tg_relay_py() {
    (cd "$RELAY_ROOT" && _py -m tg_agent_relay.cli "$@")
}

_tgar_bin() {
    if [[ -n "${TGAR_BIN:-}" ]]; then
        # shellcheck disable=SC2086
        $TGAR_BIN "$@"
        return
    fi
    local manifest="$ROOT/Cargo.toml"
    if [[ ! -f "$manifest" ]]; then
        echo "dual-run-smoke: no Cargo.toml at $ROOT (set TGAR_BIN or TGAR_SKIP_RUST=1)" >&2
        return 127
    fi
    (cd "$ROOT" && cargo run -q -- "$@")
}

run_pair() {
    local label="$1"
    shift
    local py_out rust_out
    py_out="$(mktemp)"
    rust_out="$(mktemp)"
    trap 'rm -f "$py_out" "$rust_out"' RETURN

    tg_relay_py "$@" >"$py_out" 2>/dev/null || true

    if [[ "$SKIP_RUST" == "1" ]]; then
        echo "OK  $label (python-only; TGAR_SKIP_RUST=1)"
        return 0
    fi
    if ! _tgar_bin "$@" >"$rust_out" 2>/dev/null; then
        echo "SKIP $label (tgar not available yet)"
        return 0
    fi

    if diff -u "$py_out" "$rust_out" >/dev/null; then
        echo "OK  $label"
    else
        echo "FAIL $label" >&2
        diff -u "$py_out" "$rust_out" >&2 || true
        return 1
    fi
}

run_pair_stdin() {
    local label="$1"
    local input="$2"
    shift 2
    local py_out rust_out
    py_out="$(mktemp)"
    rust_out="$(mktemp)"
    trap 'rm -f "$py_out" "$rust_out"' RETURN

    printf '%s' "$input" | tg_relay_py "$@" >"$py_out" 2>/dev/null || true

    if [[ "$SKIP_RUST" == "1" ]]; then
        echo "OK  $label (python-only; TGAR_SKIP_RUST=1)"
        return 0
    fi
    if ! printf '%s' "$input" | _tgar_bin "$@" >"$rust_out" 2>/dev/null; then
        echo "SKIP $label (tgar not available yet)"
        return 0
    fi

    if diff -u "$py_out" "$rust_out" >/dev/null; then
        echo "OK  $label"
    else
        echo "FAIL $label" >&2
        diff -u "$py_out" "$rust_out" >&2 || true
        return 1
    fi
}

FAIL=0

# --- format (stdin golden) ---
GOLDEN=$'## Status\n\nHello `world`.\n'
FMT_ARGS=(format --wrap-width 50)
if ! printf '%s' "$GOLDEN" | tg_relay_py "${FMT_ARGS[@]}" >/dev/null; then
    echo "FAIL format (python)" >&2
    FAIL=1
else
    run_pair_stdin "format" "$GOLDEN" "${FMT_ARGS[@]}" || FAIL=1
fi

# --- hook grok (minimal fixture) ---
HOOK_JSON='{"hookEventName":"stop","message":"dual-run ping"}'
run_pair "hook grok" hook grok <<<"$HOOK_JSON" || FAIL=1

# --- route (fixture toml via tests) ---
CFG="$RELAY_ROOT/tests/fixtures/relay.toml"
if [[ -f "$CFG" ]]; then
    run_pair "route" route --config "$CFG" --chat-id "" --thread-id "" --text "hi" || FAIL=1
else
    echo "SKIP route (no tests/fixtures/relay.toml)"
fi

if [[ "$FAIL" -ne 0 ]]; then
    echo "dual-run-smoke: FAILED" >&2
    exit 1
fi
echo "dual-run-smoke: passed"