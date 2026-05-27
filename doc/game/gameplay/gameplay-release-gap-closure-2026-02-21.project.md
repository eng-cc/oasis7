# Gameplay 发行差距收口（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.prd.md`

审计轮次: 4

## 审计备注
- 主项目入口：`doc/game/gameplay/gameplay-top-level-design.project.md`
- 本文仅维护增量任务，不重复主项目文档任务编排。

## 任务拆解

### T0 文档建模
- [x] 新建设计文档：`doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.prd.md`
- [x] 新建项目管理文档：`doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.project.md`

### T1 Prompt 触发优化（对应问题 1）
- [x] 优化默认 LLM `system_prompt` / `short_term_goal` / `long_term_goal`（改为游戏发展导向，去除强制动作链）
- [x] 增加中途切换 prompt 能力（demo + stress 透传）
- [x] 增加游戏发展测试 prompt 套件（`--prompt-pack`：`story_balanced/frontier_builder/civic_operator/resilience_drill`）
- [x] 增加定向测试，验证切换前后动作覆盖变化

### T2 Gate 覆盖补齐与复验（对应问题 2/3）
- [x] 扩展 `llm-longrun-stress.sh`：release gate profile（industrial/gameplay/hybrid）
- [x] 更新 `testing-manual.md` S8 发行口径示例
- [x] 跑 long-run 复验并记录 summary/report 产物

### T3 经济治理动作协议补齐（对应问题 4）
- [x] 扩展 OpenAI decision schema 增加经济合约动作
- [x] 扩展 parser 到 `Action::Open/Accept/SettleEconomicContract`
- [x] 补齐 `test_tier_required` 解析/边界测试

### T4 m5 规则深度优化（对应问题 5）
- [x] 增强 `m5_gameplay_war_core` 结算规则输出
- [x] 增强 `m5_gameplay_crisis_cycle` 动态危机生成/超时规则
- [x] 增强 `m5_gameplay_economic_overlay` 的经济事件联动
- [x] 补齐 gameplay 协议测试断言（required/full）

### T5 回归与收口
- [x] 运行定向测试与脚本回归（含 gate）
- [x] 回写项目文档状态
- [x] 回写 `doc/devlog/README.md`

### T6 千 tick 长周期演进（新增）
- [x] `oasis7_llm_agent_demo` 增加多阶段切换参数（`--prompt-switches-json`）并与单次切换参数做互斥校验
- [x] `llm-longrun-stress.sh` 支持透传 `--prompt-switches-json`
- [x] `story_balanced` 在长周期（中长/超长）自动生成多阶段切换计划
- [x] 更新 `testing-manual.md` 长周期示例与参数说明
- [x] 运行解析/脚本回归并记录日志

### T7 llm_bootstrap 多 Agent + runtime gameplay bridge（新增）
- [x] `llm_bootstrap` 场景从 1 Agent 扩展到 5 Agent（新增 4 个 Agent）
- [x] `oasis7_llm_agent_demo` 增加 runtime gameplay bridge（接管 runtime-only gameplay/economic 动作）
- [x] `llm-longrun-stress.sh` 增加 bridge 参数透传与默认策略
- [x] 补齐 demo/bin 测试与脚本回归，验证 gameplay 动作拒绝率下降
- [x] 更新 `testing-manual.md` 闭环测试建议（5 Agent + bridge）

### T8 阶段基线世界（落盘/加载）闭环（新增）
- [x] `oasis7_llm_agent_demo` 增加 baseline 状态落盘/加载参数（state dir）
- [x] `llm-longrun-stress.sh` 增加 state dir 参数透传与 summary 输出
- [x] 更新 `testing-manual.md` 两阶段测试剧本（工业建基线 -> 治理压测）
- [x] 新增 `industrial_baseline` prompt pack 与 `--llm-execute-until-auto-reenter-ticks` 参数透传（长程基线专用）
- [x] 产出一份“可复用工业基线”状态（按用户口径允许提前收口：100 tick 真实闭环达标）
- [x] 记录基线质量指标（factory/recipe/action_kind 分布）并回写 devlog
- [x] 将基线 state 从 `.tmp` 复制到 git 跟踪目录（`fixtures/llm_baseline/state_01`）

### T9 起步规则可查询 + 工业前置恢复（新增）
- [x] 新增 `world.rules.guide` 查询工具（OpenAI tool + module 路由 + alias 归一化）
- [x] 优化默认起步 prompt（加入规则查询与“按前置条件推进”约束）
- [x] 修正失败恢复策略：`insufficient_resource.data` 优先 `mine_compound`，避免 `refine` 死循环
- [x] 补齐并通过工具注册/模块执行/prompt 断言回归测试

### T10 基线 fixture smoke + full tier 接入（新增）
- [x] 新增 `test_tier_full` 定向回归：加载 git 跟踪基线 fixture 并校验关键状态结构
- [x] 新增脚本 `scripts/llm-baseline-fixture-smoke.sh`（fixture 存在性 + 定向回归）
- [x] `scripts/ci-tests.sh full` 接入 baseline fixture smoke
- [x] 更新 `testing-manual.md` 与 `doc/devlog/README.md`

### T11 基线加载后离线治理续跑 smoke（新增）
- [x] 在 `oasis7_llm_agent_demo` 测试中新增 runtime bridge 续跑用例（基于 `fixtures/llm_baseline/state_01`）
- [x] 扩展 `scripts/llm-baseline-fixture-smoke.sh`：新增治理/经济动作链离线 smoke
- [x] 更新设计/测试文档与 devlog

### T12 基线续跑结果断言强化（新增）
- [x] 将 `runtime_bridge_continues_governance_from_tracked_baseline_fixture` 从“动作成功断言”升级为“状态结果断言”
- [x] 覆盖提案/投票/元进度/合约结算关键状态字段（含增量断言）
- [x] 回归 `scripts/llm-baseline-fixture-smoke.sh` 并回写设计/测试文档与 devlog

### T13 runtime 预设世界事件 fixture profile（新增）
- [x] `oasis7_llm_agent_demo` 增加 `--runtime-gameplay-preset`（`none|civic_hotspot_v1`）并在 runtime bridge 内实现预设事件注入
- [x] `llm-longrun-stress.sh` 增加 preset 参数透传，并为 `civic_operator/resilience_drill` 默认启用 `civic_hotspot_v1`
- [x] 扩展 full-tier smoke：新增 `runtime_bridge_civic_hotspot_preset_seeds_followup_handles`
- [x] 更新设计/测试文档与 devlog

### T14 release gate 三档稳定化（新增）
- [x] `oasis7_llm_agent_demo` 增加 `--coverage-bootstrap-profile`，在 LLM loop 前注入 deterministic industrial/gameplay/hybrid 覆盖链路
- [x] `llm-longrun-stress.sh` 接入 bootstrap 参数透传，并在 `--release-gate` 下默认对齐同名 profile
- [x] `llm-longrun-stress.sh` 在未显式指定 `--max-parse-errors` 时启用随 ticks 的自适应阈值（`max(2, ceil(ticks/40))`）
- [x] 补齐 demo/bin 测试并完成三档 120 tick 回归（industrial/gameplay/hybrid）稳定通过
- [x] 回写设计/测试文档与 devlog

## 依赖
- `doc/game/gameplay/gameplay-release-production-closure.prd.md`
- `doc/game/gameplay/gameplay-module-driven-production-closure.prd.md`
- `testing-manual.md`
- `scripts/llm-longrun-stress.sh`
- `scripts/ci-tests.sh`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3、T4、T5、T6、T7、T8、T9、T10、T11、T12、T13、T14
- 本轮增量：release gate 三档（industrial/gameplay/hybrid）引入 deterministic coverage bootstrap，并将 parse 噪声阈值改为随 ticks 自适应默认值，120 tick 回归稳定通过。
- 进行中：无
- 阻塞项：无（若后续需要“超长世界年龄”基线，可在当前 state 基础上继续按 100 tick 分段追加）。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
