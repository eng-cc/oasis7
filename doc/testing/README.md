# testing 文档索引

审计轮次: 8

## 从这里开始
- 想先理解 testing 模块覆盖哪些测试层级、门禁和证据边界：`doc/testing/prd.md`
- 想看当前活跃任务、阻断与最新完成项：`doc/testing/project.md`
- 想先判断要跑哪套测试或查操作步骤：`testing-manual.md`、`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- 想先进入 `evidence` 热点子域，并按 release gate / hosted-world / p2p-shared-network / governance drill / claim-audit 问题分流：`doc/testing/evidence/README.md`
- 想先看当前 QA 阻断摘要：`doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`
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
  当前窗口只保留 blocker、next step 与少量高价值收口摘要；更细的近期完成历史应回到对应 topic `*.project.md` 与 `.pm/tasks/*.yaml` / execution log 追溯。
- `evidence/README.md` 是当前最高密度热点子域 `evidence/` 的 canonical 入口，适合先按“release gate / hosted-world / p2p-shared-network / governance drill / claim-audit / 定向验证”分流，再进入具体留痕文件。
- `testing-manual.md` 与 `manual/*.manual.md` 是 operator 手册层，用于决定跑哪套测试、按什么步骤执行。
- `prd.index.md` 是定向检索索引，适合已知主题后按文件名查找，不是新读者的首读入口。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接罗列近期专题长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`testing-manual.md`、`manual/*.manual.md`、`evidence/README.md` 与 `prd.index.md`。
- evidence、templates 与历史 blocker/closure 留痕继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径进入。

## 模块职责
- 维护系统测试手册、required/full 分层门禁与发布证据包口径。
- 汇总 CI、启动器、长稳、性能、人工手册与治理专题。
- 承接跨模块测试范围定义、证据归档与趋势基线建设。

## 热点子域导航（2026-04-10 快照）
- `evidence/`（49）：发布证据、趋势基线与审计留痕；当前已补 `evidence/README.md` 作为热点子域入口。
- `ci/`（33）：CI、wasm determinism、tiering 与 gate 保护。
- `longrun/`（24）：长稳、chaos、soak 与在线稳定性。
- `launcher/`（18）：启动器链路测试、playtest 与配置自动接线。
- `governance/`（16）：质量趋势、release-gate 指标与审计检查。
- `templates/`（12）：证据包、报告与检查清单模板；默认按需进入。
- `performance/`（12）：runtime / viewer 性能观测与方法学。
- `manual/`（7）：系统测试手册分册与 Web UI 闭环 manual。
- `chaos-plans/`（1）：专项 chaos plan 入口。

## 高密度提示
- `doc/testing/` 当前共有 178 份文件；这一层入口不再尝试把热点专题直接摊平展示。
- 需要完整活跃专题清单时，进入 `doc/testing/prd.index.md`；进入 `evidence/` 时，优先先读 `doc/testing/evidence/README.md` 再继续下钻；需要 template / blocker 留痕时，再按具体子域进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 测试门禁、required/full 分层口径或证据模板变化时，优先更新 `doc/testing/prd.md` 与 `doc/testing/project.md`；高频入口变化时，再同步回写 `doc/testing/prd.index.md` 与相关热点子域入口（例如 `doc/testing/evidence/README.md`）。
- `doc/testing/project.md` 的状态区默认只保留当前执行窗口，不再手工维护按时间追加的“最新完成”长列表；近期收口优先回写对应 topic `*.project.md` 与 `.pm/tasks/task_<32hex>.yaml`。
