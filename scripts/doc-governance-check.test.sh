#!/usr/bin/env bash
# Cross-platform test contract: large doc scans must avoid Windows argv limits while preserving Linux/macOS results.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if REAL_RG="$(command -v rg 2>/dev/null)"; then
  :
else
  REAL_RG="$TMPDIR/rg-grep-backend"
  cat >"$REAL_RG" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
  if [[ "$argument" == *F* ]]; then
    exec grep "$@"
  fi
done
exec grep -E "$@"
SH
  chmod +x "$REAL_RG"
fi

if [[ -n "${OASIS7_TEST_PYTHON:-}" ]]; then
  REAL_PYTHON="$OASIS7_TEST_PYTHON"
else
  REAL_PYTHON="$("$ROOT_DIR/scripts/pm/find-python-with-module.sh" ast)"
fi

FIXTURE="$TMPDIR/repo"
mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/doc/.governance" "$FIXTURE/doc/testing" "$FIXTURE/doc/many" "$FIXTURE/doc/devlog" "$FIXTURE/.agents/roles/templates" "$TMPDIR/bin"
if [[ ! -x "$REAL_PYTHON" ]] || ! "$REAL_PYTHON" -c 'import ast; print("ready")' | grep -Fxq ready; then
  echo "doc-governance-check.test: OASIS7_TEST_PYTHON or PATH discovery must provide a functional Python interpreter" >&2
  exit 1
fi
cp "$ROOT_DIR/scripts/doc-governance-check.sh" "$FIXTURE/scripts/doc-governance-check.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$FIXTURE/scripts/pm/find-python-with-module.sh"
cat >"$FIXTURE/scripts/product-doc-governance-check.py" <<'PY'
#!/usr/bin/env python3
raise SystemExit(0)
PY
chmod +x "$FIXTURE/scripts/doc-governance-check.sh"

cat >"$FIXTURE/doc/testing/prd.md" <<'DOC'
## 目标
## 范围
## 接口 / 数据
## 里程碑
## 风险
DOC
for number in $(seq 1 80); do
  printf 'fixture document %s\n' "$number" >"$FIXTURE/doc/many/doc-$number.md"
done
printf '%s\n' 'doc/README.md' >"$FIXTURE/doc/.governance/doc-root-md-allowlist.txt"
printf '%s\n' 'fixture documentation landing page' >"$FIXTURE/doc/README.md"
{
  printf '%s\n' 'doc/testing/prd.md'
  find "$FIXTURE/doc/many" -type f -name '*.md' | sed "s#^$FIXTURE/##" | sort
} >"$FIXTURE/doc/.governance/module-root-md-allowlist.txt"

cat >"$TMPDIR/bin/rg" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >>"${RG_INVOCATION_LOG:?}"
printf '\n' >>"${RG_INVOCATION_LOG:?}"
has_files_from=0
file_args=0
for argument in "$@"; do
  [[ "$argument" == "--files-from" ]] && has_files_from=1
  [[ "$argument" == *.md ]] && file_args=$((file_args + 1))
done
if [[ "$has_files_from" == "0" && "$file_args" -gt 64 ]]; then
  echo "fixture rg: direct file argv is too large" >&2
  exit 126
fi
exec "${REAL_RG:?}" "$@"
SH
chmod +x "$TMPDIR/bin/rg"
cat >"$TMPDIR/bin/python3" <<'SH'
#!/usr/bin/env bash
if [[ "$#" -gt 64 ]]; then
  echo "fixture python3: direct document argv is too large" >&2
  exit 126
fi
exec "${OASIS7_TEST_PYTHON:?}" "$@"
SH
chmod +x "$TMPDIR/bin/python3"

git -C "$FIXTURE" init -q -b main
git -C "$FIXTURE" config user.email test@example.invalid
git -C "$FIXTURE" config user.name Test
git -C "$FIXTURE" add .
git -C "$FIXTURE" commit -qm fixture

set +e
(
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/check.out" 2>"$TMPDIR/check.err"
status=$?
set -e

if [[ "$status" -ne 0 ]]; then
  echo "doc-governance-check.test: fixture check failed" >&2
  cat "$TMPDIR/check.out" >&2
  cat "$TMPDIR/check.err" >&2
  exit 1
fi
if grep -Fq 'direct file argv is too large' "$TMPDIR/check.err"; then
  echo "doc-governance-check.test: document scan passed a large direct file argv to rg" >&2
  exit 1
fi
if [[ "$(wc -l < "$TMPDIR/rg.log")" -lt 2 ]]; then
  echo "doc-governance-check.test: document scan did not split its rg invocation into bounded batches" >&2
  exit 1
fi
if ! grep -Fqx 'doc-governance-check: OK' "$TMPDIR/check.out"; then
  echo "doc-governance-check.test: fixture output missing successful governance verdict" >&2
  cat "$TMPDIR/check.out" >&2
  cat "$TMPDIR/check.err" >&2
  exit 1
fi

mkdir -p "$FIXTURE/doc/testing/nested"
printf '%s\n' '# forbidden ledger' >"$FIXTURE/doc/testing/nested/reintroduced.project.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/ledger.out" 2>"$TMPDIR/ledger.err"; then
  echo "doc-governance-check.test: reintroduced project ledger unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'project.md / *.project.md ledgers are retired' "$TMPDIR/ledger.out"
rm -f "$FIXTURE/doc/testing/nested/reintroduced.project.md"

printf '%s\n' 'Active task status: doc/testing/nested/reintroduced.project.md' >"$FIXTURE/doc/testing/nested/active-reference.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/reference.out" 2>"$TMPDIR/reference.err"; then
  echo "doc-governance-check.test: active project-ledger reference unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'active documentation references retired project ledgers or legacy project-management documents' "$TMPDIR/reference.out"
printf '%s\n' 'Current authority: same-name project document.' >"$FIXTURE/doc/testing/nested/active-reference.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/semantic-reference.out" 2>"$TMPDIR/semantic-reference.err"; then
  echo "doc-governance-check.test: semantic project-ledger reference unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'active documentation references retired project ledgers or legacy project-management documents' "$TMPDIR/semantic-reference.out"

echo "doc-governance-check.test: OK"
