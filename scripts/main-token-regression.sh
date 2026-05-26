#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/cargo-dev-lib.sh"

TIER="${1:-required}"

usage() {
  cat <<'USAGE'
Usage: scripts/main-token-regression.sh [required|full]

Runs main-token and NodePoints-bridge focused regression suites.
USAGE
}

run() {
  echo "+ $*"
  "$@"
}

case "${TIER}" in
  required)
    run oasis7_cargo_dev test -p oasis7 --features test_tier_required runtime::tests::main_token:: -- --nocapture
    run oasis7_cargo_dev test -p oasis7 --features test_tier_required runtime::tests::reward_asset_settlement_action:: -- --nocapture
    ;;
  full)
    run oasis7_cargo_dev test -p oasis7 --features test_tier_required runtime::tests::main_token:: -- --nocapture
    run oasis7_cargo_dev test -p oasis7 --features test_tier_required runtime::tests::reward_asset_settlement_action:: -- --nocapture
    run oasis7_cargo_dev test -p oasis7 --features test_tier_full runtime::tests::main_token:: -- --nocapture
    run oasis7_cargo_dev test -p oasis7 --features test_tier_full runtime::tests::reward_asset_settlement_action:: -- --nocapture
    run oasis7_cargo_dev test -p oasis7 --features test_tier_full runtime::tests::reward_asset:: -- --nocapture
    ;;
  *)
    usage
    exit 1
    ;;
esac
