#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/prepare-playability-l4-review.sh [options]

Create a worktree-local artifact scaffold for a complete L4 review run:
- L4A synthetic packet
- role review cards
- persona cards
- L4 summary
- copied L4B playability card
- exact commands for L4A harness and L4B producer playtest

Options:
  --output-dir <path>         Override output directory
                              (default: output/harness/<worktree>/artifacts/playability-l4-<timestamp>)
  --run-id <id>               Override run id suffix (default: YYYYMMDD-HHMMSS)
  --change-scope <text>       Prefill packet `change_scope`
  --target-claim <text>       Prefill packet `target_experience_claim`
  --target-l4-lane <lane>     L4A_only | L4A_then_L4B (default: L4A_then_L4B)
  --formal-surface <name>     Repeatable; defaults to `software_safe` + `pure_api`
  --role <role_name>          Repeatable; defaults to all standard review roles
  --persona <persona_id>      Repeatable; defaults to all fixed personas
  --question <text>           Repeatable packet question
  --known-blocker <text>      Repeatable packet blocker
  --artifact-path <path>      Repeatable packet artifact path
  --bundle-dir <path>         Bundle dir to embed into recommended L4B command
  --with-l4a-stack            Boot `./scripts/worktree-harness.sh up` before writing artifacts
                              (requires the same active LLM provider config as formal gameplay)
  --json                      Print manifest JSON path summary
  -h, --help                  Show this help

Examples:
  ./scripts/prepare-playability-l4-review.sh
  ./scripts/prepare-playability-l4-review.sh --with-l4a-stack --change-scope "software_safe onboarding followup"
  ./scripts/prepare-playability-l4-review.sh --role producer_system_designer --role qa_engineer --persona new_player_confused
USAGE
}

STANDARD_ROLES=(
  producer_system_designer
  qa_engineer
  viewer_engineer
  agent_engineer
  runtime_engineer
  wasm_platform_engineer
  liveops_community
)
DEFAULT_PERSONAS=(
  new_player_confused
  impatient_action_player
  systems_optimizer
  narrative_curiosity_player
  chaos_tester
)
DEFAULT_FORMAL_SURFACES=(
  software_safe
  pure_api
)

RUN_ID="$(date +%Y%m%d-%H%M%S)"
OUTPUT_DIR=""
CHANGE_SCOPE=""
TARGET_CLAIM=""
TARGET_L4_LANE="L4A_then_L4B"
BUNDLE_DIR=""
WITH_L4A_STACK=0
PRINT_JSON=0
declare -a FORMAL_SURFACES=()
declare -a REQUESTED_ROLES=()
declare -a SELECTED_PERSONAS=()
declare -a QUESTIONS_TO_PROBE=()
declare -a KNOWN_BLOCKERS=()
declare -a ARTIFACT_PATHS=()
FORMAL_SURFACE_OVERRIDE=0
ROLE_OVERRIDE=0
PERSONA_OVERRIDE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --run-id)
      RUN_ID="${2:-}"
      shift 2
      ;;
    --change-scope)
      CHANGE_SCOPE="${2:-}"
      shift 2
      ;;
    --target-claim)
      TARGET_CLAIM="${2:-}"
      shift 2
      ;;
    --target-l4-lane)
      TARGET_L4_LANE="${2:-}"
      shift 2
      ;;
    --formal-surface)
      if [[ "$FORMAL_SURFACE_OVERRIDE" == "0" ]]; then
        FORMAL_SURFACES=()
        FORMAL_SURFACE_OVERRIDE=1
      fi
      FORMAL_SURFACES+=("${2:-}")
      shift 2
      ;;
    --role)
      if [[ "$ROLE_OVERRIDE" == "0" ]]; then
        REQUESTED_ROLES=()
        ROLE_OVERRIDE=1
      fi
      REQUESTED_ROLES+=("${2:-}")
      shift 2
      ;;
    --persona)
      if [[ "$PERSONA_OVERRIDE" == "0" ]]; then
        SELECTED_PERSONAS=()
        PERSONA_OVERRIDE=1
      fi
      SELECTED_PERSONAS+=("${2:-}")
      shift 2
      ;;
    --question)
      QUESTIONS_TO_PROBE+=("${2:-}")
      shift 2
      ;;
    --known-blocker)
      KNOWN_BLOCKERS+=("${2:-}")
      shift 2
      ;;
    --artifact-path)
      ARTIFACT_PATHS+=("${2:-}")
      shift 2
      ;;
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --with-l4a-stack)
      WITH_L4A_STACK=1
      shift
      ;;
    --json)
      PRINT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

wh_require_git_worktree
WORKTREE_ID="$(wh_worktree_id)"
HARNESS_ROOT="$(wh_harness_root "$ROOT_DIR" "$WORKTREE_ID")"
ARTIFACT_ROOT="$(wh_artifacts_dir "$HARNESS_ROOT")"
STATE_FILE="$(wh_state_file "$HARNESS_ROOT")"
wh_prepare_dirs "$HARNESS_ROOT"

if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="$ARTIFACT_ROOT/playability-l4-$RUN_ID"
fi
if [[ "$OUTPUT_DIR" != /* ]]; then
  OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR"
fi
[[ "$TARGET_L4_LANE" == "L4A_only" || "$TARGET_L4_LANE" == "L4A_then_L4B" ]] || {
  echo "error: --target-l4-lane must be L4A_only or L4A_then_L4B" >&2
  exit 2
}
[[ -n "$RUN_ID" ]] || { echo "error: --run-id cannot be empty" >&2; exit 2; }
if [[ -e "$OUTPUT_DIR" ]]; then
  echo "error: output directory already exists: $OUTPUT_DIR" >&2
  exit 2
fi

if [[ "${#FORMAL_SURFACES[@]}" -eq 0 ]]; then
  FORMAL_SURFACES=("${DEFAULT_FORMAL_SURFACES[@]}")
fi
if [[ "${#REQUESTED_ROLES[@]}" -eq 0 ]]; then
  REQUESTED_ROLES=("${STANDARD_ROLES[@]}")
fi
if [[ "${#SELECTED_PERSONAS[@]}" -eq 0 ]]; then
  SELECTED_PERSONAS=("${DEFAULT_PERSONAS[@]}")
fi

contains_item() {
  local needle=$1
  shift
  local item
  for item in "$@"; do
    if [[ "$item" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

validate_membership() {
  local label=$1
  local -n values_ref=$2
  local -n allow_ref=$3
  local value
  for value in "${values_ref[@]}"; do
    contains_item "$value" "${allow_ref[@]}" || {
      echo "error: unsupported $label: $value" >&2
      exit 2
    }
  done
}

validate_membership "role" REQUESTED_ROLES STANDARD_ROLES
validate_membership "persona" SELECTED_PERSONAS DEFAULT_PERSONAS

mkdir -p \
  "$OUTPUT_DIR" \
  "$OUTPUT_DIR/role-review-cards" \
  "$OUTPUT_DIR/persona-cards" \
  "$OUTPUT_DIR/evidence"

json_array() {
  python3 - "$@" <<'PY'
from __future__ import annotations

import json
import sys

print(json.dumps(sys.argv[1:], ensure_ascii=False))
PY
}

emit_backticked_bullets() {
  local item
  for item in "$@"; do
    printf -- '- `%s`\n' "$item"
  done
}

append_template_body() {
  local template=$1
  sed '1d' "$template"
}

PACKET_PATH="$OUTPUT_DIR/l4-review-packet.md"
SUMMARY_PATH="$OUTPUT_DIR/l4-summary.md"
COMMANDS_PATH="$OUTPUT_DIR/commands.sh"
MANIFEST_PATH="$OUTPUT_DIR/manifest.json"
L4B_CARD_PATH="$OUTPUT_DIR/l4b-playability-test-card.md"
EVIDENCE_README_PATH="$OUTPUT_DIR/evidence/README.md"

if [[ "$WITH_L4A_STACK" == "1" ]]; then
  ./scripts/worktree-harness.sh up
  ./scripts/worktree-harness.sh status --json >"$OUTPUT_DIR/evidence/l4a-harness-state.json"
  ./scripts/worktree-harness.sh url >"$OUTPUT_DIR/evidence/l4a-viewer-url.txt"
fi

L4A_VIEWER_URL=""
if [[ -f "$OUTPUT_DIR/evidence/l4a-viewer-url.txt" ]]; then
  L4A_VIEWER_URL="$(cat "$OUTPUT_DIR/evidence/l4a-viewer-url.txt")"
fi
if [[ -z "$L4A_VIEWER_URL" && -f "$STATE_FILE" ]]; then
  L4A_VIEWER_URL="$(python3 - "$STATE_FILE" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
if not path.exists():
    raise SystemExit(1)
data = json.loads(path.read_text(encoding="utf-8"))
print(data.get("viewer_url") or "")
PY
)"
fi

{
  printf '# L4 Review Packet\n\n'
  printf -- '- Run ID: `%s`\n' "$RUN_ID"
  printf -- '- Generated at: `%s`\n' "$(date '+%Y-%m-%d %H:%M:%S %Z')"
  printf -- '- Worktree: `%s`\n' "$(pwd -P)"
  printf -- '- Harness root: `%s`\n' "$HARNESS_ROOT"
  printf -- '- Target L4 lane: `%s`\n' "$TARGET_L4_LANE"
  if [[ -n "$CHANGE_SCOPE" ]]; then
    printf -- '- Prefilled change scope: `%s`\n' "$CHANGE_SCOPE"
  fi
  if [[ -n "$TARGET_CLAIM" ]]; then
    printf -- '- Prefilled target claim: `%s`\n' "$TARGET_CLAIM"
  fi
  if [[ -n "$L4A_VIEWER_URL" ]]; then
    printf -- '- Current L4A viewer URL: `%s`\n' "$L4A_VIEWER_URL"
  fi
  if [[ "${#QUESTIONS_TO_PROBE[@]}" -gt 0 ]]; then
    printf -- '- Prefilled questions to probe:\n'
    emit_backticked_bullets "${QUESTIONS_TO_PROBE[@]}"
  fi
  if [[ "${#KNOWN_BLOCKERS[@]}" -gt 0 ]]; then
    printf -- '- Prefilled known blockers:\n'
    emit_backticked_bullets "${KNOWN_BLOCKERS[@]}"
  fi
  if [[ "${#ARTIFACT_PATHS[@]}" -gt 0 ]]; then
    printf -- '- Prefilled artifact paths:\n'
    emit_backticked_bullets "${ARTIFACT_PATHS[@]}"
  fi
  printf -- '- Requested roles:\n'
  emit_backticked_bullets "${REQUESTED_ROLES[@]}"
  printf -- '- Selected personas:\n'
  emit_backticked_bullets "${SELECTED_PERSONAS[@]}"
  printf -- '- Formal surfaces:\n'
  emit_backticked_bullets "${FORMAL_SURFACES[@]}"
  printf '\n'
  append_template_body "$ROOT_DIR/doc/testing/templates/playability-l4-review-packet-template.md"
} >"$PACKET_PATH"

{
  printf '# L4 Validation Summary\n\n'
  printf -- '- Run ID: `%s`\n' "$RUN_ID"
  printf -- '- Packet: `%s`\n' "$PACKET_PATH"
  printf -- '- L4B card copy: `%s`\n' "$L4B_CARD_PATH"
  printf -- '- Commands: `%s`\n' "$COMMANDS_PATH"
  printf '\n'
  append_template_body "$ROOT_DIR/doc/testing/templates/playability-l4-summary-template.md"
} >"$SUMMARY_PATH"

role_name=
for role_name in "${REQUESTED_ROLES[@]}"; do
  {
    printf '# Role Review Card: %s\n\n' "$role_name"
    printf -- '- Run ID: `%s`\n' "$RUN_ID"
    printf -- '- Role: `%s`\n' "$role_name"
    printf -- '- Packet: `%s`\n' "$PACKET_PATH"
    printf -- '- Summary target: `%s`\n' "$SUMMARY_PATH"
    printf '\n'
    append_template_body "$ROOT_DIR/doc/testing/templates/playability-l4-role-review-card-template.md"
  } >"$OUTPUT_DIR/role-review-cards/${role_name}.md"
done

persona_id=
for persona_id in "${SELECTED_PERSONAS[@]}"; do
  {
    printf '# Persona Card: %s\n\n' "$persona_id"
    printf -- '- Run ID: `%s`\n' "$RUN_ID"
    printf -- '- Persona ID: `%s`\n' "$persona_id"
    printf -- '- Packet: `%s`\n' "$PACKET_PATH"
    printf '\n'
    append_template_body "$ROOT_DIR/doc/testing/templates/playability-l4-persona-card-template.md"
  } >"$OUTPUT_DIR/persona-cards/${persona_id}.md"
done

cp "$ROOT_DIR/doc/playability_test_result/playability_test_card.md" "$L4B_CARD_PATH"

L4B_COMMAND=(./scripts/run-producer-playtest.sh --open-headed)
if [[ -n "$BUNDLE_DIR" ]]; then
  L4B_COMMAND+=(--bundle-dir "$BUNDLE_DIR")
fi

{
  printf '#!/usr/bin/env bash\n'
  printf 'set -euo pipefail\n\n'
  printf 'cd %q\n\n' "$ROOT_DIR"
  printf '# Boot or refresh the L4A synthetic stack and capture fresh evidence.\n'
  printf './scripts/worktree-harness.sh up\n'
  printf './scripts/worktree-harness.sh status --json > %q\n' "$OUTPUT_DIR/evidence/l4a-harness-state.json"
  printf './scripts/worktree-harness.sh url > %q\n' "$OUTPUT_DIR/evidence/l4a-viewer-url.txt"
  printf '\n'
  printf '# Launch the L4B human playtest path.\n'
  printf '%q' "${L4B_COMMAND[0]}"
  command_arg=
  for command_arg in "${L4B_COMMAND[@]:1}"; do
    printf ' %q' "$command_arg"
  done
  printf '\n'
  printf '# After the human playtest, complete %q and reflect the L4B verdict in %q.\n' "$L4B_CARD_PATH" "$SUMMARY_PATH"
} >"$COMMANDS_PATH"
chmod +x "$COMMANDS_PATH"

{
  printf '# L4 Evidence Directory\n\n'
  printf 'Drop concrete run artifacts here so the packet and summary can point to stable files.\n\n'
  printf 'Recommended contents:\n'
  printf -- '- `l4a-harness-state.json`: current harness snapshot after `worktree-harness.sh status --json`\n'
  printf -- '- `l4a-viewer-url.txt`: current L4A viewer URL\n'
  printf -- '- screenshots / recordings from L4A Web closure\n'
  printf -- '- launcher startup logs and bundle notes from L4B\n'
  printf -- '- any copied `session.meta`, console snippets, or external issue references\n'
} >"$EVIDENCE_README_PATH"

python3 - "$MANIFEST_PATH" "$RUN_ID" "$OUTPUT_DIR" "$PACKET_PATH" "$SUMMARY_PATH" "$L4B_CARD_PATH" "$COMMANDS_PATH" "$HARNESS_ROOT" "$WORKTREE_ID" "$STATE_FILE" "$(git rev-parse --abbrev-ref HEAD)" "$(git rev-parse HEAD)" "$(json_array "${FORMAL_SURFACES[@]}")" "$(json_array "${REQUESTED_ROLES[@]}")" "$(json_array "${SELECTED_PERSONAS[@]}")" "$(json_array "${QUESTIONS_TO_PROBE[@]}")" "$(json_array "${KNOWN_BLOCKERS[@]}")" "$(json_array "${ARTIFACT_PATHS[@]}")" "$TARGET_L4_LANE" "$L4A_VIEWER_URL" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

(
    manifest_path,
    run_id,
    output_dir,
    packet_path,
    summary_path,
    l4b_card_path,
    commands_path,
    harness_root,
    worktree_id,
    state_file,
    branch,
    git_head,
    formal_surfaces,
    requested_roles,
    selected_personas,
    questions_to_probe,
    known_blockers,
    artifact_paths,
    target_l4_lane,
    l4a_viewer_url,
) = sys.argv[1:]

payload = {
    "run_id": run_id,
    "output_dir": output_dir,
    "packet_path": packet_path,
    "summary_path": summary_path,
    "l4b_card_path": l4b_card_path,
    "commands_path": commands_path,
    "harness_root": harness_root,
    "worktree_id": worktree_id,
    "state_file": state_file,
    "branch": branch,
    "git_head": git_head,
    "formal_surfaces": json.loads(formal_surfaces),
    "requested_roles": json.loads(requested_roles),
    "selected_personas": json.loads(selected_personas),
    "questions_to_probe": json.loads(questions_to_probe),
    "known_blockers": json.loads(known_blockers),
    "artifact_paths": json.loads(artifact_paths),
    "target_l4_lane": target_l4_lane,
    "l4a_viewer_url": l4a_viewer_url or None,
}
pathlib.Path(manifest_path).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

{
  printf '# Complete L4 Review Scaffold\n\n'
  printf -- '- Run ID: `%s`\n' "$RUN_ID"
  printf -- '- Packet: `%s`\n' "$PACKET_PATH"
  printf -- '- Summary: `%s`\n' "$SUMMARY_PATH"
  printf -- '- L4B card copy: `%s`\n' "$L4B_CARD_PATH"
  printf -- '- Commands: `%s`\n' "$COMMANDS_PATH"
  printf -- '- Manifest: `%s`\n' "$MANIFEST_PATH"
  printf '\n## Next Steps\n'
  printf '1. Fill `l4-review-packet.md` with this change scope, target claim, known blockers, and artifact paths.\n'
  printf '2. Run `commands.sh` or the individual commands to refresh `L4A` evidence and launch `L4B` producer playtest.\n'
  printf '3. Complete every role card and persona card that applies to this run.\n'
  printf '4. After the human playtest, complete `l4b-playability-test-card.md` and record the `L4B` verdict.\n'
  printf '5. Copy concrete screenshots / logs into `evidence/` and reference them from the packet, `l4b-playability-test-card.md`, and summary.\n'
  printf '6. Summarize `L4A`, `L4B`, and combined `go/watch/hold/block` in `l4-summary.md`.\n'
} >"$OUTPUT_DIR/README.md"

if [[ "$PRINT_JSON" == "1" ]]; then
  python3 - <<PY
import json

print(json.dumps({
    "run_id": ${RUN_ID@Q},
    "output_dir": ${OUTPUT_DIR@Q},
    "packet_path": ${PACKET_PATH@Q},
    "summary_path": ${SUMMARY_PATH@Q},
    "manifest_path": ${MANIFEST_PATH@Q},
}, ensure_ascii=False, indent=2))
PY
else
  cat <<EOF
Prepared complete L4 scaffold.
- run_id: $RUN_ID
- output_dir: $OUTPUT_DIR
- packet: $PACKET_PATH
- summary: $SUMMARY_PATH
- l4b_card: $L4B_CARD_PATH
- commands: $COMMANDS_PATH
- manifest: $MANIFEST_PATH
EOF
fi
