# testing 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/testing/prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments
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

## 设计目标
- 提供 `testing` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/testing/prd.md`
- 可变执行状态：GitHub task issue evidence comments
- 索引入口：`doc/testing/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
- 若自动化、synthetic、agent 实操和真实人类信号混写，stage/release claim 会越过证据边界；QA 以 `world_activity_only`、L4B session evidence 与 L5 缺失状态阻断升级。
