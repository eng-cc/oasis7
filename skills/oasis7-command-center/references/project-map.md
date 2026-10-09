# 项目入口地图

## 使用方式

这是一份查找地图，不是冻结的规范或项目状态。核对基线为 `eng-cc/oasis7` 的 `main@60d5ee0cb621c25ffc23ca8567a77a2cdc90facc`（2026-10-09）。每次执行重新解析当前请求的分支/版本并读取有关来源；路径迁移时沿当前索引与 Git 历史定位，不恢复旧文件来凑齐地图。

## 按问题读取

| 信息 | 当前入口 | 解释边界 |
| --- | --- | --- |
| 开发通用流程 | `doc/engineering/workflow/source-of-truth.md`；`AGENTS.md` | 流程只由前者拥有；入口、脚本和 Skill 不另设条件 |
| 项目定位、当前公开阶段 | `README.md` | 公开状态与承诺范围以根 README 为准，愿景不是当前能力 |
| 当前阶段目标、全局 P0、依赖 | `doc/core/prd.md` 的“当前阶段交付目标与全局 P0” | 读取该节，不默认采用头部历史 M1–M5 或旧轮次 |
| 全局架构 | `doc/core/design.md`；core PRD 的模块地图 | 只作全局路由，细节下钻主责专业域 |
| 产品分类及条款主责 | `doc/product/README.md` | 下面四域是产品分类，工程工具不增列为第五产品模块 |
| 世界规则与玩法系统 | `doc/product/world-rules-core-gameplay/prd.md` | 产品目标、循环、成长、资源与制度；玩法细节下钻 `doc/game/prd.md` |
| 权威世界基础设施 | `doc/product/world-infrastructure/prd.md` | 持久、确定、恢复与一致性承诺；技术下钻 `doc/p2p/prd.md`、`doc/world-runtime/prd.md` |
| 智能体、世界模拟与交互 | `doc/product/agents-world-simulation/prd.md` | 游戏内 Agent、provider 和玩家交互；下钻 `doc/world-simulator/prd.md` |
| 玩家接入与发行 | `doc/product/player-entry-distribution/prd.md` | 发现、安装、访问与发行组合；不代替根 README 的公开状态 |
| 测试命令与验证方式 | `testing-manual.md`；`doc/testing/prd.md` | 按实际改动选测试；普通 CI 默认，专项环境验证按真实缺口使用 |
| 实际工作与交付 | Git、PR、CI、review；按需 Issue/Project | 完整读取所选范围，保留被测版本与结果；规范文档不提供实时交付事实 |

从产品根文档的活跃专题继续下钻，按条款范围、显式引用和后继关系判定权威。`active` 表示文档身份；`superseded`、`retired` 仅按历史用途显示。不要按文件修改时间或审计轮次给规范排优先级。

## 当前阶段的阅读提示

在上述核对基线，项目对外仍是受限可玩技术预览。近期主线是同一持久世界中的真实首产物、可恢复阻塞、成果保留与再次进入后的续接。

core 当前里程碑包含顶层收敛、持久单权威、代表性体验、受控真人验证、BFT 与交接。持久底座与代表性体验可并行，真人验证依赖所需开放条件；BFT 可并行研发，未验收不自动否定已验收的受控单权威阶段。不要把专题自身的 P0、完整战争/制度/分片等长期内容全部提升为当前试玩前置。

以上只帮助定位。实际展示时重新读取该节与其指向的 PR/Issue；不要将本参考中的里程碑写成永久默认任务，也不要由文档描述直接给完成率。

## 迁移与流程残留

- 此核对基线根目录没有 `world-rule.md`。用户提供的《Agent World Specification v0.1》及旧 README 可用于理解历史意图，但需与四产品及专业合同核对；“可能”“潜在方向”“开放问题”不能自动成为现行规则。
- 旧 `agent_world_*` 命名和无 `.prd` / `.manual` 后缀的导航不可靠，优先使用当前索引。Viewer 手册当前入口是 `doc/world-simulator/viewer/viewer-manual.manual.md`。
- `doc/README.md`、产品入口和 core 的部分旧段落仍提到强制 Issue/Project、`task_uid`、pre-PR packet、ROUND 或 `.pm` owner。对通用开发流程，以当前 `source-of-truth.md` 为准；具体产品与技术约束仍由对应条款拥有。
- `.pm/README.md` 将其余 `.pm` 内容定位为历史记忆/缓存；旧执行器与模板已退役。不要重启任务注册表、状态回执或 `.pm/github-project-sync/tasks.json`。保留用户已有历史资料。
- `AGENTS.md` 可能引用用户本机文件；在实际环境可用且相关时读取，不把某台机器的绝对路径写成可移植依赖。

发现残留冲突时说明“哪一项表述冲突、按哪份主责规范解释、影响什么”；不将整理全仓冲突变成使用看板的前置任务。

## 现有方法

按需读取 `.agents/skills/README.md`。当前包含：

- `executing-project-tasks`：实施和交付。
- `verification-before-completion`：核实完成声明。
- `requesting-repo-owned-review`：按实际风险组织评审。
- `systematic-debugging`：定位并修复问题。

根 `skills/` 是非默认专家参考库；不要全量加载或恢复固定角色链。本 Skill 不依赖安装第三方 Skill。

## UI 落点

此核对基线的仓库 Web 入口是：

| 入口 | 当前形态 | 对开发看板的意义 |
| --- | --- | --- |
| `site/index.html`、`site/en/index.html` | 公共纯静态站；共享 `site/assets/styles.css` 和 `site/assets/app.js` | `doc/site/design.md` 不允许增加必需框架/构建链；不自动放入私有开发状态或操作权限 |
| `crates/oasis7_viewer/` | Solid.js / JSX / Vite / Vitest；`software_safe_src/main.jsx` 进入 `viewer_app.jsx` | 可参考展示模型、状态表达与组件组织；没有理由把开发会话混入游戏世界协议 |

Viewer 的 Director、Agent Activity、Intent 是游戏世界的实体与行动界面。Launcher 的 `gui-agent` API 也是游戏控制，不能作为 Codex thread、PR 或开发任务的状态源。此快照未确认已有开发指挥台；构建时先重新查找，不把“没查到”当作永久不存在。

用户没有指定宿主时，优先选择与公共站点、玩家 Viewer 分离的本地开发工具。说明所选目录、启动方式与数据接入；需要新技术栈时说明具体收益，避免自动引入 React、数据库、云部署或新的管理平台。

查看实际 `package.json` 和测试手册后选择命令；此快照 Viewer 未声明 `npm run dev`。仅修改 Viewer 时才采用其 S6 验证，不把启动完整游戏栈设成开发看板的必经步骤。

## 核对来源

- [固定基线](https://github.com/eng-cc/oasis7/commit/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc)
- [开发流程规范](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/doc/engineering/workflow/source-of-truth.md)
- [当前阶段目标](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/doc/core/prd.md#当前阶段交付目标与全局-p0)
- [产品四域](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/doc/product/README.md)
- [现有开发方法](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/.agents/skills/README.md)
- [站点设计](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/doc/site/design.md)
- [Viewer 依赖](https://github.com/eng-cc/oasis7/blob/60d5ee0cb621c25ffc23ca8567a77a2cdc90facc/crates/oasis7_viewer/package.json)
