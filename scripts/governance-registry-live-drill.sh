#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/governance-registry-live-drill.sh \
    --source-world-dir <dir> \
    --baseline-manifest <public_manifest.json> \
    --slot-id <slot_id> \
    [--pass-manifest-mode <rotate|baseline>] \
    --replace-signer-id <signer_id> \
    [--replacement-signer-id <signer_id>] \
    [--block-remove-signer-id <signer_id>] \
    [--replacement-public-key <hex>] \
    --out-dir <dir>

Description:
  Runs a default/live-world governance registry drill with baseline/pass/block,
  and when the degraded block manifest is still importable, one rejoin phase:
  1. baseline pre-audit
  2. pass case: replace one signer while preserving baseline signer count/threshold
  3. block case: intentionally degrade one slot below its baseline signer count
  4. rejoin case: re-import the baseline-compatible pass manifest on top of the degraded world
  5. restore baseline manifest and re-audit
  Note:
  - --pass-manifest-mode rotate is default
  - --pass-manifest-mode baseline reuses the baseline manifest as pass/rejoin target
  - controller slots may keep the same signer_id and replace only the public key
  - finality slot rotation must use a new signer_id via --replacement-signer-id
  - --block-remove-signer-id may be repeated to model multi-signer loss

Artifacts:
  <out-dir>/run_config.json
  <out-dir>/world-backup-pre-drill/*
  <out-dir>/manifests/{rotated_pass_manifest.json,degraded_block_manifest.json}
  <out-dir>/logs/*
  <out-dir>/summary.json
  <out-dir>/summary.md
EOF
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

run_and_capture() {
  local name="$1"
  shift
  local stdout_path="$LOG_DIR/${name}.stdout"
  local stderr_path="$LOG_DIR/${name}.stderr"
  local rc_path="$LOG_DIR/${name}.rc"
  local rc=0
  if "$@" >"$stdout_path" 2>"$stderr_path"; then
    rc=0
  else
    rc=$?
  fi
  printf '%s\n' "$rc" >"$rc_path"
  return 0
}

SOURCE_WORLD_DIR=""
BASELINE_MANIFEST=""
SLOT_ID=""
PASS_MANIFEST_MODE="rotate"
REPLACE_SIGNER_ID=""
REPLACEMENT_SIGNER_ID=""
BLOCK_REMOVE_SIGNER_IDS=()
REPLACEMENT_PUBLIC_KEY=""
OUT_DIR=""
FINALITY_SLOT_ID="governance.finality.v1"
EXPECTED_THRESHOLD="2"
BLOCK_ENFORCEMENT_STAGE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-world-dir)
      SOURCE_WORLD_DIR="$2"
      shift 2
      ;;
    --baseline-manifest)
      BASELINE_MANIFEST="$2"
      shift 2
      ;;
    --slot-id)
      SLOT_ID="$2"
      shift 2
      ;;
    --pass-manifest-mode)
      PASS_MANIFEST_MODE="$2"
      shift 2
      ;;
    --replace-signer-id)
      REPLACE_SIGNER_ID="$2"
      shift 2
      ;;
    --replacement-signer-id)
      REPLACEMENT_SIGNER_ID="$2"
      shift 2
      ;;
    --block-remove-signer-id)
      BLOCK_REMOVE_SIGNER_IDS+=("$2")
      shift 2
      ;;
    --replacement-public-key)
      REPLACEMENT_PUBLIC_KEY="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$SOURCE_WORLD_DIR" || -z "$BASELINE_MANIFEST" || -z "$SLOT_ID" || -z "$REPLACE_SIGNER_ID" || -z "$OUT_DIR" ]]; then
  echo "all flags are required" >&2
  usage
  exit 1
fi

if [[ "$PASS_MANIFEST_MODE" != "rotate" && "$PASS_MANIFEST_MODE" != "baseline" ]]; then
  echo "pass manifest mode must be rotate or baseline" >&2
  exit 1
fi

if [[ "$PASS_MANIFEST_MODE" == "rotate" && -z "$REPLACEMENT_PUBLIC_KEY" ]]; then
  echo "--replacement-public-key is required when --pass-manifest-mode=rotate" >&2
  exit 1
fi

if [[ -z "$REPLACEMENT_SIGNER_ID" ]]; then
  REPLACEMENT_SIGNER_ID="$REPLACE_SIGNER_ID"
fi

if [[ "${#BLOCK_REMOVE_SIGNER_IDS[@]}" -eq 0 ]]; then
  BLOCK_REMOVE_SIGNER_IDS=("$REPLACE_SIGNER_ID")
fi

if [[ "$PASS_MANIFEST_MODE" == "rotate" && "$SLOT_ID" == "$FINALITY_SLOT_ID" && "$REPLACEMENT_SIGNER_ID" == "$REPLACE_SIGNER_ID" ]]; then
  echo "finality slot rotation requires a new signer id; pass --replacement-signer-id for $SLOT_ID" >&2
  exit 1
fi

require_command jq
require_command cp
require_command date

if [[ ! -d "$SOURCE_WORLD_DIR" ]]; then
  echo "source world dir does not exist: $SOURCE_WORLD_DIR" >&2
  exit 1
fi
if [[ ! -f "$BASELINE_MANIFEST" ]]; then
  echo "baseline manifest does not exist: $BASELINE_MANIFEST" >&2
  exit 1
fi
if [[ -n "$REPLACEMENT_PUBLIC_KEY" && ! "$REPLACEMENT_PUBLIC_KEY" =~ ^[0-9a-fA-F]{64}$ ]]; then
  echo "replacement public key must be 32-byte hex" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
MANIFEST_DIR="$OUT_DIR/manifests"
LOG_DIR="$OUT_DIR/logs"
BACKUP_DIR="$OUT_DIR/world-backup-pre-drill"
mkdir -p "$MANIFEST_DIR" "$LOG_DIR"
rm -rf "$BACKUP_DIR"
mkdir -p "$BACKUP_DIR"

PASS_MANIFEST="$MANIFEST_DIR/rotated_pass_manifest.json"
BLOCK_MANIFEST="$MANIFEST_DIR/degraded_block_manifest.json"

BASELINE_SLOT_COUNT="$(jq --arg slot "$SLOT_ID" '[.[] | select(.slot_id == $slot)] | length' "$BASELINE_MANIFEST")"
BASELINE_SLOT_THRESHOLD="$(jq -r --arg slot "$SLOT_ID" '[.[] | select(.slot_id == $slot) | (.threshold // 2)] | unique | if length == 0 then error("slot missing") elif length == 1 then .[0] else error("slot threshold mismatch") end' "$BASELINE_MANIFEST")"
MATCHING_SIGNER_COUNT="$(jq --arg slot "$SLOT_ID" --arg signer "$REPLACE_SIGNER_ID" '[.[] | select(.slot_id == $slot and .signer_id == $signer)] | length' "$BASELINE_MANIFEST")"
REPLACEMENT_SIGNER_EXISTS_COUNT="$(jq --arg slot "$SLOT_ID" --arg signer "$REPLACEMENT_SIGNER_ID" '[.[] | select(.slot_id == $slot and .signer_id == $signer)] | length' "$BASELINE_MANIFEST")"
if (( BASELINE_SLOT_COUNT < 2 )); then
  echo "expected at least 2 manifest entries for slot $SLOT_ID, got $BASELINE_SLOT_COUNT" >&2
  exit 1
fi
if [[ "$MATCHING_SIGNER_COUNT" != "1" ]]; then
  echo "expected exactly 1 manifest entry for slot $SLOT_ID signer $REPLACE_SIGNER_ID, got $MATCHING_SIGNER_COUNT" >&2
  exit 1
fi
if [[ "$PASS_MANIFEST_MODE" == "rotate" && "$REPLACEMENT_SIGNER_ID" != "$REPLACE_SIGNER_ID" && "$REPLACEMENT_SIGNER_EXISTS_COUNT" != "0" ]]; then
  echo "replacement signer id already exists in slot $SLOT_ID: $REPLACEMENT_SIGNER_ID" >&2
  exit 1
fi

BLOCK_REMOVE_SIGNER_IDS_JSON="$(printf '%s\n' "${BLOCK_REMOVE_SIGNER_IDS[@]}" | jq -R . | jq -s .)"
BLOCK_REMOVE_MATCHING_COUNT="$(jq \
  --arg slot "$SLOT_ID" \
  --argjson signers "$BLOCK_REMOVE_SIGNER_IDS_JSON" \
  '[.[] | select(.slot_id == $slot and (.signer_id as $id | $signers | index($id)) != null)] | length' \
  "$BASELINE_MANIFEST")"
if [[ "$BLOCK_REMOVE_MATCHING_COUNT" != "${#BLOCK_REMOVE_SIGNER_IDS[@]}" ]]; then
  echo "each --block-remove-signer-id must exist exactly once in slot $SLOT_ID" >&2
  exit 1
fi

if [[ "$PASS_MANIFEST_MODE" == "baseline" ]]; then
  cp "$BASELINE_MANIFEST" "$PASS_MANIFEST"
  REPLACEMENT_PUBLIC_KEY="$(jq -r \
    --arg slot "$SLOT_ID" \
    --arg signer "$REPLACE_SIGNER_ID" \
    '.[] | select(.slot_id == $slot and .signer_id == $signer) | .public_key_hex' \
    "$BASELINE_MANIFEST")"
else
  jq \
    --arg slot "$SLOT_ID" \
    --arg signer "$REPLACE_SIGNER_ID" \
    --arg replacement_signer "$REPLACEMENT_SIGNER_ID" \
    --arg replacement_public_key "$REPLACEMENT_PUBLIC_KEY" \
    '
    map(
      if .slot_id == $slot and .signer_id == $signer then
        .signer_id = $replacement_signer
        | .public_key_hex = $replacement_public_key
        | .oc_account_id = ("oc:pk:" + $replacement_public_key)
      else
        .
      end
    )
    ' \
    "$BASELINE_MANIFEST" >"$PASS_MANIFEST"
fi

jq \
  --arg slot "$SLOT_ID" \
  --argjson signers "$BLOCK_REMOVE_SIGNER_IDS_JSON" \
  '
  map(select(.slot_id != $slot or ((.signer_id as $id | $signers | index($id)) == null)))
  ' \
  "$BASELINE_MANIFEST" >"$BLOCK_MANIFEST"

PASS_SLOT_COUNT="$(jq --arg slot "$SLOT_ID" '[.[] | select(.slot_id == $slot)] | length' "$PASS_MANIFEST")"
BLOCK_SLOT_COUNT="$(jq --arg slot "$SLOT_ID" '[.[] | select(.slot_id == $slot)] | length' "$BLOCK_MANIFEST")"
if [[ "$PASS_SLOT_COUNT" != "$BASELINE_SLOT_COUNT" ]]; then
  echo "pass manifest must keep $BASELINE_SLOT_COUNT entries for slot $SLOT_ID, got $PASS_SLOT_COUNT" >&2
  exit 1
fi
if [[ "$BLOCK_SLOT_COUNT" -ge "$BASELINE_SLOT_COUNT" ]]; then
  echo "block manifest must degrade slot $SLOT_ID below $BASELINE_SLOT_COUNT entries, got $BLOCK_SLOT_COUNT" >&2
  exit 1
fi
if (( BLOCK_SLOT_COUNT < BASELINE_SLOT_THRESHOLD )); then
  BLOCK_ENFORCEMENT_STAGE="import_policy_reject"
else
  BLOCK_ENFORCEMENT_STAGE="audit_failover_gate"
fi

cp -a "$SOURCE_WORLD_DIR"/. "$BACKUP_DIR"/

run_and_capture baseline_pre_audit \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$BASELINE_MANIFEST" \
    --finality-slot-id "$FINALITY_SLOT_ID" \
    --expected-threshold "$EXPECTED_THRESHOLD" \
    --strict-manifest-match \
    --require-single-failure-tolerance

run_and_capture pass_import \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$PASS_MANIFEST"

run_and_capture pass_audit \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$PASS_MANIFEST" \
    --finality-slot-id "$FINALITY_SLOT_ID" \
    --expected-threshold "$EXPECTED_THRESHOLD" \
    --strict-manifest-match \
    --require-single-failure-tolerance

run_and_capture block_import \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$BLOCK_MANIFEST"

run_and_capture block_audit \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$BLOCK_MANIFEST" \
    --finality-slot-id "$FINALITY_SLOT_ID" \
    --expected-threshold "$EXPECTED_THRESHOLD" \
    --strict-manifest-match \
    --require-single-failure-tolerance

if [[ "$BLOCK_ENFORCEMENT_STAGE" == "audit_failover_gate" ]]; then
  run_and_capture rejoin_to_pass_import \
    env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- \
      --world-dir "$SOURCE_WORLD_DIR" \
      --public-manifest "$PASS_MANIFEST"

  run_and_capture rejoin_audit \
    env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- \
      --world-dir "$SOURCE_WORLD_DIR" \
      --public-manifest "$PASS_MANIFEST" \
      --finality-slot-id "$FINALITY_SLOT_ID" \
      --expected-threshold "$EXPECTED_THRESHOLD" \
      --strict-manifest-match \
      --require-single-failure-tolerance
else
  printf '%s\n' 0 >"$LOG_DIR/rejoin_to_pass_import.rc"
  printf '%s\n' 0 >"$LOG_DIR/rejoin_audit.rc"
  printf 'rejoin skipped: block case rejected at import stage\n' >"$LOG_DIR/rejoin_to_pass_import.stdout"
  : >"$LOG_DIR/rejoin_to_pass_import.stderr"
  printf '{"overall_status":"rejoin_skipped"}\n' >"$LOG_DIR/rejoin_audit.stdout"
  : >"$LOG_DIR/rejoin_audit.stderr"
fi

run_and_capture restore_import \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$BASELINE_MANIFEST"

run_and_capture restore_audit \
  env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- \
    --world-dir "$SOURCE_WORLD_DIR" \
    --public-manifest "$BASELINE_MANIFEST" \
    --finality-slot-id "$FINALITY_SLOT_ID" \
    --expected-threshold "$EXPECTED_THRESHOLD" \
    --strict-manifest-match \
    --require-single-failure-tolerance

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
BASELINE_MANIFEST_BATCH="$(basename "$(dirname "$BASELINE_MANIFEST")")"
BASELINE_PRE_AUDIT_RC="$(cat "$LOG_DIR/baseline_pre_audit.rc")"
PASS_IMPORT_RC="$(cat "$LOG_DIR/pass_import.rc")"
PASS_AUDIT_RC="$(cat "$LOG_DIR/pass_audit.rc")"
BLOCK_IMPORT_RC="$(cat "$LOG_DIR/block_import.rc")"
BLOCK_AUDIT_RC="$(cat "$LOG_DIR/block_audit.rc")"
REJOIN_TO_PASS_IMPORT_RC="$(cat "$LOG_DIR/rejoin_to_pass_import.rc")"
REJOIN_AUDIT_RC="$(cat "$LOG_DIR/rejoin_audit.rc")"
RESTORE_IMPORT_RC="$(cat "$LOG_DIR/restore_import.rc")"
RESTORE_AUDIT_RC="$(cat "$LOG_DIR/restore_audit.rc")"

jq -n \
  --arg generated_at_utc "$TIMESTAMP" \
  --arg source_world_dir "$SOURCE_WORLD_DIR" \
  --arg backup_dir "$BACKUP_DIR" \
  --arg baseline_manifest_batch "$BASELINE_MANIFEST_BATCH" \
  --arg slot_id "$SLOT_ID" \
  --arg pass_manifest_mode "$PASS_MANIFEST_MODE" \
  --arg replace_signer_id "$REPLACE_SIGNER_ID" \
  --arg replacement_signer_id "$REPLACEMENT_SIGNER_ID" \
  --argjson block_remove_signer_ids "$BLOCK_REMOVE_SIGNER_IDS_JSON" \
  --arg block_enforcement_stage "$BLOCK_ENFORCEMENT_STAGE" \
  --arg replacement_public_key "$REPLACEMENT_PUBLIC_KEY" \
  --arg pass_manifest "$PASS_MANIFEST" \
  --arg block_manifest "$BLOCK_MANIFEST" \
  --argjson baseline_pre_audit_rc "$BASELINE_PRE_AUDIT_RC" \
  --argjson pass_import_rc "$PASS_IMPORT_RC" \
  --argjson pass_audit_rc "$PASS_AUDIT_RC" \
  --argjson block_import_rc "$BLOCK_IMPORT_RC" \
  --argjson block_audit_rc "$BLOCK_AUDIT_RC" \
  --argjson rejoin_to_pass_import_rc "$REJOIN_TO_PASS_IMPORT_RC" \
  --argjson rejoin_audit_rc "$REJOIN_AUDIT_RC" \
  --argjson restore_import_rc "$RESTORE_IMPORT_RC" \
  --argjson restore_audit_rc "$RESTORE_AUDIT_RC" \
  --slurpfile baseline_pre_audit_json "$LOG_DIR/baseline_pre_audit.stdout" \
  --slurpfile pass_import_json "$LOG_DIR/pass_import.stdout" \
  --slurpfile pass_audit_json "$LOG_DIR/pass_audit.stdout" \
  --slurpfile block_import_json "$LOG_DIR/block_import.stdout" \
  --slurpfile block_audit_json "$LOG_DIR/block_audit.stdout" \
  --slurpfile rejoin_to_pass_import_json "$LOG_DIR/rejoin_to_pass_import.stdout" \
  --slurpfile rejoin_audit_json "$LOG_DIR/rejoin_audit.stdout" \
  --slurpfile restore_import_json "$LOG_DIR/restore_import.stdout" \
  --slurpfile restore_audit_json "$LOG_DIR/restore_audit.stdout" \
  '
  {
    generated_at_utc: $generated_at_utc,
    source_world_dir: $source_world_dir,
    backup_dir: $backup_dir,
    baseline_manifest_batch: $baseline_manifest_batch,
    slot_id: $slot_id,
    pass_manifest_mode: $pass_manifest_mode,
    replace_signer_id: $replace_signer_id,
    replacement_signer_id: $replacement_signer_id,
    block_remove_signer_ids: $block_remove_signer_ids,
    block_enforcement_stage: $block_enforcement_stage,
    replacement_public_key: $replacement_public_key,
    baseline_pre: {
      audit_rc: $baseline_pre_audit_rc,
      audit_report: $baseline_pre_audit_json[0],
      expectation_met: ($baseline_pre_audit_rc == 0 and $baseline_pre_audit_json[0].overall_status == "ready_for_ops_drill")
    },
    pass_case: {
      manifest: $pass_manifest,
      import_rc: $pass_import_rc,
      import_summary: $pass_import_json[0],
      audit_rc: $pass_audit_rc,
      audit_report: $pass_audit_json[0],
      expectation_met: ($pass_import_rc == 0 and $pass_audit_rc == 0 and $pass_audit_json[0].overall_status == "ready_for_ops_drill")
    },
    block_case: {
      enforcement_stage: $block_enforcement_stage,
      manifest: $block_manifest,
      import_rc: $block_import_rc,
      import_summary: $block_import_json[0],
      audit_rc: $block_audit_rc,
      audit_report: $block_audit_json[0],
      expectation_met: (
        if $block_enforcement_stage == "import_policy_reject" then
          ($block_import_rc != 0 and $block_audit_rc == 2 and $block_audit_json[0].overall_status == "manifest_mismatch")
        else
          ($block_import_rc == 0 and $block_audit_rc == 2 and $block_audit_json[0].overall_status == "failover_blocked")
        end
      )
    },
    rejoin_case: {
      applicable: ($block_enforcement_stage == "audit_failover_gate"),
      rejoin_import_rc: $rejoin_to_pass_import_rc,
      rejoin_import_summary: $rejoin_to_pass_import_json[0],
      audit_rc: $rejoin_audit_rc,
      audit_report: $rejoin_audit_json[0],
      expectation_met: (
        if $block_enforcement_stage == "audit_failover_gate" then
          ($rejoin_to_pass_import_rc == 0 and $rejoin_audit_rc == 0 and $rejoin_audit_json[0].overall_status == "ready_for_ops_drill")
        else
          ($rejoin_audit_json[0].overall_status == "rejoin_skipped")
        end
      )
    },
    restore: {
      import_rc: $restore_import_rc,
      import_summary: $restore_import_json[0],
      audit_rc: $restore_audit_rc,
      audit_report: $restore_audit_json[0],
      expectation_met: ($restore_import_rc == 0 and $restore_audit_rc == 0 and $restore_audit_json[0].overall_status == "ready_for_ops_drill")
    }
  }
  ' >"$OUT_DIR/summary.json"

BASELINE_STATUS="$(jq -r '.baseline_pre.audit_report.overall_status' "$OUT_DIR/summary.json")"
PASS_STATUS="$(jq -r '.pass_case.audit_report.overall_status' "$OUT_DIR/summary.json")"
BLOCK_STATUS="$(jq -r '.block_case.audit_report.overall_status' "$OUT_DIR/summary.json")"
REJOIN_STATUS="$(jq -r '.rejoin_case.audit_report.overall_status' "$OUT_DIR/summary.json")"
RESTORE_STATUS="$(jq -r '.restore.audit_report.overall_status' "$OUT_DIR/summary.json")"

cat >"$OUT_DIR/run_config.json" <<EOF
{
  "source_world_dir": "$SOURCE_WORLD_DIR",
  "baseline_manifest": "$BASELINE_MANIFEST",
  "slot_id": "$SLOT_ID",
  "pass_manifest_mode": "$PASS_MANIFEST_MODE",
  "replace_signer_id": "$REPLACE_SIGNER_ID",
  "replacement_signer_id": "$REPLACEMENT_SIGNER_ID",
  "block_remove_signer_ids": $BLOCK_REMOVE_SIGNER_IDS_JSON,
  "block_enforcement_stage": "$BLOCK_ENFORCEMENT_STAGE",
  "replacement_public_key": "$REPLACEMENT_PUBLIC_KEY",
  "out_dir": "$OUT_DIR"
}
EOF

cat >"$OUT_DIR/summary.md" <<EOF
# Governance Registry Live-World Drill Summary

- generated_at_utc: $TIMESTAMP
- source_world_dir: \`$SOURCE_WORLD_DIR\`
- baseline_manifest_batch: \`$BASELINE_MANIFEST_BATCH\`
- slot_id: \`$SLOT_ID\`
- pass_manifest_mode: \`$PASS_MANIFEST_MODE\`
- replace_signer_id: \`$REPLACE_SIGNER_ID\`
- replacement_signer_id: \`$REPLACEMENT_SIGNER_ID\`
- block_remove_signer_ids: \`$(printf '%s ' "${BLOCK_REMOVE_SIGNER_IDS[@]}" | sed 's/[[:space:]]*$//')\`
- block_enforcement_stage: \`$BLOCK_ENFORCEMENT_STAGE\`
- replacement_public_key: \`$REPLACEMENT_PUBLIC_KEY\`
- backup_dir: \`$BACKUP_DIR\`

## Baseline Pre
- audit_rc: \`$BASELINE_PRE_AUDIT_RC\`
- overall_status: \`$BASELINE_STATUS\`

## Pass Case
- import_rc: \`$PASS_IMPORT_RC\`
- audit_rc: \`$PASS_AUDIT_RC\`
- overall_status: \`$PASS_STATUS\`

## Block Case
- import_rc: \`$BLOCK_IMPORT_RC\`
- audit_rc: \`$BLOCK_AUDIT_RC\`
- overall_status: \`$BLOCK_STATUS\`

## Rejoin
- import_rc: \`$REJOIN_TO_PASS_IMPORT_RC\`
- audit_rc: \`$REJOIN_AUDIT_RC\`
- overall_status: \`$REJOIN_STATUS\`

## Restore
- import_rc: \`$RESTORE_IMPORT_RC\`
- audit_rc: \`$RESTORE_AUDIT_RC\`
- overall_status: \`$RESTORE_STATUS\`
EOF

printf 'governance registry live-world drill completed: %s\n' "$OUT_DIR"
