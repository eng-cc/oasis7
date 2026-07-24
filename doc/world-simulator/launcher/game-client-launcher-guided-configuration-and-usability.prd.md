# 客户端启动器引导配置与可用性（当前 authority）

> 本文是 launcher 引导配置、可诊断可用性与既有 chain-runtime 托管边界的当前需求 authority。它吸收四组 2026-02/03 日期化专题的已记录合同；活跃入口已迁移至本三件套，历史来源删除后仅通过 Git 与 GitHub task issue evidence 追溯。

- 对应设计: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.design.md`
- 对应项目: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md`

## 目标

- 让现有 launcher 控制面把配置阻断、启动状态与可恢复的本地运行问题呈现为可理解、可行动的反馈，而不是要求操作者从原始日志推断。
- 保持 launcher 与 chain runtime 的职责边界：launcher 只编排既有进程、配置和状态呈现；runtime 继续拥有世界状态、一致性校验与持久化语义。

## 范围

- 收敛既有 launcher 的引导配置、状态呈现、静态目录校验、chain-runtime 托管与精确 stale-execution-world 分类边界。
- 不改变任何实现、玩家控制面、runtime 语义或视觉流程。

## 当前合同

| 范围 | launcher 的当前责任 | 不得外推为 |
| --- | --- | --- |
| 引导配置与渐进披露 | 现有启动入口可呈现配置问题、必要字段、状态摘要及既有高频/高级配置分层 | 新玩家流程、权限模型或自动修复承诺 |
| 可诊断状态 | 对既有未就绪、禁用、配置或进程失败保留可解释的结构化状态/错误反馈；Web 状态刷新只由请求触发并受现有节流约束 | runtime、网络健康或后台持续监控证明 |
| native/Web 一致性 | 两种 launcher 表面应复用当前控制面状态、配置与请求语义；任一新增视觉/交互变更须另行验收 | 两端逐像素相同、所有历史字段永久存在 |
| 既有查询与输入 | launcher 可继续编排已有只读查询和请求参数；特殊输入必须按当前接口规则传递，空态/错误态可见 | 新索引、查询类别、交易或世界规则 |
| 静态目录与 runtime 托管路径 | launcher 按当前实现作确定性的静态目录解析/校验，并向现有 chain runtime 显式传递 execution-world 目录；游戏启动只在子进程健康且快照与 journal 已出现的既有条件下继续 | 静态目录恢复、cwd 无关路径、路径永久稳定或数据完整性证明 |
| stale execution world | 仅当当前实现命中 `DistributedValidationFailed` 与 `latest state root mismatch` 两个精确签名时，launcher 才可分类该问题；恢复限于非破坏性的 fresh node id，默认 node id 也使用 fresh 值 | 放宽 runtime 校验、任意错误恢复、目录重置/清理、自动删除或一次恢复成功保证 |

## 控制边界与安全约束

- 不改变 world/runtime 协议、状态演化、分布式一致性或持久化规则；runtime 拒绝仍应保持拒绝。
- launcher 不得以 UI 成功、链可达、explorer 可查询或历史回归记录，声明 network readiness、mainnet、结算、最终性、公开服务或玩家可持续性。
- 用户显式配置优先于任何既有建议值；本 authority 不授权目录清理、重建、覆盖或其他破坏性恢复。
- 对路径、目录、配置或本地状态的当前具体规则，以实现、当前 operator 文档和对应专业角色的当期证据为准；本文不冻结历史文件中的默认值、脚本名或完成状态。

## 接口 / 数据

- 现有 launcher 配置、控制面状态、子进程健康、snapshot/journal 存在性及 runtime 错误文本是本页涉及的输入；其字段、传输与解析规则仍以当期实现为真值。
- 本页不新增 endpoint、CLI 参数、GUI Agent 动作、数据存储、轮询或恢复协议。

## 表现与交互要求

- 配置不满足时，当前 UI 应优先显示可操作的原因和可达的修复入口，不能只留下灰色控件或日志依赖。
- 高使用频率的状态、启停和诊断入口应保持可扫描；低频配置可以由现有高级配置表面承载，但隐藏字段不能消除阻断说明。
- 空结果、加载、未就绪和结构化失败必须在相关表面可见；前端不得补造、重算或持久化 runtime/chain 返回结果。
- 窄屏的实际布局、文案、CTA 和触控顺序由 game_visual_interaction_designer 的验收规格决定；本文件不定义视觉方向。

## 不作出的承诺

- 不主张任何日期化来源中记录的任务、测试、截图、完成状态、二进制、端点、输出目录或脚本仍是当前事实；仅本页明确限定的 fresh node-id、精确 stale 签名和启动前置条件来自当期 runtime 只读核验。
- 不承诺源码直跑、bundle、hosted login、provider、runtime、链、Web 静态资源或本地磁盘在任意环境下可用。
- 不新增 API、GUI Agent 动作、DOM、自动修复、后台轮询、持久化/续跑、数据保留、权限或玩家控制能力。
- 本轮 authority 迁移不修改代码、UI、协议、运行时或测试；未来可见改动必须按 `testing-manual.md` S6 取 desktop/mobile 浏览器证据，并由 game_visual_interaction_designer 验收。

## 里程碑

- 已完成：四组历史专题分别记录了既有可用性、runtime 托管、execution-world 输出与 stale-world 处理。
- 本轮：将可安全保留的当前边界收敛为 stable authority，修复本 scope 的活跃入口并退役 source triplet。
- 后续：任何行为、路径、恢复或可见交互改动均在独立任务中重新定界和取证。

## 风险

- 文档不随实现同步时，历史默认值、脚本和完成状态可能被误读为当前合同。
- 错误分类若超出两个精确签名，可能把真实 runtime 阻断误导为可恢复问题；因此本 authority 不扩张该分类。
- 静态目录、启动前置条件或 Web 刷新行为不能单独证明运行完整性、持续可用性或网络 readiness。

## 验收与追溯

- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`git diff --check`。
- 四组已吸收的日期化 source triplet 已退役删除；其中历史需求、设计、测试和任务完成记录仅通过 Git 与 GitHub task issue evidence 追溯。
- 若需判断当前启动参数、路径、错误码、恢复动作或 runtime 行为，必须检查当前实现并由 viewer/runtime/QA 等对应角色在新任务中取证；不能从本页推断。
