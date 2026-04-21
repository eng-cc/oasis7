#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/doc-governance-check.sh

Checks:
  1. Non-archive/non-devlog markdown files must not contain absolute /Users/... or /home/... paths.
  2. Non-archive/non-devlog markdown files must be <= 1000 lines.
  3. Each non-archive project doc (`project.md` / `*.project.md`) must include sections:
     任务拆解 / 依赖 / 状态.
  4. Each non-archive project doc must have a paired design/PRD doc and that paired doc
     must include either:
       - Legacy sections: 目标 / 范围 / 接口/数据 / 里程碑 / 风险
       - Strict PRD sections: 1..6 chapter structure
     (except whitelisted project docs).
  5. Root-level markdown files under doc/ must match the tracked allowlist.
  6. Root-level markdown files under each module (doc/<module>/*.md) must match
     the tracked allowlist (archive/devlog/.governance excluded).
  7. Active topic PRD pairs (non-archive, non-devlog, excluding module main
     doc/<module>/prd*.md) must contain bidirectional references:
       - topic design/PRD doc includes its paired project doc path
       - topic project doc includes its paired design/PRD doc path
  8. Non-archive/non-devlog markdown files must not reference missing markdown
     paths under doc/ (wildcards/templates and explicit exemption docs excluded).
  9. Role labels in devlogs and handoff templates must use canonical names from
     .agents/roles/*.md.
  10. Newly added `project.md` task rows must not introduce fresh `TASK-*`
      sequential identifiers; they must use `topic-slug (PRD-ID) ... Trace:
      .pm/tasks/task_<32hex>.yaml` on a single line.
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

# Some handbooks are intentionally concise and do not follow design-doc section template.
# Whitelist is keyed by project doc path to keep exemptions explicit and reviewable.
readonly DESIGN_SECTION_EXEMPT_PROJECT_DOCS=(
  "doc/playability_test_result/game-test.project.md"
  "doc/game-test.project.md"
  "doc/world-runtime.project.md"
  "doc/world-simulator.project.md"
)
readonly GRANDFATHERED_ADDED_PROJECT_TASK_ROWS=(
  "doc/engineering/project.md::- [x] TASK-ENGINEERING-115 (PRD-ENGINEERING-021) [test_tier_required]: 对齐根 \`AGENTS.md\`、角色职责卡与 handoff 模板的 \`.pm\` task 创建顺序、task execution log 口径与“一个 task 收口后再开下一 task”语义，清理当前态 \`doc/devlog\` 必写残留要求。"
)
readonly REFERENCE_EXISTENCE_EXEMPT_DOCS=(
  "doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md"
)
readonly DOC_ROOT_MD_ALLOWLIST_FILE="doc/.governance/doc-root-md-allowlist.txt"
readonly MODULE_ROOT_MD_ALLOWLIST_FILE="doc/.governance/module-root-md-allowlist.txt"

mapfile -t CANONICAL_ROLE_NAMES < <(find .agents/roles -mindepth 1 -maxdepth 1 -type f -name '*.md' -printf '%f\n' | sed 's/\.md$//' | sort)
declare -A CANONICAL_ROLE_NAME_SET=()
for role_name in "${CANONICAL_ROLE_NAMES[@]}"; do
  CANONICAL_ROLE_NAME_SET["$role_name"]=1
done

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
  if command -v rg >/dev/null 2>&1; then
    rg -n -e "$regex" "$@"
    return $?
  fi
  grep -nE -- "$regex" "$@"
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

is_design_section_exempt_project_doc() {
  local project_doc="$1"
  local exempt
  for exempt in "${DESIGN_SECTION_EXEMPT_PROJECT_DOCS[@]}"; do
    if [[ "$project_doc" == "$exempt" ]]; then
      return 0
    fi
  done
  return 1
}

paired_design_doc() {
  local project_doc="$1"
  local candidate

  if [[ "$project_doc" =~ ^doc/([^.]+)\.project\.md$ ]]; then
    candidate="doc/${BASH_REMATCH[1]}/prd.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  if [[ "$project_doc" =~ ^doc/([^.]+)\.prd\.project\.md$ ]]; then
    candidate="doc/${BASH_REMATCH[1]}/prd.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  if [[ "$project_doc" =~ ^doc/[^/]+/prd\.project\.md$ ]]; then
    candidate="${project_doc%/project.md}/design.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
    candidate="${project_doc%.project.md}.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  if [[ "$project_doc" =~ ^doc/[^/]+/project\.md$ ]]; then
    candidate="${project_doc%/project.md}/design.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
    candidate="${project_doc%/project.md}/prd.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  if [[ "$project_doc" =~ \.prd\.project\.md$ ]]; then
    candidate="${project_doc%.project.md}.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
    candidate="${project_doc%.project.md}.design.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  if [[ "$project_doc" =~ \.project\.md$ ]]; then
    candidate="${project_doc%.project.md}.prd.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
    candidate="${project_doc%.project.md}.design.md"
    [[ -f "$candidate" ]] && printf '%s\n' "$candidate" && return
  fi

  printf '%s\n' "${project_doc%.project.md}.md"
}

is_topic_project_doc() {
  local project_doc="$1"
  [[ ! "$project_doc" =~ ^doc/[^/]+/(prd\.project|project)\.md$ ]]
}

is_reference_exempt_doc() {
  local doc_file="$1"
  local exempt
  for exempt in "${REFERENCE_EXISTENCE_EXEMPT_DOCS[@]}"; do
    if [[ "$doc_file" == "$exempt" ]]; then
      return 0
    fi
  done
  return 1
}

extract_doc_markdown_references() {
  local file="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -o --no-filename 'doc/[A-Za-z0-9_./-]+\.md' "$file" | sort -u
    return
  fi
  grep -oE 'doc/[A-Za-z0-9_./-]+\.md' "$file" | sort -u
}

check_doc_path_references() {
  local file="$1"
  local ref_path

  if is_reference_exempt_doc "$file"; then
    return
  fi

  while IFS= read -r ref_path; do
    [[ -z "$ref_path" ]] && continue
    case "$ref_path" in
      *'*'*|*'?'*|*'['*|*']'*|*'{'*|*'}'*|*'YYYY-MM-DD'*)
        continue
        ;;
    esac
    if [[ ! -f "$ref_path" ]]; then
      fail "$file references missing markdown path: $ref_path"
    fi
  done < <(extract_doc_markdown_references "$file")
}

is_canonical_role_name() {
  local role_name="$1"
  [[ -n "${CANONICAL_ROLE_NAME_SET[$role_name]:-}" ]]
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

is_grandfathered_added_project_task_row() {
  local project_doc="$1"
  local added_line="$2"
  local entry=""
  for entry in "${GRANDFATHERED_ADDED_PROJECT_TASK_ROWS[@]}"; do
    if [[ "$entry" == "${project_doc}::${added_line}" ]]; then
      return 0
    fi
  done
  return 1
}

normalize_project_task_row_for_policy() {
  local row="$1"
  if [[ "$row" =~ ^-[[:space:]]\[[[:space:]x]\][[:space:]](.*)$ ]]; then
    row="${BASH_REMATCH[1]}"
  fi
  printf '%s\n' "$row"
}

check_added_project_task_row_policy() {
  local diff_blob="$1"
  local current_file=""
  local line=""
  local added_line=""
  local removed_line=""
  local normalized_row=""
  local task_row_regex='^-[[:space:]]\[[ x]\][[:space:]]'
  local deprecated_task_regex='^-[[:space:]]\[[ x]\][[:space:]]TASK-'
  local new_format_regex='^- \[[ x]\] [a-z0-9]+(-[a-z0-9]+)* \(PRD-[A-Z0-9_-]+(/[A-Z0-9_-]+)*\) \[test_tier_(required|full)\]( \+ \[test_tier_(required|full)\])?: .+ Trace: \.pm/tasks/task_[0-9a-f]{32}\.yaml$'
  declare -A removed_task_rows=()

  while IFS= read -r line; do
    if [[ "$line" =~ ^diff[[:space:]]--git[[:space:]] ]]; then
      current_file=""
      continue
    fi
    if [[ "$line" =~ ^\+\+\+[[:space:]]b/(doc/.+project\.md)$ ]]; then
      current_file="${BASH_REMATCH[1]}"
      continue
    fi
    [[ -n "$current_file" ]] || continue
    if [[ "$line" =~ ^- ]] && [[ ! "$line" =~ ^---[[:space:]] ]]; then
      removed_line="${line#-}"
      [[ "$removed_line" =~ $task_row_regex ]] || continue
      normalized_row="$(normalize_project_task_row_for_policy "$removed_line")"
      removed_task_rows["${current_file}::${normalized_row}"]=1
      continue
    fi
    [[ "$line" =~ ^\+[^+] ]] || continue
    added_line="${line#+}"
    [[ "$added_line" =~ $task_row_regex ]] || continue
    normalized_row="$(normalize_project_task_row_for_policy "$added_line")"

    if [[ -n "${removed_task_rows["${current_file}::${normalized_row}"]:-}" ]]; then
      continue
    fi

    if [[ "$added_line" =~ $deprecated_task_regex ]]; then
      if is_grandfathered_added_project_task_row "$current_file" "$added_line"; then
        continue
      fi
      fail "${current_file} adds a new project task row using deprecated sequential TASK-* identifier: ${added_line}"
      continue
    fi

    if [[ ! "$added_line" =~ $new_format_regex ]]; then
      fail "${current_file} adds a project task row that does not match the required topic-slug/PRD-ID/Trace template: ${added_line}"
    fi
  done <<< "$diff_blob"
}

mapfile -t all_doc_files < <(find doc -type f -name '*.md' ! -path 'doc/devlog/*' ! -path '*/archive/*' | sort)
mapfile -t project_docs < <(find doc -type f -name '*.project.md' ! -path '*/archive/*' | sort)
mapfile -t devlog_files < <(find doc/devlog -type f -name '*.md' | sort)
mapfile -t handoff_template_files < <(find .agents/roles/templates -type f -name '*.md' | sort)

if [[ ${#all_doc_files[@]} -eq 0 ]]; then
  fail "no markdown files found under doc/"
fi

if [[ ${#project_docs[@]} -eq 0 ]]; then
  fail "no project docs found under doc/"
fi

# 1) absolute path check
if abs_hits=$(regex_match_with_line_numbers '/(Users|home)/[^[:space:]]+' "${all_doc_files[@]}"); then
  echo "doc-governance-check: absolute path hits:"
  echo "$abs_hits"
  fail "absolute user-home paths found in non-archive docs"
fi

# 2) line count check
for file in "${all_doc_files[@]}"; do
  line_count=$(wc -l < "$file" | tr -d ' ')
  if ((line_count > 1000)); then
    fail "$file exceeds 1000 lines (${line_count})"
  fi
done

# 3) project docs required sections + paired design required sections
for project_doc in "${project_docs[@]}"; do
  project_headings="$(collect_headings "$project_doc")"
  check_required_sections "$project_doc" "$project_headings" "任务拆解" "依赖" "状态"

  design_doc="$(paired_design_doc "$project_doc")"
  if [[ ! -f "$design_doc" ]]; then
    fail "$project_doc has no paired design doc: $design_doc"
    continue
  fi

  if is_topic_project_doc "$project_doc"; then
    if ! contains_literal "$project_doc" "$design_doc"; then
      fail "$design_doc missing bidirectional link to paired project doc: $project_doc"
    fi
    if ! contains_literal "$design_doc" "$project_doc"; then
      fail "$project_doc missing bidirectional link to paired design doc: $design_doc"
    fi
  fi

  if is_design_section_exempt_project_doc "$project_doc"; then
    continue
  fi

  design_headings="$(collect_headings "$design_doc")"
  if has_strict_prd_sections "$design_headings"; then
    check_required_sections "$design_doc" "$design_headings"       "Executive Summary"       "User Experience[[:space:]]*&[[:space:]]*Functionality"       "AI System Requirements[[:space:]]*\(If Applicable\)"       "Technical Specifications"       "Risks[[:space:]]*&[[:space:]]*Roadmap"       "Validation[[:space:]]*&[[:space:]]*Decision Record"
  else
    check_required_sections "$design_doc" "$design_headings" "目标" "范围" "接口[[:space:]]*/[[:space:]]*数据" "里程碑" "风险"
  fi
done

# 4) markdown doc path references must exist (except explicit exemptions)
for file in "${all_doc_files[@]}"; do
  check_doc_path_references "$file"
done

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

# 5) canonical role names must be used in devlogs and handoff templates
for file in "${devlog_files[@]}"; do
  check_devlog_role_labels "$file"
done

for file in "${handoff_template_files[@]}"; do
  check_handoff_role_fields "$file"
done

project_task_policy_diff=""
if git rev-parse --verify origin/main >/dev/null 2>&1; then
  project_task_policy_diff+=$(git diff --unified=0 --no-color origin/main...HEAD -- 'doc/**/*.project.md' 'doc/*/project.md' 'doc/*.project.md' || true)
fi
project_task_policy_diff+=$'\n'
project_task_policy_diff+=$(git diff --unified=0 --no-color --cached -- 'doc/**/*.project.md' 'doc/*/project.md' 'doc/*.project.md' || true)
project_task_policy_diff+=$'\n'
project_task_policy_diff+=$(git diff --unified=0 --no-color -- 'doc/**/*.project.md' 'doc/*/project.md' 'doc/*.project.md' || true)
check_added_project_task_row_policy "$project_task_policy_diff"

if ((failures > 0)); then
  echo "doc-governance-check: failed with ${failures} issue(s)"
  exit 1
fi

echo "doc-governance-check: OK"
