#!/usr/bin/env bash
# Cross-platform maintenance: preserve Windows Git Bash/PowerShell and Linux/macOS governance-check behavior.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

if ! PYTHON_BIN="$("$repo_root/scripts/pm/find-python-with-module.sh" ast)"; then
  echo "doc-governance-check: cannot find a functional Python interpreter" >&2
  exit 1
fi

usage() {
  cat <<'USAGE'
Usage: ./scripts/doc-governance-check.sh

Checks:
  1. Repository-owned documentation must not reintroduce project.md / *.project.md
     ledgers or active references to them.
  2. Non-archive/non-devlog markdown files must not contain absolute /Users/... or /home/... paths.
  2. Non-archive/non-devlog markdown files must be <= 1000 lines.
  3. Root-level markdown files under doc/ must match the tracked allowlist.
  4. Root-level markdown files under each module (doc/<module>/*.md) must match
     the tracked allowlist (archive/devlog/.governance excluded).
  5. Non-archive/non-devlog markdown files must not reference missing markdown
     paths under doc/ (wildcards/templates and explicit exemption docs excluded).
  6. Role labels in devlogs and handoff templates must use canonical names from
     .agents/roles/*.md.
  7. The thin product overlay must contain exactly four product-owned PRDs with
      stable metadata, lifecycle, authority backlinks, and acceptance traceability.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 0 ]]; then
  usage
  exit 1
fi

failures=0

readonly REFERENCE_EXISTENCE_EXEMPT_DOCS=(
  "__no_reference_existence_exempt_docs__"
)
readonly DOC_ROOT_MD_ALLOWLIST_FILE="doc/.governance/doc-root-md-allowlist.txt"
readonly MODULE_ROOT_MD_ALLOWLIST_FILE="doc/.governance/module-root-md-allowlist.txt"

CANONICAL_ROLE_NAMES=()
while IFS= read -r role_name; do
  CANONICAL_ROLE_NAMES+=("$role_name")
done < <(find .agents/roles -mindepth 1 -maxdepth 1 -type f -name '*.md' | sed 's#^.*/##; s/\.md$//' | sort)
fail() {
  echo "doc-governance-check: FAIL: $*"
  failures=$((failures + 1))
}

regex_match_file() {
  local regex="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -q -e "$regex" "$file"
    return $?
  fi
  grep -Eq -- "$regex" "$file"
}

regex_match_with_line_numbers() {
  local regex="$1"
  shift
  local status=1 matched=0 file
  local -a batch=()
  if command -v rg >/dev/null 2>&1; then
    if [[ "$#" -eq 0 ]]; then
      rg -n -e "$regex"
      return $?
    fi
    for file in "$@"; do
      batch+=("$file")
      if [[ "${#batch[@]}" -lt 64 ]]; then
        continue
      fi
      if rg -n -e "$regex" "${batch[@]}"; then
        matched=1
      else
        status=$?
        [[ "$status" -eq 1 ]] || return "$status"
      fi
      batch=()
    done
    if [[ "${#batch[@]}" -gt 0 ]]; then
      if rg -n -e "$regex" "${batch[@]}"; then
        matched=1
      else
        status=$?
        [[ "$status" -eq 1 ]] || return "$status"
      fi
    fi
    [[ "$matched" -eq 1 ]]
    return
  fi
  for file in "$@"; do
    if grep -nE -- "$regex" "$file"; then
      status=0
    elif [[ "$?" -gt 1 ]]; then
      status=2
    fi
  done
  return "$status"
}

contains_literal() {
  local needle="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -Fq -- "$needle" "$file"
    return $?
  fi
  grep -Fq -- "$needle" "$file"
}

collect_headings() {
  local file="$1"
  if command -v rg >/dev/null 2>&1; then
    rg '^#{1,6}[[:space:]].*$' "$file" || true
    return
  fi
  grep -E '^#{1,6}[[:space:]].*$' "$file" || true
}

headings_match_pattern() {
  local headings="$1"
  local pattern="$2"
  local regex="^#{1,6}[[:space:]]*([0-9]+([.][0-9]+)*[.]?[[:space:]]*)?${pattern}.*$"
  local line
  while IFS= read -r line; do
    if [[ "$line" =~ $regex ]]; then
      return 0
    fi
  done <<< "$headings"
  return 1
}

check_required_sections() {
  local file="$1"
  local headings="$2"
  shift 2
  local missing=()
  local token
  for token in "$@"; do
    if ! headings_match_pattern "$headings" "$token"; then
      missing+=("$token")
    fi
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    fail "$file missing sections: ${missing[*]}"
  fi
}

has_strict_prd_sections() {
  local headings="$1"
  headings_match_pattern "$headings" "Executive Summary"     && headings_match_pattern "$headings" "User Experience[[:space:]]*&[[:space:]]*Functionality"     && headings_match_pattern "$headings" "AI System Requirements[[:space:]]*\(If Applicable\)"     && headings_match_pattern "$headings" "Technical Specifications"     && headings_match_pattern "$headings" "Risks[[:space:]]*&[[:space:]]*Roadmap"     && headings_match_pattern "$headings" "Validation[[:space:]]*&[[:space:]]*Decision Record"
}

check_allowlist_match() {
  local label="$1"
  local allowlist_file="$2"
  local actual_file="$3"
  local allowlist_tmp
  allowlist_tmp=$(mktemp)

  if [[ ! -f "$allowlist_file" ]]; then
    fail "${label} allowlist file missing: ${allowlist_file}"
    rm -f "$allowlist_tmp"
    return
  fi

  grep -Ev '^[[:space:]]*($|#)' "$allowlist_file" | sort -u > "$allowlist_tmp"
  sort -u -o "$actual_file" "$actual_file"

  local unexpected missing
  unexpected=$(comm -23 "$actual_file" "$allowlist_tmp" || true)
  missing=$(comm -13 "$actual_file" "$allowlist_tmp" || true)

  if [[ -n "$unexpected" ]]; then
    echo "doc-governance-check: ${label} unexpected entries:"
    echo "$unexpected"
    fail "${label} contains paths not tracked in allowlist"
  fi

  if [[ -n "$missing" ]]; then
    echo "doc-governance-check: ${label} missing entries (stale allowlist):"
    echo "$missing"
    fail "${label} allowlist contains paths that no longer exist"
  fi

  rm -f "$allowlist_tmp"
}

check_doc_path_references_batch() {
  local exempt_tmp doc_list_tmp
  local status=0
  exempt_tmp=$(mktemp)
  doc_list_tmp=$(mktemp)
  printf '%s\n' "${REFERENCE_EXISTENCE_EXEMPT_DOCS[@]}" > "$exempt_tmp"
  printf '%s\0' "${all_doc_files[@]}" > "$doc_list_tmp"

  "$PYTHON_BIN" - "$exempt_tmp" "$doc_list_tmp" <<'PY' || status=$?
from __future__ import annotations

from pathlib import Path
import re
import sys


exempt_path = Path(sys.argv[1])
doc_list_path = Path(sys.argv[2])
doc_files = [
    entry.decode("utf-8")
    for entry in doc_list_path.read_bytes().split(b"\0")
    if entry
]
exempt_docs = {
    line.strip()
    for line in exempt_path.read_text(encoding="utf-8").splitlines()
    if line.strip()
}
reference_re = re.compile(r"doc/[A-Za-z0-9_./-]+\.md")
skip_markers = ("*", "?", "[", "]", "{", "}", "YYYY-MM-DD")

for file in doc_files:
    if file in exempt_docs:
        continue
    path = Path(file)
    text = path.read_text(encoding="utf-8")
    for ref_path in sorted(set(reference_re.findall(text))):
        if any(marker in ref_path for marker in skip_markers):
            continue
        if not Path(ref_path).is_file():
            print(f"{file}\t{ref_path}")
PY

  rm -f "$exempt_tmp"
  rm -f "$doc_list_tmp"
  return "$status"
}

is_canonical_role_name() {
  local role_name="$1"
  local canonical_role_name=""
  if [[ "${#CANONICAL_ROLE_NAMES[@]}" -eq 0 ]]; then
    return 1
  fi
  for canonical_role_name in "${CANONICAL_ROLE_NAMES[@]}"; do
    if [[ "$canonical_role_name" == "$role_name" ]]; then
      return 0
    fi
  done
  return 1
}

trim_whitespace() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s
' "$value"
}

check_devlog_role_labels() {
  local file="$1"
  local line
  local role_name

  while IFS= read -r line; do
    [[ "$line" == '## '* ]] || continue
    [[ "$line" == *' / '* ]] || continue
    role_name="${line##* / }"
    role_name="${role_name#\`}"
    role_name="${role_name%\`}"
    role_name="$(trim_whitespace "$role_name")"
    [[ -z "$role_name" ]] && continue
    if ! is_canonical_role_name "$role_name"; then
      fail "$file uses unknown role label in heading: $role_name"
    fi
  done < <(if command -v rg >/dev/null 2>&1; then rg '^## ' "$file" || true; else grep '^## ' "$file" || true; fi)
}

check_handoff_role_fields() {
  local file="$1"
  local line
  local payload
  local role_name

  while IFS= read -r line; do
    if [[ "$line" =~ ^-[[:space:]]*(From\ Role|To\ Role):[[:space:]]*\`(.*)\`$ ]]; then
      payload="${BASH_REMATCH[2]}"
    else
      continue
    fi
    while IFS= read -r role_name; do
      role_name="$(trim_whitespace "$role_name")"
      [[ -z "$role_name" ]] && continue
      if ! is_canonical_role_name "$role_name"; then
        fail "$file references unknown canonical role name: $role_name"
      fi
    done < <(printf '%s
' "$payload" | tr '|' '
')
  done < <(if command -v rg >/dev/null 2>&1; then rg '^-[[:space:]]*(From Role|To Role): ' "$file" || true; else grep -E '^-[[:space:]]*(From Role|To Role): ' "$file" || true; fi)
}

all_doc_files=()
while IFS= read -r file; do
  all_doc_files+=("$file")
done < <(find doc -type f -name '*.md' ! -path 'doc/devlog/*' ! -path '*/archive/*' | sort)

# 0) project-ledger retirement is permanent
project_ledger_paths=()
while IFS= read -r file; do
  project_ledger_paths+=("$file")
done < <(
  find doc site tools skills .agents -type f \
    \( -name 'project.md' -o -name '*.project.md' \) \
    ! -path '*/third_party/*' ! -path '*/target/*' ! -path '*/node_modules/*' \
    | sort
)
if [[ ${#project_ledger_paths[@]} -gt 0 ]]; then
  printf 'doc-governance-check: retired project ledger paths:\n%s\n' "${project_ledger_paths[*]}"
  fail "project.md / *.project.md ledgers are retired; use GitHub Issue/Project task truth"
fi

active_governance_docs=()
while IFS= read -r file; do
  active_governance_docs+=("$file")
done < <(
  find doc site skills .agents -type f -name '*.md' \
    ! -path 'doc/devlog/*' ! -path '*/archive/*' ! -path '*/evidence/*' \
    ! -path 'doc/engineering/workflow/source-of-truth.md' \
    | sort
)
if project_reference_hits=$(regex_match_with_line_numbers '(^|[^A-Za-z0-9_])([A-Za-z0-9_./*-]+)?(\.project|/project)\.md([^A-Za-z0-9_]|$)|同名[ `]*(project|项目)|配套[ `]*project( 文档)?|项目管理文档|same[- ]name[ `]*project' "${active_governance_docs[@]}"); then
  echo "doc-governance-check: retired project ledger reference hits:"
  echo "$project_reference_hits"
  fail "active documentation references retired project ledgers or legacy project-management documents"
fi

devlog_files=()
while IFS= read -r file; do
  devlog_files+=("$file")
done < <(find doc/devlog -type f -name '*.md' | sort)

handoff_template_files=()
while IFS= read -r file; do
  handoff_template_files+=("$file")
done < <(find .agents/roles/templates -type f -name '*.md' | sort)

if [[ ${#all_doc_files[@]} -eq 0 ]]; then
  fail "no markdown files found under doc/"
fi

# 1) absolute path check
if abs_hits=$(regex_match_with_line_numbers '/(Users|home)/[^[:space:]]+' "${all_doc_files[@]}"); then
  echo "doc-governance-check: absolute path hits:"
  echo "$abs_hits"
  fail "absolute user-home paths found in non-archive docs"
fi

# 2) line count check
if [[ ${#all_doc_files[@]} -gt 0 ]]; then
  while IFS=$'\t' read -r line_count file; do
    [[ -n "$file" ]] || continue
    if ((line_count > 1000)); then
      fail "$file exceeds 1000 lines (${line_count})"
    fi
  done < <(awk 'FNR == 1 { if (NR > 1) print count "\t" prev; prev = FILENAME; count = 0 } { count++ } END { if (NR > 0) print count "\t" prev }' "${all_doc_files[@]}")
fi

# 3) markdown doc path references must exist (except explicit exemptions)
doc_reference_scan_tmp=$(mktemp)
if ! check_doc_path_references_batch > "$doc_reference_scan_tmp"; then
  rm -f "$doc_reference_scan_tmp"
  fail "markdown doc path reference scan failed"
else
while IFS=$'\t' read -r file ref_path; do
  [[ -n "${file:-}" && -n "${ref_path:-}" ]] || continue
  fail "$file references missing markdown path: $ref_path"
done < "$doc_reference_scan_tmp"
  rm -f "$doc_reference_scan_tmp"
fi

doc_root_actual_tmp=$(mktemp)
module_root_actual_tmp=$(mktemp)

find doc -mindepth 1 -maxdepth 1 -type f -name '*.md' | sort > "$doc_root_actual_tmp"
find doc -mindepth 2 -maxdepth 2 -type f -name '*.md' \
  ! -path 'doc/archive/*' \
  ! -path 'doc/devlog/*' \
  ! -path 'doc/.governance/*' \
  | sort > "$module_root_actual_tmp"

check_allowlist_match "doc root markdown set" "$DOC_ROOT_MD_ALLOWLIST_FILE" "$doc_root_actual_tmp"
check_allowlist_match "module root markdown set" "$MODULE_ROOT_MD_ALLOWLIST_FILE" "$module_root_actual_tmp"

rm -f "$doc_root_actual_tmp" "$module_root_actual_tmp"

# 4) canonical role names must be used in devlogs and handoff templates
if [[ ${#devlog_files[@]} -gt 0 ]]; then
  for file in "${devlog_files[@]}"; do
    check_devlog_role_labels "$file"
  done
fi

if [[ ${#handoff_template_files[@]} -gt 0 ]]; then
  for file in "${handoff_template_files[@]}"; do
    check_handoff_role_fields "$file"
  done
fi

if ! "$PYTHON_BIN" scripts/product-doc-governance-check.py; then
  fail "product documentation overlay contract failed"
fi

if ((failures > 0)); then
  echo "doc-governance-check: failed with ${failures} issue(s)"
  exit 1
fi

echo "doc-governance-check: OK"
