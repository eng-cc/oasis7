# testing 文档索引

审计轮次: 10

## 从这里开始
- 想先理解 testing 模块覆盖哪些测试层级、门禁和证据边界：`doc/testing/prd.md`
- 想判断好玩性证据、`L4A/L4B/L5` 边界、角色 subagent review 或 simulated player persona 的 canonical topic：`doc/testing/governance/README.md`
- 想在一个 worktree 里直接准备一轮完整 `L4A + L4B` 验证产物：先经 `doc/testing/governance/README.md` 确认所需证据层，再读 `testing-manual.md` 的 `L4A/L4B/L5` 章节并执行 `./scripts/prepare-playability-l4-review.sh`；正式 `L4B` embodied-agent run 由 `./scripts/run-playability-l4b-agent.sh --l4-manifest <artifact>/manifest.json` 收口。
- 想执行 Web UI、Playwright、public-testnet attach 或模型视觉评审手册：先读 `doc/testing/manual/README.md`，再按问题进入对应 manual。
- 想看当前执行窗口、活跃任务、QA 阻断、覆盖缺口与最新高价值收口：`doc/testing/project.md`
- 想先判断要跑哪套测试或查操作步骤：先读 `testing-manual.md`；涉及专项操作再进入 `doc/testing/manual/README.md`。
- 想先进入 `evidence` 热点子域，并按 release gate / hosted access / public-testnet readiness evidence / legacy p2p rehearsal / governance drill / claim-audit 问题分流：`doc/testing/evidence/README.md`
- 想先确认云上测试/正式环境、hosted-login 服务清单与 testnet/mainnet 口径边界：`doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md`
- 想按子域或文件名继续下钻，而不是从长表里逐行找：`doc/testing/prd.index.md`

## 入口
- PRD: `doc/testing/prd.md`
- 设计总览: `doc/testing/design.md`
- 标准执行入口: `doc/testing/project.md`
- 文件级索引: `doc/testing/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：告诉读者先去哪个权威入口，不重复长表索引内容。
- `prd.md` 是模块权威规格入口，适合先理解 required/full 分层、证据包与跨模块测试边界。
- `project.md` 是执行台账，适合确认当前 QA 阻断、活跃测试治理任务与最新完成项。
  当前窗口只保留 blocker、next step 与少量高价值收口摘要；更细的近期完成历史应回到对应 topic `*.project.md` 与 GitHub task issue evidence comments 追溯。
- `evidence/README.md` 是当前最高密度热点子域 `evidence/` 的 canonical 入口，适合先按“release gate / hosted access / public-testnet readiness evidence / legacy p2p rehearsal / governance drill / claim-audit / 定向验证”分流，再进入具体留痕文件。
- `testing-manual.md` 与 `manual/README.md` 是 operator 手册层：前者决定通用测试路径，后者把 Web UI、Playwright、public-testnet attach 与模型视觉评审分流到对应步骤。
- `prd.index.md` 是定向检索索引，适合已知主题后按文件名查找，不是新读者的首读入口。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接罗列近期专题长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`testing-manual.md`、`manual/*.manual.md`、`evidence/README.md` 与 `prd.index.md`。
- evidence、templates 与历史 blocker/closure 留痕继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径进入。

## 模块职责
- 维护系统测试手册、required/full 分层门禁、模型视觉评审 SOP 与发布证据包口径。
- 汇总 CI、启动器、长稳、性能、人工手册与治理专题。
- 承接跨模块测试范围定义、证据归档与趋势基线建设。

## 热点子域导航
- `evidence/`：发布证据、趋势基线与审计留痕；当前已补 `evidence/README.md` 作为热点子域入口。
- `ci/`：CI、wasm determinism、tiering 与 gate 保护。
- `longrun/`：长稳、chaos、soak 与在线稳定性。
- [`launcher/`](launcher/README.md)：启动器人工验证清单与 bundle-first playtest 入口。
- `governance/`：质量趋势、release-gate 指标、审计检查与 playability 证据治理；先读 `doc/testing/governance/README.md` 再按问题下钻。
- `templates/`：证据包、报告、模型视觉评审卡与检查清单模板；默认按需进入。
- `performance/`：runtime / viewer 性能观测与方法学。
- `manual/`：系统测试手册分册、Web UI 闭环、Playwright 实跑、public-testnet attach 与模型视觉评审；先读 `doc/testing/manual/README.md`。
- `chaos-plans/`：专项 chaos plan 入口。

## 高密度提示
- 本页不维护容易漂移的文件数量或状态快照；当前模块库存、热点子目录与 `action_required` 等治理状态统一以 `./scripts/doc-inventory-report.sh` 为准。`find` / `rg --files` 只可作为本地探索辅助，不能替代正式报告状态。
- `evidence/` 已有 `doc/testing/evidence/README.md` 作为热点子域入口。
- 子域数量只用于选择下一轮 focused follow-up；不要在本页扩展全量专题长表。
- 需要完整活跃专题清单时，进入 `doc/testing/prd.index.md`；进入 `evidence/` 时，优先先读 `doc/testing/evidence/README.md` 再继续下钻；需要 template / blocker 留痕时，再按具体子域进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 测试门禁、required/full 分层口径或证据模板变化时，优先更新 `doc/testing/prd.md` 与 `doc/testing/project.md`；高频入口变化时，再同步回写 `doc/testing/prd.index.md` 与相关热点子域入口（例如 `doc/testing/evidence/README.md`）。
- `doc/testing/project.md` 的状态区默认只保留当前执行窗口，不再手工维护按时间追加的“最新完成”长列表；近期收口优先回写对应 topic `*.project.md` 与 GitHub task issue evidence comments。
