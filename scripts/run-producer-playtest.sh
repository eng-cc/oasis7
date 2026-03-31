#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

BUNDLE_DIR=""
PROFILE="release"
REBUILD=0
OPEN_HEADED=0
SESSION_NAME="producer-playtest"
STARTUP_TIMEOUT_SECS=120
STARTUP_LOG=""
STACK_ARGS=()

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-producer-playtest.sh [options] [run-game-test options...]

Prepare a bundle-first Web stack for producer manual play.

Default behavior:
- reuse the current worktree-local producer bundle if it already exists and is fresh
- otherwise build or rebuild a fresh bundle there
- then start `./scripts/run-game-test.sh --bundle-dir <bundle>`
- when `--open-headed` is used, `agent-browser` defaults to hardware WebGL args
  `--use-angle=gl,--ignore-gpu-blocklist` (override with `AGENT_BROWSER_ARGS`)

Options:
  --bundle-dir <path>      Bundle directory to reuse/build (default: output/harness/<worktree>/bundle/game-launcher-producer-local)
  --profile <name>         Bundle build profile: release|dev (default: release)
  --rebuild                Force rebuild even if bundle already exists
  --open-headed            After stack ready, auto-open the Viewer URL in headed `agent-browser`
                           with default hardware WebGL args, and close that browser session when
                           the script exits
  --startup-log <path>     Override startup log path used by --open-headed mode
  --session <name>         `agent-browser` session name for `--open-headed` (default: producer-playtest)
  --startup-timeout <secs> Wait timeout for stack URL when `--open-headed` is used (default: 120)
  -h, --help               Show this help

Examples:
  ./scripts/run-producer-playtest.sh
  ./scripts/run-producer-playtest.sh --profile dev
  ./scripts/run-producer-playtest.sh --open-headed
  ./scripts/run-producer-playtest.sh --bundle-dir output/release/game-launcher-local
  ./scripts/run-producer-playtest.sh --no-llm   # negative-path only; launcher boot is expected to fail
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --profile)
      PROFILE="${2:-}"
      shift 2
      ;;
    --rebuild)
      REBUILD=1
      shift
      ;;
    --open-headed)
      OPEN_HEADED=1
      shift
      ;;
    --startup-log)
      STARTUP_LOG="${2:-}"
      shift 2
      ;;
    --session)
      SESSION_NAME="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      STARTUP_TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      STACK_ARGS+=("$1")
      shift
      ;;
  esac
done

[[ "$PROFILE" == "release" || "$PROFILE" == "dev" ]] || { echo "error: --profile must be release or dev" >&2; exit 2; }
[[ -n "$SESSION_NAME" ]] || { echo "error: --session cannot be empty" >&2; exit 2; }
[[ "$STARTUP_TIMEOUT_SECS" =~ ^[0-9]+$ ]] && [[ "$STARTUP_TIMEOUT_SECS" -gt 0 ]] || { echo "error: --startup-timeout must be a positive integer" >&2; exit 2; }

if [[ -z "$BUNDLE_DIR" ]]; then
  BUNDLE_DIR="$(wh_default_producer_bundle_dir "$(wh_harness_root "$ROOT_DIR" "$(wh_worktree_id)")")"
fi

if [[ "$BUNDLE_DIR" != /* ]]; then
  ABS_BUNDLE_DIR="$ROOT_DIR/$BUNDLE_DIR"
else
  ABS_BUNDLE_DIR="$BUNDLE_DIR"
fi

BUNDLE_REBUILD_REASON=""
if [[ "$REBUILD" == "1" ]]; then
  BUNDLE_REBUILD_REASON="forced by --rebuild"
elif [[ ! -x "$ABS_BUNDLE_DIR/run-game.sh" ]]; then
  BUNDLE_REBUILD_REASON="bundle missing run-game.sh"
elif ! freshness_note=$(bundle_check_freshness "$ROOT_DIR" "$ABS_BUNDLE_DIR" 2>&1); then
  echo "info: stale producer bundle detected: $freshness_note"
  BUNDLE_REBUILD_REASON="workspace drift detected"
fi

if [[ -n "$BUNDLE_REBUILD_REASON" ]]; then
  echo "info: preparing producer playtest bundle at $ABS_BUNDLE_DIR (profile=$PROFILE, reason=$BUNDLE_REBUILD_REASON)"
  ./scripts/build-game-launcher-bundle.sh --profile "$PROFILE" --out-dir "$ABS_BUNDLE_DIR"
else
  echo "info: reusing existing producer playtest bundle at $ABS_BUNDLE_DIR"
fi

if [[ "$OPEN_HEADED" != "1" ]]; then
  exec ./scripts/run-game-test.sh --bundle-dir "$ABS_BUNDLE_DIR" "${STACK_ARGS[@]}"
fi

ab_require
WORKTREE_HARNESS_ROOT="$(wh_harness_root "$ROOT_DIR" "$(wh_worktree_id)")"
mkdir -p "$WORKTREE_HARNESS_ROOT"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
if [[ -n "$STARTUP_LOG" ]]; then
  if [[ "$STARTUP_LOG" != /* ]]; then
    RUN_LOG="$ROOT_DIR/$STARTUP_LOG"
  else
    RUN_LOG="$STARTUP_LOG"
  fi
else
  RUN_LOG="$WORKTREE_HARNESS_ROOT/producer-launch-${RUN_ID}.log"
fi
mkdir -p "$(dirname "$RUN_LOG")"
STACK_PID=""
BROWSER_OPENED=0
META_FILE="$WORKTREE_HARNESS_ROOT/producer-launch-${RUN_ID}.meta"

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$STACK_PID" ]] && kill -0 "$STACK_PID" >/dev/null 2>&1; then
    kill "$STACK_PID" >/dev/null 2>&1 || true
    wait "$STACK_PID" >/dev/null 2>&1 || true
  fi
  if [[ "$BROWSER_OPENED" == "1" ]]; then
    ab_cmd "$SESSION_NAME" close >/dev/null 2>&1 || true
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

if command -v stdbuf >/dev/null 2>&1; then
  stdbuf -oL -eL ./scripts/run-game-test.sh --bundle-dir "$ABS_BUNDLE_DIR" --meta-file "$META_FILE" "${STACK_ARGS[@]}" > >(tee "$RUN_LOG") 2>&1 &
else
  ./scripts/run-game-test.sh --bundle-dir "$ABS_BUNDLE_DIR" --meta-file "$META_FILE" "${STACK_ARGS[@]}" > >(tee "$RUN_LOG") 2>&1 &
fi
STACK_PID=$!

GAME_URL=""
STACK_OUTPUT_DIR=""
for ((i = 0; i < STARTUP_TIMEOUT_SECS; i++)); do
  if ! kill -0 "$STACK_PID" >/dev/null 2>&1; then
    echo "error: producer playtest stack exited unexpectedly" >&2
    tail -n 120 "$RUN_LOG" >&2 || true
    exit 1
  fi
  GAME_URL="$(wh_env_file_get "$META_FILE" GAME_URL 2>/dev/null || true)"
  STACK_OUTPUT_DIR="$(wh_env_file_get "$META_FILE" OUTPUT_DIR 2>/dev/null || true)"
  [[ -n "$GAME_URL" ]] && break
  sleep 1
done

if [[ -z "$GAME_URL" ]]; then
  echo "error: timeout waiting for game URL from run-game-test.sh" >&2
  tail -n 120 "$RUN_LOG" >&2 || true
  exit 1
fi

BROWSER_ARGS=$(ab_browser_args)
echo "info: opening headed browser session '$SESSION_NAME' -> $GAME_URL"
if [[ -n "$BROWSER_ARGS" ]]; then
  echo "info: agent-browser args: $BROWSER_ARGS"
else
  echo "info: agent-browser args: <none>"
fi
ab_open "$SESSION_NAME" 1 "$GAME_URL"
BROWSER_OPENED=1
ab_cmd "$SESSION_NAME" wait --load networkidle >/dev/null 2>&1 || true

echo "info: browser session: $SESSION_NAME"
echo "info: startup log: $RUN_LOG"
echo "info: stack logs: ${STACK_OUTPUT_DIR:-unknown}"

echo "Press Ctrl+C to stop the producer playtest stack and close the opened browser session."
wait "$STACK_PID"
