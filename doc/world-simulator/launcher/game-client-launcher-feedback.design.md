# 客户端启动器反馈设计（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md`
> 历史迁移、验证与 task 状态：GitHub task issue evidence。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [最小诊断](game-client-launcher-feedback.prd.md#feedback-diagnostic-minimum) | allowlist、200 行及各已知报告的独立保护/误报义务 | [诊断设计](#feedback-diagnostic-design) | Launcher / QA / 独立隐私评审 | 手写文本与任意未知秘密格式不在自动保证内 |
| [文件不覆盖](game-client-launcher-feedback.prd.md#feedback-local-file-identity) | native Ready 后远端失败的本地保存不覆盖既有文件 | [文件设计](#feedback-local-file-design) | Launcher writer / 文件系统 / QA | 不增加 Web fallback、崩溃持久化或历史资源删除 |

## 结构

1. **交互承载层**：native 以独立反馈窗口承载 `Bug/Suggestion`、标题、描述与输出目录；Web 以对应窗口承载可提交草稿。主面板只负责发现和打开入口。
2. **校验与状态层**：提交前验证必填字段；in-flight 与可用性门控防止无效重复请求；成功、校验失败、代理/远端失败均保留可读状态，不因失败关闭草稿。
3. **native 提交层**：当前玩家路径先以 chain `Ready` 门控；仅在该门控通过后，`submit_feedback_with_fallback` 请求 `/v1/chain/feedback/submit`，再在远端失败时调用本地反馈包写入。链禁用或未就绪时在请求前拒绝提交，不触发本地写入。时间戳与类别保持可识别，字段和日志上限保持兼容；文件创建需满足下述不覆盖合同。
4. **Web 提交层**：Web 请求 `/api/chain/feedback`，由 `oasis7_web_launcher` 代理到 runtime；响应以结构化结果回到窗口。该层没有浏览器文件写入回落。

## 表现与控制边界

- 反馈窗口不能改变世界规则、链状态或提交协议；它只编排已有输入、门控和结果。
- native/Web 需复用同一类别、必填和结果语义，但必须明确呈现不同的回落能力，不能把 Web 失败描述为已本地保存。
- 链禁用或未就绪时，当前实现拒绝提交且不触发 native 本地回落；本设计不把该状态解释为已提供可操作修复 CTA。
- 反馈窗口、草稿和结果应保持可自动化定位；任何实际 UI 改动须遵循 `testing-manual.md` S6 的 desktop/mobile 浏览器证据要求。

## 代码与协议接点

- native：`crates/oasis7_client_launcher/src/feedback_entry.rs`、`feedback_window.rs`。
- Web：`crates/oasis7_client_launcher/src/feedback_window_web.rs`、`app_process_web.rs`。
- 控制面/runtime：`/api/chain/feedback` -> `/v1/chain/feedback/submit`。

本设计保留当前结构，不新增字段、CTA、fallback 或端点。以下为既有诊断保护和本地文件保存所缺的实施边界与验收目标，不声明实现已经修复。

<a id="feedback-diagnostic-design"></a>
## 最小自动诊断设计

对应[最小自动诊断合同](game-client-launcher-feedback.prd.md#feedback-diagnostic-minimum)，`crates/oasis7_client_launcher/src/feedback_privacy.rs::capture_feedback_diagnostics` 负责构造独立摘要，不直接序列化完整配置为反馈字段。获准布尔、枚举、有界数值之外默认排除；日志截取最近 200 行后清洗，保留原时间顺序。原配置值仅作本地敏感值匹配输入，不能随摘要输出。

清洗器对已配置敏感值和可识别 CLI/赋值/路径语法分别处理。CLI 凭据既覆盖赋值形式，也覆盖 `--api-key value` 等空格分隔形式；引号包围的私有绝对路径须以完整值为范围，含空格不能只遮住第一段。短值匹配要保留无关单词和已有 `<redacted>` 等标记；无法安全确定值范围时遮蔽该诊断片段或整行，不退回未清洗内容。每行扫描达到既有预算仍须遮蔽未检查部分，避免有界执行以泄露尾部为代价。使用虚构凭据和路径测试，证据不复制真实私有样本。

基准 `5e930face77dd284bbba6186fd997e7e63fa9164` 已有 allowlist 和 200 行裁剪；清洗函数仍有逐值 `.replace` 与最多 32 次凭据扫描等具体接点。它们需按 [#4406 F1/F2/F3a/F3b](https://github.com/eng-cc/oasis7/issues/4406) 分别验证适用性，不能仅凭历史评审状态认定已修复或今天仍可复现。扩展或调整保护逻辑时保留独立隐私评审，不把本次文档改动作为安全实现结论。

<a id="feedback-local-file-design"></a>
## 本地保存设计

对应[文件身份合同](game-client-launcher-feedback.prd.md#feedback-local-file-identity)，在现有本地回落边界内先完成内容构造，再用独占创建获取本次文件。时间戳用于阅读和排序，冲突时选择新的候选名称并在有界次数内重试；不能用覆盖写入解决冲突。成功必须在完整内容写入后返回实际路径，失败保留原错误，不报告完整保存。只可清理本次独占创建但未完成的文件，不碰预存路径。

基准的 `crates/oasis7_client_launcher/src/feedback_entry.rs::submit_feedback_report` 使用秒级时间戳加 kind 形成路径并调用 `fs::write`；两次同名写入可能覆盖，因此仅提高时钟精度不能满足并发和预存文件条件。目标实现不改 Ready 门控、远端优先、Web proxy 或字段语义，也不增加 fsync/断电恢复保证。

## 独立验证目标

| 子项 | 目标回归与准确接点 | 需要观察的结果 |
| --- | --- | --- |
| F1 CLI 凭据 | `feedback_privacy.rs` 的清洗测试，未配置的可识别空格分隔 CLI 值及既有赋值形式 | 值受保护，安全上下文仍可读；独立记录当前适用性 |
| F2 扫描上限 | 同文件构造超过 32 个可识别字段和未检查尾部 | 执行有界，尾部不以未清洗内容输出 |
| F3a 短配置值 | 同文件使用短值、无关诊断单词和已有脱敏标记 | 敏感值受保护，无关安全文本和标记不被逐字符破坏 |
| F3b 引号路径 | 同文件使用虚构带引号、含空格的绝对路径 | 完整私有路径受保护，非敏感错误仍可诊断 |
| F4 最小诊断与门控 | `feedback_privacy.rs` allowlist/日志上限；`feedback_window.rs`、`feedback_window_web.rs` 及提交 transport | 不含凭据/provider URL/私有路径；至多 200 行；disabled/not-Ready 无本地文件，Ready 后远端失败本地回落，Web proxy 保持 |
| F5 不覆盖 | `feedback_entry.rs` 的离线 writer/transport fixture：固定时钟两报告、预存同名文件、并发 writer、写入失败 | 路径互异、旧字节不变、成功包可解析；失败无虚假保存且不删旧文件 |

上述目标尚需实现候选的真实结果；文档检查不代替隐私或 writer 回归。原任务资源归档/删除事实与代码和验收独立，本专题不执行历史资源删除。

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 验证方法、test/manual source 与环境 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [最小诊断](game-client-launcher-feedback.prd.md#feedback-diagnostic-minimum) | [诊断设计](#feedback-diagnostic-design) | F1/F2/F3a/F3b 分别核适用性；F4 allowlist/200 行/门控与传输保持 | [测试手册](../../../testing-manual.md) launcher 回归；按上表在 `feedback_privacy.rs`、native/Web 窗口与 transport 注入虚构样本并检查实际输出，独立隐私评审；planned | 实现 PR 每项适用版本、实际回归或有依据不适用结果 | 本次未运行，不证明任意秘密发现或手写文本安全 |
| [文件不覆盖](game-client-launcher-feedback.prd.md#feedback-local-file-identity) | [文件设计](#feedback-local-file-design) | F5 固定时钟/预存路径/并发/失败，与既有传输分开 | [测试手册](../../../testing-manual.md) launcher 回归；`feedback_entry.rs` 离线 writer/transport fixture 按上表检查返回路径、旧字节与解析结果；planned | 实现 PR 的当前候选和 writer 结果 | 本次未运行，不证明远端/Web 存储或断电恢复 |
