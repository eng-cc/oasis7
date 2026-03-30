#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/scaffold.sh <role_name>

Scaffold one role directory under .pm/roles/<role_name>/ using the templates in
.pm/templates/. This command creates:
  - memory/active.yaml
  - memory/superseded.yaml
  - backlog/candidate.yaml
  - backlog/committed.yaml
  - backlog/blocked.yaml
  - backlog/done.yaml

The command does not modify .pm/registry/roles.yaml.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 2
fi

ROLE_NAME="$1"
[[ -n "$ROLE_NAME" ]] || { echo "pm-scaffold: role_name cannot be empty" >&2; exit 2; }

render_template() {
  local template="$1"
  local target="$2"
  local status="${3:-}"

  sed \
    -e "s/__ROLE_NAME__/${ROLE_NAME}/g" \
    -e "s/__STATUS__/${status}/g" \
    "$template" > "$target"
}

ROLE_DIR=".pm/roles/${ROLE_NAME}"
mkdir -p "${ROLE_DIR}/memory" "${ROLE_DIR}/backlog"

render_template ".pm/templates/role-memory-active.yaml" "${ROLE_DIR}/memory/active.yaml"
render_template ".pm/templates/role-memory-superseded.yaml" "${ROLE_DIR}/memory/superseded.yaml"
render_template ".pm/templates/role-backlog.yaml" "${ROLE_DIR}/backlog/candidate.yaml" "candidate"
render_template ".pm/templates/role-backlog.yaml" "${ROLE_DIR}/backlog/committed.yaml" "committed"
render_template ".pm/templates/role-backlog.yaml" "${ROLE_DIR}/backlog/blocked.yaml" "blocked"
render_template ".pm/templates/role-backlog.yaml" "${ROLE_DIR}/backlog/done.yaml" "done"

echo "pm-scaffold: created ${ROLE_DIR}"
