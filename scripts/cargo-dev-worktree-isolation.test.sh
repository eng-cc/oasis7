#!/usr/bin/env bash
# Contract: divergent git worktrees must not share development Cargo artifacts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TMPDIR="$(mktemp -d)"
REPO="$TMPDIR/repo"
WORKTREE_A="$TMPDIR/worktree-a"
WORKTREE_B="$TMPDIR/worktree-b"
FAKE_BIN="$TMPDIR/fake-bin"
REAL_CARGO="$(command -v cargo)"
PYTHON_BIN="$("$ROOT_DIR/scripts/pm/find-python-with-module.sh" ast)"

cleanup() {
  set +e
  git -C "$REPO" worktree remove --force "$WORKTREE_A" >/dev/null 2>&1 || true
  git -C "$REPO" worktree remove --force "$WORKTREE_B" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$REPO/scripts/pm" "$FAKE_BIN"
cp "$ROOT_DIR/scripts/cargo-dev.sh" "$REPO/scripts/cargo-dev.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/find-python-with-module.sh"
chmod +x "$REPO/scripts/cargo-dev.sh" "$REPO/scripts/pm/find-python-with-module.sh"
mkdir -p "$REPO/proto/src" "$REPO/consumer/src"
cat >"$REPO/Cargo.toml" <<'EOF'
[workspace]
members = ["proto", "consumer"]
resolver = "2"
EOF
cat >"$REPO/proto/Cargo.toml" <<'EOF'
[package]
name = "proto"
version = "0.1.0"
edition = "2021"
EOF
cat >"$REPO/consumer/Cargo.toml" <<'EOF'
[package]
name = "consumer"
version = "0.1.0"
edition = "2021"

[dependencies]
proto = { path = "../proto" }
EOF
cat >"$REPO/proto/src/lib.rs" <<'EOF'
pub struct Ack { pub field_a: u8 }
EOF
cat >"$REPO/consumer/src/lib.rs" <<'EOF'
use proto::Ack;

pub fn read(a: Ack) -> u8 { a.field_a }
EOF

cat >"$FAKE_BIN/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  printf '%s\n' 'cargo 1.0.0-fixture'
  exit 0
fi
mkdir -p "${CARGO_TARGET_DIR:?}/fixture-artifacts"
printf '%s\n' "${FAKE_SOURCE_ID:?}" >"$CARGO_TARGET_DIR/fixture-artifacts/oasis7_proto-source.txt"
EOF
chmod +x "$FAKE_BIN/cargo"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name 'Cargo cache fixture'
git -C "$REPO" add .
git -C "$REPO" commit -qm base
env -u RUSTC_WRAPPER "$REAL_CARGO" generate-lockfile --manifest-path "$REPO/Cargo.toml" --offline >/dev/null
git -C "$REPO" add Cargo.lock
git -C "$REPO" commit -qm lockfile
git -C "$REPO" checkout -qb divergent-a
git -C "$REPO" checkout -q main
git -C "$REPO" checkout -qb divergent-b
"$PYTHON_BIN" - "$REPO/proto/src/lib.rs" "$REPO/consumer/src/lib.rs" <<'PY'
from pathlib import Path
import sys

proto, consumer = map(Path, sys.argv[1:])
proto.write_text("pub struct Ack { pub field_b: u8 }\n", encoding="utf-8")
consumer.write_text(
    "use proto::Ack;\n\npub fn read(a: Ack) -> u8 { a.field_b }\n",
    encoding="utf-8",
)
PY
git -C "$REPO" add proto/src/lib.rs consumer/src/lib.rs
git -C "$REPO" commit -qam divergent-b
git -C "$REPO" checkout -q main
git -C "$REPO" worktree add -q "$WORKTREE_A" divergent-a
git -C "$REPO" worktree add -q "$WORKTREE_B" divergent-b

TARGET_A="$(cd "$WORKTREE_A" && PATH="$FAKE_BIN:$PATH" ./scripts/cargo-dev.sh --print-target-dir)"
TARGET_B="$(cd "$WORKTREE_B" && PATH="$FAKE_BIN:$PATH" ./scripts/cargo-dev.sh --print-target-dir)"
TARGET_A_AGAIN="$(cd "$WORKTREE_A" && PATH="$FAKE_BIN:$PATH" ./scripts/cargo-dev.sh --print-target-dir)"
if [[ "$TARGET_A" != "$TARGET_A_AGAIN" ]]; then
  echo "same worktree did not reuse its stable Cargo target namespace" >&2
  exit 1
fi
if [[ "$TARGET_A" == "$TARGET_B" ]]; then
  echo "divergent worktrees resolved the same Cargo target namespace: $TARGET_A" >&2
  exit 1
fi

LEGACY_CACHE="$TMPDIR/.oasis7-cache/cargo-target/git-legacy/rustc-legacy"
mkdir -p "$LEGACY_CACHE"
printf '%s\n' 'must-survive-wrapper-migration' >"$LEGACY_CACHE/sentinel.txt"
ln -s "$LEGACY_CACHE" "$WORKTREE_A/target"

(cd "$WORKTREE_A" && PATH="$FAKE_BIN:$PATH" FAKE_SOURCE_ID=divergent-a ./scripts/cargo-dev.sh build -p oasis7_proto)
(cd "$WORKTREE_B" && PATH="$FAKE_BIN:$PATH" FAKE_SOURCE_ID=divergent-b ./scripts/cargo-dev.sh build -p oasis7_proto)

ACTUAL_TARGET_A="$("$PYTHON_BIN" - "$WORKTREE_A/target" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"
if [[ "$ACTUAL_TARGET_A" != "$TARGET_A" ]]; then
  echo "cargo-dev did not migrate the legacy target symlink: $ACTUAL_TARGET_A != $TARGET_A" >&2
  exit 1
fi
if [[ "$(<"$LEGACY_CACHE/sentinel.txt")" != must-survive-wrapper-migration ]]; then
  echo "cargo-dev changed data in the legacy target cache" >&2
  exit 1
fi
(cd "$WORKTREE_A" && env -u CARGO_TARGET_DIR "$REAL_CARGO" test --manifest-path Cargo.toml --locked --offline)
(cd "$WORKTREE_B" && CARGO_TARGET_DIR="$TARGET_B" "$REAL_CARGO" test --manifest-path Cargo.toml --locked --offline)
if [[ "$(<"$TARGET_A/fixture-artifacts/oasis7_proto-source.txt")" != divergent-a ]]; then
  echo "worktree A artifact marker was contaminated by another worktree" >&2
  exit 1
fi
if [[ "$(<"$TARGET_B/fixture-artifacts/oasis7_proto-source.txt")" != divergent-b ]]; then
  echo "worktree B artifact marker was contaminated by another worktree" >&2
  exit 1
fi

EXTERNAL_CACHE="$TMPDIR/external-cache"
mkdir -p "$EXTERNAL_CACHE"
printf '%s\n' external >"$EXTERNAL_CACHE/sentinel.txt"
ln -s "$EXTERNAL_CACHE" "$WORKTREE_B/target"
set +e
(cd "$WORKTREE_B" && PATH="$FAKE_BIN:$PATH" ./scripts/cargo-dev.sh build -p oasis7_proto) \
  >"$TMPDIR/external.out" 2>"$TMPDIR/external.err"
rc=$?
set -e
if [[ "$rc" == "0" ]] || ! grep -Fq 'outside the managed Cargo cache' "$TMPDIR/external.err"; then
  echo "cargo-dev accepted an unmanaged target symlink" >&2
  cat "$TMPDIR/external.err" >&2
  exit 1
fi
if [[ "$(<"$EXTERNAL_CACHE/sentinel.txt")" != external ]]; then
  echo "cargo-dev changed data in the unmanaged cache" >&2
  exit 1
fi

echo "cargo-dev-worktree-isolation.test: OK"
