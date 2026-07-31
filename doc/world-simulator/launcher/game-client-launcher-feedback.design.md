# 客户端启动器反馈设计（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md`
> 历史迁移、验证与 task 状态：GitHub task issue evidence。

## 结构

1. **交互承载层**：native 以独立反馈窗口承载 `Bug/Suggestion`、标题、描述与输出目录；Web 以对应窗口承载可提交草稿。主面板只负责发现和打开入口。
2. **校验与状态层**：提交前验证必填字段；in-flight 与可用性门控防止无效重复请求；成功、校验失败、代理/远端失败均保留可读状态，不因失败关闭草稿。
3. **native 提交层**：当前玩家路径先以 chain `Ready` 门控；仅在该门控通过后，`submit_feedback_with_fallback` 请求 `/v1/chain/feedback/submit`，再在远端失败时调用本地反馈包写入。链禁用或未就绪时在请求前拒绝提交，不触发本地写入。包名、字段和日志上限保持稳定、可测试。
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

本设计只记录当前结构，不授权新增字段、CTA、fallback、端点或交互。
