# Three-Loop Completion and Dependency — Proposed Design

Version: **v0.1.0**
Last Updated: **2026-09-09**
Status: **non-normative design companion; implementation and activation pending**

The normative authority is the [proposed completion contract](./source-of-truth.md#three-loop-completion-proposal), alongside the existing [ready/done gates](./source-of-truth.md#ready-and-done) and [terminal runbook](./source-of-truth.md#terminal-runbook). This documentation stage defines subsequent work in the user-authorized single PR; the specification is frozen and independently reviewed before compatible implementation, without requiring a separate prior merge to main. It does not change executable policy, migrate gameplay documents, authorize external operations, release held PR #3645, or activate its candidate helpers. Current production behavior below is distinguished from proposed admission semantics.

The companion [role-adaptation proposal](./three-loop-role-adaptation.design.md) separates professional ownership and slice duties from loop and completion mode. Role text alone does not enable the empty-scope or accepted-dependency capabilities specified here; their producer/consumer validation prerequisites remain unchanged.

## 1. Three ownership loops, two existing completion routes

Loop ownership answers what professional authority a task may change. Completion route answers what proof finishes that task. They are independent dimensions; research does not create a fourth loop, and a deployment without repository edits does not become a repository PR merely to obtain a completion receipt.

| Dimension | Repository-change route | No-repository-change route |
| --- | --- | --- |
| Ownership | One of product, system, code, according to the changed authority | The same three loops, according to the question or authorized operation |
| Existing completion mechanism | Existing task PR, role review, CI, merge and terminal receipts | Existing `completion_mode=non_pr_task`, classification evidence, verified `task_complete`, and `non_pr_completed` |
| Repository scope | Exact declared owned paths; all changed endpoints checked | Empty `write_scope` is allowed in the proposed loop admission; no tracked changes or task commits may be hidden |
| Evidence | Exact reviewed and tested source/integration identities, accepted contracts, merge and cleanup readbacks | Exact classification artifact, acceptance result, task identity and existing non-merge terminal receipt/readbacks |
| External effects | Separate explicit authorization still required | Separate explicit authorization still required; non-PR classification is not permission |

Current production `github-project-task.py classify-non-pr-task` records the existing classification and evidence digest and rejects already-terminal or PR-bound tasks. `non-merge-finalize.py` validates the canonical evidence file and its bytes/digest, task worktree identity, verified closeout claim and absence of PR/merge authority. The proposal reuses those producers and validators; it does not introduce a new generic completion record or pretend that current loop binding already accepts empty scope or accepted dependencies.

Current classification evidence is nonempty text, not a structured proof of research quality, object version or operation authorization. The verified `task_complete` claim is lifecycle evidence; the current `repository_required` profile does not independently establish those domain acceptance facts. The existing receipt has no accepted-version or authorization-envelope fields. Proposed accepted-dependency validation must check separately bound object/version and professional acceptance evidence alongside the unchanged existing terminal receipt. `resolve_non_merge_receipt` resolves canonical or source-bound migrated receipt artifacts; calling it alone is not a live dependency validator. Actual non-PR terminal state is `closed_without_merge` with Issue close reason `COMPLETED`, not `post_merge_done`.

Research acceptance identifies the exact question, source material versions, result, limitations and reviewer acceptance. If the result must be committed as repository-owned documentation, it is repository-change work and follows the PR route. Scratch or server evidence for genuinely non-PR research must remain retrievable after terminal cleanup; a deleted local path is not sufficient dependency authority.

For an external operation, record the explicit user authorization reference, target environment and exact object/version, allowed actions, bounds or expiry, preconditions, observation and acceptance method, and rollback/compensation procedure or an explicit authorized limitation when reversal is impossible. Credentials remain in their existing protected mechanisms. Readback proves what occurred; intention or a successful command exit alone does not prove acceptance. Unknown or partial outcomes remain unresolved and reconcile before retry; no duplicate external effect is inferred safe from a missing response.

Changing expected output, repository scope, environment, operation limits or completion route invalidates affected admission/acceptance evidence. Stop the affected action, record the revised task truth and authorization, and repeat applicable gates. A non-PR task that acquires repository changes cannot finish through its old classification; a PR-bound task cannot be converted by deleting its PR fields. If current helpers cannot safely reclassify the situation, retain the blocker and use the sanctioned lifecycle process rather than editing receipts or cache.

The explicit mode-change options are to revise the current task through the sanctioned lifecycle process, or create a linked task with a recorded handoff that names the remaining work, changed scope and applicable authority. Close or retain the original task with its truthful disposition; a linked task does not inherit acceptance or bypass gates merely through that link. Neither option permits a silent completion-mode switch.

## 2. Dependency satisfaction binds exact versions

The proposed default is **merged**. It requires the dependency's actual merged-delivery terminal truth and exact consumed contract/content version. An Issue merely marked done, a closed-without-merge receipt, or a design approval does not establish merged delivery. The existing finalizer's Issue, Project, receipt and cleanup semantics remain authoritative; consumers must not invent a terminal body string that real producers do not emit.

**Accepted** is an explicit dependency choice for a completed non-PR result. Its consumer-side validator must verify all of the following using existing production receipt producers and authoritative readback:

1. The selected task UID, repository and canonical Issue identity match the dependency declaration and the receipt. The task was genuinely classified `non_pr_task` and finalized for `non_pr_completed`, rather than cancelled or abandoned.
2. The existing verified `task_complete` lifecycle claim and non-merge terminal proof are authentic, mutually consistent, and complete. Reuse the actual `oasis7_closed_without_merge` receipt, issuer `non-merge-finalize`, reason `non_pr_completed`, task/repository/Issue/Project binding, classification evidence digest and null PR authority; its type alone is insufficient because that producer also represents unsuccessful outcomes. Validate independent professional acceptance evidence for the result; the lifecycle claim does not supply it.
3. The consumer names the exact accepted result or external object version, relevant clauses or scope, task evidence epoch where applicable, and immutable evidence/receipt identity and digest. For example, pin a canonical Issue evidence comment ID plus content digest, or an immutable external artifact version plus digest. A mutable URL or latest revision is insufficient. Acceptance evidence must establish that exact version and applicable environment; these additional consumer bindings are not existing receipt fields.
4. Live authoritative state has no revocation, superseding acceptance or unresolved readback that invalidates consumption. A later version is not substituted silently; changing the consumed version requires explicit rebinding and fresh relevant review/validation.
5. Required design approval, published contracts and their eligibility, CI, review, merge permission, dependency closure and cycle checks remain separate obligations. Accepted research supplies an input; it cannot satisfy a missing mandatory merged implementation or authorize a product promise.

The dependency declaration and validator representation are proposed implementation work, not new supported CLI flags or a schema in this document. A missing mode defaults to merged only in the approved implementation; an unknown mode, missing exact version, inaccessible evidence or unverifiable receipt blocks. `duplicate`, `superseded`, `not_planned`, cancelled and deferred tasks cannot be promoted to accepted by a caller label. Duplication may point to another task, but that actual task must independently satisfy the selected rule.

## 3. Complete path classification before activation

Inventory every tracked path at the frozen effective-policy revision, including dotfiles, assets and fixture files, rather than sampling directories. Each path must have an explicit product/system/code rule, a prohibition, or a separately approved migration disposition with a responsible owner and completion condition. Migration disposition records work to resolve; it does not grant write authority. Admission still fails for unresolved or unknown paths. Additions must satisfy the same policy, and deletes, renames, copies, mode changes and symlink/submodule boundaries must check every affected endpoint.

| Tracked family | Proposed explicit treatment | Boundary |
| --- | --- | --- |
| `site/**` | Code for site implementation/build assets; explicitly identify any authoritative product or system documents separately | Public wording remains reviewed against approved promises and appropriate external-message review |
| `docker/**`, Dockerfiles, `.dockerignore` | Code for executable packaging/deployment configuration | External deployment still needs its operation authorization |
| `fixtures/**` and domain fixture trees | Code for test inputs and executable validation assets | Fixture placement cannot hide normative gameplay/design authority |
| `deny.toml`, `config.example.toml` | Code for tool policy and example runtime configuration | Changes retain their corresponding security/configuration and CI reviews |
| Existing professional gameplay/product authorities | Product for player promises, rules and acceptance | Preserve the four-module `doc/product/` portfolio; domain authorities do not create a fifth module |
| Human-readable technical contracts | System | Executable instructions retain code precedence |
| All remaining tracked assets and dotfiles | Individually covered by the reviewed inventory/rules or prohibited/pending migration | No unknown-to-code fallback, implicit extension rule or broad mixed-file waiver |

The implementation must produce an auditable coverage report with frozen repository/policy identity, every uncovered or ambiguous path and its disposition, and negative evidence for attempted unclassified changes. Full path coverage does not by itself prove semantic ownership: mixed content requires the bounded migration below.

At admission and planning, before execution starts, an unknown declared or discovered path must block with the diagnostic `missing classification; update policy first`. Terminal diff validation repeats the check; discovering the problem only at completion is insufficient. Updating the effective policy requires its normal review and approval, not a caller fallback or candidate self-authorization.

### 3.1. Document placement before execution

This procedure applies the existing [engineering navigation](../../README.md), [sole product entrance](../../product/README.md) and [document-structure standard](../doc-governance/doc-structure-standard.design.md). It is not another directory registry or role router. Directories identify the subject; filename suffixes identify the document's responsibility. `*.prd.md` can express a professional technical requirement in a system domain, while `*.design.md` can express product design in the product tree. Neither suffix determines loop. Executable agent instructions remain code assets regardless of Markdown format.

The general preference for same-object paired basenames in one directory operates within these semantic boundaries. The more specific four-module product-placement rule prevents co-locating product promises and technical How merely to form a PRD/design pair; link their distinct authorities instead. Directory registration distinguishes professional, evidence and retired entries, so follow each existing entry's actual purpose rather than creating a directory named after the owner role.

Choose the product module by the product promise, not the implementing subsystem:

| Product subject | Existing destination root | Example and professional boundary |
| --- | --- | --- |
| Player goals, rules, progression and resource pressure | `doc/product/world-rules-core-gameplay/` | A new progression/ROI promise belongs here; atomic ledger or ABI mechanics remain professional technical authority |
| Authoritative world continuity, finality, recovery and infrastructure guarantees | `doc/product/world-infrastructure/` | A recovery-continuity promise belongs here; consensus protocol, storage schema and recovery runbooks do not become product implementation detail |
| Player-observable Agent decisions and interactive simulation | `doc/product/agents-world-simulation/` | The visible Agent/simulation experience belongs here; provider wire format and planning-engine interfaces remain technical |
| Learning about, installing, entering and verifying the public experience | `doc/product/player-entry-distribution/` | An installation/onboarding or public-preview promise belongs here; HTML implementation and channel operation procedures retain their own authority |

Read the chosen root `prd.md` and existing active-topic list first. Extend the existing authority where the subject already exists. A genuinely long-lived new subject may use a stable topical PRD and, when needed, a paired product design under that same root; it must be reachable from the module entrance/root PRD and link back to the root. Do not create a fifth product module, a dated feature-fragment authority or a new product promise under a professional directory merely because an older related file exists there.

Professional system placement follows the existing domain map; these are concrete existing navigation targets, not blanket loop grants for every file beneath them:

| Technical or professional subject | First authority and existing placement | Do not put here merely because… |
| --- | --- | --- |
| Project-wide technical integration and module boundaries | `doc/core/prd.md`, `doc/core/design.md`; existing stable subjects under `doc/core/` | A new player-facing cross-domain promise needs a product-module owner |
| Workflow, harness, CI and document governance | `doc/engineering/prd.md`, then `doc/engineering/workflow/` or `doc/engineering/doc-governance/`; script capability contracts use `doc/scripts/prd.md` and its tree | “System” is not a reason to send every technical design to engineering |
| Runtime kernel, WASM interfaces and deterministic execution | `doc/world-runtime/prd.md`, `doc/world-runtime/design.md`, then the matching existing topic under `doc/world-runtime/` | Runtime product guarantees still belong to their product root |
| Headless execution and production runtime chain | `doc/headless-runtime/prd.md`, `doc/headless-runtime/design.md`, then its stable topic | A headless implementation does not create an additional product module |
| Network, consensus, distributed storage and node recovery | `doc/p2p/prd.md`, `doc/p2p/design.md`, then its matching protocol or operations topic | Public finality/recovery promises are not operational runbook text |
| In-game Agent, simulation and scene mechanisms | `doc/world-simulator/prd.md`, `doc/world-simulator/design.md`, then its existing subject | Agent naming does not make development harness design belong here |
| Viewer and launcher technical contracts/manuals | `doc/world-simulator/viewer/` or `doc/world-simulator/launcher/`, reached through the module entrance | Visual direction, player powers and public onboarding promises remain their professional/product authority |
| Verification design, manuals and playability evidence | `doc/testing/prd.md` and its tree; `testing-manual.md` for suite selection; `doc/playability_test_result/` for its existing professional evidence purpose | A test result or task status is not product acceptance authority by itself |
| Site architecture and public-document projection | `doc/site/prd.md` / `doc/site/design.md`; `doc/readme/prd.md` / `doc/readme/design.md` for public-entry consistency | Site/README documentation cannot independently introduce unapproved public promises |
| Existing professional gameplay technical contracts | `doc/game/prd.md` and the existing `doc/game/gameplay/` subject | Not every gameplay PRD is system: precise player rules/defaults retain product-loop professional rule authority |

The professional gameplay exception is semantic, not a license for new scattered product design. Maintain existing precise gameplay rules/default values in their authoritative paired gameplay PRD, with product-root links; new or rewritten product promises, product designs, combinations and end-to-end acceptance go into the appropriate four-module tree. A change spanning both may need explicitly scoped linked work and relevant professional review. Do not duplicate rules into both files, change a numerical default under the label “technical cleanup,” or reclassify every external PRD as system.

For system work, use the module `prd.md` for Why/What/Done and `design.md` for overall structure, then the existing stable subject directory. A new independently maintainable topic may use matching `topic.prd.md` / `topic.design.md` basenames; use manuals/runbooks for usage and operations rather than inventing another design authority. Repair the relevant `README.md`, root/topic PRD and `prd.index.md` reachability and backlinks according to the existing structure standard. Root/module flat-file allowlists and directory registry remain applicable; this table does not authorize new root files or directories. Avoid per-date or one-task fragments. Mutable task status and evidence stay in GitHub-backed task truth, not a local project ledger or copied documentation router.

Before document work starts, the task/slice must identify: change kind (create/modify/rename/split), subject and semantic class, loop and owner, existing authoritative document/clauses, proposed exact paths, intended stable topic, affected product/professional references and index/backlink repair, and effective policy plus exact scope. For a new file, explain why an existing topic is insufficient; for a rename, list both endpoints and consumers; for a mixed source, supply the separately authorized clause migration plan. These are proposed admission inputs, not new currently supported packet fields or CLI options.

| Change or example | Required result | Refusal before execution |
| --- | --- | --- |
| New infrastructure recovery-continuity product promise | Extend `doc/product/world-infrastructure/prd.md` or an approved reachable stable topic; link the professional recovery contract | A new `doc/p2p/` product-promise document: `product placement mismatch; choose one of the four product modules` |
| Change `doc/world-runtime/design.md` interface/structure semantics | System owner, exact existing authority/scope and associated technical validation plan | Choosing product solely because a related requirement uses a PRD suffix: `document responsibility does not determine loop; resolve semantic authority` |
| Change player costs in existing `gameplay-agent-claim-economy-contract.prd.md` | Treat as professional product-rule authority, with corresponding product promise/reference review | Treating all `doc/game/` PRDs as system, or using it to add an unrelated product promise outside the product tree |
| Create a stable Viewer technical topic | First inspect existing `doc/world-simulator/viewer/` authority; if new subject is justified, propose its paired documents and navigation repairs | No subject/authority/backlink plan: `document placement unresolved; identify domain, authority and stable topic` |
| Rename a professional or product document | Re-evaluate semantic placement and effective classification of both endpoints, repair every active reference, preserve authority identity | Moving into a directory/suffix to gain write permission, or an unknown endpoint: `missing classification; update policy first` |
| Modify a mixed legacy design | Stop and use the separately authorized four-document clause split where applicable; retain known authority until migration acceptance | A broad directory permission, legacy label or filename change standing in for semantic migration |

For ordinary modifications, confirm the existing location is still appropriate for the requested semantics; “file already exists” is not automatic placement approval. Admission/planning checks proposed paths before any edit and validates the actual diff afterward. Current loop path rules and documentation governance validate their enumerated path/structure contracts; they do not understand all semantic placement, ownership or backlink completeness. The proposed entry input/refusal checks and real positive/negative producer-consumer tests remain implementation work. These examples define future acceptance, not a claim that new executable placement validation is already active.

## 4. Four mixed-document migrations require separate authorization

The four source documents remain unchanged in this documentation stage. Before activation, obtain explicit authorization for a document-only semantic split naming exact source sections, destination authorities, preserved promises and technical constraints, reference/index repair, reviewers and acceptance. Do not translate a migration plan into permission to edit, label it fake legacy, or permit mixed writes indefinitely.

The bounded source set is `doc/game/design.md` and these files under `doc/game/gameplay/`: `gameplay-regional-infrastructure-micro-depot-contract.design.md`, `gameplay-agent-claim-economy-contract.design.md`, and `gameplay-top-level-design.design.md`. Product rules/acceptance move only to the appropriate existing product or professional gameplay authority; technical mechanisms move to their professional system authority. Keep semantic traceability from every source clause to exactly one authoritative destination or an explicitly reviewed retirement. Do not copy whole mixed documents into both layers.

The producer's bounded proposal uses existing destinations rather than creating a new product module. Section references below identify extraction candidates, not permission to copy entire mixed sections. The separately authorized migration must resolve each clause and preserve its current meaning without gameplay redesign.

| Source | Product promises and professional rules | Technical authority after split |
| --- | --- | --- |
| `doc/game/design.md` | Product goals, loops and feedback promises to `doc/product/world-rules-core-gameplay/prd.md` and its existing `playability-evidence-and-claim-boundaries.prd.md` leaf | Retain this source as pure professional integration/navigation authority; remove duplicate product promises and link their authorities |
| `gameplay-regional-infrastructure-micro-depot-contract.design.md` | Player staging, ROI and experience clauses in sections 1–4/12 to existing `governed-regional-capabilities-and-extensions.prd.md` and `mature-world-progression.prd.md` under the same product module; exact rules/defaults to the paired `gameplay-regional-infrastructure-micro-depot-contract.prd.md` | Retain ABI, runtime/DTO, technical section 10 and implementation/receipt evidence in the professional design, after removing mixed player authority |
| `gameplay-agent-claim-economy-contract.design.md` | Player costs, runway, exclusivity and provenance promises to existing product leaf `agent-ownership-and-stewardship.prd.md`; exact professional rules/defaults to paired `gameplay-agent-claim-economy-contract.prd.md` | Retain atomic ledger, state/event, DTO and technical QA mechanisms in the professional design; distinguish player-visible provenance guarantees from ledger implementation |
| `gameplay-top-level-design.design.md` | Product clauses to the existing world-rules module root and playability leaf; professional gameplay rules to paired `gameplay-top-level-design.prd.md` | Consolidate technical navigation into `doc/game/design.md`; remove the redundant thin design only after every backlink is repaired |

Gameplay source and paired PRD basenames in this table are under `doc/game/gameplay/`; product leaf basenames are under `doc/product/world-rules-core-gameplay/`. Migration acceptance requires clause-level old/new traceability, no duplicated authoritative promise, preserved numerical defaults and acceptance thresholds, repaired module indices/backlinks, and matching producer/gameplay/system role review. Retaining a technical file is acceptable only after its mixed semantics are resolved; renaming or adding frontmatter alone is insufficient.

After the separately authorized split is reviewed, update references and remove superseded duplicate authority. Then deliver the separately validated policy/code stage in the same PR that recognizes the resulting paths and exact dependency/completion semantics. Activation requires those changes plus real acceptance evidence; approval of this plan alone satisfies none of those implementation steps.

Explicitly close each bounded transitional migration entry with its clause mapping, reference repairs, reviewer acceptance and resulting policy classification. Once all four entries are resolved, close the transitional entry itself before activation; it must not survive as a permanent `allow-mixed` exception or a fabricated legacy label.

## 5. Required real acceptance scenarios for later implementation

| Scenario | Positive evidence | Required rejection |
| --- | --- | --- |
| Edit `site/deck/deck.css` | Explicit code ownership, exact scope, existing review/CI and merged completion | Unknown path fallback or a non-PR receipt for the tracked edit |
| Complete research without repository changes | Empty repository scope, existing non-PR classification, accepted exact research result, verified closeout and non-PR terminal receipt | Empty scope used to hide tracked edits, or a local success note replacing the receipt |
| Code task consumes accepted research | Explicit accepted dependency binds the actual non-PR receipt and exact accepted result version; code still completes through PR/merge | Default merged dependency silently treating research as merged; revoked or substituted result version |
| Accept an authorized staging service restart without repository changes | Prior bounded authorization, exact environment/service and pre/post deployment version, execution readback, acceptance and rollback evidence, existing completion proof | Classification granting deployment permission, target drift, unapproved irreversible action or unresolved response |
| Edit migrated micro-depot gameplay authority | Explicitly authorized semantic split already completed; product task edits stage/ROI clauses in approved product/paired gameplay PRD, separate system task edits ABI in pure technical design; exact ownership and clause traceability | Editing an unresolved mixed source, duplicate authority, combining product and ABI changes in one leaf, or enabling policy before migration acceptance |

Tests must exercise real producer/consumer interfaces and authoritative readback, including retry, version drift, cancellation and inaccessible evidence. Retain exact Git before/after identities for file scenarios and actual external readback for operation acceptance. Fixtures establish behavior, not live activation; none of these five real scenarios is accepted merely by approving this document. Preserve existing manual trigger, holds, contracts, professional review, current integration CI and cleanup gates throughout the staged work.
