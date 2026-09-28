#!/usr/bin/env bash
# Cross-platform contract: the review chain has one executable facade with a
# canonical operator interface and fail-closed immutable-plan behavior.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FACADE="$ROOT_DIR/scripts/pm/review-closeout.sh"

if [[ ! -x "$FACADE" ]]; then
  echo "review-closeout facade must be an executable: $FACADE" >&2
  exit 1
fi
HELP="$($FACADE --help)"
for required in '--task-uid' '--review-plan' '--role-returns' '--print-only'; do
  grep -F -- "$required" <<<"$HELP" >/dev/null || {
    echo "review-closeout --help missing canonical option: $required" >&2
    exit 1
  }
done
SOURCE="$(cat "$FACADE")"
for helper in 'review-batch-epoch.py' 'reconcile' 'collect' 'record-pre-pr-review.sh'; do
  grep -F -- "$helper" <<<"$SOURCE" >/dev/null || {
    echo "review-closeout facade does not delegate canonical helper: $helper" >&2
    exit 1
  }
done
for ordering_contract in 'planned_roles=plan.get("roles")' 'rows=[by_role[role] for role in planned_roles]'; do
  grep -F -- "$ordering_contract" <<<"$SOURCE" >/dev/null || {
    echo "review-closeout must restore immutable review-plan role order after batch reconciliation" >&2
    exit 1
  }
done

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
REPO="$TMPDIR/repo"
UID_VALUE="task_11111111111111111111111111111111"
ROLE="repository_health_engineer"
SLICE="11111111-1111-4111-8111-111111111111"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync"
for helper in review-closeout.sh review-batch-epoch.py record-pre-pr-review.sh validate-review-provenance.py review-findings-resolution.py review_preflight_handoff.py ci_ready_receipt_identity.py; do
  cp "$ROOT_DIR/scripts/pm/$helper" "$REPO/scripts/pm/$helper"
done
chmod +x "$REPO/scripts/pm/review-closeout.sh" "$REPO/scripts/pm/record-pre-pr-review.sh"
printf 'scratch/\n' >"$REPO/.pm/.gitignore"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"project":{"repo":"eng-cc/oasis7"},"tasks":{"$UID_VALUE":{"issue_number":3379}}}
EOF

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/README.md"
git -C "$REPO" add README.md .pm/.gitignore .pm/github-project-sync/tasks.json scripts
git -C "$REPO" commit -qm base
BASE_OID="$(git -C "$REPO" rev-parse HEAD)"
printf 'implementation\n' >>"$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -qm implementation
HEAD_OID="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" branch review-base "$BASE_OID"

TASK_ROOT="$REPO/.pm/scratch/$UID_VALUE"
BATCH="$TASK_ROOT/review-batches/epoch.json"
PREFLIGHT_DIR="$TASK_ROOT/review-preflight"
mkdir -p "$(dirname "$BATCH")"
EVIDENCE_DIGEST="$(printf '%s' review-evidence | shasum -a 256 | awk '{print $1}')"
SOURCE_REVIEW_DIGEST="$(python3 - "$UID_VALUE" "$HEAD_OID" "$BASE_OID" "$EVIDENCE_DIGEST" "$ROLE" <<'PY'
import hashlib, json, sys
task, head, comparison, changed, role = sys.argv[1:]
identity = {
    "task_uid": task, "bootstrap_epoch": 1, "repository": "eng-cc/oasis7", "pr_number": 4139,
    "source_head_oid": head, "source_scope_oid": comparison,
    "changed_paths_digest": changed, "ordered_role_ids": [role],
    "role_contract_digest": "2" * 64, "review_policy_digest": "3" * 64,
    "input_contract_digest": "4" * 64,
}
raw = json.dumps(identity, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
print(hashlib.sha256(raw).hexdigest())
PY
)"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" create \
  --task-uid "$UID_VALUE" --head "$HEAD_OID" --evidence-digest "$SOURCE_REVIEW_DIGEST" \
  --slice "$ROLE=$SLICE" --out "$BATCH" >"$TMPDIR/batch.json"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" preflight \
  --batch "$BATCH" --out-dir "$PREFLIGHT_DIR" >"$TMPDIR/preflight.json"
LEDGER="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["ledger_path"])' "$TMPDIR/preflight.json")"
EPOCH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$BATCH")"
ARTIFACT="$PREFLIGHT_DIR/$SLICE.json"
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"status": "completed", "activation": "message-assigned",
               "context_delivery": "minimal-task-packet",
               "actual_runtime": (
                   "inherited/unverified: message-assigned fallback; adapter inactive on this surface; "
                   "actual runtime/model/reasoning unverified"
               ),
               "scope_verdict": "approved", "risk_verdict": "approved",
               "disposition": "no_findings", "findings": [], "residual_risk": "fixture residual risk"})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
PLAN="$TASK_ROOT/review-plans/fixture.json"
mkdir -p "$(dirname "$PLAN")"
python3 - "$PLAN" "$BATCH" "$HEAD_OID" "$BASE_OID" "$EVIDENCE_DIGEST" "$EPOCH" "$LEDGER" <<'PY'
import hashlib, json, pathlib, sys
plan, batch, head, comparison, evidence, epoch, ledger = sys.argv[1:]
roles = ["repository_health_engineer"]
applicability_identity = {
    "changed_paths_digest": evidence, "input_contract_digest": "4" * 64,
    "ordered_role_ids": roles, "role_contract_digest": "2" * 64,
    "review_policy_digest": "3" * 64,
}
def digest(value):
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(raw).hexdigest()
source_identity = {
    "task_uid": "task_11111111111111111111111111111111", "bootstrap_epoch": 1,
    "repository": "eng-cc/oasis7", "pr_number": 4139, "source_head_oid": head,
    "source_scope_oid": comparison, "changed_paths_digest": evidence,
    "ordered_role_ids": roles, "role_contract_digest": "2" * 64,
    "review_policy_digest": "3" * 64, "input_contract_digest": "4" * 64,
}
payload = {
    "schema": "oasis7-review-plan/v2", "task_uid": "task_11111111111111111111111111111111",
    "frozen_head": head, "comparison_ref": "refs/heads/review-base", "comparison_oid": comparison,
    "source_scope_oid": comparison, "source_review_identity": source_identity,
    "source_review_digest": digest(source_identity), "relevant_evidence_digest": digest(source_identity),
    "professional_review_applicability": {
        "identity": applicability_identity, "identity_digest": digest(applicability_identity), "verified": True,
    }, "roles": roles,
    "expected_slices": [{"role": roles[0], "slice_id": "11111111-1111-4111-8111-111111111111"}],
    "packet_refs": [{
        "role": roles[0], "slice_id": "11111111-1111-4111-8111-111111111111",
        "packet_ref": (
            ".pm/scratch/task_11111111111111111111111111111111/"
            "slice-packets/11111111-1111-4111-8111-111111111111.json"
        ),
    }],
    "epoch": epoch, "batch_path": batch, "preflight": {"status": "incomplete", "ledger_path": ledger},
}
with open(plan, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
packet_payload = {
    "schema": "oasis7-subagent-task-packet/v1",
    "created_at": "2026-09-28T00:00:00+00:00",
    "identity": {
        "task_uid": payload["task_uid"],
        "issue_url": "https://github.com/eng-cc/oasis7/issues/3379",
        "repository": "eng-cc/oasis7", "project_item_id": "fixture-project-item",
        "task_status": "committed", "packet_producer": "tpm",
        "worktree": str(pathlib.Path(plan).parents[4]), "branch": "main",
        "base_ref": "refs/heads/review-base", "base_binding": "immutable_oid",
        "base_sha": comparison, "head": head,
    },
    "slice": {
        "slice_id": "11111111-1111-4111-8111-111111111111",
        "role": "repository_health_engineer", "slice_type": "focused_review",
        "owner_role": "repository_health_engineer", "integration_owner": "tpm",
        "integration_order": "1",
        "context_delivery_mode": "minimal_head_bound_task_packet",
        "intended_model_configuration": "inherit current parent selection",
        "actual_dispatched_model_reasoning": "inherited/unverified",
        "actual_runtime_evidence_reason": (
            "message-assigned fallback; adapter inactive on this surface; "
            "actual runtime/model/reasoning unverified"
        ),
        "role_activation": "message_assigned_adapter_inactive",
        "write_scope": "isolated closeout facade test fixture",
        "return_contract": "complete immutable return fixture",
        "validation_command": "rtk ./scripts/pm/review-closeout-facade.test.sh",
        "formal_sink": "https://github.com/eng-cc/oasis7/issues/3379",
        "full_history_escalation_reason": "",
    },
    "context": {
        "user_intent": "exercise the valid v2 closeout promotion fixture",
        "work_item": "isolated review-closeout facade test",
        "non_goals": "No production changes or external writes",
        "acceptance_target": "valid packet-bound v2 promotion passes",
        "governance_refs": ["AGENTS.md", "doc/engineering/workflow/source-of-truth.md"],
        "scoped_refs": ["scripts/pm/review-closeout-facade.test.sh"],
        "evidence_summary": "synthetic immutable plan and packet fixture",
        "collaboration_boundary": "temporary test repository only",
    },
}
packet = {**packet_payload, "packet_digest": digest(packet_payload)}
packet_path = pathlib.Path(plan).parents[1] / "slice-packets" / "11111111-1111-4111-8111-111111111111.json"
packet_path.parent.mkdir(parents=True, exist_ok=True)
packet_path.write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
PY

# A malformed plan must fail before reconcile can rewrite the ledger or publish
# a collection receipt.
cp "$PLAN" "$TMPDIR/valid-plan.json"
cp "$LEDGER" "$TMPDIR/pre-malformed-ledger.jsonl"
python3 - "$PLAN" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload["roles"].append(payload["roles"][0])
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/malformed-plan.out" 2>"$TMPDIR/malformed-plan.err"); then
  echo "review-closeout accepted duplicate review-plan roles" >&2
  exit 1
fi
grep -F 'roles must be unique' "$TMPDIR/malformed-plan.err" >/dev/null
cmp -s "$LEDGER" "$TMPDIR/pre-malformed-ledger.jsonl" || {
  echo "malformed review plan mutated the preflight ledger" >&2
  exit 1
}
[[ ! -e "${BATCH%.json}.collection.json" ]] || {
  echo "malformed review plan published a collection receipt" >&2
  exit 1
}
cp "$TMPDIR/valid-plan.json" "$PLAN"

# Repository-owned artifacts are required before reconcile/collection. An
# absolute artifact outside the repository must fail without rewriting the
# preflight ledger or creating a collection receipt.
OUTSIDE_ARTIFACT="$TMPDIR/outside-review-return.json"
cp "$ARTIFACT" "$OUTSIDE_ARTIFACT"
cp "$LEDGER" "$TMPDIR/original-outside-ledger.jsonl"
python3 - "$LEDGER" "$OUTSIDE_ARTIFACT" <<'PY'
import hashlib, json, sys
ledger_path, outside = sys.argv[1:]
row = json.loads(open(ledger_path, encoding="utf-8").readline())
row["artifacts"] = [outside]
row["artifact_digest"] = hashlib.sha256(open(outside, "rb").read()).hexdigest()
open(ledger_path, "w", encoding="utf-8").write(json.dumps(row, sort_keys=True) + "\n")
PY
cp "$LEDGER" "$TMPDIR/pre-outside-ledger.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/outside-artifact.out" 2>"$TMPDIR/outside-artifact.err"); then
  echo "review-closeout accepted an artifact outside the repository root" >&2
  exit 1
fi
grep -Eqi 'escapes|repository root|artifact' "$TMPDIR/outside-artifact.err"
cmp -s "$LEDGER" "$TMPDIR/pre-outside-ledger.jsonl" || {
  echo "outside-root artifact mutated the preflight ledger" >&2
  exit 1
}
[[ ! -e "${BATCH%.json}.collection.json" ]] || {
  echo "outside-root artifact published a collection receipt" >&2
  exit 1
}
cp "$TMPDIR/original-outside-ledger.jsonl" "$LEDGER"

COLLECTION="${BATCH%.json}.collection.json"
HANDOFF="$TASK_ROOT/review-handoffs/$EPOCH.json"
MANIFEST="$TASK_ROOT/review-resolutions/$EPOCH.json"
mkdir -p "$(dirname "$HANDOFF")" "$(dirname "$MANIFEST")"
python3 - "$REPO" "$PLAN" "$BATCH" "$LEDGER" "$ARTIFACT" "$HANDOFF" "$MANIFEST" <<'PY'
import hashlib, json, pathlib, sys
root, plan_path, batch_path, ledger_path, artifact_path, handoff_path, manifest_path = map(pathlib.Path, sys.argv[1:])
root = root.resolve()
def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()
def relative(path):
    return path.resolve().relative_to(root).as_posix()
plan = json.loads(plan_path.read_text(encoding="utf-8"))
source_identity = plan["source_review_identity"]
returned = json.loads(artifact_path.read_text(encoding="utf-8"))
handoff_payload = {
    "schema": "oasis7-review-return-handoff/v1", "repository": "eng-cc/oasis7",
    "task_uid": plan["task_uid"], "pr_number": source_identity["pr_number"],
    "comparison_ref": plan["comparison_ref"], "comparison_oid": plan["comparison_oid"],
    "frozen_head": plan["frozen_head"], "source_review_identity": source_identity,
    "source_review_digest": plan["source_review_digest"], "epoch": plan["epoch"],
    "plan_path": relative(plan_path), "plan_sha256": hashlib.sha256(plan_path.read_bytes()).hexdigest(),
    "batch_path": relative(batch_path), "batch_sha256": hashlib.sha256(batch_path.read_bytes()).hexdigest(),
    "preflight_ledger_path": relative(ledger_path),
    "preflight_ledger_sha256": hashlib.sha256(ledger_path.read_bytes()).hexdigest(),
    "rows": [{
        "role": returned["role"], "slice_id": returned["slice_id"],
        "artifact_path": relative(artifact_path),
        "return_sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
        "findings_digest": digest(returned["findings"]),
    }],
}
handoff = {**handoff_payload, "handoff_digest": digest(handoff_payload)}
handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
manifest_payload = {
    "schema": "oasis7-review-resolution/v2", "task_uid": plan["task_uid"],
    "head": plan["frozen_head"], "epoch": plan["epoch"],
    "handoff_digest": handoff["handoff_digest"], "role_records": [],
}
manifest_digest = digest(manifest_payload)
manifest_path.write_text(json.dumps({**manifest_payload, "manifest_digest": manifest_digest}, sort_keys=True) + "\n", encoding="utf-8")
body_payload = {
    "marker": "oasis7-review-resolution", "schema": "oasis7-review-resolution/v2",
    "task_uid": plan["task_uid"], "head": plan["frozen_head"],
    "epoch": plan["epoch"], "manifest_digest": manifest_digest,
}
body = json.dumps(body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
readback = {
    **body_payload, "repository": "eng-cc/oasis7", "issue_number": 3379,
    "comment_id": 3934017999,
    "comment_url": "https://github.com/eng-cc/oasis7/issues/3379#issuecomment-3934017999",
    "author": "repo-admin", "created_at": "2026-09-06T10:00:00Z",
    "observed_at": "2026-09-06T10:01:00Z",
    "body_digest": hashlib.sha256(body.encode()).hexdigest(),
}
manifest_path.with_name(f"{plan['epoch']}.readback.json").write_text(
    json.dumps(readback, sort_keys=True) + "\n", encoding="utf-8"
)
(manifest_path.parent / "expected-body.txt").write_text(body, encoding="utf-8")
PY

mkdir -p "$TMPDIR/bin"
cat >"$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
endpoint = None
if args[:2] == ["api", "repos/eng-cc/oasis7/issues/3379"]:
    endpoint = "issue"
elif args[:2] == ["api", f"repos/eng-cc/oasis7/issues/comments/{int(os.environ.get('GH_COMMENT_ID', '3934017999'))}"]:
    endpoint = "comment"
elif args[:2] == ["api", "repos/eng-cc/oasis7/collaborators/repo-admin/permission"]:
    endpoint = "permission"
if endpoint is None:
    raise SystemExit("unexpected fake gh invocation: " + " ".join(args))

mode = os.environ.get("GH_MODE", "valid")
state = pathlib.Path(os.environ["GH_STATE_DIR"])
state.mkdir(parents=True, exist_ok=True)
counter = state / f"{endpoint}.count"
count = int(counter.read_text() or "0") + 1 if counter.exists() else 1
counter.write_text(str(count))

if endpoint == "issue":
    body = "<!-- oasis7-pm-task -->\ntask_uid: task_11111111111111111111111111111111\n"
    if count >= 2 and mode == "issue-marker":
        body = "task_uid: task_11111111111111111111111111111111\n"
    elif count >= 2 and mode == "issue-task-uid":
        body = "<!-- oasis7-pm-task -->\ntask_uid: task_22222222222222222222222222222222\n"
    response = {"number": 3379, "body": body}
elif endpoint == "comment":
    body = pathlib.Path(os.environ["V2_BODY_FILE"]).read_text()
    author = "repo-admin"
    if count >= 2 and mode == "comment-body":
        payload = json.loads(body)
        payload["task_uid"] = "task_22222222222222222222222222222222"
        body = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    elif count >= 2 and mode == "comment-author":
        author = "different-user"
    comment_id = int(os.environ.get("GH_COMMENT_ID", "3934017999"))
    response = {"id": comment_id, "body": body,
                "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/3379",
                "html_url": f"https://github.com/eng-cc/oasis7/issues/3379#issuecomment-{comment_id}",
                "user": {"login": author}, "created_at": "2026-09-06T10:00:00Z"}
else:
    permission = "write" if count >= 2 and mode == "permission" else "admin"
    response = {"permission": permission}

print(json.dumps(response))
event_log = os.environ.get("EVENT_LOG")
if event_log:
    raw = (json.dumps({"event": "GH_RESPONSE", "endpoint": endpoint,
                       "call_index": count, "worker": os.environ.get("WORKER", "single")},
                      sort_keys=True) + "\n").encode()
    fd = os.open(event_log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, raw)
    finally:
        os.close(fd)
EOF
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"
V2_BODY_FILE="$TASK_ROOT/review-resolutions/expected-body.txt"
GH_STATE_DIR="$TMPDIR/gh-state"
GH_MODE=valid
GH_COMMENT_ID=3934017999
export V2_BODY_FILE
export GH_STATE_DIR GH_MODE GH_COMMENT_ID

cp "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl"
RED_FAILURES=()
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" --task-uid "$UID_VALUE" --review-plan "$PLAN" \
  --role-returns "$LEDGER" --print-only >"$TMPDIR/no-v2-manifest.out" 2>"$TMPDIR/no-v2-manifest.err"); then
  RED_FAILURES+=("all-no-findings preflight was promoted without a v2 handoff manifest")
  printf 'EXPECTED RED (missing manifest): closeout output from the no-manifest preflight attempt follows\n' >&2
  cat "$TMPDIR/no-v2-manifest.out" >&2
elif ! grep -Eqi 'resolution|handoff|v2' "$TMPDIR/no-v2-manifest.err"; then
  cat "$TMPDIR/no-v2-manifest.err" >&2
  echo "no-manifest rejection failed for an unrelated setup reason" >&2
  exit 1
fi
if ! cmp -s "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl" || [[ -e "$COLLECTION" ]]; then
  RED_FAILURES+=("no-manifest all-no-findings attempt changed the ledger or collection")
  printf 'EXPECTED RED (missing-manifest side effects): original=%s current=%s collection_exists=%s\n' \
    "$(shasum -a 256 "$TMPDIR/original-preflight-ledger.jsonl" | awk '{print $1}')" \
    "$(shasum -a 256 "$LEDGER" | awk '{print $1}')" \
    "$( [[ -e "$COLLECTION" ]] && printf true || printf false )" >&2
fi
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"

# Build the one-role all-no-findings expected bytes from the immutable fixture
# inputs. This oracle is test-local and deliberately does not call reconcile or
# collect to produce its own expected result.
NO_FINDINGS_EXPECTED_LEDGER="$TMPDIR/expected-no-findings-promoted-ledger.jsonl"
NO_FINDINGS_RETURN_SNAPSHOT="$TMPDIR/no-findings-return-before-v2.json"
python3 - "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER" "$ARTIFACT" "$HANDOFF" "$REPO" \
  "$UID_VALUE" "$HEAD_OID" "$EPOCH" "$ROLE" "$SLICE" \
  "$NO_FINDINGS_EXPECTED_LEDGER" "$NO_FINDINGS_RETURN_SNAPSHOT" <<'PY'
import hashlib, json, pathlib, sys
ledger_snapshot_path, ledger_path, artifact_path, handoff_path, root_path = map(pathlib.Path, sys.argv[1:6])
task_uid, head, epoch, role, slice_id = sys.argv[6:11]
expected_path, return_snapshot_path = map(pathlib.Path, sys.argv[11:13])
root = root_path.resolve(strict=True)

def sha256(raw):
    return hashlib.sha256(raw).hexdigest()

ledger_bytes = ledger_snapshot_path.read_bytes()
ledger_rows = [json.loads(line) for line in ledger_bytes.decode("utf-8").splitlines() if line.strip()]
if len(ledger_rows) != 1:
    raise SystemExit("all-no-findings expected-byte fixture must contain exactly one preflight row")
row = ledger_rows[0]
identity = {"task_uid": task_uid, "head": head, "epoch": epoch,
            "role": role, "slice_id": slice_id}
for key, expected in identity.items():
    if row.get(key) != expected:
        raise SystemExit(f"all-no-findings preflight {key} does not match the fixture identity")

artifact_path = artifact_path.resolve(strict=True)
relative_artifact = artifact_path.relative_to(root).as_posix()
return_bytes = artifact_path.read_bytes()
returned = json.loads(return_bytes.decode("utf-8"))
for key, expected in {**identity, "status": "completed", "disposition": "no_findings"}.items():
    if returned.get(key) != expected:
        raise SystemExit(f"all-no-findings return {key} does not match the fixture identity")
if returned.get("findings") != []:
    raise SystemExit("all-no-findings return must contain an empty findings array")

handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
if (handoff.get("task_uid") != task_uid or handoff.get("frozen_head") != head
        or handoff.get("epoch") != epoch
        or handoff.get("preflight_ledger_path") != ledger_path.resolve().relative_to(root).as_posix()
        or handoff.get("preflight_ledger_sha256") != sha256(ledger_bytes)):
    raise SystemExit("all-no-findings handoff does not bind the exact preflight fixture")
handoff_rows = handoff.get("rows")
if not isinstance(handoff_rows, list) or len(handoff_rows) != 1:
    raise SystemExit("all-no-findings handoff must contain exactly one role row")
handoff_row = handoff_rows[0]
if (handoff_row.get("role") != role or handoff_row.get("slice_id") != slice_id
        or handoff_row.get("artifact_path") != relative_artifact
        or handoff_row.get("return_sha256") != sha256(return_bytes)):
    raise SystemExit("all-no-findings handoff does not bind the exact role return")

return_snapshot_path.write_bytes(return_bytes)
row.update({
    "status": "completed",
    "activation": returned["activation"],
    "context_delivery": returned["context_delivery"],
    "actual_runtime": returned["actual_runtime"],
    "scope_verdict": returned["scope_verdict"],
    "risk_verdict": returned["risk_verdict"],
    "findings": returned["disposition"],
    "residual_risk": returned["residual_risk"],
    "artifact_digest": sha256(return_bytes),
    "artifacts": [str(artifact_path)],
})
expected_path.write_text(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
print(f"NO-FINDINGS EXPECTED LEDGER SHA-256: {sha256(expected_path.read_bytes())}")
print(f"NO-FINDINGS RETURN SNAPSHOT SHA-256: {sha256(return_snapshot_path.read_bytes())}")
PY

capture_bound_artifacts() {
  local snapshot_dir="$1" root="$2" plan="$3" handoff="$4" manifest="$5"
  python3 - "$snapshot_dir" "$root" "$plan" "$handoff" "$manifest" <<'PY'
import json, pathlib, sys
snapshot, root, plan_path, handoff_path, manifest_path = map(pathlib.Path, sys.argv[1:])
root = root.resolve(strict=True)
handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
paths = [plan_path, root / handoff["batch_path"], handoff_path,
         root / handoff["preflight_ledger_path"], manifest_path,
         manifest_path.with_name(f"{manifest_path.stem}.readback.json")]
paths.extend(root / row["artifact_path"] for row in handoff["rows"])
resolved = []
for path in paths:
    path = path.resolve(strict=True)
    path.relative_to(root)
    if path not in resolved:
        resolved.append(path)
snapshot.mkdir(parents=True, exist_ok=False)
entries = []
ledger_path = (root / handoff["preflight_ledger_path"]).resolve(strict=True)
for index, path in enumerate(resolved):
    raw = path.read_bytes()
    name = f"{index:04d}.bytes"
    (snapshot / name).write_bytes(raw)
    entries.append({"path": str(path), "snapshot": name,
                    "sha256": __import__("hashlib").sha256(raw).hexdigest(),
                    "is_preflight_ledger": path == ledger_path})
(snapshot / "manifest.json").write_text(
    json.dumps({"entries": entries}, sort_keys=True) + "\n", encoding="utf-8")
PY
}

assert_bound_artifacts_unchanged() {
  local snapshot_dir="$1" mode="$2" expected_ledger="${3:-}"
  python3 - "$snapshot_dir" "$mode" "$expected_ledger" <<'PY'
import json, pathlib, sys
snapshot, mode, expected_ledger = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
manifest = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
expected = pathlib.Path(expected_ledger).read_bytes() if expected_ledger else None
for entry in manifest["entries"]:
    path = pathlib.Path(entry["path"])
    before = (snapshot / entry["snapshot"]).read_bytes()
    after = path.read_bytes()
    if entry["is_preflight_ledger"] and mode in {"promoted", "injected"}:
        if after != expected:
            raise SystemExit(f"ledger bytes differ from independent expected oracle: {path}")
    elif after != before:
        raise SystemExit(f"bound artifact bytes changed during facade invocation: {path}")
PY
}

restore_bound_artifacts() {
  local snapshot_dir="$1"
  python3 - "$snapshot_dir" <<'PY'
import json, pathlib, sys
snapshot = pathlib.Path(sys.argv[1])
manifest = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
for entry in manifest["entries"]:
    pathlib.Path(entry["path"]).write_bytes((snapshot / entry["snapshot"]).read_bytes())
PY
}

assert_bound_artifact_drift() {
  local snapshot_dir="$1" target="$2" injected="$3"
  python3 - "$snapshot_dir" "$target" "$injected" <<'PY'
import json, pathlib, sys
snapshot, target, injected = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]).resolve(), pathlib.Path(sys.argv[3])
expected_target = injected.read_bytes()
manifest = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
found = False
for entry in manifest["entries"]:
    path = pathlib.Path(entry["path"])
    before = (snapshot / entry["snapshot"]).read_bytes()
    after = path.read_bytes()
    if path == target:
        found = True
        if after != expected_target:
            raise SystemExit(f"parent-injected target bytes were not preserved: {path}")
    elif after != before:
        raise SystemExit(f"non-target bound artifact changed during drift rejection: {path}")
if not found:
    raise SystemExit(f"drift target is not one of the immutable handoff-bound paths: {target}")
PY
}

restore_bound_artifacts() {
  local snapshot_dir="$1"
  python3 - "$snapshot_dir" <<'PY'
import json, pathlib, sys
snapshot = pathlib.Path(sys.argv[1])
manifest = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
for entry in manifest["entries"]:
    pathlib.Path(entry["path"]).write_bytes((snapshot / entry["snapshot"]).read_bytes())
PY
}

assert_bound_artifact_drift() {
  local snapshot_dir="$1" target="$2" injected="$3"
  python3 - "$snapshot_dir" "$target" "$injected" <<'PY'
import json, pathlib, sys
snapshot, target, injected = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]).resolve(), pathlib.Path(sys.argv[3])
expected_target = injected.read_bytes()
manifest = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
found = False
for entry in manifest["entries"]:
    path = pathlib.Path(entry["path"])
    before = (snapshot / entry["snapshot"]).read_bytes()
    after = path.read_bytes()
    if path == target:
        found = True
        if after != expected_target:
            raise SystemExit(f"parent-injected target bytes were not preserved: {path}")
    elif after != before:
        raise SystemExit(f"non-target bound artifact changed during drift rejection: {path}")
if not found:
    raise SystemExit(f"drift target is not one of the immutable handoff-bound paths: {target}")
PY
}

check_positive_event_order() {
  local event_log="$1"
  python3 - "$event_log" <<'PY'
import json, pathlib, sys
events = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
for endpoint in ("issue", "comment", "permission"):
    reads = [index for index, item in enumerate(events)
             if item.get("event") == "GH_RESPONSE" and item.get("endpoint") == endpoint]
    if len(reads) != 2:
        raise SystemExit(f"ordered remote-read oracle expected exactly two {endpoint} responses, got {len(reads)}")
held = [index for index, item in enumerate(events) if item.get("event") == "RESERVATION_HELD"]
cas = [index for index, item in enumerate(events) if item.get("event") == "CAS_ATTEMPT"]
reconcile = [item for item in events if item.get("event") == "RECONCILE_START"]
if len(held) != 1 or len(cas) != 1 or reconcile:
    raise SystemExit(f"positive v2 oracle requires one held reservation/CAS and zero reconcile; held={len(held)} cas={len(cas)} reconcile={len(reconcile)}")
for endpoint in ("issue", "comment", "permission"):
    reads = [index for index, item in enumerate(events)
             if item.get("event") == "GH_RESPONSE" and item.get("endpoint") == endpoint]
    if not reads[0] < held[0] < reads[1] < cas[0]:
        raise SystemExit(f"{endpoint} first/second remote reads are not ordered around reservation and CAS")
if sum(item.get("event") == "CAS_APPLIED" for item in events) != 1:
    raise SystemExit("positive v2 oracle did not observe exactly one applied CAS")
if sum(item.get("event") == "COLLECTION_CREATE" for item in events) != 1:
    raise SystemExit("positive v2 oracle did not observe exactly one collection creation")
print("positive v2 oracle: first reads < acquired reservation < second reads < one CAS; zero reconcile")
PY
}

# Install test-local event instrumentation for both positives and all later
# plan-owned failure matrices. The same O_APPEND JSONL stream also receives
# successful fake-GitHub response events.
PROMOTION_HOOK="$TMPDIR/promotion-hook"
mkdir -p "$PROMOTION_HOOK"
cat >"$PROMOTION_HOOK/sitecustomize.py" <<'PY'
import errno, hashlib, json, os, pathlib, sys

def emit(event, **details):
    record = json.dumps({"event": event, "worker": os.environ.get("WORKER", "single"), **details}, sort_keys=True) + "\n"
    raw = record.encode()
    event_log = os.environ.get("EVENT_LOG")
    if event_log:
        fd = os.open(event_log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        try:
            os.write(fd, raw)
        finally:
            os.close(fd)
    event_fd = os.environ.get("EVENT_FD")
    if event_fd:
        os.write(int(event_fd), raw)

_ledger_target = pathlib.Path(os.environ.get("LEDGER_TARGET", "/__not-a-ledger__")).resolve()
_collection_target = pathlib.Path(os.environ.get("COLLECTION_TARGET", "/__not-a-collection__")).resolve()
_replace = os.replace
def tracked_replace(source, destination):
    destination_path = pathlib.Path(destination).resolve()
    if destination_path != _ledger_target:
        return _replace(source, destination)
    payload = pathlib.Path(source).read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    emit("CAS_ATTEMPT", digest=digest, destination=str(destination_path))
    result = _replace(source, destination)
    emit("CAS_APPLIED", digest=digest, destination=str(destination_path))
    return result
os.replace = tracked_replace

_path_open = pathlib.Path.open
def tracked_path_open(self, mode="r", *args, **kwargs):
    handle = _path_open(self, mode, *args, **kwargs)
    if self.resolve() == _collection_target and "x" in mode:
        emit("COLLECTION_CREATE", path=str(_collection_target))
    return handle
pathlib.Path.open = tracked_path_open

try:
    import fcntl
    _flock = fcntl.flock
    _lock_seen = False
    _reservation_identity = None
    def tracked_flock(fd, operation):
        global _lock_seen, _reservation_identity
        if not os.environ.get("LOCK_TEST"):
            return _flock(fd, operation)
        role = os.environ.get("WORKER", "single")
        info = os.fstat(fd)
        identity = {"dev": info.st_dev, "ino": info.st_ino}
        if operation & fcntl.LOCK_UN:
            if _reservation_identity == (info.st_dev, info.st_ino):
                emit("RESERVATION_UNLOCK", **identity)
            return _flock(fd, operation)
        if not (operation & fcntl.LOCK_EX):
            return _flock(fd, operation)
        if role == "A" and not _lock_seen:
            _lock_seen = True
            try:
                _flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as exc:
                emit("A_PROBE_NOT_FREE", errno=exc.errno, **identity)
                raise
            _flock(fd, fcntl.LOCK_UN)
            result = _flock(fd, operation)
            _reservation_identity = (info.st_dev, info.st_ino)
            emit("RESERVATION_HELD", **identity)
            release_fd = int(os.environ["RELEASE_FD"])
            if os.read(release_fd, 1) != b"R":
                raise RuntimeError("parent did not release reservation worker A")
            emit("A_RELEASE_GATE_OPEN", **identity)
            return result
        if role == "B" and not _lock_seen:
            _lock_seen = True
            try:
                _flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as exc:
                if exc.errno not in {errno.EAGAIN, errno.EWOULDBLOCK}:
                    emit("B_PROBE_WRONG_ERROR", errno=exc.errno, **identity)
                    raise
                emit("RESERVATION_ATTEMPT_BLOCKED", errno=exc.errno, **identity)
                result = _flock(fd, operation)
                _reservation_identity = (info.st_dev, info.st_ino)
                emit("RESERVATION_ACQUIRED", **identity)
                return result
            else:
                _flock(fd, fcntl.LOCK_UN)
                emit("B_PROBE_SUCCEEDED_WHILE_A_HELD", **identity)
                result = _flock(fd, operation)
                _reservation_identity = (info.st_dev, info.st_ino)
                emit("RESERVATION_ACQUIRED", **identity)
                return result
        if role == "single" and not _lock_seen and not (operation & fcntl.LOCK_NB):
            result = _flock(fd, operation)
            _lock_seen = True
            _reservation_identity = (info.st_dev, info.st_ino)
            emit("RESERVATION_HELD", **identity)
            return result
        return _flock(fd, operation)
    fcntl.flock = tracked_flock
except ImportError:
    pass

if pathlib.Path(sys.argv[0]).name == "review-batch-epoch.py" and "reconcile" in sys.argv[1:]:
    emit("RECONCILE_START")
PY

NO_FINDINGS_EVENTS="$TMPDIR/no-findings.events"
NO_FINDINGS_BOUND="$TMPDIR/no-findings-bound-before"
rm -rf "$NO_FINDINGS_BOUND"
capture_bound_artifacts "$NO_FINDINGS_BOUND" "$REPO" "$PLAN" "$HANDOFF" "$MANIFEST"
: >"$NO_FINDINGS_EVENTS"
rm -rf "$GH_STATE_DIR"
GH_STATE_DIR="$TMPDIR/gh-state-no-findings"
export GH_STATE_DIR

V2_OUT="$TMPDIR/v2-promotion.out"
V2_ERR="$TMPDIR/v2-promotion.err"
NO_FINDINGS_V2_OK=0
if (cd "$TMPDIR" && EVENT_LOG="$NO_FINDINGS_EVENTS" LOCK_TEST=1 \
  LEDGER_TARGET="$LEDGER" COLLECTION_TARGET="$COLLECTION" \
  PYTHONPATH="$PROMOTION_HOOK${PYTHONPATH:+:$PYTHONPATH}" \
  "$REPO/scripts/pm/review-closeout.sh" --task-uid "$UID_VALUE" --review-plan "$PLAN" \
  --role-returns "$LEDGER" --finding-resolution "$MANIFEST" --print-only \
  >"$V2_OUT" 2>"$V2_ERR"); then
  grep -F 'Pre-PR Local Role Review: passed' "$V2_OUT" >/dev/null
  [[ -e "$COLLECTION" ]] || { echo "v2 promotion omitted collection receipt" >&2; exit 1; }
  cmp -s "$LEDGER" "$NO_FINDINGS_EXPECTED_LEDGER" || {
    echo "all-no-findings v2 promotion did not write the independently expected ledger bytes" >&2; exit 1;
  }
  NO_FINDINGS_EXPECTED_LEDGER_SHA256="$(shasum -a 256 "$NO_FINDINGS_EXPECTED_LEDGER" | awk '{print $1}')"
  python3 - "$COLLECTION" "$NO_FINDINGS_EXPECTED_LEDGER_SHA256" "$EPOCH" "$UID_VALUE" "$HEAD_OID" "$ROLE" <<'PY'
import json, pathlib, sys
collection_path = pathlib.Path(sys.argv[1])
expected = {
    "schema": "oasis7-review-collection/v1",
    "status": "passed",
    "epoch": sys.argv[3],
    "task_uid": sys.argv[4],
    "frozen_head": sys.argv[5],
    "ledger_digest": sys.argv[2],
    "roles": [sys.argv[6]],
}
receipt = json.loads(collection_path.read_text(encoding="utf-8"))
if receipt != expected:
    raise SystemExit("all-no-findings collection receipt identity or expected-ledger digest mismatch")
PY
  cmp -s "$ARTIFACT" "$NO_FINDINGS_RETURN_SNAPSHOT" || {
    echo "all-no-findings v2 promotion rewrote its immutable role return" >&2; exit 1;
  }
  assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" promoted "$NO_FINDINGS_EXPECTED_LEDGER"
  check_positive_event_order "$NO_FINDINGS_EVENTS"
  NO_FINDINGS_V2_OK=1
else
  if grep -Fqx 'review-batch-epoch: direct reconcile cannot write a candidate incomplete preflight ledger without v2 handoff promotion proof' "$V2_ERR"; then
    RED_FAILURES+=("valid all-no-findings v2 promotion still reaches the blocked direct-reconcile path")
    RECONCILE_COUNT="$(python3 - "$NO_FINDINGS_EVENTS" <<'PY'
import json, pathlib, sys
events = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
print(sum(item.get("event") == "RECONCILE_START" for item in events))
PY
)"
    [[ "$RECONCILE_COUNT" -gt 0 ]] || { cat "$V2_ERR" >&2; echo "expected direct-reconcile RED lacked a positive RECONCILE_START event" >&2; exit 1; }
    printf 'EXPECTED RED (all-no-findings v2): exact direct-reconcile denial; observed RECONCILE_START=%s (promotion path missing)\n' "$RECONCILE_COUNT" >&2
    cat "$V2_ERR" >&2
  else
    cat "$V2_ERR" >&2
    echo "valid v2 promotion fixture failed for an unrelated setup reason" >&2
    exit 1
  fi
  cmp -s "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl" || {
    echo "failed v2 positive control mutated original preflight ledger" >&2
    exit 1
  }
  [[ ! -e "$COLLECTION" ]] || { echo "failed v2 positive control created collection" >&2; exit 1; }
  cmp -s "$ARTIFACT" "$NO_FINDINGS_RETURN_SNAPSHOT" || {
    echo "failed all-no-findings v2 positive control mutated the role return" >&2; exit 1;
  }
  assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" unchanged
fi

# Keep the accepted all-no-findings v2 positive above, and add a separate
# finding-bearing v2 handoff whose typed finding, evidence bytes, entry digest,
# and return digest are all source-shaped. Matrices remain gated until both
# positive controls succeed against the exact no-reconcile implementation.
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"
BATCH_FINDINGS="$TASK_ROOT/review-batches/epoch-findings.json"
PREFLIGHT_FINDINGS="$TASK_ROOT/review-preflight-findings"
PLAN_FINDINGS="$TASK_ROOT/review-plans/findings.json"
mkdir -p "$(dirname "$BATCH_FINDINGS")" "$(dirname "$PLAN_FINDINGS")" \
  "$TASK_ROOT/review-handoffs" "$TASK_ROOT/review-resolutions"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" create \
  --task-uid "$UID_VALUE" --head "$HEAD_OID" --evidence-digest "$SOURCE_REVIEW_DIGEST" \
  --slice "$ROLE=$SLICE" --out "$BATCH_FINDINGS" >"$TMPDIR/batch-findings.json"
EPOCH_FINDINGS="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$TMPDIR/batch-findings.json")"
HANDOFF_FINDINGS="$TASK_ROOT/review-handoffs/$EPOCH_FINDINGS.json"
MANIFEST_FINDINGS="$TASK_ROOT/review-resolutions/$EPOCH_FINDINGS.json"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" preflight \
  --batch "$BATCH_FINDINGS" --out-dir "$PREFLIGHT_FINDINGS" >"$TMPDIR/preflight-findings.json"
LEDGER_FINDINGS="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["ledger_path"])' "$TMPDIR/preflight-findings.json")"
ARTIFACT_FINDINGS="$PREFLIGHT_FINDINGS/$SLICE.json"
VERIFY_FINDINGS="$TASK_ROOT/verify-finding-v2.txt"
printf 'exact finding verification bytes\n' >"$VERIFY_FINDINGS"
python3 - "$ARTIFACT_FINDINGS" "$PLAN" "$PLAN_FINDINGS" "$BATCH_FINDINGS" "$LEDGER_FINDINGS" \
  "$HANDOFF_FINDINGS" "$MANIFEST_FINDINGS" "$REPO" "$VERIFY_FINDINGS" \
  "$TMPDIR/expected-findings-promoted-ledger.jsonl" <<'PY'
import hashlib, json, pathlib, sys
artifact_path, base_plan_path, plan_path, batch_path, ledger_path, handoff_path, manifest_path, root_path, verify_path, expected_ledger_path = map(pathlib.Path, sys.argv[1:])
root = root_path.resolve()
def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()
def relative(path):
    return path.resolve().relative_to(root).as_posix()

returned = json.loads(artifact_path.read_text(encoding="utf-8"))
finding = {"id": "FIX-V2-REJECTED", "summary": "fixture finding with repository verification evidence",
           "triage": {"classification": "blocking", "basis": "the exact fixture verification output"}}
returned.update({"status": "completed", "activation": "message-assigned",
                 "context_delivery": "minimal-task-packet",
                 "actual_runtime": (
                     "inherited/unverified: message-assigned fallback; adapter inactive on this surface; "
                     "actual runtime/model/reasoning unverified"
                 ),
                 "scope_verdict": "approved", "risk_verdict": "approved",
                 "disposition": "findings", "findings": [finding],
                 "residual_risk": "fixture finding remains rejected with evidence"})
artifact_path.write_text(json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
ledger_row = json.loads(ledger_path.read_text(encoding="utf-8").splitlines()[0])
ledger_row.update({"status": "completed", "activation": returned["activation"],
                   "context_delivery": returned["context_delivery"],
                   "actual_runtime": returned["actual_runtime"],
                   "scope_verdict": returned["scope_verdict"],
                   "risk_verdict": returned["risk_verdict"],
                   "findings": returned["disposition"],
                   "residual_risk": returned["residual_risk"],
                   "artifact_digest": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
                   "artifacts": [str(artifact_path.resolve())]})
expected_ledger_path.write_text(json.dumps(ledger_row, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
plan = json.loads(base_plan_path.read_text(encoding="utf-8"))
batch = json.loads(batch_path.read_text(encoding="utf-8"))
plan["epoch"] = batch["epoch"]
plan["batch_path"] = str(batch_path.resolve())
plan["preflight"] = {"status": "incomplete", "ledger_path": str(ledger_path.resolve())}
plan_path.write_text(json.dumps(plan, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
source_identity = plan["source_review_identity"]
handoff_payload = {
    "schema": "oasis7-review-return-handoff/v1", "repository": "eng-cc/oasis7",
    "task_uid": plan["task_uid"], "pr_number": source_identity["pr_number"],
    "comparison_ref": plan["comparison_ref"], "comparison_oid": plan["comparison_oid"],
    "frozen_head": plan["frozen_head"], "source_review_identity": source_identity,
    "source_review_digest": plan["source_review_digest"], "epoch": batch["epoch"],
    "plan_path": relative(plan_path), "plan_sha256": hashlib.sha256(plan_path.read_bytes()).hexdigest(),
    "batch_path": relative(batch_path), "batch_sha256": hashlib.sha256(batch_path.read_bytes()).hexdigest(),
    "preflight_ledger_path": relative(ledger_path),
    "preflight_ledger_sha256": hashlib.sha256(ledger_path.read_bytes()).hexdigest(),
    "rows": [{"role": returned["role"], "slice_id": returned["slice_id"],
              "artifact_path": relative(artifact_path),
              "return_sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
              "findings_digest": digest(returned["findings"])}],
}
handoff = {**handoff_payload, "handoff_digest": digest(handoff_payload)}
handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
evidence = verify_path.read_bytes()
finding_digest = hashlib.sha256(canonical(finding)).hexdigest()
entry = {"status": "completed", "index": 0, "finding_digest": finding_digest,
         "disposition": "rejected_with_evidence", "evidence_kind": "repository_verification",
         "evidence_ref": relative(verify_path), "evidence_digest": hashlib.sha256(evidence).hexdigest(),
         "verification_result": {"status": "passed", "output_digest": hashlib.sha256(evidence).hexdigest()}}
entry["entry_digest"] = digest(entry)
manifest_payload = {
    "schema": "oasis7-review-resolution/v2", "task_uid": plan["task_uid"],
    "head": plan["frozen_head"], "epoch": batch["epoch"],
    "handoff_digest": handoff["handoff_digest"],
    "role_records": [{"role": returned["role"], "slice_id": returned["slice_id"],
                      "findings_digest": digest([finding]), "entries": [entry]}],
}
manifest_digest = digest(manifest_payload)
manifest_path.write_text(json.dumps({**manifest_payload, "manifest_digest": manifest_digest},
                                    ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
body_payload = {"marker": "oasis7-review-resolution", "schema": "oasis7-review-resolution/v2",
                "task_uid": plan["task_uid"], "head": plan["frozen_head"],
                "epoch": batch["epoch"], "manifest_digest": manifest_digest}
body = json.dumps(body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
readback = {**body_payload, "repository": "eng-cc/oasis7", "issue_number": 3379,
            "comment_id": 3934018000,
            "comment_url": "https://github.com/eng-cc/oasis7/issues/3379#issuecomment-3934018000",
            "author": "repo-admin", "created_at": "2026-09-06T10:00:00Z",
            "observed_at": "2026-09-06T10:01:00Z",
            "body_digest": hashlib.sha256(body.encode()).hexdigest()}
manifest_path.with_name(f"{batch['epoch']}.readback.json").write_text(
    json.dumps(readback, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
manifest_path.with_name(f"{manifest_path.stem}.expected-body.txt").write_text(body, encoding="utf-8")
PY
COLLECTION_FINDINGS="${BATCH_FINDINGS%.json}.collection.json"
cp "$LEDGER_FINDINGS" "$TMPDIR/original-findings-preflight-ledger.jsonl"
cp "$ARTIFACT_FINDINGS" "$TMPDIR/finding-return-before-v2.json"
V2_BODY_FILE="${MANIFEST_FINDINGS%.json}.expected-body.txt"
GH_STATE_DIR="$TMPDIR/gh-state-findings"
GH_COMMENT_ID=3934018000
export V2_BODY_FILE GH_STATE_DIR GH_COMMENT_ID
FINDINGS_EVENTS="$TMPDIR/finding-bearing.events"
FINDINGS_BOUND="$TMPDIR/finding-bearing-bound-before"
rm -rf "$FINDINGS_BOUND" "$GH_STATE_DIR"
capture_bound_artifacts "$FINDINGS_BOUND" "$REPO" "$PLAN_FINDINGS" "$HANDOFF_FINDINGS" "$MANIFEST_FINDINGS"
: >"$FINDINGS_EVENTS"
FINDINGS_V2_OK=0
if (cd "$TMPDIR" && EVENT_LOG="$FINDINGS_EVENTS" LOCK_TEST=1 \
  LEDGER_TARGET="$LEDGER_FINDINGS" COLLECTION_TARGET="$COLLECTION_FINDINGS" \
  PYTHONPATH="$PROMOTION_HOOK${PYTHONPATH:+:$PYTHONPATH}" \
  "$REPO/scripts/pm/review-closeout.sh" --task-uid "$UID_VALUE" \
  --review-plan "$PLAN_FINDINGS" --role-returns "$LEDGER_FINDINGS" \
  --finding-resolution "$MANIFEST_FINDINGS" --print-only \
  >"$TMPDIR/v2-findings-promotion.out" 2>"$TMPDIR/v2-findings-promotion.err"); then
  grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/v2-findings-promotion.out" >/dev/null
  [[ -e "$COLLECTION_FINDINGS" ]] || { echo "finding-bearing v2 promotion omitted collection receipt" >&2; exit 1; }
  cmp -s "$LEDGER_FINDINGS" "$TMPDIR/expected-findings-promoted-ledger.jsonl" || {
    echo "finding-bearing v2 promotion did not write the independently expected ledger bytes" >&2; exit 1;
  }
  FINDINGS_EXPECTED_LEDGER_SHA256="$(shasum -a 256 "$TMPDIR/expected-findings-promoted-ledger.jsonl" | awk '{print $1}')"
  python3 - "$COLLECTION_FINDINGS" "$FINDINGS_EXPECTED_LEDGER_SHA256" "$EPOCH_FINDINGS" "$UID_VALUE" "$HEAD_OID" "$ROLE" <<'PY'
import json, pathlib, sys
collection = pathlib.Path(sys.argv[1])
expected = {
    "schema": "oasis7-review-collection/v1",
    "status": "passed",
    "epoch": sys.argv[3],
    "task_uid": sys.argv[4],
    "frozen_head": sys.argv[5],
    "ledger_digest": sys.argv[2],
    "roles": [sys.argv[6]],
}
receipt = json.loads(collection.read_text(encoding="utf-8"))
if receipt != expected:
    raise SystemExit("finding-bearing v2 collection receipt differs from the independently expected full object")
PY
  cmp -s "$ARTIFACT_FINDINGS" "$TMPDIR/finding-return-before-v2.json" || {
    echo "finding-bearing v2 promotion rewrote its immutable role return" >&2; exit 1;
  }
  assert_bound_artifacts_unchanged "$FINDINGS_BOUND" promoted "$TMPDIR/expected-findings-promoted-ledger.jsonl"
  check_positive_event_order "$FINDINGS_EVENTS"
  FINDINGS_V2_OK=1
else
  if grep -Fqx 'review-batch-epoch: direct reconcile cannot write a candidate incomplete preflight ledger without v2 handoff promotion proof' "$TMPDIR/v2-findings-promotion.err"; then
    RED_FAILURES+=("valid finding-bearing v2 promotion still reaches the blocked direct-reconcile path")
    RECONCILE_COUNT="$(python3 - "$FINDINGS_EVENTS" <<'PY'
import json, pathlib, sys
events = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
print(sum(item.get("event") == "RECONCILE_START" for item in events))
PY
)"
    [[ "$RECONCILE_COUNT" -gt 0 ]] || { cat "$TMPDIR/v2-findings-promotion.err" >&2; echo "expected finding-bearing direct-reconcile RED lacked a positive RECONCILE_START event" >&2; exit 1; }
    printf 'EXPECTED RED (finding-bearing v2): exact direct-reconcile denial; observed RECONCILE_START=%s (promotion path missing)\n' "$RECONCILE_COUNT" >&2
    cat "$TMPDIR/v2-findings-promotion.err" >&2
  else
    cat "$TMPDIR/v2-findings-promotion.err" >&2
    echo "finding-bearing v2 positive failed for an unrelated setup reason" >&2
    exit 1
  fi
  cmp -s "$LEDGER_FINDINGS" "$TMPDIR/original-findings-preflight-ledger.jsonl" || {
    echo "failed finding-bearing v2 positive control mutated original preflight ledger" >&2; exit 1;
  }
  [[ ! -e "$COLLECTION_FINDINGS" ]] || { echo "failed finding-bearing v2 positive control created collection" >&2; exit 1; }
  cmp -s "$ARTIFACT_FINDINGS" "$TMPDIR/finding-return-before-v2.json" || {
    echo "failed finding-bearing v2 positive control mutated the role return" >&2; exit 1;
  }
  assert_bound_artifacts_unchanged "$FINDINGS_BOUND" unchanged
fi
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"

if [[ "$NO_FINDINGS_V2_OK" != 1 || "$FINDINGS_V2_OK" != 1 ]]; then
  printf 'NOT EXERCISED: reservation-held plan/batch/handoff/preflight-ledger/each-return local-drift matrix; both v2 positives are not yet successful.\n' >&2
  printf 'NOT EXERCISED: successful all-no-findings full receipt/bound-artifact immutability oracle and positive first-read < reservation < second-read < CAS ordering.\n' >&2
  printf 'NOT EXERCISED: second remote Issue/comment/permission rereads and five targeted second-read drift cases; both v2 positives are not yet successful.\n' >&2
  printf 'NOT EXERCISED: finding-bearing full exact receipt and successful bound-artifact immutability postcondition; finding-bearing v2 positive did not succeed.\n' >&2
  printf 'NOT EXERCISED: before/after/third-state uncertain replacement matrix; both v2 positives are not yet successful.\n' >&2
  printf 'NOT EXERCISED: same-inode flock contention oracle; both v2 positives are not yet successful.\n' >&2
  printf 'NOT EXERCISED: later legacy facade assertions: alternate-ledger rejection, unresolved and relative findings, ledger/artifact mismatch, restored valid path, missing-role, and stale-head controls.\n' >&2
  printf 'EXPECTED RED: %s\n' "${RED_FAILURES[@]}" >&2
  exit 1
fi

# All adversarial promotion cases below depend on the valid v2 control above.
# Reuse the independently serialized expected bytes above; never call a
# production writer to derive the expected result for later CAS-state fixtures.
EXPECTED_LEDGER="$NO_FINDINGS_EXPECTED_LEDGER"
EXPECTED_LEDGER_SHA256="$(shasum -a 256 "$EXPECTED_LEDGER" | awk '{print $1}')"
V2_BODY_FILE="$TASK_ROOT/review-resolutions/expected-body.txt"
GH_STATE_DIR="$TMPDIR/gh-state-matrix"
GH_COMMENT_ID=3934017999
export V2_BODY_FILE GH_STATE_DIR GH_COMMENT_ID

# Exercise each plan-owned local input only after the positive reservation
# event. A watchdog may fail and clean up this harness but never proves a pass.
cat >"$TMPDIR/local-drift-worker.py" <<'PY'
import json, os, subprocess, sys
role, closeout, task, plan, ledger, manifest, workdir = sys.argv[1:]
if sys.stdin.readline() != "GO\n":
    raise SystemExit("parent did not open the local-drift worker start gate")
event_fd = int(os.environ["EVENT_FD"])
release_fd = int(os.environ["RELEASE_FD"])
command = [closeout, "--task-uid", task, "--review-plan", plan,
           "--role-returns", ledger, "--finding-resolution", manifest, "--print-only"]
result = subprocess.run(command, cwd=workdir, env=os.environ.copy(), capture_output=True,
                        text=True, pass_fds=(event_fd, release_fd))
print(json.dumps({"returncode": result.returncode, "stdout": result.stdout,
                  "stderr": result.stderr}, sort_keys=True))
PY

cat >"$TMPDIR/local-drift-parent.py" <<'PY'
import json, os, pathlib, re, select, signal, subprocess, sys, time

(case, expected_kind, target_raw, snapshot_raw, repo, task, plan, ledger, manifest,
 collection, tmp_root, worker_script, hook, event_log, injected_path, gh_state,
 body_file) = sys.argv[1:]
target = pathlib.Path(target_raw).resolve(strict=True)
snapshot = pathlib.Path(snapshot_raw)
repo = pathlib.Path(repo).resolve(strict=True)
event_log = pathlib.Path(event_log)
injected_path = pathlib.Path(injected_path)
event_read = event_write = release_read = release_write = None
process = None
event_buffer = bytearray()
events = []
stdout_raw = stderr_raw = b""
released = False
cleanup_done = False
child_output_collected = False

def read_event_ready(timeout):
    global event_buffer
    ready, _, _ = select.select([event_read], [], [], timeout)
    if not ready:
        return False
    chunk = os.read(event_read, 65536)
    if not chunk:
        raise RuntimeError(f"TEST HARNESS FAILURE [{case}]: event stream closed before required positive event")
    event_buffer.extend(chunk)
    while b"\n" in event_buffer:
        line, _, tail = event_buffer.partition(b"\n")
        event_buffer = bytearray(tail)
        if line:
            events.append(json.loads(line.decode("utf-8")))
    return True

def wait_reservation(seconds):
    deadline = time.monotonic() + seconds
    while not any(item.get("event") == "RESERVATION_HELD" and item.get("worker") == "A" for item in events):
        if process.poll() is not None:
            read_event_ready(0)
            if not any(item.get("event") == "RESERVATION_HELD" and item.get("worker") == "A" for item in events):
                raise RuntimeError(f"TEST HARNESS FAILURE [{case}]: worker exited before positive RESERVATION_HELD event")
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("RESERVATION_HELD acquisition watchdog expired")
        read_event_ready(remaining)

def append_parent_event(name, **details):
    raw = (json.dumps({"event": name, "worker": "parent", **details}, sort_keys=True) + "\n").encode()
    fd = os.open(event_log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, raw)
    finally:
        os.close(fd)

def cleanup():
    global release_write, stdout_raw, stderr_raw, cleanup_done, child_output_collected
    if cleanup_done:
        return
    if release_write is not None:
        try:
            os.write(release_write, b"R")
        except OSError:
            pass
        try:
            os.close(release_write)
        except OSError:
            pass
        release_write = None
    if process is not None:
        if process.stdin is not None:
            if not process.stdin.closed:
                try:
                    process.stdin.close()
                except OSError:
                    pass
            # Popen.communicate() flushes stdin unless it is detached. This
            # harness already closed its start-gate pipe before waiting for
            # RESERVATION_HELD, so passing the closed object through cleanup
            # raises ValueError and masks the worker's real result.
            process.stdin = None
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        if not child_output_collected:
            try:
                captured_stdout, captured_stderr = process.communicate(timeout=2)
                if not stdout_raw:
                    stdout_raw = captured_stdout or b""
                if not stderr_raw:
                    stderr_raw = captured_stderr or b""
                child_output_collected = True
            except subprocess.TimeoutExpired as exc:
                if not stdout_raw:
                    stdout_raw = exc.output or b""
                if not stderr_raw:
                    stderr_raw = exc.stderr or b""
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    captured_stdout, captured_stderr = process.communicate(timeout=2)
                    if not stdout_raw:
                        stdout_raw = captured_stdout or b""
                    if not stderr_raw:
                        stderr_raw = captured_stderr or b""
                    child_output_collected = True
                except subprocess.TimeoutExpired:
                    sys.stderr.write(f"TEST HARNESS CLEANUP FAILURE [{case}]: worker output streams did not close after SIGKILL\n")
    for fd in (event_read, event_write, release_read, release_write):
        if fd is not None:
            try:
                os.close(fd)
            except OSError:
                pass
    cleanup_done = True

try:
    saved = json.loads((snapshot / "manifest.json").read_text(encoding="utf-8"))
    for entry in saved["entries"]:
        pathlib.Path(entry["path"]).write_bytes((snapshot / entry["snapshot"]).read_bytes())
    pathlib.Path(collection).unlink(missing_ok=True)
    event_log.write_bytes(b"")
    pathlib.Path(gh_state).mkdir(parents=True, exist_ok=True)

    event_read, event_write = os.pipe()
    release_read, release_write = os.pipe()
    os.set_inheritable(event_write, True)
    os.set_inheritable(release_read, True)
    env = os.environ.copy()
    env.update({"EVENT_FD": str(event_write), "RELEASE_FD": str(release_read),
                "EVENT_LOG": str(event_log), "WORKER": "A", "LOCK_TEST": "1",
                "LEDGER_TARGET": ledger, "COLLECTION_TARGET": collection,
                "GH_MODE": "valid", "GH_STATE_DIR": gh_state, "V2_BODY_FILE": body_file,
                "PYTHONPATH": hook + ((os.pathsep + env["PYTHONPATH"]) if env.get("PYTHONPATH") else "")})
    process = subprocess.Popen(
        [sys.executable, worker_script, "A", str(repo / "scripts/pm/review-closeout.sh"),
         task, plan, ledger, manifest, tmp_root], cwd=tmp_root,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        env=env, pass_fds=(event_write, release_read), start_new_session=True)
    os.close(event_write)
    event_write = None
    os.close(release_read)
    release_read = None
    process.stdin.write(b"GO\n")
    process.stdin.close()
    process.stdin = None
    try:
        wait_reservation(30)
    except TimeoutError as exc:
        raise RuntimeError(f"TEST HARNESS FAILURE [{case}]: {exc}") from exc

    held = next(item for item in events if item.get("event") == "RESERVATION_HELD" and item.get("worker") == "A")
    before = target.read_bytes()
    if expected_kind == "handoff":
        payload = json.loads(before.decode("utf-8"))
        payload["comparison_ref"] = "refs/heads/drift-injected"
        injected = (json.dumps(payload, ensure_ascii=False, sort_keys=True) + "\n").encode()
    else:
        # A trailing blank JSONL/JSON line is deterministic, keeps the object
        # parseable, and changes the exact raw bytes bound by the handoff.
        injected = before + b"\n"
    target.write_bytes(injected)
    injected_path.write_bytes(injected)
    append_parent_event("DRIFT_INJECTED", case=case, path=str(target),
                        sha256=__import__("hashlib").sha256(injected).hexdigest(),
                        dev=held["dev"], ino=held["ino"])
    os.write(release_write, b"R")
    os.close(release_write)
    release_write = None
    try:
        stdout_raw, stderr_raw = process.communicate(timeout=60)
        child_output_collected = True
    except subprocess.TimeoutExpired as exc:
        stdout_raw = exc.output or b""
        stderr_raw = exc.stderr or b""
        raise RuntimeError(f"TEST HARNESS FAILURE [{case}]: post-gate worker completion watchdog expired") from exc
    result = json.loads(stdout_raw.decode("utf-8").splitlines()[-1])
    if result["returncode"] == 0:
        raise RuntimeError(f"TEST ASSERTION FAILURE [{case}]: drifted bound input was accepted")
    patterns = {
        "plan": r"review handoff.*plan|plan.*(binding|digest)",
        "batch": r"review handoff.*batch|batch.*(binding|digest)",
        "handoff": r"review handoff digest mismatch|handoff.*digest",
        "ledger": r"preflight ledger.*(binding|digest)|handoff.*preflight ledger|v2 resolution ledger",
        "return": r"review handoff return digest|artifact digest mismatch",
    }
    if not re.search(patterns[expected_kind], result["stderr"], re.IGNORECASE):
        raise RuntimeError(f"TEST ASSERTION FAILURE [{case}]: rejection lacked targeted {expected_kind} diagnostic")
    log_events = [json.loads(line) for line in event_log.read_text().splitlines() if line]
    held_indexes = [i for i, item in enumerate(log_events) if item.get("event") == "RESERVATION_HELD" and item.get("worker") == "A"]
    drift_indexes = [i for i, item in enumerate(log_events) if item.get("event") == "DRIFT_INJECTED" and item.get("case") == case]
    release_indexes = [i for i, item in enumerate(log_events) if item.get("event") == "A_RELEASE_GATE_OPEN" and item.get("worker") == "A"]
    if len(held_indexes) != 1 or len(drift_indexes) != 1 or len(release_indexes) != 1 or not held_indexes[0] < drift_indexes[0] < release_indexes[0]:
        raise RuntimeError(f"TEST ASSERTION FAILURE [{case}]: events do not prove acquire, parent drift, then gate release")
    for forbidden in ("CAS_ATTEMPT", "RECONCILE_START", "COLLECTION_CREATE"):
        if any(item.get("event") == forbidden for item in log_events):
            raise RuntimeError(f"TEST ASSERTION FAILURE [{case}]: observed forbidden {forbidden}")
    print(f"local drift positive control: {case}; reservation held before injection; targeted rejection preserved bytes; zero CAS/reconcile/collection")
except Exception as exc:
    cleanup()
    sys.stderr.write(f"{exc}\n")
    sys.stderr.write("worker stdout: " + stdout_raw.decode(errors="replace") + "\n")
    sys.stderr.write("worker stderr: " + stderr_raw.decode(errors="replace") + "\n")
    if event_log.exists():
        sys.stderr.write("worker event bytes:\n" + event_log.read_text(errors="replace"))
    raise SystemExit(2)
finally:
    cleanup()
PY

run_local_drift_case() {
  local name="$1" kind="$2" target="$3" event_log="$TMPDIR/local-drift-$1.events"
  local injected="$TMPDIR/local-drift-$1.injected" gh_state="$TMPDIR/local-drift-$1-gh"
  rm -rf "$gh_state"
  if ! python3 "$TMPDIR/local-drift-parent.py" "$name" "$kind" "$target" "$NO_FINDINGS_BOUND" \
    "$REPO" "$UID_VALUE" "$PLAN" "$LEDGER" "$MANIFEST" "$COLLECTION" "$TMPDIR" \
    "$TMPDIR/local-drift-worker.py" "$PROMOTION_HOOK" "$event_log" "$injected" "$gh_state" "$V2_BODY_FILE" \
    >"$TMPDIR/local-drift-$name.out" 2>"$TMPDIR/local-drift-$name.err"; then
    cat "$TMPDIR/local-drift-$name.err" >&2
    return 1
  fi
  cat "$TMPDIR/local-drift-$name.out"
  assert_bound_artifact_drift "$NO_FINDINGS_BOUND" "$target" "$injected"
  [[ ! -e "$COLLECTION" ]] || { echo "local drift rejection created a collection: $name" >&2; return 1; }
}

run_local_drift_case plan plan "$PLAN"
run_local_drift_case batch batch "$BATCH"
run_local_drift_case handoff handoff "$HANDOFF"
run_local_drift_case preflight-ledger ledger "$LEDGER"
RETURN_INDEX=0
while IFS= read -r RETURN_PATH; do
  [[ -n "$RETURN_PATH" ]] || continue
  run_local_drift_case "return-$RETURN_INDEX" return "$RETURN_PATH"
  RETURN_INDEX=$((RETURN_INDEX + 1))
done < <(python3 - "$REPO" "$HANDOFF" <<'PY'
import json, pathlib, sys
root, path = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2])
handoff = json.loads(path.read_text(encoding="utf-8"))
for row in handoff["rows"]:
    print((root / row["artifact_path"]).resolve())
PY
)
restore_bound_artifacts "$NO_FINDINGS_BOUND"
rm -f "$COLLECTION"

THIRD_STATE="$TMPDIR/third-state-ledger.jsonl"
python3 - "$EXPECTED_LEDGER" "$THIRD_STATE" <<'PY'
import json, sys
source, destination = sys.argv[1:]
rows = [json.loads(line) for line in open(source, encoding="utf-8") if line.strip()]
rows[0]["actual_runtime"] = "third-state-writer"
with open(destination, "w", encoding="utf-8") as handle:
    for row in rows:
        json.dump(row, handle, ensure_ascii=False, sort_keys=True)
        handle.write("\n")
PY

run_second_read_drift_case() {
  local mode="$1" endpoint="$2" rc=0 count=0 diagnostic event_log="$TMPDIR/$1.events"
  restore_bound_artifacts "$NO_FINDINGS_BOUND"
  rm -f "$COLLECTION"
  rm -f "$event_log"
  rm -rf "$GH_STATE_DIR"
  mkdir -p "$GH_STATE_DIR"
  if (cd "$TMPDIR" && GH_MODE="$mode" EVENT_LOG="$event_log" LOCK_TEST=1 \
    LEDGER_TARGET="$LEDGER" COLLECTION_TARGET="$COLLECTION" \
    PYTHONPATH="$PROMOTION_HOOK${PYTHONPATH:+:$PYTHONPATH}" \
    "$REPO/scripts/pm/review-closeout.sh" \
    --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
    --finding-resolution "$MANIFEST" --print-only \
    >"$TMPDIR/$mode.out" 2>"$TMPDIR/$mode.err"); then
    echo "second-read drift case was accepted: $mode" >&2
    return 1
  else
    rc=$?
  fi
  [[ "$rc" -ne 0 ]] || { echo "second-read drift did not fail closed: $mode" >&2; return 1; }
  case "$mode" in
    issue-marker) diagnostic='issue|marker|identity' ;;
    issue-task-uid) diagnostic='issue|task uid|identity' ;;
    comment-body) diagnostic='comment|body|binding' ;;
    comment-author) diagnostic='author|comment' ;;
    permission) diagnostic='admin|permission' ;;
  esac
  grep -Eiq "$diagnostic" "$TMPDIR/$mode.err" || {
    echo "second-read rejection was not attributable to targeted live drift: $mode" >&2
    return 1
  }
  [[ -f "$GH_STATE_DIR/$endpoint.count" ]] && count="$(<"$GH_STATE_DIR/$endpoint.count")"
  [[ "$count" -ge 2 ]] || {
    echo "second-read case did not observe the targeted endpoint twice: $mode count=$count" >&2
    return 1
  }
  assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" unchanged
  [[ ! -e "$COLLECTION" ]] || { echo "second-read drift created a collection: $mode" >&2; return 1; }
  python3 - "$event_log" "$endpoint" "$mode" <<'PY'
import json, pathlib, sys
events = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
endpoint, mode = sys.argv[2:]
reads = [index for index, item in enumerate(events)
         if item.get("event") == "GH_RESPONSE" and item.get("endpoint") == endpoint]
held = [index for index, item in enumerate(events) if item.get("event") == "RESERVATION_HELD"]
if len(reads) != 2 or len(held) != 1 or not reads[0] < held[0] < reads[1]:
    raise SystemExit(f"{mode} drift did not positively order its second {endpoint} read after reservation")
for forbidden in ("RECONCILE_START", "CAS_ATTEMPT", "COLLECTION_CREATE"):
    if any(item.get("event") == forbidden for item in events):
        raise SystemExit(f"{mode} second-read drift observed forbidden {forbidden}")
PY
}

for drift_case in issue-marker issue-task-uid comment-body comment-author permission; do
  case "$drift_case" in
    issue-marker|issue-task-uid) endpoint=issue ;;
    comment-body|comment-author) endpoint=comment ;;
    permission) endpoint=permission ;;
  esac
  run_second_read_drift_case "$drift_case" "$endpoint"
done
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"

# Isolated child-process interposition injects the three atomic-replacement
# outcomes at the exact plan-owned ledger destination. The hook is never on
# the normal task path and reports each attempted payload digest explicitly.
REPLACE_HOOK="$TMPDIR/replace-hook"
mkdir -p "$REPLACE_HOOK"
cat >"$REPLACE_HOOK/sitecustomize.py" <<'PY'
import errno, hashlib, json, os, pathlib, sys

_replace = os.replace
_target = pathlib.Path(os.environ.get("LEDGER_TARGET", "/__not-a-ledger__")).resolve()
_events = os.environ.get("EVENT_LOG")

def emit(event, **details):
    record = json.dumps({"event": event, "worker": os.environ.get("WORKER", "single"), **details}, sort_keys=True) + "\n"
    if _events:
        fd = os.open(_events, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        try:
            os.write(fd, record.encode())
        finally:
            os.close(fd)
    event_fd = os.environ.get("EVENT_FD")
    if event_fd:
        os.write(int(event_fd), record.encode())

def tracked_replace(source, destination):
    destination_path = pathlib.Path(destination).resolve()
    if destination_path != _target:
        return _replace(source, destination)
    payload = pathlib.Path(source).read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    mode = os.environ.get("REPLACE_MODE", "normal")
    emit("CAS_ATTEMPT", digest=digest, destination=str(destination_path), mode=mode)
    if mode == "before":
        emit("INJECT_BEFORE_REPLACE", digest=digest)
        raise OSError("test-local failure before ledger replace")
    if mode == "third-state":
        third = pathlib.Path(os.environ["THIRD_STATE_FILE"]).read_bytes()
        destination_path.write_bytes(third)
        emit("THIRD_STATE_WRITTEN", digest=hashlib.sha256(third).hexdigest())
        raise OSError("test-local uncertain replacement with third-state writer")
    result = _replace(source, destination)
    emit("CAS_APPLIED", digest=digest)
    if mode == "after":
        emit("INJECT_AFTER_REPLACE", digest=digest)
        raise OSError("test-local uncertain replacement after atomic replace")
    return result

os.replace = tracked_replace

_ledger_target = pathlib.Path(os.environ.get("LEDGER_TARGET", "/__not-a-ledger__")).resolve()
_collection_target = pathlib.Path(os.environ.get("COLLECTION_TARGET", "/__not-a-collection__")).resolve()
_expected_digest = os.environ.get("EXPECTED_LEDGER_SHA256")
_path_open = pathlib.Path.open
_path_read_bytes = pathlib.Path.read_bytes
_path_read_text = pathlib.Path.read_text

def tracked_path_open(self, mode="r", *args, **kwargs):
    handle = _path_open(self, mode, *args, **kwargs)
    if self.resolve() == _collection_target and "x" in mode:
        emit("COLLECTION_CREATE", path=str(_collection_target))
    return handle

def observed_ledger(path, raw):
    if path.resolve() == _ledger_target and os.environ.get("WORKER") == "B" and _expected_digest:
        digest = hashlib.sha256(raw).hexdigest()
        if digest == _expected_digest:
            emit("EXPECTED_LEDGER_OBSERVED", digest=digest)

def tracked_read_bytes(self):
    raw = _path_read_bytes(self)
    observed_ledger(self, raw)
    return raw

def tracked_read_text(self, *args, **kwargs):
    value = _path_read_text(self, *args, **kwargs)
    observed_ledger(self, value.encode(kwargs.get("encoding") or "utf-8"))
    return value

pathlib.Path.open = tracked_path_open
pathlib.Path.read_bytes = tracked_read_bytes
pathlib.Path.read_text = tracked_read_text

try:
    import fcntl
    _flock = fcntl.flock
    _lock_seen = False
    _reservation_identity = None
    def tracked_flock(fd, operation):
        global _lock_seen, _reservation_identity
        if not os.environ.get("LOCK_TEST"):
            return _flock(fd, operation)
        role = os.environ.get("WORKER", "single")
        info = os.fstat(fd)
        identity = {"dev": info.st_dev, "ino": info.st_ino}
        if operation & fcntl.LOCK_UN:
            if _reservation_identity == (info.st_dev, info.st_ino):
                # Record the unlock call before the kernel releases ownership.
                emit("RESERVATION_UNLOCK", **identity)
            return _flock(fd, operation)
        if not (operation & fcntl.LOCK_EX):
            return _flock(fd, operation)
        if role == "A" and not _lock_seen:
            _lock_seen = True
            try:
                _flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as exc:
                emit("A_PROBE_NOT_FREE", errno=exc.errno, **identity)
                raise
            _flock(fd, fcntl.LOCK_UN)
            result = _flock(fd, operation)
            _reservation_identity = (info.st_dev, info.st_ino)
            emit("RESERVATION_HELD", **identity)
            release_fd = int(os.environ["RELEASE_FD"])
            if os.read(release_fd, 1) != b"R":
                raise RuntimeError("parent did not release reservation worker A")
            emit("A_RELEASE_GATE_OPEN", **identity)
            return result
        if role == "B" and not _lock_seen:
            _lock_seen = True
            try:
                _flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as exc:
                if exc.errno not in {errno.EAGAIN, errno.EWOULDBLOCK}:
                    emit("B_PROBE_WRONG_ERROR", errno=exc.errno, **identity)
                    raise
                emit("RESERVATION_ATTEMPT_BLOCKED", errno=exc.errno, **identity)
                result = _flock(fd, operation)
                _reservation_identity = (info.st_dev, info.st_ino)
                emit("RESERVATION_ACQUIRED", **identity)
                return result
            else:
                _flock(fd, fcntl.LOCK_UN)
                emit("B_PROBE_SUCCEEDED_WHILE_A_HELD", **identity)
                result = _flock(fd, operation)
                _reservation_identity = (info.st_dev, info.st_ino)
                emit("RESERVATION_ACQUIRED", **identity)
                return result
        return _flock(fd, operation)
    fcntl.flock = tracked_flock
except ImportError:
    pass

if pathlib.Path(sys.argv[0]).name == "review-batch-epoch.py" and "reconcile" in sys.argv[1:]:
    emit("RECONCILE_START")
PY

run_replace_fault_case() {
  local mode="$1" rc=0 event_count=0
  restore_bound_artifacts "$NO_FINDINGS_BOUND"
  rm -f "$COLLECTION" "$TMPDIR/$mode.events"
  rm -rf "$GH_STATE_DIR"
  mkdir -p "$GH_STATE_DIR"
  if (cd "$TMPDIR" && GH_MODE=valid PYTHONPATH="$REPLACE_HOOK${PYTHONPATH:+:$PYTHONPATH}" \
    LEDGER_TARGET="$LEDGER" COLLECTION_TARGET="$COLLECTION" EVENT_LOG="$TMPDIR/$mode.events" REPLACE_MODE="$mode" \
    THIRD_STATE_FILE="$THIRD_STATE" EXPECTED_LEDGER_SHA256="$EXPECTED_LEDGER_SHA256" \
    "$REPO/scripts/pm/review-closeout.sh" \
    --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
    --finding-resolution "$MANIFEST" --print-only \
    >"$TMPDIR/$mode.out" 2>"$TMPDIR/$mode.err"); then
    rc=0
  else
    rc=$?
  fi
  [[ -s "$TMPDIR/$mode.events" ]] || {
    echo "test-local os.replace interposer did not observe the plan ledger: $mode" >&2
    return 1
  }
  grep -q '"event": "CAS_ATTEMPT"' "$TMPDIR/$mode.events" || {
    echo "replacement fault was not attempted for mode $mode" >&2
    return 1
  }
  [[ "$(rg -c '"event": "CAS_ATTEMPT"' "$TMPDIR/$mode.events" || true)" == 1 ]] || {
    echo "uncertain replacement attempted more than one ledger CAS: $mode" >&2
    return 1
  }
  grep -Fq "\"digest\": \"$EXPECTED_LEDGER_SHA256\"" "$TMPDIR/$mode.events" || {
    echo "ledger CAS did not attempt the independently expected payload bytes: $mode" >&2
    return 1
  }
  event_count="$(rg -c '"event": "RECONCILE_START"' "$TMPDIR/$mode.events" || true)"
  event_count="${event_count:-0}"
  [[ "$event_count" == 0 ]] || { echo "plan-owned $mode path invoked forbidden reconcile: count=$event_count" >&2; return 1; }
  case "$mode" in
    before)
      [[ "$rc" -ne 0 ]] || { echo "pre-replace injected failure was reported as success" >&2; return 1; }
      grep -q '"event": "INJECT_BEFORE_REPLACE"' "$TMPDIR/$mode.events"
      event_count="$(rg -c '"event": "CAS_APPLIED"' "$TMPDIR/$mode.events" || true)"
      event_count="${event_count:-0}"
      [[ "$event_count" == 0 ]]
      cmp -s "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl"
      [[ ! -e "$COLLECTION" ]]
      [[ ! -s "$TMPDIR/$mode.out" ]]
      assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" unchanged
      ;;
    after)
      cmp -s "$LEDGER" "$EXPECTED_LEDGER" || {
        echo "post-replace uncertain outcome did not leave exact expected bytes" >&2
        return 1
      }
      [[ "$rc" -eq 0 && -e "$COLLECTION" ]] || {
        echo "expected-byte recovery did not continue to exactly one valid collection" >&2
        return 1
      }
      grep -q '"event": "INJECT_AFTER_REPLACE"' "$TMPDIR/$mode.events"
      [[ "$(rg -c '"event": "CAS_APPLIED"' "$TMPDIR/$mode.events")" == 1 ]] || {
        echo "post-replace recovery attempted more than one replacement" >&2
        return 1
      }
      [[ "$(rg -c '"event": "COLLECTION_CREATE"' "$TMPDIR/$mode.events")" == 1 ]] || {
        echo "expected-byte recovery did not create exactly one collection" >&2
        return 1
      }
      grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/$mode.out" >/dev/null
      assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" promoted "$EXPECTED_LEDGER"
      ;;
    third-state)
      cmp -s "$LEDGER" "$THIRD_STATE" || {
        echo "uncertain third-state outcome was overwritten or normalized" >&2
        return 1
      }
      [[ "$rc" -ne 0 ]] || { echo "third-state ledger drift was reported as success" >&2; return 1; }
      [[ ! -e "$COLLECTION" ]] || { echo "third-state outcome created a collection" >&2; return 1; }
      [[ ! -s "$TMPDIR/$mode.out" ]]
      event_count="$(rg -c '"event": "CAS_APPLIED"' "$TMPDIR/$mode.events" || true)"
      event_count="${event_count:-0}"
      [[ "$event_count" == 0 ]] || {
        echo "third-state outcome observed CAS_APPLIED count=$event_count" >&2
        return 1
      }
      assert_bound_artifacts_unchanged "$NO_FINDINGS_BOUND" injected "$THIRD_STATE"
      ;;
  esac
}

run_replace_fault_case before
run_replace_fault_case after
run_replace_fault_case third-state
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"

# The parent gates workers before invocation. The sitecustomize observer emits
# only positive kernel reservation events; elapsed time and missing writes are
# never used as contention evidence.
cat >"$TMPDIR/contention-worker.py" <<'PY'
import json, os, subprocess, sys

role, closeout, task, plan, ledger, manifest, workdir = sys.argv[1:]
if sys.stdin.readline() != "GO\n":
    raise SystemExit("parent did not open the pre-invocation gate")
event_fd = int(os.environ["EVENT_FD"])
release_fd = int(os.environ["RELEASE_FD"])
command = [closeout, "--task-uid", task, "--review-plan", plan, "--role-returns", ledger,
           "--finding-resolution", manifest, "--print-only"]
result = subprocess.run(command, cwd=workdir, env=os.environ.copy(), capture_output=True, text=True,
                        pass_fds=(event_fd, release_fd))
print(json.dumps({"returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}))
PY

cat >"$TMPDIR/contention-parent.py" <<'PY'
import errno, hashlib, json, os, pathlib, re, selectors, signal, subprocess, sys, time

repo, task, plan, ledger, manifest, collection, expected, artifact, handoff, readback = sys.argv[1:]
repo_path = pathlib.Path(repo).resolve()
handoff_value = json.loads(pathlib.Path(handoff).read_text(encoding="utf-8"))
paths = [pathlib.Path(plan), repo_path / handoff_value["batch_path"], pathlib.Path(handoff),
         pathlib.Path(manifest), pathlib.Path(readback)]
paths.extend(repo_path / row["artifact_path"] for row in handoff_value["rows"])
before = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
expected_bytes = pathlib.Path(expected).read_bytes()
expected_digest = hashlib.sha256(expected_bytes).hexdigest()
implementation_sources = [path for path in (pathlib.Path(repo) / "scripts/pm").iterdir() if path.is_file()]
source_text = "\n".join(path.read_text(encoding="utf-8", errors="replace") for path in implementation_sources)
if not re.search(r"(?:fcntl\.flock|from\s+fcntl\s+import\s+flock)", source_text):
    sys.stderr.write("BLOCKING TESTABILITY GAP: the isolated implementation does not select POSIX fcntl.flock; no contention workers launched, and an equally positive primitive-specific attempt oracle is required.\n")
    raise SystemExit(2)
event_read, event_write = os.pipe()
release_read, release_write = os.pipe()
os.set_inheritable(event_write, True)
os.set_inheritable(release_read, True)
worker_script = str(pathlib.Path(os.environ["TMPDIR"]) / "contention-worker.py")
env_base = os.environ.copy()
env_base.update({"EVENT_FD": str(event_write), "RELEASE_FD": str(release_read),
                 "EXPECTED_LEDGER_SHA256": expected_digest, "LOCK_TEST": "1"})
workers = {}
try:
    for role in ("A", "B"):
        env = dict(env_base, WORKER=role)
        workers[role] = subprocess.Popen(
            [sys.executable, worker_script, role, str(pathlib.Path(repo) / "scripts/pm/review-closeout.sh"),
             task, plan, ledger, manifest, os.environ["TMPDIR"]],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env=env, pass_fds=(event_write, release_read),
            start_new_session=True,
        )
except OSError as exc:
    for process in workers.values():
        if process.stdin is not None:
            process.stdin.close()
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=2)
    for fd in (event_read, event_write, release_read, release_write):
        try:
            os.close(fd)
        except OSError:
            pass
    raise SystemExit(f"TEST HARNESS FAILURE: could not start contention worker: {exc}")
os.close(event_write)
os.close(release_read)

selector = selectors.DefaultSelector()
os.set_blocking(event_read, False)
selector.register(event_read, selectors.EVENT_READ, ("event", None))
for role, process in workers.items():
    for kind, stream in (("stdout", process.stdout), ("stderr", process.stderr)):
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, (kind, role))
events = []
completed = {}
event_buffer = bytearray()
stdout_buffers = {role: bytearray() for role in workers}
stderr_buffers = {role: bytearray() for role in workers}
event_eof = False
release_sent = False
class HarnessFailure(RuntimeError):
    pass

def poll_one(timeout):
    global event_buffer, event_eof
    ready = selector.select(timeout)
    if not ready:
        return False
    for key, _ in ready:
        kind, role = key.data
        if kind == "event":
            chunk = os.read(event_read, 65536)
            if not chunk:
                event_eof = True
                selector.unregister(event_read)
                if event_buffer:
                    raise HarnessFailure("TEST HARNESS FAILURE: event stream closed with a partial JSON event")
            else:
                event_buffer.extend(chunk)
                while b"\n" in event_buffer:
                    line, _, tail = event_buffer.partition(b"\n")
                    event_buffer = bytearray(tail)
                    if line:
                        events.append(json.loads(line.decode("utf-8")))
        else:
            chunk = os.read(key.fileobj.fileno(), 65536)
            if not chunk:
                selector.unregister(key.fileobj)
                if kind == "stdout":
                    if stdout_buffers[role]:
                        completed[role] = json.loads(stdout_buffers[role].decode("utf-8"))
                    elif role not in completed:
                        completed[role] = {"returncode": 255, "stdout": "", "stderr": "worker exited without a result"}
                continue
            if kind == "stdout":
                stdout_buffers[role].extend(chunk)
                if b"\n" in stdout_buffers[role]:
                    line, _, tail = stdout_buffers[role].partition(b"\n")
                    stdout_buffers[role] = bytearray(tail)
                    completed[role] = json.loads(line.decode("utf-8"))
            else:
                stderr_buffers[role].extend(chunk)
    return True

def wait_event(name, role, owner):
    deadline = time.monotonic() + 30
    while not any(event.get("event") == name and event.get("worker") == role for event in events):
        if owner in completed:
            return None
        if event_eof:
            raise HarnessFailure(f"TEST HARNESS FAILURE: event stream ended before required {name} from worker {role}")
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not poll_one(remaining):
            raise HarnessFailure(f"TEST HARNESS FAILURE: {name} acquisition watchdog expired for worker {role}")
    return next(event for event in events
                if event.get("event") == name and event.get("worker") == role)

def release_a():
    global release_sent
    if release_sent:
        return
    try:
        os.write(release_write, b"R")
    except OSError:
        pass
    release_sent = True

try:
    workers["A"].stdin.write(b"GO\n")
    workers["A"].stdin.flush()
    workers["A"].stdin.close()
    held = wait_event("RESERVATION_HELD", "A", "A")
    if held is None:
        result = completed["A"]
        sys.stderr.write("BLOCKING TESTABILITY GAP: valid promotion did not expose an fcntl.flock reservation acquisition; provide a positive test-local attempt observation for the selected primitive.\n")
        sys.stderr.write(result.get("stderr", ""))
        raise SystemExit(2)
    workers["B"].stdin.write(b"GO\n")
    workers["B"].stdin.flush()
    workers["B"].stdin.close()
    blocked = wait_event("RESERVATION_ATTEMPT_BLOCKED", "B", "B")
    if blocked is None:
        result = completed["B"]
        sys.stderr.write("BLOCKING TESTABILITY GAP: worker B completed without an observed same-inode fcntl.flock blocked attempt.\n")
        sys.stderr.write(result.get("stderr", ""))
        raise SystemExit(2)
    if (held["dev"], held["ino"]) != (blocked["dev"], blocked["ino"]):
        raise SystemExit("worker B blocked on a different reservation inode than worker A")
    if blocked.get("errno") not in {errno.EAGAIN, errno.EWOULDBLOCK}:
        raise SystemExit(f"worker B contention probe returned the wrong errno: {blocked}")
    release_a()

    completion_deadline = time.monotonic() + 60
    while len(completed) < len(workers) or any(process.poll() is None for process in workers.values()) or not event_eof:
        remaining = completion_deadline - time.monotonic()
        if remaining <= 0 or not poll_one(remaining):
            raise HarnessFailure("TEST HARNESS FAILURE: post-gate worker completion watchdog expired")
    for role, process in workers.items():
        try:
            process.wait(timeout=0)
        except subprocess.TimeoutExpired as exc:
            raise HarnessFailure(f"TEST HARNESS FAILURE: worker {role} did not exit after its result") from exc
        completed[role]["worker_stderr"] = bytes(stderr_buffers[role]).decode(errors="replace")
    os.close(release_write)
    release_write = None
    os.close(event_read)
    event_read = None

except HarnessFailure as exc:
    sys.stderr.write(str(exc) + "\n")
    for role in workers:
        sys.stderr.write(f"worker {role} stdout: {bytes(stdout_buffers[role]).decode(errors='replace')}\n")
        sys.stderr.write(f"worker {role} stderr: {bytes(stderr_buffers[role]).decode(errors='replace')}\n")
    raise SystemExit(2)
finally:
    # On every path, release the reservation before bounded process-group
    # termination/reap. A deadline is cleanup-only and never a contention proof.
    release_a()
    for process in workers.values():
        if process.stdin is not None and not process.stdin.closed:
            try:
                process.stdin.close()
            except OSError:
                pass
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    for process in workers.values():
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                sys.stderr.write("TEST HARNESS CLEANUP FAILURE: worker remained unreaped after SIGKILL\n")
    cleanup_deadline = time.monotonic() + 2
    while selector.get_map() and time.monotonic() < cleanup_deadline:
        remaining = cleanup_deadline - time.monotonic()
        try:
            if not poll_one(remaining):
                break
        except Exception as exc:
            sys.stderr.write(f"TEST HARNESS CLEANUP FAILURE: could not drain worker/event output: {exc}\n")
            break
    for fd in (release_write, event_read):
        if fd is not None:
            try:
                os.close(fd)
            except OSError:
                pass
    selector.close()

for role, result in completed.items():
    if result.get("returncode") != 0:
        raise SystemExit(f"contention worker {role} failed: {result.get('stderr')} {result.get('worker_stderr')}")
    if result.get("worker_stderr"):
        raise SystemExit(f"contention worker {role} wrote stderr: {result['worker_stderr']}")

def index(event, role=None):
    return next((i for i, item in enumerate(events)
                 if item.get("event") == event and (role is None or item.get("worker") == role)), None)

cas_events = [item for item in events if item.get("event") == "CAS_APPLIED"]
if len(cas_events) != 1 or cas_events[0].get("digest") != expected_digest or cas_events[0].get("worker") != "A":
    raise SystemExit(f"expected exactly one A ledger replacement with the expected payload digest, got {cas_events}")
cas_attempts = [item for item in events if item.get("event") == "CAS_ATTEMPT"]
if len(cas_attempts) != 1 or cas_attempts[0].get("digest") != expected_digest or cas_attempts[0].get("worker") != "A":
    raise SystemExit(f"expected exactly one A compare-and-swap attempt with the expected bytes, got {cas_attempts}")
cas_index = index("CAS_APPLIED", "A")
unlock_index = index("RESERVATION_UNLOCK", "A")
acquire_index = index("RESERVATION_ACQUIRED", "B")
expected_read_index = index("EXPECTED_LEDGER_OBSERVED", "B")
collection_index = index("COLLECTION_CREATE")
if None in (cas_index, unlock_index, acquire_index, expected_read_index, collection_index) or not (
    cas_index < unlock_index < acquire_index < expected_read_index and cas_index < collection_index
):
    raise SystemExit("event order did not prove A CAS/unlock before B exact-byte observation")
if index("B_PROBE_SUCCEEDED_WHILE_A_HELD", "B") is not None:
    raise SystemExit("worker B's real nonblocking same-inode reservation probe unexpectedly succeeded")
if sum(item.get("event") == "COLLECTION_CREATE" for item in events) != 1:
    raise SystemExit("contention did not produce exactly one exclusive collection creation")
reconcile_events = [item for item in events if item.get("event") == "RECONCILE_START"]
if reconcile_events:
    raise SystemExit(f"contention performed forbidden plan-owned reconcile calls: {reconcile_events}")
for role in ("A", "B"):
    if any(item.get("event") == "RECONCILE_START" and item.get("worker") == role for item in events):
        raise SystemExit(f"contention worker {role} performed a forbidden reconcile")
if pathlib.Path(ledger).read_bytes() != expected_bytes:
    raise SystemExit("final ledger bytes differ from the independently computed expected promotion")
collection_value = json.loads(pathlib.Path(collection).read_text(encoding="utf-8"))
if collection_value.get("status") != "passed" or collection_value.get("ledger_digest") != expected_digest:
    raise SystemExit("single collection does not bind the exact expected ledger bytes")
after = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
if after != before:
    raise SystemExit("promotion changed a bound return, handoff, manifest, or readback artifact")
print("contention positive oracle: A held; B same-inode kernel probe blocked; one expected CAS/collection; B observed exact bytes")
PY

cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"
rm -rf "$GH_STATE_DIR"
mkdir -p "$GH_STATE_DIR"
HANDOFF="$TASK_ROOT/review-handoffs/$EPOCH.json"
READBACK="$TASK_ROOT/review-resolutions/$EPOCH.readback.json"
CONTENTION_EVENTS="$TMPDIR/contention.events"
rm -f "$CONTENTION_EVENTS"
if ! (cd "$TMPDIR" && GH_MODE=valid REPLACE_MODE=normal LOCK_TEST=1 \
  LEDGER_TARGET="$LEDGER" COLLECTION_TARGET="$COLLECTION" \
  EVENT_LOG="$CONTENTION_EVENTS" \
  EXPECTED_LEDGER_SHA256="$EXPECTED_LEDGER_SHA256" PYTHONPATH="$REPLACE_HOOK${PYTHONPATH:+:$PYTHONPATH}" \
  python3 "$TMPDIR/contention-parent.py" "$REPO" "$UID_VALUE" "$PLAN" "$LEDGER" "$MANIFEST" \
    "$COLLECTION" "$EXPECTED_LEDGER" "$ARTIFACT" "$HANDOFF" "$READBACK" \
  >"$TMPDIR/contention.out" 2>"$TMPDIR/contention.err"); then
  cat "$TMPDIR/contention.err" >&2
  exit 1
fi
cat "$TMPDIR/contention.out"

VALID_OUT="$TMPDIR/valid.out"
VALID_ERR="$TMPDIR/valid.err"
if ! (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --finding-resolution "$MANIFEST" --print-only \
  >"$VALID_OUT" 2>"$VALID_ERR"); then
  cat "$VALID_ERR" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$VALID_OUT" >/dev/null
grep -F 'Review Plan: .pm/scratch/' "$VALID_OUT" >/dev/null
[[ ! -s "$VALID_ERR" ]] || { cat "$VALID_ERR" >&2; exit 1; }
cp "$LEDGER" "$TMPDIR/complete-ledger.jsonl"

# The immutable plan owns the preflight ledger identity. A caller-provided
# repository-owned alternate ledger must fail before collection or packet
# publication, even when it has matching task/head/slice contents.
ALTERNATE_LEDGER="$TASK_ROOT/alternate-slice-ledger.jsonl"
cp "$LEDGER" "$ALTERNATE_LEDGER"
cp "$LEDGER" "$TMPDIR/canonical-before-alternate.jsonl"
cp "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl"
  if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$ALTERNATE_LEDGER" \
  --print-only \
  >"$TMPDIR/alternate-ledger.out" 2>"$TMPDIR/alternate-ledger.err"); then
  echo "review-closeout accepted a role-return ledger outside the plan preflight path" >&2
  exit 1
fi
grep -Eqi 'preflight ledger|immutable|role-return ledger|ledger path' "$TMPDIR/alternate-ledger.err"
cmp -s "$LEDGER" "$TMPDIR/canonical-before-alternate.jsonl" || {
  echo "alternate ledger changed the canonical preflight ledger" >&2
  exit 1
}
cmp -s "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl" || {
  echo "alternate ledger was mutated before rejection" >&2
  exit 1
}
test -f "$COLLECTION"

# Unresolved role findings must fail closed before collection or packet
# publication. Keep the earlier no-findings success above as the positive
# control for the same facade.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"disposition": "findings", "findings": [{"id": "FIX1-UNRESOLVED", "summary": "fixture unresolved finding"}]})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/unresolved-findings.out" 2>"$TMPDIR/unresolved-findings.err"); then
  echo "review-closeout accepted unresolved role findings" >&2
  exit 1
fi
grep -Eiq 'unresolved|findings|blocked' "$TMPDIR/unresolved-findings.err"
[[ ! -s "$TMPDIR/unresolved-findings.out" ]] || {
  echo "unresolved findings produced a review packet" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "unresolved findings published a collection receipt" >&2
  exit 1
}

# A finding artifact may be ledger-parent-relative. The facade must resolve it
# before reconcile; otherwise the no-resolution scan misses it and reconcile
# mutates the preflight ledger before packet publication is rejected.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" "$LEDGER" <<'PY'
import hashlib, json, sys
from pathlib import Path
artifact_path, ledger_path = sys.argv[1:]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "findings", "findings": [{"id": "FIX7-RELATIVE", "summary": "relative unresolved finding"}]})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "findings"
    row["artifact_digest"] = digest
    row["artifacts"] = [Path(artifact_path).name]
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
PY
cp "$LEDGER" "$TMPDIR/relative-finding-before.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/relative-finding.out" 2>"$TMPDIR/relative-finding.err"); then
  echo "review-closeout accepted an unresolved ledger-parent-relative finding" >&2
  exit 1
fi
grep -Eqi 'unresolved|findings|blocked' "$TMPDIR/relative-finding.err"
cmp -s "$LEDGER" "$TMPDIR/relative-finding-before.jsonl" || {
  echo "ledger-parent-relative finding mutated the ledger before rejection" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "ledger-parent-relative finding published a collection receipt" >&2
  exit 1
}

# An explicit ledger finding disposition must agree with the returned artifact
# before reconcile. Otherwise reconcile downgrades the ledger to no_findings,
# allowing collection and packet publication to proceed.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" "$LEDGER" <<'PY'
import hashlib, json, sys
artifact_path, ledger_path = sys.argv[1:]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "no_findings", "findings": []})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "findings"
    row["artifact_digest"] = digest
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
PY
cp "$LEDGER" "$TMPDIR/ledger-artifact-mismatch-before.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/ledger-artifact-mismatch.out" 2>"$TMPDIR/ledger-artifact-mismatch.err"); then
  echo "review-closeout accepted a ledger finding/artifact no-findings mismatch" >&2
  exit 1
fi
grep -Eiq 'mismatch|findings|disposition' "$TMPDIR/ledger-artifact-mismatch.err"
cmp -s "$LEDGER" "$TMPDIR/ledger-artifact-mismatch-before.jsonl" || {
  echo "ledger/artifact mismatch mutated the ledger before rejection" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "ledger/artifact mismatch published a collection receipt" >&2
  exit 1
}

# Restore the no-findings fixture so the remaining immutable-plan checks keep
# their original positive-control collection.
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"disposition": "no_findings", "findings": []})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
cp "$TMPDIR/complete-ledger.jsonl" "$LEDGER"
if ! (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/restored-valid.out" 2>"$TMPDIR/restored-valid.err"); then
  cat "$TMPDIR/restored-valid.err" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/restored-valid.out" >/dev/null

# An empty role-return ledger must be rejected before packet generation.
: >"$LEDGER"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/missing-role.out" 2>"$TMPDIR/missing-role.err"); then
  echo "review-closeout accepted a missing role return" >&2
  exit 1
fi
grep -Eiq 'identity mismatch|missing expected|ledger digest|role|handoff' "$TMPDIR/missing-role.err"

# A changed working HEAD must invalidate the immutable review plan.
cp "$TMPDIR/complete-ledger.jsonl" "$LEDGER"
test -f "$COLLECTION"
cp "$LEDGER" "$TMPDIR/stale-ledger-before.jsonl"
cp "$COLLECTION" "$TMPDIR/stale-collection-before.json"
git -C "$REPO" commit --allow-empty -qm stale-head
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" \
  --print-only \
  >"$TMPDIR/stale-head.out" 2>"$TMPDIR/stale-head.err"); then
  echo "review-closeout accepted a stale frozen review head" >&2
  exit 1
fi
grep -Eiq 'source head|frozen HEAD|head mismatch' "$TMPDIR/stale-head.err"
cmp "$TMPDIR/stale-ledger-before.jsonl" "$LEDGER"
cmp "$TMPDIR/stale-collection-before.json" "$COLLECTION"

echo "review-closeout-facade.test: OK"
