#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/oasis7-liveops-grant.sh status [beneficiary_account_id] [options]
  ./scripts/oasis7-liveops-grant.sh issue <beneficiary_account_id> <amount> <issuance_reason> <expires_at_epoch> [options]
  ./scripts/oasis7-liveops-grant.sh revoke <beneficiary_account_id> <revoke_reason> [options]

Description:
  Thin liveops wrapper around `oasis7_liveops_grant_cli`.
  It keeps the same runtime/governance boundary, but removes the need to type
  the full `cargo run -p oasis7 --bin oasis7_liveops_grant_cli -- ...` command.

Options:
  --world-dir <dir>   World directory. Falls back to $OASIS7_WORLD_DIR.
  --issuer-id <id>    Issuer account id. Defaults to $OASIS7_LIVEOPS_ISSUER_ID
                      or `liveops`.
  --json              Forward --json to the underlying CLI.
  --dry-run           Forward --dry-run to the underlying CLI.
  --print-cmd         Print the resolved command before executing it.
  --cli-bin <path>    Run a prebuilt CLI binary directly instead of `cargo run`.
  -h, --help          Show this help.

Examples:
  ./scripts/oasis7-liveops-grant.sh status
  ./scripts/oasis7-liveops-grant.sh status player.alice --world-dir ./output/world
  ./scripts/oasis7-liveops-grant.sh issue player.alice 325 preview_allowlist 48 --world-dir ./output/world
  ./scripts/oasis7-liveops-grant.sh revoke player.alice qa_window_closed --world-dir ./output/world
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

quote_cmd() {
  local quoted=()
  local token
  for token in "$@"; do
    quoted+=("$(printf '%q' "$token")")
  done
  printf '%s\n' "${quoted[*]}"
}

COMMAND="${1:-}"
if [[ -z "$COMMAND" ]]; then
  usage
  exit 1
fi
shift || true

case "$COMMAND" in
  status|issue|revoke)
    ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    die "unknown command: $COMMAND"
    ;;
esac

WORLD_DIR="${OASIS7_WORLD_DIR:-}"
ISSUER_ID="${OASIS7_LIVEOPS_ISSUER_ID:-liveops}"
JSON_MODE=0
DRY_RUN=0
PRINT_CMD=0
CLI_BIN="${OASIS7_LIVEOPS_GRANT_CLI_BIN:-}"
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --world-dir)
      WORLD_DIR="${2:-}"
      shift 2
      ;;
    --issuer-id)
      ISSUER_ID="${2:-}"
      shift 2
      ;;
    --json)
      JSON_MODE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --print-cmd)
      PRINT_CMD=1
      shift
      ;;
    --cli-bin)
      CLI_BIN="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      while [[ $# -gt 0 ]]; do
        POSITIONAL+=("$1")
        shift
      done
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

[[ -n "$WORLD_DIR" ]] || die "--world-dir is required (or set OASIS7_WORLD_DIR)"
[[ -d "$WORLD_DIR" ]] || die "world dir does not exist: $WORLD_DIR"
[[ -n "$ISSUER_ID" ]] || die "--issuer-id cannot be empty"

CLI_ARGS=("$COMMAND" "--world-dir" "$WORLD_DIR" "--issuer-id" "$ISSUER_ID")

case "$COMMAND" in
  status)
    if [[ "${#POSITIONAL[@]}" -gt 1 ]]; then
      die "status accepts at most 1 positional argument: [beneficiary_account_id]"
    fi
    if [[ "${#POSITIONAL[@]}" -eq 1 ]]; then
      CLI_ARGS+=("--beneficiary-account-id" "${POSITIONAL[0]}")
    fi
    ;;
  issue)
    if [[ "${#POSITIONAL[@]}" -ne 4 ]]; then
      die "issue requires 4 positional arguments: <beneficiary_account_id> <amount> <issuance_reason> <expires_at_epoch>"
    fi
    CLI_ARGS+=(
      "--beneficiary-account-id" "${POSITIONAL[0]}"
      "--amount" "${POSITIONAL[1]}"
      "--issuance-reason" "${POSITIONAL[2]}"
      "--expires-at-epoch" "${POSITIONAL[3]}"
    )
    ;;
  revoke)
    if [[ "${#POSITIONAL[@]}" -ne 2 ]]; then
      die "revoke requires 2 positional arguments: <beneficiary_account_id> <revoke_reason>"
    fi
    CLI_ARGS+=(
      "--beneficiary-account-id" "${POSITIONAL[0]}"
      "--revoke-reason" "${POSITIONAL[1]}"
    )
    ;;
esac

if [[ "$JSON_MODE" == "1" ]]; then
  CLI_ARGS+=("--json")
fi
if [[ "$DRY_RUN" == "1" ]]; then
  CLI_ARGS+=("--dry-run")
fi

if [[ -n "$CLI_BIN" ]]; then
  CMD=("$CLI_BIN" "${CLI_ARGS[@]}")
else
  CMD=(env -u RUSTC_WRAPPER cargo run -q -p oasis7 --bin oasis7_liveops_grant_cli -- "${CLI_ARGS[@]}")
fi

if [[ "$PRINT_CMD" == "1" ]]; then
  quote_cmd "${CMD[@]}"
fi

exec "${CMD[@]}"
