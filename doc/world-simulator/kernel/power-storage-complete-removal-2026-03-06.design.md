# PowerStorage 全量下线设计（2026-03-06）

- 对应需求文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`
- 对应项目管理文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.project.md`

## 1. 设计定位
定义 `PowerStorage` 从 simulator、viewer、脚本与文档中的一次性硬删除方案，使电力语义完全收敛到 `PowerPlant` 与 owner 入账模型。

## 2. 设计结构
- simulator 删除层：移除 `PowerStorage` 类型、模型字段、初始化 seed、动作、事件与 replay/power 分支。
- viewer 删除层：移除 storage 资产槽位、实体 marker、selection kind、UI 详情与自动化目标。
- 脚本治理层：删除 theme pack、texture inspector、视觉评审模板中的 `power_storage` 项。
- 文档同步层：活跃 PRD、评审卡与手册不再把 storage 作为有效实体。

## 3. 关键接口 / 入口
- `WorldModel` / `WorldInitConfig` / `WorldScenarioSpec`
- `Action::{RegisterPowerStorage, StorePower, DrawPower}`
- `PowerEvent::{PowerStorageRegistered, PowerStored, PowerDischarged}`
- `SelectionKind::PowerStorage`
- 历史上的旧 3D 主题/贴图校验脚本（现已删除）

## 4. 约束与边界
- 旧场景若仍携带 `power_storages`，必须在解析阶段直接报错，不能静默忽略。
- 删除必须覆盖 simulator、viewer、脚本、文档全链路，避免半删除状态。
- 本阶段不移除 `PowerPlant`，也不重做电力平衡或 runtime builtin 存储模块。
- 历史含 storage 事件的回放可声明不兼容，但需要保留明确拒绝原因。

## 5. 设计演进计划
- 先删除 simulator 结构、动作与事件。
- 再删除 viewer 实体、选中态与自动化入口。
- 最后清理脚本、评审模板和文档口径，并用 targeted checks 收口。
