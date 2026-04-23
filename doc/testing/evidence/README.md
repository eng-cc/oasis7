# `testing/evidence` 热点子域入口

更新时间: 2026-04-17

## 从这里开始
- 想确认 release readiness、candidate/trust gate 或 release 证据包：先读 `release-evidence-bundle-task-game-018-2026-03-10.md`、`closed-beta-candidate-release-gate-2026-03-22.md` 或 `gameplay-ten-minute-trust-gate-2026-04-09.md`
- 想确认 hosted world、浏览器鉴权、web surface 或滥用矩阵：先读 `hosted-world-browser-auth-surface-2026-03-26.md`、`hosted-world-abuse-suite-matrix-2026-03-27.md` 或 `mainchain-token-signed-transfer-web-validation-2026-03-23.md`
- 想确认 p2p/shared-network triad、mixed-topology、same-window snapshot 或 rollout follow-up：先读 `shared-network-ecs-triad-chain-status-metrics-rollout-2026-04-23.md`、`p2p-real-env-triad-snapshot-2026-04-07.md`、`p2p-mixed-topology-validation-matrix-2026-04-07.md` 或 `shared-network-ecs-triad-upgrade-2026-04-07.md`
- 想确认治理演练、live world drill 或 finality 证据：先读 `governance-registry-live-world-drill-finality-2026-03-24.md`、`governance-registry-live-world-drill-foundation-ops-2026-03-24.md` 或 `governance-registry-clone-world-drill-foundation-ops-2026-03-24.md`
- 想确认 claim/restricted grant、token allocation audit 或质量基线：先读 `game-agent-claim-abuse-matrix-2026-03-27.md`、`token-genesis-allocation-audit-template-2026-03-22.md` 或 `testing-quality-trend-baseline-2026-03-11.md`
- 想确认 provider recertification、software-safe web entry、pure-api parity、headless smoke 或 launcher UX：先读 `provider-agent-dual-mode-recertification-evidence-2026-04-07.md`、`software-safe-primary-web-entry-evidence-2026-04-07.md` 或 `post-onboarding-headless-smoke-2026-03-19.md`
- 想精确找某份 evidence 文件，而不是按问题阅读：回到 `../prd.index.md` 或直接按文件名进入目标 evidence

## 入口分工
- 当前页只承担 `evidence/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 `testing` 模块级 landing page，负责跨 `evidence / ci / longrun / launcher / governance / templates / performance / manual` 分流。
- `../prd.index.md` 是 `testing` 模块完整文件级索引，适合已知主题后按文件名查找。
- `testing-manual.md` 与 `manual/*.manual.md` 仍是 operator 手册层，不由本页替代。

## 密度快照
- 治理前快照（2026-04-17）:
  - `doc/testing/evidence/`: 49 份 Markdown
  - `doc/testing/`: 178 份 Markdown
- 当前子域属于 `testing` 模块最高密度热点路径；本页的目标是压缩首读路径，而不是在本批直接减少文件数。

## 首读主题簇

### 1. Release gate 与证据包
- 首读入口:
  - `release-evidence-bundle-task-game-018-2026-03-10.md`
  - `closed-beta-candidate-release-gate-2026-03-22.md`
  - `gameplay-ten-minute-trust-gate-2026-04-09.md`
- 适合问题:
  - 当前 release evidence bundle 该从哪里开始看
  - closed beta / trust gate 的判定留痕在哪
  - 哪些 evidence 直接影响 release readiness 讨论

### 2. Hosted world、浏览器与 Web surface
- 首读入口:
  - `hosted-world-browser-auth-surface-2026-03-26.md`
  - `hosted-world-abuse-suite-matrix-2026-03-27.md`
  - `mainchain-token-signed-transfer-web-validation-2026-03-23.md`
- 适合问题:
  - 浏览器 auth surface 和 hosted world 风险留痕该看哪里
  - hosted world 并发、revoke recovery、strong auth 证据在哪一组
  - 主链 web validation 或网页资产动作验证证据怎么找

### 3. P2P、shared network 与 triad rollout
- 首读入口:
  - `shared-network-ecs-triad-chain-status-metrics-rollout-2026-04-23.md`
  - `p2p-real-env-triad-snapshot-2026-04-07.md`
  - `p2p-mixed-topology-validation-matrix-2026-04-07.md`
  - `shared-network-ecs-triad-upgrade-2026-04-07.md`
- 适合问题:
  - 三节点现在跑的是哪一版 runtime，链状态 metrics 是否已经真实部署
  - same-window triad snapshot / rollout confirm 该看哪里
  - 新增 `/v1/chain/status` metrics contract 有没有真实三节点证据
  - mixed-topology、shared devnet、observer gap sync 或 blob root cause 在哪组 evidence
  - shared-network ECS triad 与 shared-devnet 相关留痕怎么进入

### 4. Governance drill 与 live world finality
- 首读入口:
  - `governance-registry-live-world-drill-finality-2026-03-24.md`
  - `governance-registry-live-world-drill-foundation-ops-2026-03-24.md`
  - `governance-registry-clone-world-drill-foundation-ops-2026-03-24.md`
- 适合问题:
  - governance registry live world drill 的主证据在哪
  - finality / rejoin / revocation / foundation ops 留痕怎么分组
  - clone world drill 与 live world drill 的入口差别是什么

### 5. Claim、grant、token audit 与质量基线
- 首读入口:
  - `game-agent-claim-abuse-matrix-2026-03-27.md`
  - `token-genesis-allocation-audit-template-2026-03-22.md`
  - `testing-quality-trend-baseline-2026-03-11.md`
- 适合问题:
  - claim abuse / restricted grant / restricted starter balance 矩阵该从哪里进
  - token genesis allocation audit 和测试基线证据在哪
  - 哪些 evidence 更适合作为治理/审计基线，而不是 runtime 行为规格

### 6. 定向验证与补充 evidence
- 首读入口:
  - `provider-agent-dual-mode-recertification-evidence-2026-04-07.md`
  - `software-safe-primary-web-entry-evidence-2026-04-07.md`
  - `post-onboarding-headless-smoke-2026-03-19.md`
- 适合问题:
  - provider dual-mode recertification 证据在哪
  - software-safe primary web entry、pure-api parity、headless smoke 或 launcher UX 该去哪看
  - 某条 evidence 是局部验证还是 release gate 主证据

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md` 或按文件名打开目标 evidence，不要指望本页替代完整索引。
- 本页负责“先看哪一类 evidence”，不负责复述 evidence 文件里的事实结论。
- 如果某个主题未来形成正式主文档或专题三件套，应优先进入主文档，而不是继续把散落 evidence 文件维持为默认首读入口。

## 维护约定
- 新增 `evidence/` 文件后，若改变了默认首读路径，应同步更新本页。
- 本页只维护簇级入口，不维护完整文件清单。
- 若未来 `evidence/` 内部继续分裂出更高密度簇，再另开簇内治理专题，而不是把本页扩写成长表。
