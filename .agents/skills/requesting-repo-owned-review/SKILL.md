---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and needs fresh involved-role review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [handoff and v2 resolution](../../../doc/engineering/workflow/source-of-truth.md#review-resolution-handoff-promotion), [Freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Pre-PR local role review is required for the frozen source head before promotion. The default v2 path creates a source-only plan from the verified impact projection, then runs source-bound PR CI and professional review concurrently. Ordinary changes may retain a successful PR check whose recorded target base is older than current main when the source head, target ref, check identity, source-review applicability, and mergeability remain valid. High-risk or unknown impact uses trusted exact-current-target integration CI. Pre-PR Ready and promotion fail closed until the applicable current CI identity and role-complete review identity join; review supplements, never replaces, GitHub checks, comments, requested changes, or mergeability. Explicit v1 is compatibility-only.

## When to Use

Use after implementation freeze and before the canonical Pre-PR Ready gate.

## Procedure

Before dispatch, record the plan and batch paths and digests in the single structured GitHub task-Issue dispatch comment described in step 4.

1. Freeze the implementation head and comparison ref using the canonical Freeze gate. Before entering this expensive cycle, run the smallest real boundary loop applicable to the change: configuration generation through the real parser/admission path, viewer through the runtime protocol, persistence write through real recovery, or the standard skill command through the actual helper output. These loops are early contract checks; they do not replace final trusted integration, environment/browser, or professional-review evidence.
2. Classify documentation changes with `./scripts/pm/review-role-selector.py`: mechanical/workflow docs use repository health plus QA; domain-semantic docs use repository health plus one canonical domain specialist (never TPM, QA, repository health, LiveOps/community, or an unknown role) and add QA only when verification changes; external messaging uses repository health plus LiveOps/community and adds QA only when verification changes. Unknown/mixed scope requires one or more ordered `--manual-role <canonical-review-role>` values; missing, duplicate, TPM, and unknown roles fail closed, and explicit documentation classes reject manual roles. Preserve changed-path inference as the safety floor for non-document or unclassified changes.
3. After freeze, generate one digest-bound impact projection from write scope, consumed contracts, public semantics, affected consumers, tests, CI capabilities and required roles; unknown closure widens conservatively. Set `closure_status` only from changed-scope analysis completeness and verifiable scope evidence; never derive it from CI conclusion, review status, or readiness. Create the default v2 source plan with `review-plan.py --impact-projection <projection.json> ... --preflight-dir <dir>`; the helper derives source identity from bound task truth. CI planner, role selector, plan, admission and closeout must bind the same projection digest. `--source-review-input` is compatibility input and must carry that exact projection digest; explicit `--review-schema oasis7-review-plan/v1` remains compatibility-only, and legacy `--evidence-digest` cannot satisfy v2.
4. Confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations, passing `--base <canonical_base_ref> --frozen-base-oid <receipt.base_oid>` and, when the plan carries incremental context, `--review-plan <plan>` so the packet embeds the validated context and role obligation. Before dispatching any role, publish and read back the complete plan-bound packet set once:

   ```sh
   DISPATCH_RESULT="$(./scripts/pm/review-batch-epoch.py --root . dispatch --plan "$PLAN")"
   DISPATCH_COMMENT_ID="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["dispatch_comment_id"])' "$DISPATCH_RESULT")"
   ```

   The helper writes one canonical dispatch comment to the live task Issue and verifies full comment pagination, the exact task/UID and reciprocal PR binding, unique epoch marker, exact body, server comment readback, author, and current repository-admin permission. Do not dispatch if this command fails. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result. Its `packet_digest` must equal the dispatch comment's row for that role/slice; pass the packet bytes bound by that row to the specialist, and require the structured return to include `admitted_packet_digest` equal to that same digest. Admission validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the receipt base and review HEAD. Live HEAD, base-object/ancestry, receipt authority, batch/plan role set, packet, role, or slice drift fails closed; later symbolic-ref movement alone does not. All expected roles remain required: `full_review` roles assess the current delta, while `impact_confirmation` roles must record fresh confirmation against their bound scope digest; uncertainty or authority/policy drift escalates to `full_review`. The admission output alone is ephemeral; the live-read-back dispatch comment is the durable declaration binding the complete packet set. Do not fork full parent history unless a role has a recorded escalation reason.
5. Require each role to return `findings` or `no_findings`, plus `residual_risk`; every structured finding must carry typed triage (`blocking` or `nonblocking`) with an evidence basis. A blocking finding needs a repair or evidence-backed rejection and may not be marked `non_actionable`; nonblocking findings may be marked `non_actionable` only with rationale and residual-risk/revisit evidence. Resolve valid findings or reject them with evidence. Do not manually reconcile or collect the batch. Once all structured returns are present, create the immutable handoff and v2 resolution manifest below; the closeout facade validates and promotes the handoff, collects once, and generates the canonical packet. Do not redispatch a complete unchanged source identity epoch. A transport retry reuses the same immutable batch and slice identities. Reused source review joins with the applicable current PR CI identity. An unrelated target advance does not require a new dispatch or source re-review in ordinary mode. High-risk or unknown impact, a verified related consumer/contract change, a workflow/check policy change, a merge conflict, wrong execution ref/base, source-head or target-ref drift, failure, pending or uncertain readback requires the strict integration path and any affected review epoch.
Before Pre-PR Ready or promotion, perform the fail-closed join against the
current PR: the applicable source-bound PR CI or strict integration identity
and the role-complete source-review identity must bind the same frozen head
and applicable impact projection. Missing, pending, uncertain, stale,
drifted, wrong-head, wrong-app, wrong-ref, failed, or unreadable CI/review
evidence blocks. A legacy `--evidence-digest` or audit-only shadow result
never satisfies this join.

6. After every structured role return is complete, create and publish the
   plan-owned handoff before closeout. From the repository root, run the
   handoff creator once with the live-read-back pre-dispatch comment ID; it
   validates handoff/v2, the current Issue/comment/admin evidence, every packet
   digest, and the admitted digest in each return, then writes
   `.pm/scratch/<uid>/review-handoffs/<epoch>.json` create-once. Historical
   handoff/v1 records are audit-only and cannot promote a new plan-owned
   preflight ledger. Set task UID,
   frozen head, epoch, plan, role ledger, and live task-Issue number from the
   validated plan and task binding, then derive the artifact paths:

   ```sh
   HANDOFF=".pm/scratch/${TASK_UID}/review-handoffs/${EPOCH}.json"
   MANIFEST=".pm/scratch/${TASK_UID}/review-resolutions/${EPOCH}.json"
   READBACK_FILE=".pm/scratch/${TASK_UID}/review-resolutions/${EPOCH}.readback.json"
   BODY_FILE="$(mktemp)"
   POSTED_COMMENT_JSON="$(mktemp)"
   COMMENT_READBACK_JSON="$(mktemp)"
   ```

   ```sh
   ./scripts/pm/review-batch-epoch.py --root . handoff --plan "$PLAN" \
     --dispatch-comment-id "$DISPATCH_COMMENT_ID"
   ```

   Build `role-records.json` as the canonical array of resolution records for
   every finding-bearing return. When every role returned `no_findings`, use
   the empty array `[]`; do not invent role or finding records. Create the
   handoff-bound v2 manifest at its canonical epoch path:

   ```sh
   python3 scripts/pm/review-findings-resolution.py create \
     --root . --task-uid "$TASK_UID" --head "$FROZEN_HEAD" --epoch "$EPOCH" \
     --role-records <repository-relative-role-records.json> \
     --handoff "$HANDOFF"
   ```

   Publish exactly one compact JSON task-Issue comment with fields
   `marker=oasis7-review-resolution`, `schema=oasis7-review-resolution/v2`,
   `task_uid`, `head`, `epoch`, and the created manifest's
   `manifest_digest`. The body is the UTF-8, `ensure_ascii=false`, sorted-key,
   compact JSON serialization of that object, with no surrounding prose. Use
   the canonical live task Issue and a currently repository-admin account.
   Generate the exact body from the created manifest, publish it, and read the
   returned server comment back by ID:

   ```sh
   python3 - "$MANIFEST" "$BODY_FILE" <<'PY'
   import json, pathlib, sys
   manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
   body = {key: manifest[key] for key in ("task_uid", "head", "epoch", "manifest_digest")}
   body.update(marker="oasis7-review-resolution", schema="oasis7-review-resolution/v2")
   pathlib.Path(sys.argv[2]).write_text(
       json.dumps(body, ensure_ascii=False, sort_keys=True, separators=(",", ":")),
       encoding="utf-8",
   )
   PY
   gh api "repos/eng-cc/oasis7/issues/${ISSUE_NUMBER}/comments" --method POST \
     --field "body=$(cat "$BODY_FILE")" > "$POSTED_COMMENT_JSON"
   COMMENT_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$POSTED_COMMENT_JSON")"
   gh api "repos/eng-cc/oasis7/issues/comments/${COMMENT_ID}" > "$COMMENT_READBACK_JSON"
   ```

   Require the read-back server body's exact equality with `BODY_FILE`. Write
   `.pm/scratch/<uid>/review-resolutions/<epoch>.readback.json` from that
   response, taking ID, URL, author, and creation time from the server and
   setting `observed_at` to the current UTC time:

   ```sh
   python3 - "$MANIFEST" "$BODY_FILE" "$COMMENT_READBACK_JSON" \
     "$ISSUE_NUMBER" "$COMMENT_ID" "$READBACK_FILE" <<'PY'
   import datetime, hashlib, json, pathlib, sys
   manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
   body = pathlib.Path(sys.argv[2]).read_bytes()
   comment = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding="utf-8"))
   if comment.get("body") != body.decode("utf-8") or str(comment.get("id")) != sys.argv[5]:
       raise SystemExit("server comment body or ID does not match the published resolution")
   envelope = {
       "schema": manifest["schema"], "marker": "oasis7-review-resolution",
       "task_uid": manifest["task_uid"], "head": manifest["head"],
       "epoch": manifest["epoch"], "manifest_digest": manifest["manifest_digest"],
       "repository": "eng-cc/oasis7", "issue_number": int(sys.argv[4]),
       "comment_id": comment["id"], "comment_url": comment["html_url"],
       "author": comment["user"]["login"], "created_at": comment["created_at"],
       "observed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
       "body_digest": hashlib.sha256(body).hexdigest(),
   }
   with pathlib.Path(sys.argv[6]).open("x", encoding="utf-8") as output:
       output.write(json.dumps(envelope, ensure_ascii=False, sort_keys=True) + chr(10))
   PY
   ```

   For an all-no-findings review, publish and read back the same v2 envelope
   with `role_records=[]`; the handoff still covers every required role and
   slice.

   Then use the facade as the sole promotion/collection/packet entry:

   ```sh
   ./scripts/pm/review-closeout.sh --task-uid "$TASK_UID" --review-plan "$PLAN" \
     --role-returns "$ROLE_RETURNS" --finding-resolution "$MANIFEST"
   ```

   The facade and recorder live-validate the task Issue, exact v2 comment,
   server author/admin permission, readback, handoff, plan, batch, ledger, and
   bound returns before atomic promotion and collection. Use lower-level
   promotion, reconciliation, or collection helpers only for recovery.
   Validate the resulting frozen-head, role-complete ledger and artifacts with
   the repository helper.
7. Continue only when the canonical Pre-PR Ready gate passes. Require trusted runtime attestation only when operating the future unattended supervisor.

Role selection exceptions:

- include `agent_engineer` only when in-world Agent perception, planning, tools, prompt/policy, or agent-facing runtime behavior changed
- repository Codex config/adapter projection/validation contracts require `repository_health_engineer` and `qa_engineer`
- for `.codex/agents/<role>.toml`, require `repository_health_engineer`, `qa_engineer`, and the matching canonical `<role>`
- include `liveops_community` for external messaging, community impact, incidents, player commitments, or channel runbooks

## Return Contract

- reviewed comparison range and frozen head
- involved roles and immutable returns
- findings disposition and residual risk
- canonical packet evidence link, or a canonical blocker with resume instruction

Do not use chat-only review or local fixture output as live task evidence. Do not resolve GitHub review threads solely from this local review. Self-signed evidence never substitutes for runtime attestation in unattended mode.

## Guardrails

Do not omit involved roles or record a passed result before findings are closed.

## Known Failure Modes

Stale-head review; hand-authored attestation; chat-only evidence; confusing local review with GitHub merge readiness.
