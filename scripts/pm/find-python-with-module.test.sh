#!/usr/bin/env bash
# Cross-platform test contract: verify Windows shim rejection while preserving Linux/macOS interpreter discovery.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if [[ -n "${OASIS7_TEST_PYTHON:-}" ]]; then
  REAL_PYTHON="$OASIS7_TEST_PYTHON"
else
  REAL_PYTHON="$("$ROOT_DIR/scripts/pm/find-python-with-module.sh" ast)"
fi

if [[ ! -x "$REAL_PYTHON" ]] || ! "$REAL_PYTHON" -c 'import ast; print("ready")' | grep -Fxq ready; then
  echo "find-python-with-module.test: OASIS7_TEST_PYTHON or PATH discovery must provide a functional Python interpreter" >&2
  exit 1
fi

mkdir -p "$TMPDIR/broken-bin" "$TMPDIR/working-bin"
for name in python python3; do
  cat >"$TMPDIR/broken-bin/$name" <<'SH'
#!/usr/bin/env bash
# Simulates the Windows user-level python shim that exits 0 without executing input.
exit 0
SH
  chmod +x "$TMPDIR/broken-bin/$name"
done
ln -s "$REAL_PYTHON" "$TMPDIR/working-bin/python42"

selected="$(PATH="$TMPDIR/broken-bin:$TMPDIR/working-bin:$PATH" \
  "$ROOT_DIR/scripts/pm/find-python-with-module.sh" ast)"
if [[ "$selected" == "$TMPDIR/broken-bin/"* ]]; then
  echo "find-python-with-module.test: accepted a non-executing interpreter: $selected" >&2
  exit 1
fi
if ! "$selected" -c 'import ast; print("selected")' | grep -Fxq selected; then
  echo "find-python-with-module.test: selected interpreter cannot execute Python" >&2
  exit 1
fi

echo "find-python-with-module.test: OK"
