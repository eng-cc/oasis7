#!/usr/bin/env bash
# This claim-ready regression must remain cross-platform while exercising native Windows Python under Git Bash.
set -euo pipefail

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *) echo "claim-ready-native-python-git-path.test: SKIP (requires Git for Windows)"; exit 0 ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
NATIVE_PYTHON3="${TEST_NATIVE_PYTHON3:?set TEST_NATIVE_PYTHON3 to a native Windows Python executable}"
REAL_GIT="$(command -v git)"
FIXTURE_TMPDIR="$(mktemp -d)"
TMPDIR="$(cygpath -am "$FIXTURE_TMPDIR")"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
BIN="$TMPDIR/bin"
BIN_PATH="$(cygpath -u "$BIN")"
mkdir -p "$REPO" "$BIN"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf '.pm/\n' >"$REPO/.gitignore"
printf 'fixture\n' >"$REPO/file"
git -C "$REPO" add .gitignore file
git -C "$REPO" commit -qm fixture

cat >"$BIN/python3" <<EOF
#!/usr/bin/env bash
MSYS2_ENV_CONV_EXCL=PATH exec "$NATIVE_PYTHON3" "\$@"
EOF
chmod +x "$BIN/python3"
cat >"$BIN/git" <<EOF
#!/usr/bin/env bash
exec "$REAL_GIT" "\$@"
EOF
chmod +x "$BIN/git"

# Bash can resolve Git from this POSIX PATH, but native Windows Python cannot
# pass that spelling directly to CreateProcess when fingerprinting the snapshot.
RESTRICTED_PATH="$BIN_PATH:/usr/bin:/bin"
PATH="$RESTRICTED_PATH" command -v git >/dev/null

set +e
OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1 \
  PM_ROOT_DIR="$REPO" PATH="$RESTRICTED_PATH" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type task_complete \
  --verification-profile fixture_repository_state \
  --verify-command true --json \
  >"$TMPDIR/claim.json" 2>"$TMPDIR/claim.err"
status=$?
set -e
if [[ "$status" != 0 ]]; then
  cat "$TMPDIR/claim.err" >&2
  echo "claim-ready must make Git discoverable to native Windows Python" >&2
  exit 1
fi

"$NATIVE_PYTHON3" - "$TMPDIR/claim.json" <<'PY'
import json, sys

payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["status"] == "verified", payload
assert payload["allowed_to_claim"] is True, payload
assert payload["verification_mode"] == "detached_frozen_tree", payload
PY
test "$(git -C "$REPO" worktree list --porcelain | grep -c '^worktree ')" = 1
echo "claim-ready-native-python-git-path.test: OK"
