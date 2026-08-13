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
mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/doc/.governance" "$FIXTURE/doc/testing" "$FIXTURE/doc/many" "$FIXTURE/doc/devlog" "$FIXTURE/.agents/roles/templates" "$FIXTURE/.agents/roles" "$TMPDIR/bin"
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
printf '%s\n' 'fixture many-doc landing page' >"$FIXTURE/doc/many/README.md"
printf '%s\n' 'fixture retired archive landing page' >"$FIXTURE/doc/devlog/README.md"
printf '%s\n' '# testing fixture landing page' >"$FIXTURE/doc/testing/README.md"
{
  printf '%s\n' 'doc/many/README.md' 'doc/testing/README.md' 'doc/testing/prd.md'
  find "$FIXTURE/doc/many" -type f -name '*.md' | sed "s#^$FIXTURE/##" | sort
} >"$FIXTURE/doc/.governance/module-root-md-allowlist.txt"
cat >"$FIXTURE/doc/.governance/top-level-directory-registry.json" <<'JSON'
{
  "version": 1,
  "directories": [
    {"name": "devlog", "type": "retired_archive", "owner": "repository_health_engineer", "entry": "doc/devlog/README.md", "reason": "fixture archive", "exception": {"entry_conditions": "history", "review_trigger": "change", "exit_conditions": "retire"}},
    {"name": "many", "type": "professional_domain", "owner": "repository_health_engineer", "entry": "doc/many/README.md", "reason": "fixture corpus", "exception": null},
    {"name": "testing", "type": "professional_domain", "owner": "repository_health_engineer", "entry": "doc/testing/README.md", "reason": "fixture tests", "exception": null}
  ]
}
JSON
cat >"$FIXTURE/doc/README.md" <<'DOC'
fixture documentation landing page
doc/devlog/README.md
doc/many/README.md
doc/testing/README.md
DOC
printf '%s\n' '# repository health role' >"$FIXTURE/.agents/roles/repository_health_engineer.md"

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

REGISTRY_BASE="$TMPDIR/top-level-directory-registry.json"
cp "$FIXTURE/doc/.governance/top-level-directory-registry.json" "$REGISTRY_BASE"
reset_registry() {
  cp "$REGISTRY_BASE" "$FIXTURE/doc/.governance/top-level-directory-registry.json"
}

sed -i.bak 's/"reason": "fixture tests", //' "$FIXTURE/doc/.governance/top-level-directory-registry.json"
rm -f "$FIXTURE/doc/.governance/top-level-directory-registry.json.bak"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-required-field.out" 2>"$TMPDIR/registry-required-field.err"; then
  echo "doc-governance-check.test: registry missing required field unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'missing fields: reason' "$TMPDIR/registry-required-field.out"
reset_registry

sed -i.bak 's/"owner": "repository_health_engineer"/"owner": "unknown_owner"/' "$FIXTURE/doc/.governance/top-level-directory-registry.json"
rm -f "$FIXTURE/doc/.governance/top-level-directory-registry.json.bak"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-owner.out" 2>"$TMPDIR/registry-owner.err"; then
  echo "doc-governance-check.test: registry unknown owner unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'has unknown owner' "$TMPDIR/registry-owner.out"
reset_registry

sed -i.bak 's#doc/many/README.md#doc/many/missing.md#' "$FIXTURE/doc/.governance/top-level-directory-registry.json"
rm -f "$FIXTURE/doc/.governance/top-level-directory-registry.json.bak"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-entry.out" 2>"$TMPDIR/registry-entry.err"; then
  echo "doc-governance-check.test: registry missing entry unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'many has missing entry' "$TMPDIR/registry-entry.out"
reset_registry

sed -i.bak '/doc\/many\/README.md/d' "$FIXTURE/doc/README.md"
rm -f "$FIXTURE/doc/README.md.bak"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-navigation.out" 2>"$TMPDIR/registry-navigation.err"; then
  echo "doc-governance-check.test: registry navigation omission unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'many directory is absent from doc/README.md navigation' "$TMPDIR/registry-navigation.out"
printf '%s\n' 'doc/many/README.md' >>"$FIXTURE/doc/README.md"

sed -i.bak 's/"review_trigger": "change", //' "$FIXTURE/doc/.governance/top-level-directory-registry.json"
rm -f "$FIXTURE/doc/.governance/top-level-directory-registry.json.bak"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-lifecycle.out" 2>"$TMPDIR/registry-lifecycle.err"; then
  echo "doc-governance-check.test: registry missing lifecycle field unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'devlog exception missing review_trigger' "$TMPDIR/registry-lifecycle.out"
reset_registry

mkdir -p "$FIXTURE/doc/unregistered"
printf '%s\n' 'unregistered fixture directory' >"$FIXTURE/doc/unregistered/README.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry.out" 2>"$TMPDIR/registry.err"; then
  echo "doc-governance-check.test: unregistered top-level directory unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'top-level directory registry contract failed' "$TMPDIR/registry.out"
rm -rf "$FIXTURE/doc/unregistered"

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

# A top-level documentation symlink must not be able to register an external
# corpus whose contents are outside the scanner's real-directory traversal.
mkdir -p "$FIXTURE/linked-target"
printf '%s\n' 'external fixture landing page' >"$FIXTURE/linked-target/README.md"
ln -s "$FIXTURE/linked-target" "$FIXTURE/doc/linked"
"$REAL_PYTHON" - "$FIXTURE/doc/.governance/top-level-directory-registry.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["directories"].append(
    {
        "name": "linked",
        "type": "professional_domain",
        "owner": "repository_health_engineer",
        "entry": "doc/linked/README.md",
        "reason": "fixture symlink",
        "exception": None,
    }
)
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
printf '%s\n' 'doc/linked/README.md' >>"$FIXTURE/doc/README.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-symlink.out" 2>"$TMPDIR/registry-symlink.err"; then
  echo "doc-governance-check.test: top-level documentation symlink unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'doc/linked must be a real directory, not a symlink' "$TMPDIR/registry-symlink.out"

printf '%s\n' 'not a directory' >"$FIXTURE/linked-file-target"
ln -s "$FIXTURE/linked-file-target" "$FIXTURE/doc/linked-file"
ln -s "$FIXTURE/missing-linked-target" "$FIXTURE/doc/linked-dangling"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/registry-nondirectory-symlink.out" 2>"$TMPDIR/registry-nondirectory-symlink.err"; then
  echo "doc-governance-check.test: non-directory documentation symlinks unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'doc/linked-file must be a real directory, not a symlink' "$TMPDIR/registry-nondirectory-symlink.out"
grep -Fq 'doc/linked-dangling must be a real directory, not a symlink' "$TMPDIR/registry-nondirectory-symlink.out"

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
printf '%s\n' 'Current authority: same-named design and project.' >"$FIXTURE/doc/testing/nested/active-reference.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/stale-wording.out" 2>"$TMPDIR/stale-wording.err"; then
  echo "doc-governance-check.test: stale project-ledger wording unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'active documentation references retired project ledgers or legacy project-management documents' "$TMPDIR/stale-wording.out"
printf '%s\n' '- Corresponding GitHub Issue/Project task truth: `doc/testing/prd.md`' >"$FIXTURE/doc/testing/nested/active-reference.md"
if (
  cd "$FIXTURE"
  OASIS7_TEST_PYTHON="$REAL_PYTHON" RG_INVOCATION_LOG="$TMPDIR/rg.log" REAL_RG="$REAL_RG" PATH="$TMPDIR/bin:$PATH" ./scripts/doc-governance-check.sh
) >"$TMPDIR/false-task-truth-link.out" 2>"$TMPDIR/false-task-truth-link.err"; then
  echo "doc-governance-check.test: local markdown falsely labelled as GitHub task truth unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'active documentation references retired project ledgers or legacy project-management documents' "$TMPDIR/false-task-truth-link.out"

echo "doc-governance-check.test: OK"
