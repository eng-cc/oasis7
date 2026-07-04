#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/public-testnet-faucet/package-public-testnet-faucet.sh [--profile dev|release] [--out-dir <dir>] [--archive <path>]

Build and package the repo-owned public_testnet guarded faucet service bundle.
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PROFILE="release"
OUT_DIR="${REPO_ROOT}/output/public-testnet-faucet-service"
ARCHIVE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "--profile requires a value" >&2
        usage >&2
        exit 64
      fi
      PROFILE="${2:-}"
      shift 2
      ;;
    --out-dir)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "--out-dir requires a value" >&2
        usage >&2
        exit 64
      fi
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --archive)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "--archive requires a value" >&2
        usage >&2
        exit 64
      fi
      ARCHIVE="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

if [[ "$PROFILE" != "dev" && "$PROFILE" != "release" ]]; then
  echo "--profile must be dev or release: ${PROFILE}" >&2
  exit 64
fi

case "$(uname -s)" in
  Darwin) HOST_OS="darwin" ;;
  Linux) HOST_OS="linux" ;;
  *) HOST_OS="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
esac

HOST_ARCH="$(uname -m)"
HOST_TRIPLE="${HOST_ARCH}-${HOST_OS}"
STAGE_DIR="${OUT_DIR}/stage"
if [[ -z "$ARCHIVE" ]]; then
  ARCHIVE="${OUT_DIR}/public-testnet-faucet-service-${HOST_TRIPLE}.tar.gz"
fi

mkdir -p "$OUT_DIR"
rm -rf "$STAGE_DIR"
mkdir -p \
  "$STAGE_DIR/scripts/public-testnet-faucet" \
  "$STAGE_DIR/systemd" \
  "$STAGE_DIR/examples"

cd "$REPO_ROOT"
if [[ "$PROFILE" == "release" ]]; then
  env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_testnet_faucet
  BIN_PATH="${REPO_ROOT}/target/release/oasis7_testnet_faucet"
else
  env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_testnet_faucet
  BIN_PATH="${REPO_ROOT}/target/debug/oasis7_testnet_faucet"
fi

install -m 0755 "$BIN_PATH" "$STAGE_DIR/oasis7_testnet_faucet"
install -m 0755 "$SCRIPT_DIR/start-public-testnet-faucet.sh" \
  "$STAGE_DIR/scripts/public-testnet-faucet/start-public-testnet-faucet.sh"
install -m 0644 "$SCRIPT_DIR/oasis7-public-testnet-faucet.service" \
  "$STAGE_DIR/systemd/oasis7-public-testnet-faucet.service"
install -m 0644 "$SCRIPT_DIR/public-testnet-faucet.env.example" \
  "$STAGE_DIR/examples/public-testnet-faucet.env.example"
install -m 0644 "${REPO_ROOT}/doc/p2p/blockchain/p2p-public-testnet-faucet-operator-runbook-2026-07-04.md" \
  "$STAGE_DIR/RUNBOOK.md"

"$STAGE_DIR/oasis7_testnet_faucet" --help > "$STAGE_DIR/HELP.txt" 2>&1

{
  echo "created_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "profile=${PROFILE}"
  echo "host_triple=${HOST_TRIPLE}"
  echo "git_commit=$(git rev-parse HEAD)"
  echo "git_branch=$(git rev-parse --abbrev-ref HEAD)"
} > "$STAGE_DIR/BUILDINFO"

(
  cd "$STAGE_DIR"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 shasum -a 256
) > "$STAGE_DIR/SHA256SUMS"

mkdir -p "$(dirname "$ARCHIVE")"
tar -C "$STAGE_DIR" -czf "$ARCHIVE" .

echo "wrote ${ARCHIVE}"
