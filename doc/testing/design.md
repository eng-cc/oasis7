# testing 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/testing/prd.md`
- 可变任务状态与历史: PR、实际 CI 与评审记录（Issue 按需）
- 对应文件级索引: `doc/testing/prd.index.md`

## 1. 设计定位
`testing` 模块的 `design.md` 负责描述测试分层、验证策略、证据采集与发布门禁的总体设计。

## 2. 阅读顺序
1. `doc/testing/prd.md`
2. `doc/testing/design.md`
3. `doc/testing/prd.index.md`
5. 下钻 `ci/`、`governance/`、`launcher/`、`longrun/`、`performance/` 等专题目录

## 3. 设计结构
- 分层层：`test_tier_required` / `test_tier_full` 的职责分工。
- 证据层：测试结果、失败签名、门禁与复审记录。
- 发布层：go/no-go、回归范围与阻断结论。
- 性能证据层：`performance-coverage-gap-matrix-2026-06-09.md` 统一当前 runtime/LLM observability、Viewer Web browser metrics、tier 与 report-only/blocking 边界；历史 native probe schema 不反向定义当前 Web harness。
- 好玩性证据层：`L1` automation、`L2` probe、`L3` telemetry/experiment、`L4A` synthetic review、`L4B` embodied-agent playtest 与 `L5` external signals 按证明强度递进；低层不替代高层，世界活动不等于玩家杠杆。
- 内部评审层：standard-role packet/card 收口，persona panel 只提供结构化假设并回流角色结论，不新增正式 `player` 角色或外部验证结论。

## 4. 集成点
- `testing-manual.md`
- `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`
- `doc/playability_test_result/prd.md`
- `doc/core/prd.md`
- `doc/scripts/prd.md`
- `scripts/prepare-playability-l4-review.sh`：在一个 worktree 固定 L4 packet、role/persona cards、L4B card、可选校准 notes、summary 与命令。
- `scripts/run-playability-l4b-agent.sh`：执行真实 agent 操作并写入 L4B state/screenshot/summary/card evidence。

## 5. 专题导航
- CI 与覆盖进入 `ci/`
- 线上/长时验证进入 `longrun/`
- 发行与治理验证进入 `governance/`、`launcher/`

<a id="qa-infrastructure-evidence-method"></a>
### QA infrastructure evidence method

The [QA receiver contract](prd.md#qa-infrastructure-receiver) is the stable entry for combining product obligations, professional contracts, scenes, and test-tier evidence. For each scene/cell/specimen, freeze the exact clause fragment and source identity, candidate/base/tested tree, world/branch/root/window/configuration, applicable domain owner, execution state, dependency/topology and provider identity, ordered phases, artifact locator/bytes/digest, oracle, verdict, and residual proof boundary. Keep `planned`, `located`, `implemented`, and `executed` as independent state fields.

The verification path is source clause → named scene and phase → actual environment/dependency → professional oracle → raw evidence artifact and readback → tier-specific verdict. First bind the candidate and source; then enumerate required cases/cells; capture a no-mutation baseline; validate artifact identity and bytes; run structural checks separately from canonical, trust, semantic, and cross-consumer checks; compare same-candidate before/outage/fresh/recovery/retry/replay observations; and finally check that the selected tier's real environment is present. Missing mandatory identity, phase, cell, proof, archive readback, or environment is blocked/unverified. A contradictory observed invariant is failed with its exact signature. A shape-only pass remains shape-verified. Pass applies only to the declared scope when all its assertions and required environment are observed. Archive failures and successes append-only. This method does not turn a fixture, a link, a schema, or a previous candidate's pass into current execution or release evidence.

The root QA scene families are `V-FINALITY`, `V-RECOVERY`, `V-EXECUTION`, `V-INDUSTRIAL`, `V-OUTAGE`, `V-WORLD-SCOPE`, and `V-CONSUMER`; their source and oracle mapping is in the [testing PRD](prd.md#qa-infrastructure-scene-families). They retain all distinct product SC/narrative, DCS/DWE REQ/AC, P2P/runtime DC/DE, and GWSC 17/10/8 axes. Gameplay owns industrial quantities and W/window meaning; Runtime/WASM own execution and migration semantics; P2P owns finality and topology proof; Ops establishes the observed environment; Agent/Viewer own consumer observations; QA joins evidence without replacing those authorities.

<a id="qa-infrastructure-89-consumption-matrix"></a>
### Frozen 89-object dependency-consumption and evidence-retention matrix

This matrix is per frozen object, not a whole-file rewrite list. It preserves the exact 89 input paths from the admitted dependency inventory. Five of those objects receive bounded QA edits in this slice: the GWSC PRD, Playwright manual, Agent-browser manual, testing PRD, and `testing-manual.md`. `doc/testing/design.md` and the Playwright design companion are method documents outside the 89-object denominator. Every dated or candidate-bound artifact remains at its original path; it is eligible as current evidence only after its own source/candidate/window identity and bytes are read back. A locator, template, historical result, or neighboring object's status cannot close another object's gap.

| Frozen object | QA receiver consumption | Retention, owner, and unresolved proof |
| --- | --- | --- |
| `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` | `V-INDUSTRIAL`; retain the onboarding steps and evidence questions as the gameplay sample input. | Preserve its date, candidate, and verdict; gameplay owns experience interpretation. A current same-candidate execution is still required for current claims. |
| `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md` | `V-FINALITY` and `V-RECOVERY`; use benchmark dimensions only to select topology/scale context. | Keep benchmark design as a reference, not a run result; Ops/P2P own actual environment and finality evidence. |
| `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md` | `V-FINALITY`; use the benchmark acceptance context when a chain-scale claim is in scope. | Retain its distinct PRD obligations; QA still needs current candidate measurements and Ops provenance. |
| `doc/testing/evidence/README.md` | Evidence index used to locate candidate-specific raw artifacts across the receiver scenes. | Preserve as navigation only; each artifact must independently satisfy identity, digest, window, and owner checks. |
| `doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md` | `V-FINALITY`/`V-RECOVERY`; historical shared-devnet provenance can explain an older sample's origin. | Preserve date and limitation; Ops must supply same-candidate topology/provenance before current use. |
| `doc/testing/evidence/network-tier-signer-truth-binding-2026-06-05.md` | `V-FINALITY`; signer identity and tier binding context. | Retain historical signer binding evidence; P2P/Ops must read back the current governed signer set and threshold. |
| `doc/testing/evidence/p2p-public-testnet-faucet-service-2026-05-19.md` | `V-RECOVERY` only when the selected scenario depends on faucet service or funding availability. | Preserve as scoped service evidence; Ops must establish current service/dependency. It does not prove consensus or world effects. |
| `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05.json` | `V-CONSUMER`; historical envelope joining API and Viewer projection. | Retain original JSON and candidate identity; Viewer/runtime must provide a current same-window readback for parity claims. |
| `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05/api-projection.json` | `V-CONSUMER`; API-side fields for the dated projection sample. | Keep bytes/digest with the parent envelope; current canonical API oracle remains required. |
| `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05/chain-status-samples.json` | `V-CONSUMER`/`V-FINALITY`; dated chain-status observations. | Preserve as historical samples; Ops/runtime must bind new samples to current candidate and window. |
| `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05/viewer-projection.json` | `V-CONSUMER`; Viewer-side fields for the dated projection sample. | Keep with parent envelope; Viewer owner must supply current surface readback, not inferred parity. |
| `doc/testing/evidence/public-testnet-claims-boundary-review-2026-05-21.md` | Tier/claim boundary reference for `V-FINALITY` and `V-CONSUMER`. | Retain its reviewed date and boundaries; release owner must reassess current candidate before external claims. |
| `doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md` | Latest dated claims-boundary context for `V-CONSUMER`/release-scoped scenes. | Preserve as an independent dated review; it does not supply current same-window operational evidence. |
| `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md` | `V-FINALITY`/`V-RECOVERY`; identify required operational lane names and their historical state. | Keep as a dated lane record; Ops must provide current same-window lane results. |
| `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv` | Machine-readable counterpart for the dated lane record. | Retain exact rows and digest; schema/TSV presence is not a passing lane or current readback. |
| `doc/testing/evidence/public-testnet-ecs-freshness-audit-2026-05-22.md` | `V-FINALITY`/`V-RECOVERY`; freshness audit history. | Preserve original freshness window; Ops must observe current service/readiness and peer head. |
| `doc/testing/evidence/public-testnet-faucet-guard-ready-2026-07-05.md` | `V-RECOVERY` only for scenarios requiring faucet guard behavior. | Retain scoped dated result; does not establish current faucet health or unrelated finality. |
| `doc/testing/evidence/public-testnet-five-node-inventory-2026-06-23.md` | `V-FINALITY`/`V-RECOVERY`; topology inventory source. | Preserve inventory identity/date; Ops must verify actual current nodes, roles, and connectivity. |
| `doc/testing/evidence/public-testnet-governance-public-signers-2026-06-05.json` | `V-FINALITY`; historical governed public signer set. | Keep immutable signer bytes; P2P/Ops must bind actual current signer membership and proof. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt` | `V-FINALITY`/`V-RECOVERY`; peer bootstrap input for the dated governed candidate. | Preserve exact list; Ops must capture current peer discovery and heads. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json` | `V-RECOVERY`; dated governed bootstrap bundle envelope. | Retain envelope and linked bytes; state-sync verification must independently validate closure and candidate identity. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json` | `V-WORLD-SCOPE`/`V-RECOVERY`; genesis/world identity of the dated bootstrap. | Preserve exact identity; runtime/Ops must compare against the current target world, not reuse by filename. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json` | `V-RECOVERY`; manifest-to-artifact binding for the dated bootstrap. | Retain exact manifest bytes; missing/mismatched current artifacts remain blocked. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md` | `V-FINALITY`; dated node/role/topology description. | Preserve the historical topology; Ops must capture current roles and actual dependency classification. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json` | `V-FINALITY`; dated validator membership input. | Retain registry digest; P2P/Ops must prove the live governed set and threshold. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt` | `V-FINALITY`/`V-RECOVERY`; newer dated triad peer seed. | Preserve as its own candidate input; current Ops readback must prove actual peer use and heads. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world/journal.json` | `V-EXECUTION`/`V-RECOVERY`; dated world history sample. | Retain raw journal and digest; runtime must bind replay/readback to the current world and source. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world/journal.segments.json` | `V-EXECUTION`/`V-RECOVERY`; journal segment index for that world. | Preserve segment boundaries and identity; current replay/closure requires actual segment bytes. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world/module_registry.json` | `V-EXECUTION`; module registry state of the dated world. | Retain exact registry; runtime/WASM must read back candidate-compatible module/version state. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world/snapshot.json` | `V-RECOVERY`; dated snapshot state. | Preserve original snapshot; runtime must verify root/world and current restore outcome. |
| `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world/snapshot.manifest.json` | `V-RECOVERY`; snapshot-to-artifact manifest. | Keep manifest and referenced digests; absent/mismatched closure is a blocker, not a template fill. |
| `doc/testing/evidence/public-testnet-governed-reset-policy-announcement-2026-07-03.md` | `V-RECOVERY`; records announced reset policy boundaries. | Preserve announcement; Ops must prove actual governed reset execution and unchanged confirmed history. |
| `doc/testing/evidence/public-testnet-live-candidate-bootstrap-peers-2026-05-22.txt` | `V-FINALITY`/`V-RECOVERY`; historical live-candidate peer seeds. | Retain exact dated peers; no current connectivity or topology claim without Ops observation. |
| `doc/testing/evidence/public-testnet-live-candidate-bundle-2026-05-22.json` | `V-RECOVERY`; old live-candidate bundle envelope. | Preserve envelope and source candidate; closure/readback must be repeated for a current candidate. |
| `doc/testing/evidence/public-testnet-live-candidate-endpoint-deploy-2026-05-19.md` | `V-FINALITY`; dated endpoint deployment context. | Keep deployment history; Ops must establish current endpoint route and actual services. |
| `doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-22.tsv` | `V-FINALITY`/`V-RECOVERY`; old candidate lane rows. | Retain raw TSV; all current lane outcomes and evidence refs remain to be captured. |
| `doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json` | `V-RECOVERY`; manifest identity of the old candidate. | Preserve digest; cannot bind a different candidate or prove current artifact availability. |
| `doc/testing/evidence/public-testnet-liveops-public-signers-2026-06-05.json` | `V-FINALITY`; LiveOps signer material reference. | Retain immutable file; P2P/Ops must prove signer use and governance state in the selected window. |
| `doc/testing/evidence/public-testnet-provider-resource-provenance-2026-07-05.md` | `V-INDUSTRIAL`/`V-OUTAGE`; provider-resource source and dependency context. | Preserve its provenance window; Ops must identify which dependency actually gates authority versus read paths. |
| `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.md` | `V-CONSUMER`; historical public-surface freshness observation. | Retain sampled time and endpoint; current Viewer/API freshness requires a same-window re-read. |
| `doc/testing/evidence/public-testnet-resource-delta-replay-2026-07-05.md` | `V-INDUSTRIAL`; resource delta/replay evidence context. | Preserve raw quantities and candidate; gameplay/runtime must revalidate conservation and once-only effect. |
| `doc/testing/evidence/public-testnet-runtime-world-resource-closure-2026-07-05.md` | `V-INDUSTRIAL`/`V-RECOVERY`; resource-to-world closure evidence. | Keep its exact closure scope; it does not prove current roots, windows, or all industrial transitions. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05.json` | `V-CONSUMER`/`V-WORLD-SCOPE`; hosted entry identity envelope. | Retain the dated same-world association; current world identity and consumer receipt continuity still need readback. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/chain-status-sample.json` | `V-FINALITY`/`V-CONSUMER`; chain-side sample for that entry. | Preserve bytes/digest; Ops/runtime must supply a current same-window status sample. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/hosted-entry.json` | `V-WORLD-SCOPE`; hosted entry world association. | Retain exact association; missing/mismatched target identity must fail closed. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/launcher-config.json` | `V-CONSUMER`; launcher configuration of the dated entry. | Preserve configuration identity; current candidate config must be separately captured and joined. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/pure-api-config.json` | `V-CONSUMER`; pure API configuration of the dated entry. | Keep exact settings; active-provider pure API parity needs current candidate and provider provenance. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/pure-api-snapshot.json` | `V-CONSUMER`; dated pure API world snapshot. | Retain snapshot/root identity; runtime must compare current authoritative state, not only shape. |
| `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/viewer-config.json` | `V-CONSUMER`; Viewer configuration of the dated entry. | Preserve exact config; Viewer must provide current read-only projection and surface evidence. |
| `doc/testing/evidence/pure-api-shared-player-gameplay-parity-2026-04-28.md` | `V-CONSUMER`; historical parity method/result reference. | Retain its date and limit; no current parity without same-candidate active-provider API plus surface observations. |
| `doc/testing/evidence/shared-network-shared-devnet-follow-up-promotion-record-2026-03-24.md` | `V-FINALITY`/`V-RECOVERY`; promotion decision history. | Preserve decision provenance; current promotion/readiness remains an Ops/release evidence question. |
| `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` | `V-CONSUMER`; historical Viewer entry observation. | Retain as bounded historical evidence; it cannot prove current headed desktop+narrow behavior. |
| `doc/testing/evidence/viewer-wasm-only-runtime-proof-2026-05-13.md` | `V-EXECUTION`/`V-CONSUMER`; WASM-only execution evidence. | Preserve its exact runtime scope; it is not provider-backed, full-browser, or consumer-parity proof. |
| `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md` | QA semantic authority for 17 cases, 10 runtime groups, 8 cells and their receiver oracles. | Read-only in this slice; preserve all design anchors and limitations. QA links only, does not redefine runtime/P2P/gameplay semantics. |
| `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md` | `V-RECOVERY`/`V-EXECUTION`/`V-OUTAGE`; §§2/6 now receive QA source/tier record and preserve 17/10/8 identities. | QA-owned bounded sections; test implementation, topology, real provider, and release evidence remain unexecuted here. |
| `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md` | `V-FINALITY`/`V-RECOVERY`; soak and fault-selection source. | Retain its duration/topology authority; Ops/P2P own actual runs, and a short or proxy run cannot claim endurance. |
| `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | `V-RECOVERY`/`V-CONSUMER`; five-node integration/release source. | Preserve same-window/API projection requirements; current S10 execution remains a separate evidence obligation. |
| `doc/testing/manual/public-testnet-fresh-validator-host-bootstrap-2026-07-28.manual.md` | `V-FINALITY`/`V-RECOVERY`; operational bootstrap and environment procedure. | Keep Ops procedure unchanged; actual host/bootstrap/peer proofs require a separately authorized run. |
| `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` | `V-CONSUMER`; evidence boundary links visible action/read-only oracle and artifact joins. | QA-owned limited evidence section; actual browser/provider execution remains absent from this authoring slice. |
| `doc/testing/manual/web-ui-playwright-closure-manual.manual.md` | `V-INDUSTRIAL`/`V-EXECUTION`/`V-RECOVERY`/`V-CONSUMER`; PWT-003/005/006/007 capture contracts. | QA-owned planned rows; preserve PWT-001 and dated PWT-004 status; no planned case is an executed pass. |
| `doc/testing/prd.md` | QA receiver root, seven scene families, tier boundary, semantic equal-version expected negative, PRD traceability. | QA-owned §§2/6; scope states expected design and proof limits, not implementation or release readiness. |
| `doc/testing/templates/mainnet-bootstrap.example.txt` | `V-FINALITY`/`V-RECOVERY`; bootstrap input shape reference only. | Preserve example; copy to candidate-owned evidence before use and prove actual peers/roles independently. |
| `doc/testing/templates/mainnet-genesis.example.json` | `V-WORLD-SCOPE`/`V-RECOVERY`; genesis example shape. | Template is not a current genesis; Ops/runtime must bind real genesis bytes and root. |
| `doc/testing/templates/network-tier-mainnet.example.json` | `V-FINALITY`; mainnet tier envelope example. | Keep as template; no network tier is passed by validating this example. |
| `doc/testing/templates/network-tier-public-testnet-rehearsal.example.json` | `V-FINALITY`/`V-RECOVERY`; rehearsal tier example. | Preserve example status; use actual rehearsal candidate and same-window evidence for any claim. |
| `doc/testing/templates/network-tier-public-testnet.example.json` | `V-FINALITY`/`V-RECOVERY`; public-testnet tier example. | Template only; release/Ops lane evidence and claims boundary remain required. |
| `doc/testing/templates/public-testnet-genesis.example.json` | `V-WORLD-SCOPE`/`V-RECOVERY`; public-testnet genesis example. | Never treat sample identity as a live world; bind actual genesis and manifest. |
| `doc/testing/templates/public-testnet-rehearsal-bootstrap.example.txt` | `V-FINALITY`/`V-RECOVERY`; rehearsal peer bootstrap example. | Preserve the sample; actual peer discovery/topology belongs to Ops evidence. |
| `doc/testing/templates/public-testnet-rehearsal-genesis.example.json` | `V-WORLD-SCOPE`; rehearsal genesis example. | Template bytes are not world identity proof; actual candidate must provide its genesis readback. |
| `doc/testing/templates/public-testnet-skeleton-evidence.example.md` | All applicable scene families only as a field-shape aid. | Skeleton is never a run receipt; every field must be backed by candidate-bound raw evidence. |
| `doc/testing/templates/public-testnet-validator-pair-rebuild-evidence-v1.json` | `V-FINALITY`/`V-RECOVERY`; validator-pair rebuild envelope shape. | Keep schema/example distinct from proof; Ops must retain actual rebuild logs and readback. |
| `doc/testing/templates/shared-network-exit-decision-template.md` | `V-RECOVERY`; exit-decision record shape. | Template is not an exit decision; release/Ops owners retain an actual signed/current decision. |
| `doc/testing/templates/shared-network-incident-review-template.md` | `V-RECOVERY`/`V-OUTAGE`; post-incident review shape. | Preserve template; actual incident evidence and owner assessment must fill a separate artifact. |
| `doc/testing/templates/shared-network-incident-template.md` | `V-OUTAGE`; incident capture fields. | Not an observed fault; actual before/outage/fresh/recovery samples remain required. |
| `doc/testing/templates/shared-network-mixed-topology-gate-template.md` | `V-FINALITY`; mixed-topology gate record shape. | Retain template; Ops/P2P must produce current topology and gate outputs. |
| `doc/testing/templates/shared-network-promotion-record-template.md` | `V-FINALITY`/`V-RECOVERY`; promotion record envelope. | Not a promotion outcome; actual governance and same-window lanes remain missing until supplied. |
| `doc/testing/templates/shared-network-rollback-target-template.md` | `V-RECOVERY`; rollback target declaration shape. | Do not infer rollback target or success from example fields; actual root/history preservation must be observed. |
| `doc/testing/templates/shared-network-shared-access-check-template.md` | `V-FINALITY`; shared-access check envelope. | Template does not prove isolation; current role/access observations remain an Ops/P2P gap. |
| `doc/testing/templates/state-sync-closure-evidence-packet-template.md` | `V-RECOVERY`; expected closure-packet envelope fields. | Template only; actual attachments, raw proof bytes, digests and readback are required for semantic closure. |
| `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md` | `V-CONSUMER`; launcher cross-surface action selection when the scenario traverses launcher. | Launcher owner retains its surface contract; QA compares same candidate only where applicable, no launcher parity execution here. |
| `doc/world-simulator/launcher/game-client-launcher-runtime-session-continuity.prd.md` | `V-CONSUMER`/`V-RECOVERY`; session continuity when the selected consumer path uses launcher runtime. | Preserve launcher lifecycle clauses; actual session recovery and same-world readback remain separate proof. |
| `doc/world-simulator/llm/continuous-agent-harness.prd.md` | `V-CONSUMER`; Agent is a consumer of same-candidate state, receipts and next action. | Agent owns any PRD edits; QA link does not prove provider-backed Agent parity or receipt-gated memory/goal behavior. |
| `doc/world-simulator/llm/decision-provider-contract.prd.md` | `V-EXECUTION`/`V-CONSUMER`; provider lane and response semantics. | Provider owner retains contract; QA records provider identity and actual preflight, not a prose or mock substitution. |
| `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md` | `V-INDUSTRIAL`; source for inputs, capacity, reservation, handoff, sinks and industrial output. | Gameplay/runtime retain rule and state authority; QA requires actual quantity/root/once-only evidence. |
| `doc/world-simulator/prd.md` | Product consumer/root context for player-visible world scope and gameplay status. | Preserve module authority; QA maps only accepted clauses to consumer scenes and does not author product semantics. |
| `doc/world-simulator/viewer/README.md` | `V-CONSUMER`; Viewer navigation and surface owner locator. | Index only; current Viewer observation and authoritative projection need actual artifacts. |
| `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md` | `V-CONSUMER`/`V-EXECUTION`; applied/control feedback and readmodel scope. | Viewer owns its contract; QA checks same-candidate status and application boundary without claiming headed validation. |
| `doc/world-simulator/viewer/viewer-manual.manual.md` | `V-CONSUMER`; Viewer user-facing status, blocker, and readback surface. | Viewer owns any manual changes; QA requires fresh visible state and matching authority refs for parity. |
| `testing-manual.md` | S6/S9A tier selector and receiver navigation; current provider/full proof boundary and 17/10/8 links. | QA-owned S6/S9A additions; real headed/provider/runtime execution stays unperformed and cannot be inferred from this method. |

## 设计目标
- 提供 `testing` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/testing/prd.md`
- 可变执行状态：PR、实际 CI 与评审记录（Issue 按需）
- 索引入口：`doc/testing/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
- 若自动化、synthetic、agent 实操和真实人类信号混写，stage/release claim 会越过证据边界；QA 以 `world_activity_only`、L4B session evidence 与 L5 缺失状态阻断升级。
