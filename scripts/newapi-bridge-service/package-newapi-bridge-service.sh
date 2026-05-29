#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/newapi-bridge-service/package-newapi-bridge-service.sh [options]

Build and stage the deployable oasis7_newapi_bridge_service package.

Options:
  --profile <release|dev>  Cargo profile to build (default: release)
  --out-dir <path>         Staging directory (default: output/newapi-bridge-service)
  --archive <path>         Archive path (default: output/newapi-bridge-service-linux-x86_64.tar.gz)
  -h, --help               Show help
USAGE
}

die() {
  echo "package-newapi-bridge-service: $*" >&2
  exit 1
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

PROFILE="release"
OUT_DIR="output/newapi-bridge-service"
ARCHIVE_PATH="output/newapi-bridge-service-linux-x86_64.tar.gz"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --archive)
      ARCHIVE_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

case "$PROFILE" in
  release)
    BINARY_DIR="release"
    ;;
  dev)
    BINARY_DIR="debug"
    ;;
  *)
    die "unsupported profile: $PROFILE"
    ;;
esac

sha256_emit() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$@"
    return
  fi
  shasum -a 256 "$@"
}

resolve_git_ref_name() {
  if [[ -n "${GITHUB_REF_NAME:-}" ]]; then
    printf '%s\n' "$GITHUB_REF_NAME"
    return
  fi
  git branch --show-current
}

resolve_git_ref() {
  if [[ -n "${GITHUB_REF:-}" ]]; then
    printf '%s\n' "$GITHUB_REF"
    return
  fi
  local branch
  branch="$(git branch --show-current)"
  if [[ -n "$branch" ]]; then
    printf 'refs/heads/%s\n' "$branch"
  else
    git rev-parse HEAD
  fi
}

HOST_TRIPLE="$(rustc -vV | awk '/^host: / { print $2 }')"
GIT_SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"
GIT_REF="$(resolve_git_ref)"
GIT_REF_NAME="$(resolve_git_ref_name)"
TMP_DIR=".tmp/newapi-bridge-service"

mkdir -p "$TMP_DIR" "$(dirname "$ARCHIVE_PATH")"

CARGO_CMD=(env -u RUSTC_WRAPPER cargo build)
if [[ "$PROFILE" == "release" ]]; then
  CARGO_CMD+=(--release)
fi
CARGO_CMD+=(-p oasis7 --bin oasis7_newapi_bridge_service)
"${CARGO_CMD[@]}"

"target/${BINARY_DIR}/oasis7_newapi_bridge_service" \
  --help > "$TMP_DIR/HELP.txt"

rm -rf "$OUT_DIR"
mkdir -p \
  "$OUT_DIR/scripts/newapi-bridge-service" \
  "$OUT_DIR/doc/world-runtime/runtime"

install -m 0755 \
  "target/${BINARY_DIR}/oasis7_newapi_bridge_service" \
  "$OUT_DIR/oasis7_newapi_bridge_service"
install -m 0755 \
  scripts/newapi-bridge-service/start-newapi-bridge-service.sh \
  "$OUT_DIR/scripts/newapi-bridge-service/start-newapi-bridge-service.sh"
install -m 0644 \
  scripts/newapi-bridge-service/newapi-bridge-service.env.example \
  "$OUT_DIR/scripts/newapi-bridge-service/newapi-bridge-service.env.example"
install -m 0644 \
  scripts/newapi-bridge-service/oasis7-newapi-bridge.service \
  "$OUT_DIR/scripts/newapi-bridge-service/oasis7-newapi-bridge.service"
install -m 0644 \
  doc/world-runtime/runtime/newapi-bridge-service-operator-runbook.md \
  "$OUT_DIR/doc/world-runtime/runtime/newapi-bridge-service-operator-runbook.md"
install -m 0644 \
  "$TMP_DIR/HELP.txt" \
  "$OUT_DIR/HELP.txt"

{
  echo "git_sha=${GIT_SHA}"
  echo "git_ref=${GIT_REF}"
  echo "git_ref_name=${GIT_REF_NAME}"
  echo "profile=${PROFILE}"
  echo "target=${HOST_TRIPLE}"
  echo "binary=oasis7_newapi_bridge_service"
  echo "service=scripts/newapi-bridge-service/oasis7-newapi-bridge.service"
  echo "start_script=scripts/newapi-bridge-service/start-newapi-bridge-service.sh"
  echo "env_example=scripts/newapi-bridge-service/newapi-bridge-service.env.example"
  echo "runbook=doc/world-runtime/runtime/newapi-bridge-service-operator-runbook.md"
} > "$OUT_DIR/BUILDINFO"

(
  cd "$OUT_DIR"
  sha256_emit \
    oasis7_newapi_bridge_service \
    scripts/newapi-bridge-service/start-newapi-bridge-service.sh \
    scripts/newapi-bridge-service/newapi-bridge-service.env.example \
    scripts/newapi-bridge-service/oasis7-newapi-bridge.service \
    doc/world-runtime/runtime/newapi-bridge-service-operator-runbook.md \
    HELP.txt \
    BUILDINFO > SHA256SUMS
  tar -czf "$ROOT_DIR/$ARCHIVE_PATH" .
)

echo "NewAPI bridge service package written to $ARCHIVE_PATH"
