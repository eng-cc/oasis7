# README 与模块 PRD 口径一致性巡检清单（2026-03-11）

审计轮次: 4

## 使用方式
- 适用场景：`README.md`、`doc/README.md`、`site` 顶层入口、核心模块 PRD 发生变更后。
- 执行角色：默认 `producer_system_designer`；必要时联动 `qa_engineer`。
- 执行原则：先看权威源，再决定 README 是否需要回写。

## 巡检清单
| 编号 | 检查对象 | 通过条件 | 权威源 | 失败动作 |
| --- | --- | --- | --- | --- |
| `RC-01` | README 顶层叙事 | `README.md` 的项目定位不与 `doc/core/prd.md`、模块主 PRD 冲突 | `doc/core/prd.md`、相关模块 `prd.md` | 先按模块 PRD / core 裁定，再回写 README |
| `RC-02` | 产品状态口径 | README 与 site 不得出现与真实状态冲突的“已可玩/已上线”表述 | `doc/site/prd.md`、`doc/core/prd.md` | 立即回写 README / site，并复核中英文同构页面 |
| `RC-03` | 术语与边界 | README 中世界规则、玩家权能、WASM / runtime / viewer 边界与权威文档一致 | `doc/product/world-rules-core-gameplay/world-rule.prd.md`、`doc/game/*`、`doc/world-runtime/*` | 将细节下沉到权威源，README 只保留导航级摘要 |
| `RC-04` | 入口链接 | README / `doc/README.md` 中指向 `doc/product/world-rules-core-gameplay/world-rule.prd.md`、`testing-manual.md`、模块主 PRD 的链接可用 | `README.md`、`doc/README.md` | 修复链接并重跑文档治理 / 后续自动检查 |
| `RC-05` | 同步触发条件 | 当 `site`、`core`、模块主 PRD 或对外状态文案变化时，清单被重新执行 | `doc/readme/project.md`、`doc/site/project.md`、`doc/core/project.md` | 在对应 project / devlog 回写“已触发巡检” |
| `RC-06` | 权威源声明 | README 只做导航级摘要，详细行为/需求不在此重复定义 | `README.md`、模块主 PRD | 删除重复细节，改为链接权威源 |

## 最小执行记录
- 巡检日期：
- 执行角色：
- 覆盖对象：`README.md` / `doc/README.md` / 其他
- 结论：`pass` / `fix_required`
- 问题摘要：
- 回写文件：
