#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
  shared)
    cat <<'EOF'
Canonical local playtest entries:
- Isolated source-mode QA/subagent stack: ./scripts/worktree-harness.sh up
- Producer/release bundle-first playtest: ./scripts/run-producer-playtest.sh --open-headed
- Lower-level targeted bootstrap/debugging: ./scripts/run-launcher-stack.sh [options]

Mode boundary:
- source mode is the fast worktree development path
- bundle mode is the producer/release manual-play path
- provider-backed gameplay requires an explicitly reachable provider; negative-path flags never count as gameplay proof
EOF
    ;;
  *)
    echo "usage: $0 shared" >&2
    exit 2
    ;;
esac
