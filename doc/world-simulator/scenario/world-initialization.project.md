# oasis7 Simulator：世界初始化（项目管理文档）

- 对应设计文档: `doc/world-simulator/scenario/world-initialization.design.md`
- 对应需求文档: `doc/world-simulator/scenario/world-initialization.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] I1 定义初始化配置结构（WorldInitConfig/Origin/AsteroidFragment/Agent）
- [x] I1 实现世界初始化输出（WorldModel + Report）
- [x] I2 提供 WorldKernel 便捷构造接口并接入校验
- [x] I2 补充初始化单元测试（默认流程/确定性/错误分支）
- [x] 文档更新：同步设计分册与导出入口
- [x] I3 支持自定义地点列表（LocationSeedConfig）
- [x] I3 支持初始资源配置（Origin/Location/Agent）
- [x] I3 补充资源/多地点初始化测试
- [x] I4 支持电力设施种子（PowerPlant；PowerStorage 已于 2026-03-06 下线）
- [x] I4 增加设施参数校验与错误分支
- [x] I4 补充电力设施初始化测试
- [x] I5 提供场景模板（WorldScenario）
- [x] I5 提供示例工具（oasis7_init_demo）
- [x] I6 扩展场景模板（resource_bootstrap 初始库存）
- [x] I7 README 补充示例工具说明（oasis7_init_demo）
- [x] I8 扩展场景模板（twin_region_bootstrap 多区域）
- [x] I9 补充文档使用示例与 demo 帮助输出
- [x] I10 demo 输出地点资源摘要
- [x] I11 扩展场景模板（triad_region_bootstrap 多区域）
- [x] I12 demo 输出 Agent 资源摘要
- [x] I13 扩展场景模板（asteroid_fragment_bootstrap 启用小行星带碎片）
- [x] I14 补充场景别名解析测试
- [x] I15 demo 输出小行星带碎片数量
- [x] I16 demo 输出地点设施统计
- [x] I17 文档补充场景使用建议
- [x] I18 文档补充场景别名说明
- [x] I19 扩展场景模板（asteroid_fragment_twin_region_bootstrap 多区域小行星带碎片）
- [x] I20 扩展场景模板（asteroid_fragment_triad_region_bootstrap 三方小行星带碎片）
- [x] I21 demo 增加 summary-only 开关
- [x] I22 场景稳定性测试（关键字段校验）
- [x] I23 文档补充小行星带碎片种子策略
- [x] I24 增加 oasis7_init_demo summary-only 冒烟测试
- [x] I25 增加 asteroid_fragment_bootstrap 冒烟测试
- [x] I26 增加 asteroid_fragment_twin_region_bootstrap 冒烟测试
- [x] I27 增加 asteroid_fragment_triad_region_bootstrap 冒烟测试
- [x] I28 文档补充 seed_offset 使用约束
- [x] I29 增加 triad_region_bootstrap 冒烟测试
- [x] I30 文档补充场景 ID 稳定性说明
- [x] I31 增加 triad_p2p_bootstrap 场景（多节点单 Agent）与出生地点列表支持
- [x] I32 文档更新：世界观调整为破碎小行星带
- [x] space-unit-centimeter-enforcement (PRD-WORLD_SIMULATOR-002/003) [test_tier_required]: 收口 1cm 空间最小单位契约：初始化/场景/碎片生成写入世界状态前统一 canonicalize 到整厘米，并补充 runtime/simulator 回归测试。 Trace: .pm/tasks/task_d7c44249a4aa42c6b2ec25415873ae7f.yaml

## 依赖
- doc/world-simulator/scenario/world-initialization.prd.md
- `generate_fragments`（小行星带碎片生成器）
- `WorldKernel` / `WorldModel` 基础结构

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：I33（1cm 坐标契约收口完成）
