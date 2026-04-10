# 工程文档总入口（模块设计）

审计轮次: 6

更新时间：2026-03-30

本文件用于导航各模块设计文档与执行文档。所有新需求与在研需求均以模块 PRD 为唯一入口。

## 快速阅读路径（推荐）
1. 先读本文件，获取导航。
2. 读根 `README.md`，先确认当前公开状态、技术预览边界与公开说明准备态。
3. 读 `site/index.html`，确认公开站点当前入口、预览验证路径与下载区口径。
4. 读 `doc/core/prd.md`，获取项目全局设计总览（模块地图、关键链路、关键分册）。
5. 进入目标模块 `doc/<module>/prd.md`，确认问题定义、方案、验收标准与技术边界。
6. 若目标模块已补齐 `design.md`，继续读模块设计总览，确认模块总体设计、分层和主链路。
7. 继续读 `doc/<module>/project.md`，确认任务拆解、PRD-ID 映射、依赖与状态。
8. 按需下钻模块子文档（`doc/<module>/**/*.md`）。
9. 对照系统测试策略：`testing-manual.md` 与 `doc/testing/prd.md`。
10. 若已知 `task_uid`，读取对应 `.pm/tasks/task_<32hex>.execution.md`；未知具体任务时，先看模块 `project.md`。

## 按目标进入
| 你的目标 | 第一入口 | 第二入口 | 说明 |
| --- | --- | --- | --- |
| 想先知道项目当前公开状态与技术预览边界 | `README.md` | `site/index.html` | 先确认“现在能看什么”，再决定是否深入仓库文档 |
| 想参与功能开发或治理任务 | `doc/core/prd.md` | `doc/<module>/prd.md` + `doc/<module>/project.md` | 先看全局目标，再进入目标模块 |
| 想做本地验证、回归或验收 | `testing-manual.md` | `doc/testing/prd.md` | 手册负责 suite 选择，testing 模块负责测试体系建模 |
| 想调试 Viewer / Web 链路 | `doc/world-simulator/viewer/viewer-manual.manual.md` | `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` | 前者是专项操作手册，后者是 Web 闭环步骤 |
| 想补过程上下文或追溯具体任务决策 | `doc/<module>/project.md` | `.pm/tasks/task_<32hex>.execution.md` | 先靠正式追踪定位任务，再看 task-scoped execution log |

## 根目录入口说明
- 根目录 legacy redirect 仅保留兼容跳转；正文与执行状态统一回收到各模块目录。
- 高频兼容入口示例：
  - `doc/viewer-manual.md` -> `doc/world-simulator/viewer/viewer-manual.manual.md`
  - `doc/game-test.prd.md` -> `doc/playability_test_result/game-test.prd.md`
  - `doc/playability_test_card.md` -> `doc/playability_test_result/playability_test_card.md`

## 模块入口矩阵
| 模块 | PRD 主文档 | 设计主文档 | 项目管理文档 | 设计关注点 |
| --- | --- | --- | --- | --- |
| core | `doc/core/prd.md` | `doc/core/design.md` | `doc/core/project.md` | 项目全局总览与跨模块治理基线 |
| engineering | `doc/engineering/prd.md` | `doc/engineering/design.md` | `doc/engineering/project.md` | 工程规范、质量门禁、文件治理 |
| game | `doc/game/prd.md` | `doc/game/design.md` | `doc/game/project.md` | 玩法循环、规则层、发行可玩性 |
| headless-runtime | `doc/headless-runtime/prd.md` | `doc/headless-runtime/design.md` | `doc/headless-runtime/project.md` | 无界面运行链路与生产稳定性 |
| p2p | `doc/p2p/prd.md` | `doc/p2p/design.md` | `doc/p2p/project.md` | 网络、共识、分布式存储 |
| playability_test_result | `doc/playability_test_result/prd.md` | `doc/playability_test_result/design.md` | `doc/playability_test_result/project.md` | 可玩性测试数据与收口闭环 |
| readme | `doc/readme/prd.md` | `doc/readme/design.md` | `doc/readme/project.md` | 对外口径与文档入口一致性 |
| scripts | `doc/scripts/prd.md` | `doc/scripts/design.md` | `doc/scripts/project.md` | 自动化脚本能力与维护规范 |
| site | `doc/site/prd.md` | `doc/site/design.md` | `doc/site/project.md` | 站点体验、内容发布、SEO |
| testing | `doc/testing/prd.md` | `doc/testing/design.md` | `doc/testing/project.md` | 分层测试体系与发布门禁 |
| world-runtime | `doc/world-runtime/prd.md` | `doc/world-runtime/design.md` | `doc/world-runtime/project.md` | 运行时内核、WASM、治理与审计 |
| world-simulator | `doc/world-simulator/prd.md` | `doc/world-simulator/design.md` | `doc/world-simulator/project.md` | 世界模拟、Viewer、LLM 与场景系统 |

## 目录结构说明
- `doc/<module>/prd.md`：模块设计主文档（唯一 PRD 入口）。
- `doc/<module>/design.md`：模块总体设计入口（结构、分层、主链路，ROUND-006 逐步补齐）。
- `doc/<module>/project.md`：模块任务拆解与执行状态。
- `doc/<module>/prd.index.md`：模块文件级 PRD 索引（活跃专题文档可达入口）。
- `doc/<module>/**/*.md`：专题设计、实现方案、复盘与历史说明。
- `doc/<module>/README.md`：模块目录索引（按主题子目录导航）。
- `.pm/tasks/task_<32hex>.execution.md`：按任务维护的 canonical 过程日志。
- `doc/devlog/`：历史归档，仅作回溯参考，不再作为运行态真值。
- `doc/.governance/*-allowlist.txt`：文档组织门禁基线（根目录与模块根目录平铺文件冻结清单）。
- `doc/**/archive/` 目录已移除；历史专题仅在模块目录内保留并在索引中标注。

## 共享规则
- 新功能或行为变更必须先更新模块 `prd.md`，再更新 `project.md`，最后实现与测试。
- 代码、测试、文档任务必须可追溯到 PRD-ID。
- 模块根入口、专题落位、README 职责与 legacy redirect 约定统一以 `doc/engineering/doc-structure-standard.design.md` 为准。
