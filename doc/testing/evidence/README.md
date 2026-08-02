# `testing/evidence` 热点子域入口

更新时间: 2026-08-02

## 从这里开始
- 想确认当前 release readiness 或测试层级：先读 [`../testing-manual.md`](../testing-manual.md) 与 [`../README.md`](../README.md)；`release-evidence-bundle-task-game-018-2026-03-10.md`、`closed-beta-candidate-release-gate-2026-03-22.md` 和 `gameplay-ten-minute-trust-gate-2026-04-09.md` 仅作为对应窗口的历史/待领域复核证据
- 想确认当前 hosted access、托管身份与签名安全边界：先读 [`../../p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md`](../../p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md) 与 [`../../p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`](../../p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md)；2026-03 的 hosted/browser/mainchain 文件仅作历史验证窗口
- 想确认当前 public-testnet 机制与 readiness：先读 [`../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md`](../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md)，再把 `public-testnet-current-required-lanes-2026-07-03.md` 与 `public-testnet-claims-boundary-review-2026-07-06.md` 作为其采集窗口观测；candidate、triad、shared-network 和 incident 文件只作为历史 provenance
- 想确认治理演练、live world drill 或 finality 证据：先读 `governance-registry-live-world-drill-finality-2026-03-24.md`、`governance-registry-live-world-drill-foundation-ops-2026-03-24.md` 或 `governance-registry-clone-world-drill-foundation-ops-2026-03-24.md`
- 想确认 claim/restricted grant、token allocation audit 或质量基线：先读 `game-agent-claim-abuse-matrix-2026-03-27.md`、`token-genesis-allocation-audit-template.md` 或 `testing-quality-trend-baseline-2026-03-11.md`
- 想确认当前 provider、Web entry 或 onboarding 合同：分别进入 [`../../world-simulator/llm/provider-agent-dual-mode.prd.md`](../../world-simulator/llm/provider-agent-dual-mode.prd.md)、[`../../world-simulator/viewer/viewer-web-entry-compatibility.prd.md`](../../world-simulator/viewer/viewer-web-entry-compatibility.prd.md) 与 [`../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md`](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md)；本目录对应 dated evidence 仅作历史 provenance 或 supporting artifact
- 想精确找某份 evidence 文件，而不是按问题阅读：回到 `../prd.index.md` 或直接按文件名进入目标 evidence

## 入口分工
- 当前页只承担 `evidence/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 `testing` 模块级 landing page，负责跨 `evidence / ci / longrun / launcher / governance / templates / performance / manual` 分流。
- `../prd.index.md` 是 `testing` 模块完整文件级索引，适合已知主题后按文件名查找。
- `testing-manual.md` 与 `manual/*.manual.md` 仍是 operator 手册层，不由本页替代。

## 逐对象语义清单与新鲜度边界
- [`inventory.json`](inventory.json) 是本子域的逐路径清单：它覆盖每个现存对象（不含清单自身），记录 lifecycle、语义角色、retention/domain owner、权威入口、处置建议与 residual risk。
- 清单快照为 2026-08-02。`WINDOW_OBSERVATION` 只保存一个有边界、内容寻址的观测窗口，不构成当前 endpoint availability、fleet health、release readiness、recovery completion 或领域正确性结论；当前行动必须回到 formal runbook，重新采集 deployment truth 与同窗健康证据，再由对应领域 owner、QA/LiveOps 裁决。
- `HISTORICAL_PROVENANCE`、`ARCHIVED_PROVENANCE` 与 `SUPPORTING_ARTIFACT` 不得被重新表述为当前发布或运行态真值。`AMBIGUOUS_LIFECYCLE` 必须先由清单列出的领域 owner 裁决。

## 密度快照
- 治理前快照（2026-04-17）:
  - `doc/testing/evidence/`: 49 份 Markdown
  - `doc/testing/`: 178 份 Markdown
- 当前子域属于 `testing` 模块最高密度热点路径；本页的目标是压缩首读路径，而不是在本批直接减少文件数。
- 本轮清单快照（2026-08-02）: 124 个 evidence 对象，其中 81 份 Markdown、28 个 JSON、9 张 PNG、4 个 TSV、2 个 TXT；Markdown 数量已达到并超过下述 lifecycle review trigger，故已发起逐对象分类复核。

## 首读主题簇

### 1. 历史 Release gate 与证据包
- 历史 provenance:
  - `release-evidence-bundle-task-game-018-2026-03-10.md`
  - `closed-beta-candidate-release-gate-2026-03-22.md`
  - `gameplay-ten-minute-trust-gate-2026-04-09.md`
- 适合问题:
  - closed beta / trust gate 的历史判定留痕在哪
  - 某个旧 evidence bundle 在其采集窗口记录了什么
  - 当前 release readiness 必须回到 `../testing-manual.md` 与 `../README.md`，不得由本簇旧文件单独推出

### 2. Hosted access、浏览器与 Web surface 历史验证
- 当前 authority:
  - `../../p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md`
  - `../../p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
- 历史 provenance:
  - `hosted-world-browser-auth-surface-2026-03-26.md`
  - `hosted-world-abuse-suite-matrix-2026-03-27.md`
  - `mainchain-token-signed-transfer-web-validation-2026-03-23.md`
- 适合问题:
  - 2026-03 浏览器 auth、并发、revoke recovery 与 strong-auth 采样如何追溯
  - 历史 `awt:pk:` web validation 与当前 `OC` / `oc:pk:` 合同的演进边界
  - 当前 hosted/custody/token 结论必须回到上述 P2P authority，并用 fresh evidence 验证

### 3. Public-testnet 当前机制、窗口观测与历史 rehearsal
- 当前机制入口:
  - `../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md`
- 2026-07-03--06 窗口观测:
  - `public-testnet-current-required-lanes-2026-07-03.md`
  - `public-testnet-claims-boundary-review-2026-07-06.md`
- 历史 provenance:
  - `public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`
  - `p2p-real-env-triad-current-version-full-game-nodes-2026-05-16.md`
  - `shared-network-ecs-triad-chain-status-metrics-rollout-2026-04-23.md`
  - `p2p-real-env-triad-incident-provenance-2026-07-31.md`（2026-04-07--08 historical incident/rollout provenance，不是 current readiness）
  - `p2p-mixed-topology-validation-matrix-2026-04-07.md`
  - `shared-network-ecs-triad-upgrade-2026-04-07.md`
- 适合问题:
  - public-testnet 的当前机制与验证命令从 formal runbook 进入；窗口观测不能替代 fresh rerun
  - 2026-07-03--06 的 lanes、claims boundary 与投影采样记录在哪里
  - 历史 triad snapshot、rollout 与 `/v1/chain/status` 部署链如何追溯
  - mixed-topology、observer gap sync 或 blob root cause 的历史 incident chain 在哪组 evidence
  - legacy shared-network ECS triad 与 shared-devnet 相关留痕怎么进入
- 归档边界:
  - `public-testnet-claims-boundary-review-2026-05-21.md` 是旧 live-candidate packet 的 historical claims-boundary evidence；当前 claims boundary 以 `public-testnet-claims-boundary-review-2026-07-06.md` 为首读入口。
  - `public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`、`p2p-public-testnet-faucet-service-2026-05-19.md`、`public-testnet-live-candidate-lanes-2026-05-21.tsv` 与 2026-05-22 live-candidate bundle / manifest / bootstrap-peers / lanes 文件保留为 public-testnet 演进链证据；其中 endpoint、faucet、signer、peer 与 reset 只描述历史窗口。当前 readiness 结论仍以 formal `public_testnet` runbook、正式 lanes TSV 与 `network-tier-public-testnet-readiness.sh` 汇总为准。
  - 2026-06 governed-bootstrap 的 bundle、manifest、genesis、validator registry、bootstrap peers、signer manifests 与 `public-testnet-governed-bootstrap-world-2026-06-06/` 六件套构成不可拆分的历史 replay bundle。不得改写其 payload、hash、路径或身份绑定；当前部署/恢复从 `../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md` 生成新 evidence window。
  - shared-network / shared-devnet 文件只作为 legacy rehearsal provenance，不作为当前 test 环境、formal `public_testnet` readiness、`mainnet` readiness 或公开统一大世界上线证据。先读 `legacy-shared-devnet-provenance-2026-07-26.md`；它保留 replay identity、最终 lane disposition 和底层记录索引。
  - 查询任何历史 shared-devnet incident、rehearsal 或 rollback 时，必须先进入上述 legacy authority。旧记录中的 `pass`、`eligible_for_promotion`、`live` 和 rollback 只描述当时窗口，不代表当前服务或公开可用性，也不授权恢复操作；当前 operator 行动必须使用 formal public-testnet runbook 与 fresh lanes/claims evidence。
  - `archive/visual-cleanup-2026-06-14/manifest.md` 记录从 active evidence path 移出的历史 visual evidence；这些文件只作为追溯归档，不作为当前 release / viewer / gameplay 首读证据。
  - `assets/manifest.md` 记录 3 张 supporting visual assets 的捕获上下文与保留边界；其中两张缺精确父文档但仍保留为 provenance，不构成删除授权或当前视觉验收。

### 4. 历史 Governance drill 与 live world finality
- 当前 authority:
  - `../../p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
  - `../../world-runtime/runtime/chain-pos-control-plane.prd.md`
- 历史 diagnostic provenance:
  - `governance-registry-live-world-drill-finality-2026-03-24.md`
  - `governance-registry-live-world-drill-foundation-ops-2026-03-24.md`
  - `governance-registry-clone-world-drill-foundation-ops-2026-03-24.md`
- 适合问题:
  - 2026-03 governance registry、finality、rejoin、revocation 与 foundation ops 的诊断链如何追溯
  - clone world drill 与 live world drill 的历史入口差别是什么
  - 旧 `pass_for_default_live_world`、阈值和 signer 结果不证明当前 finality、安全性、mainnet 或 readiness

### 5. Claim、grant、token audit 与质量基线
- 当前 claim authority:
  - `../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md`
- 历史 provenance:
  - `game-agent-claim-abuse-matrix-2026-03-27.md`
  - `testing-quality-trend-baseline-2026-03-11.md`
- 非 evidence 模板:
  - `token-genesis-allocation-audit-template.md`
- 适合问题:
  - claim abuse / restricted grant / restricted starter balance 的历史矩阵如何追溯
  - 当前 claim/grant/refund 合同必须从 claim economy authority 进入
  - token genesis allocation 模板不可作为 pass evidence；质量基线也只代表其采集窗口

### 6. 历史定向验证与 supporting evidence
- 历史 provenance / supporting artifact:
  - `provider-agent-dual-mode-recertification-evidence-2026-04-07.md`
  - `software-safe-primary-web-entry-evidence-2026-04-07.md`
  - `post-onboarding-headless-smoke-2026-03-19.md`
  - `p2p-user-mode-launcher-ux-2026-04-07.md`（历史 P2PARCH-9 UX 自动化证据；当前行为回到 `../../p2p/network/mainnet-private-reachability-architecture.prd.md` 与 `../testing-manual.md` S9C）
- 适合问题:
  - provider dual-mode、旧 `software_safe` alias、headless smoke 或 launcher UX 的历史验证在哪
  - 某条 evidence 是局部 supporting artifact 还是历史 provenance
  - 当前 provider/Web entry/onboarding 合同须从本页顶部列出的专业 authority 进入

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md` 或按文件名打开目标 evidence，不要指望本页替代完整索引。
- 本页负责“先看哪一类 evidence”，不负责复述 evidence 文件里的事实结论。
- 如果某个主题未来形成正式主文档或专题三件套，应优先进入主文档，而不是继续把散落 evidence 文件维持为默认首读入口。

## 维护约定
- 新增 `evidence/` 文件后，若改变了默认首读路径，应同步更新本页。
- 本页只维护簇级入口，不维护完整文件清单。
- 若未来 `evidence/` 内部继续分裂出更高密度簇，再另开簇内治理专题，而不是把本页扩写成长表。
- `qa_engineer` 决定每份证据的有效性、保留与删除语义；`repository_health_engineer` 负责密度、导航和阈值治理。
- lifecycle review trigger：Markdown 文件数达到 80、任何 active evidence 超过 180 天仍被默认首读引用、或同一 claim 的三份以上 evidence 未明确 current/historical 边界时，发起聚合、归档或删除复核。触发器不授权批量删除，也不替代 release 或领域 owner 的结论。
