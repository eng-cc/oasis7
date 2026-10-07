# Human-operated workflow simplification: requirement and evidence map

**Status:** non-normative implementation and test map for the authorized atomic
same-PR migration. The only workflow authority is
[`source-of-truth.md#workflow-simplification`](./source-of-truth.md#workflow-simplification)
and its adjacent publication-recovery and policy-adoption clauses. If this map
or an acceptance test disagrees with that source, the source controls and the
discrepancy must be corrected before release.

This migration simplifies the existing human-operated path while retaining
the Project-backed task, five lifecycle gates, review/CI evidence, publication
journal, and terminal lifecycle. It does not add another task ledger, service,
or unattended supervisor. The current trusted policy remains the admission and
release authority until the single compatible PR is merged and its effective
revision is read back.

## Implementation boundaries

| Slice | Owned surface | Boundary |
| --- | --- | --- |
| A — normative source | `source-of-truth.md` and this companion | Defines and maps the migration; no helper implementation. |
| B — entrypoints and review completion | `AGENTS.md`, `.agents/skills/`, role/PM/engineering indexes, `review-role-selector.py`, `review-closeout.sh`, v2 resolution/closeout helpers, and focused tests | Keeps entrypoints thin; automates only existing v2 mechanical handoff, resolution, publication/readback, collection, and validation. It does not invent reviewer findings or dispositions. |
| C — PR publication and existing-task adoption | `github-project-task.py`, strict live context and Project readers, PR projection publisher, policy-binding/adoption consumers, and `pr-projection-publish-cli.integration.test.py` | Uses the existing publication journal, canonical binding serializer, live readback, and explicit adoption authority. The actual publisher/`record-pr` subprocess harness uses fake GitHub and exercises the production boundary. No generic recovery ledger. |
| D — hosted validation and CI wiring | Active policy-pin readers in `loop-ci.py` and `workflow-next.py`, `.github/workflows/rust.yml`, `scripts/ci-tests.sh`, and the hosted acceptance harness | Implements validation-start-only and final binding checks; hosted start does not prove Project membership or promotion permission. |

Slices share one task, worktree, and PR. Their ownership table is a conflict
boundary, not a sequence of independently releasable changes. Freeze and
independently review the combined tree before claiming acceptance.

## 2. Upstream constraints and roles

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [FS-R01](source-of-truth.md#workflow-simplification-r01) | Read-only requests avoid task bootstrap and persistent or remote writes; a professional conclusion still comes from its matching role. | [FS-R01](workflow-simplification.design.md#fs-r01-read-only-direct-path) | `repository_health_engineer`; TPM entry routing | Static tests do not prove model behavior or authorize a later write. |
| [FS-R02](source-of-truth.md#workflow-simplification-r02) | Preserve the authorized goal, scope, exclusions, acceptance, and dangerous-effect boundary across ordinary HEAD movement. | [FS-R02](workflow-simplification.design.md#fs-r02-stable-task-authorization) | `tpm`; task/scope admission owners | A goal, scope, side-effect, delivery-object, hold, or expiry change requires renewed authorization. |
| [FS-R03](source-of-truth.md#workflow-simplification-r03) | Keep reproduce/test/implement/targeted-verification within one authorized slice and retain a valid witness for stable automated defects. | [FS-R03](workflow-simplification.design.md#fs-r03-continuous-development) | `qa_engineer`; test-writer and loop owners | Static policy checks cannot prove human pause intent or runtime model compliance. |
| [FS-R04](source-of-truth.md#workflow-simplification-r04) | Recover only a reachable intermediate vector using the existing publisher journal, exact remote identity, write barrier, and unique readback. | [FS-R04](workflow-simplification.design.md#fs-r04-publication-recovery) | `repository_health_engineer`, publisher owner, `qa_engineer` | No recovery from individually plausible but unreachable field combinations; uncertainty never authorizes a duplicate POST. |
| [FS-R05](source-of-truth.md#workflow-simplification-r05) | Report pending, conflict, validation failure, and cleanup-pending as blockers mapped onto existing lifecycle states. | [FS-R05](workflow-simplification.design.md#fs-r05-blocker-classification) | TPM lifecycle coordinator; helper owners | These blocker classes do not add persistent workflow statuses. |
| [FS-R06](source-of-truth.md#workflow-simplification-r06) | Select the complete reviewer set from trusted impact and use the existing frozen v2 review evidence contract. | [FS-R06](workflow-simplification.design.md#fs-r06-review-reduction) | TPM dispatch; `repository_health_engineer` and QA reviewers | The facade cannot create professional findings or dispositions. |
| [FS-R07](source-of-truth.md#workflow-simplification-r07) | Keep task authorization, defect witness, development result, final verification, review, and CI identities distinct. | [FS-R07](workflow-simplification.design.md#fs-r07-authorization-and-evidence-separation) | `qa_engineer`; CI and review evidence owners | Adapter intended configuration is not runtime attestation. |
| [FS-R08](source-of-truth.md#workflow-simplification-r08) | Gate semantic structure, anchors, links, safety contracts, and behavior evidence; document length is advisory only. | [FS-R08](workflow-simplification.design.md#fs-r08-semantic-documentation-gates) | `repository_health_engineer`; documentation governance | Exact prose or line budgets do not prove semantic coverage. |
| [FS-R09](source-of-truth.md#workflow-simplification-r09) | Keep existing tasks on their effective policy until one selected, authorized adoption is written through the canonical binding writer and read back. | [FS-R09](workflow-simplification.design.md#fs-r09-bounded-compatibility) | Publisher owner; live context/Project reader; `qa_engineer` | No batch migration or rewriting historical task evidence; candidate policy cannot authorize itself. |
| [FS-R10](source-of-truth.md#workflow-simplification-r10) | Ship source, compatible implementation, tests, skills, and necessary projections in one PR under the current trusted policy. | [FS-R10](workflow-simplification.design.md#fs-r10-one-complete-pr) | TPM integration; all changed-path reviewers | Merged candidate text alone does not activate the candidate policy. |

## Normative requirement map

| Requirement | Canonical source | Primary implementation / verification evidence |
| --- | --- | --- |
| FS-R01 read-only direct path | [`#workflow-simplification-r01`](./source-of-truth.md#workflow-simplification-r01) | `default-workflow-bootstrap`, `repo-owned-workflow-router`; read-only and entrypoint contract tests. |
| FS-R02 stable task authorization | [`#workflow-simplification-r02`](./source-of-truth.md#workflow-simplification-r02) | `executing-project-tasks`, task/scope admission helpers; task authorization and unchanged-scope tests. |
| FS-R03 continuous development | [`#workflow-simplification-r03`](./source-of-truth.md#workflow-simplification-r03) | `executing-project-tasks`, `tdd-test-writer`; same-slice behavior tests, explicit-pause and valid-witness cases. |
| FS-R04 publication recovery | [`#workflow-simplification-r04`](./source-of-truth.md#workflow-simplification-r04) and [`#workflow-publication-recovery`](./source-of-truth.md#workflow-publication-recovery) | C owns production journal/publisher recovery and the actual publisher/`record-pr` subprocess fake-GitHub harness. B's closeout facade covers its own mechanical handoff/publication/readback path; existing recovery and state-vector suites remain separate evidence. |
| FS-R05 blocker classification | [`#workflow-simplification-r05`](./source-of-truth.md#workflow-simplification-r05) | Existing lifecycle status and helper error surfaces; state-specific pending/conflict/validation tests. |
| FS-R06 review reduction | [`#workflow-simplification-r06`](./source-of-truth.md#workflow-simplification-r06) | `review-role-selector.py`, `review-closeout.sh`, v2 handoff/resolution/packet validators; role-selection and complete-facade tests. |
| FS-R07 authorization/evidence separation | [`#workflow-simplification-r07`](./source-of-truth.md#workflow-simplification-r07) | Existing plan, packet, CI receipt, and adapter validation contracts; identity/applicability and runtime-evidence tests. |
| FS-R08 semantic documentation gates | [`#workflow-simplification-r08`](./source-of-truth.md#workflow-simplification-r08) | `tpm-workflow-doc-contract.test.py`, `doc-governance-check.sh`; anchor, structure, and negative-behavior assertions rather than line budgets. |
| FS-R09 bounded compatibility | [`#workflow-simplification-r09`](./source-of-truth.md#workflow-simplification-r09) and [`#workflow-policy-adoption`](./source-of-truth.md#workflow-policy-adoption) | C owns the adoption adapter and strict live context/Project readers using the canonical binding writer and trusted effective-policy readback; selected-task, rejection-zero-write, replay, and epoch-preservation evidence. |
| FS-R10 one complete PR | [`#workflow-simplification-r10`](./source-of-truth.md#workflow-simplification-r10) | Combined changed-path review plan, frozen-tree verification, and existing old-policy PR/merge/terminal gates. |

The table names likely consumers, not extra authority. A test can support a
source clause but cannot relax it. If an implementation surface or test target
changes, update this map in the same task and keep the source anchor stable.

## 4. Local design obligations

The following fragments make the implementation boundary linkable for the
repository's design traceability gate. They summarize rather than replace the
canonical source.

### FS-R01 read-only direct path

Objective reads and analysis remain direct; formal review, completion, and
write effects require the existing authorized task and matching role.

### FS-R02 stable task authorization

Ordinary commits and HEAD movement within the same accepted goal and scope do
not create a new task epoch or implementation permit.

### FS-R03 continuous development

One authorized slice may reproduce, test, implement, and run targeted checks;
valid defect witnesses and explicit pauses remain meaningful boundaries.

### FS-R04 publication recovery

Use only reachable same-transition recovery with the existing journal and
exact live readback. Uncertain appends remain pending until uniquely resolved.

### FS-R05 blocker classification

Expose the affected action and valid next step while retaining the existing
lifecycle status model.

### FS-R06 review reduction

Keep one unchanged frozen review epoch and automate only mechanical v2
handoff, resolution, publication/readback, collection, and validation.

### FS-R07 authorization and evidence separation

Bind each evidence kind to its actual task, candidate, head, applicability, or
hosted check identity; intended adapter settings are not runtime observations.

### FS-R08 semantic documentation gates

Preserve stable links, anchors, safety semantics, and executable behavior
checks without treating text length as a release decision.

### FS-R09 bounded compatibility

Existing tasks retain their active pin until explicit adoption is proved and
read back; adoption preserves epoch and historical evidence.

### FS-R10 one complete PR

All authorized migration surfaces share one candidate PR, and the old trusted
policy remains the admission and release authority for that candidate.

## Acceptance test map

The T-cases are behavior obligations from the approved design. Each case is
assigned to the existing lifecycle owner that can produce its evidence; no
single focused suite implies the rest of the migration passed.

| Cases | Behavior to prove | Expected evidence owner / surface |
| --- | --- | --- |
| T01 | Read-only facts/analysis have no bootstrap or remote mutation. | B: bootstrap/router entry tests and documented invocation behavior. |
| T02–T03 | Professional judgment and write authorization remain human/role-bound; static tests do not prove LLM behavior or infer user permission. | Out-of-band: independent professional review and human authorization-path evidence; `workflow-simplification.test.py` checks that these rows remain explicitly out-of-band. |
| T04–T06 | Same-scope work stays in one authorized task and epoch; goal/scope/effect changes stop before writes. | D: bootstrap, packet, loop-policy, and loop-CI tests from the acceptance manifest. |
| T07–T09 | Old behavior is captured as a valid defect witness where stable; protective cases may begin green, and fixture/setup failures are not counted as RED evidence. | D: `loop-ci.test.py` and C: publication integration tests provide automated behavior evidence for their owned surfaces. |
| T10–T12 | Explicit pause and ordinary same-slice RED/GREEN do not create a new authorization step; static contracts cannot prove an LLM's behavior or that a human steered a pause. | Out-of-band: independent frozen-diff review, red/green ownership review, and user-steered pause evidence; these cases are not static-CI behavior proofs. |
| T13–T14 | Mechanical docs select RH; workflow/permission/recovery changes retain RH+QA. | B: role-selector and changed-path floor tests. |
| T15–T18 | No-findings v2 completion, unresolved-blocker stop, unchanged-epoch idempotence, and exact frozen-head preservation. | B: closeout facade, v2 resolution tests, duplicate packet/readback and head-drift cases. |
| T19–T28 | Publication state-vector recovery is limited to reachable same-transition states; identity, draft, holds, permissions, exact final binding, and readback uncertainty fail closed. | C owns production behavior and the actual publisher/`record-pr` subprocess fake-GitHub harness; B's review-closeout facade is separate evidence for that facade. C/D helper suites remain independent evidence. |
| T29–T35 | Validation start uses actual trusted event/provenance; final binding rechecks the full hosted identity, while active policy-pin readers follow the frozen rule. | D: `loop-ci.test.py`, `loop-ingress.test.py`, and `workflow-next.test.py`; hosted start/final-binding cases are not proven by local plan metadata alone. |
| T36–T38 | Length alone does not fail docs; stable anchors/safety checks remain; valid implementation-only corrections do not require redundant source text. | B: workflow documentation contract and governance checks. |
| T39 | Existing tasks remain on effective pinned policy until selected authorized adoption; adoption preserves epoch and historical evidence. | C: policy-adoption and closed-schema compatibility tests. |
| T40–T42 | Cleanup pending remains pending; closed schemas reject unsupported fields; candidate policy cannot self-authorize this migration. | C/D and integration owner: lifecycle, schema, old-policy release-gate evidence. |

Process-level fault injection should distinguish pre-write rejection, a remote
write applied with a lost response, and a successful response followed by
local persistence failure. Retries must reconcile one exact live action before
any retry; ambiguous or unreadable state remains pending. Tests use synthetic
identities only in isolated fixtures and must not feed production selectors.

## 11. Validation design and traceability

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [FS-R01](source-of-truth.md#workflow-simplification-r01) | [FS-R01](workflow-simplification.design.md#fs-r01-read-only-direct-path) | Preserve the direct read-only route and do not infer write authorization from analysis. | [workflow-bootstrap-fallback.test.py](../../../scripts/pm/workflow-bootstrap-fallback.test.py) exercises task-binding/bootstrap fallback in an isolated repository fixture. | Task-bound test receipts and independent professional review. | Local fixtures do not prove model behavior or absence of every external side effect. |
| [FS-R02](source-of-truth.md#workflow-simplification-r02) | [FS-R02](workflow-simplification.design.md#fs-r02-stable-task-authorization) | Same authorized goal/scope survives ordinary source movement; changed goal/scope/effect remains a stop. | [subagent-task-packet.test.py](../../../scripts/pm/subagent-task-packet.test.py) checks bound identity and context in an isolated fixture. | Frozen task evidence and reviewed scope projection. | Human intent and live Project authorization use the canonical task path. |
| [FS-R03](source-of-truth.md#workflow-simplification-r03) | [FS-R03](workflow-simplification.design.md#fs-r03-continuous-development) | Keep same-slice RED/GREEN attribution and explicit pause as separate evidence. | [workflow-simplification.test.py](../../../scripts/pm/workflow-simplification.test.py) verifies automated versus out-of-band evidence classification. | Frozen test logs and independent review evidence. | This aggregate does not prove runtime behavior or whether a user requested a pause. |
| [FS-R04](source-of-truth.md#workflow-simplification-r04) | [FS-R04](workflow-simplification.design.md#fs-r04-publication-recovery) | Reconcile only an exact same-transition publication action; do not duplicate a possible append. | [pr-projection-publish-cli.integration.test.py](../../../scripts/pm/pr-projection-publish-cli.integration.test.py) exercises actual publisher/`record-pr` subprocess recovery, including the T21 Issue-complete/Project-old prefix. B's closeout facade is covered separately under FS-R06. | Exact event logs, mutation counts, and journal readback in isolated fake-GitHub fixtures. | Fake services do not prove live GitHub availability or credentials. |
| [FS-R05](source-of-truth.md#workflow-simplification-r05) | [FS-R05](workflow-simplification.design.md#fs-r05-blocker-classification) | Keep unreadability, conflict, validation failure, and cleanup-pending distinct from success. | [workflow-delivery-readiness.test.py](../../../scripts/pm/workflow-delivery-readiness.test.py) checks delivery blockers. | Exact command output and task evidence. | External service outage timing remains environment-dependent. |
| [FS-R06](source-of-truth.md#workflow-simplification-r06) | [FS-R06](workflow-simplification.design.md#fs-r06-review-reduction) | One unchanged frozen plan is collected once; blocking findings need explicit disposition. | [review-closeout-facade.test.sh](../../../scripts/pm/review-closeout-facade.test.sh) covers role selection inputs and v2 closeout. | Immutable plan, scoped returns, resolution readback, and collection receipt. | Local fixtures do not replace independent review of the final candidate. |
| [FS-R07](source-of-truth.md#workflow-simplification-r07) | [FS-R07](workflow-simplification.design.md#fs-r07-authorization-and-evidence-separation) | Bind each evidence kind to its task/head/epoch/applicability; adapter config is not observed runtime. | [review-identity-v2.test.py](../../../scripts/pm/review-identity-v2.test.py) exercises immutable source-review identity. | Review plan, actual dispatch/return evidence, and hosted CI receipt. | Static adapter configuration cannot prove activation or runtime selection. |
| [FS-R08](source-of-truth.md#workflow-simplification-r08) | [FS-R08](workflow-simplification.design.md#fs-r08-semantic-documentation-gates) | Preserve required anchors and behavior contracts without a line-count threshold. | [tpm-workflow-doc-contract.test.py](../../../scripts/pm/tpm-workflow-doc-contract.test.py) checks semantic navigation and structural invariants. | Test output and exact changed-document diff. | Structure checks cannot judge professional semantic adequacy. |
| [FS-R09](source-of-truth.md#workflow-simplification-r09) | [FS-R09](workflow-simplification.design.md#fs-r09-bounded-compatibility) | Adoption is selected, authorized, exact-current-policy, idempotently read back, and epoch-preserving. | [loop-policy.test.py](../../../scripts/pm/loop-policy.test.py) covers effective policy pins; C owns adoption-specific tests. | Protected-tip identity, canonical binding readback, and zero-write rejection evidence. | Local tests do not prove live admin/Project permissions. |
| [FS-R10](source-of-truth.md#workflow-simplification-r10) | [FS-R10](workflow-simplification.design.md#fs-r10-one-complete-pr) | The exact final candidate is reviewed and validated under current trusted policy before release. | [prepare-task-pr-review-risk.test.py](../../../scripts/pm/prepare-task-pr-review-risk.test.py) checks review risk requirements. | Frozen role-complete packet, current CI evidence, and old-policy terminal gates. | Candidate text cannot authorize its own activation. |

## Validation and release evidence

For documentation and skill changes, use `./scripts/lint-skills.sh`,
`./scripts/doc-governance-check.sh`, the focused workflow doc-contract tests,
and `git diff --check`. For the review facade, use its CLI-level fake-GitHub
test and the existing v2 handoff/resolution suites. Publication/CI owners run
the tests listed in their own slice and the final combined acceptance suite.

The final candidate is reviewed on one immutable base/head range with explicit
cross-author path coverage. Required roles are selected from the complete
changed-path impact projection, not from this table or author convenience.
Current trusted policy, holds, review, CI, merge, and terminal gates continue
to govern this PR. Local test fixtures, a source freeze, or a merged document
alone cannot activate the candidate workflow.
