# 客户端启动器反馈（当前 authority）

> 本文是启动器反馈能力的当前需求 authority。它收敛入口、窗口化、分布式提交与 Web parity 的已实现合同；三个 2026-03 源三件套及其配对文档已删除，审计追溯仅保留在 Git 与 `.pm` task evidence。

- 对应设计: `doc/world-simulator/launcher/game-client-launcher-feedback.design.md`
- 历史迁移、验证与 task 状态：GitHub task issue evidence。

## 目标

- 玩家或操作者可从 launcher 的反馈入口提交 `Bug` 或 `Suggestion`，并得到可区分、可诊断的结果。
- native 路径在 chain `Ready` 后保留“分布式提交优先、远端失败本地 JSON 回落”的保障；本地包带稳定时间戳、反馈内容、启动配置快照和有上限的最近日志。
- Web 路径经 `oasis7_web_launcher` 代理到 chain runtime；其浏览器环境不承担 native 本地文件回落。
- 本文不新增反馈类型、附件、历史检索、远端遥测、权限模型或 runtime 协议。

## 范围

- 覆盖 launcher native 与 Web 的既有反馈入口、草稿校验、提交结果与各自的存储/代理边界。
- 不覆盖 feedback 网络的底层规则、反馈检索、附件、世界状态或视觉方案；这些变化需由对应专题和专业角色另行授权。

## 当前合同

| 入口 | 当前行为 | 成功/失败结果 | 边界 |
| --- | --- | --- | --- |
| native launcher | 主面板 `反馈 / Feedback` 打开独立窗口；字段为类别、标题、描述、输出目录 | 仅在链已 `Ready` 时尝试远端提交；成功呈现 `feedback_id/event_id`。该已就绪路径的远端提交失败时保存本地 JSON，并保留可诊断失败信息 | 链禁用或未就绪时，现有门控拒绝提交且不写本地包；草稿在窗口关闭前保持会话内状态；本地目录不可写或字段无效时显示失败，用户可修正后重试 |
| Web launcher | Web 反馈窗口校验草稿并请求 `/api/chain/feedback`；控制面再代理 runtime 提交 | 显示结构化成功、校验或代理失败结果；失败后可继续重试 | 浏览器不能落 native 本地反馈包；链不可用时提交受现有可用性门控约束 |

反馈内容只包含必要文本和已有最小诊断上下文；不得把 LLM 凭据或其他敏感配置写入反馈包或代理请求。

## 接口 / 数据

- native 提交入口：`/v1/chain/feedback/submit`；成功结果可包含 `feedback_id` 与 `event_id`。
- Web 提交入口：`/api/chain/feedback`，由 launcher 控制面代理上述 runtime 接口。
- 本地回落包使用稳定时间戳文件名，并承载 `kind`、`title`、`description`、`created_at`、`launcher_config` 与受上限约束的 `recent_logs`。

## 已知限制与不作出的承诺

- native/Web 共享反馈语义目标，不等于共享相同存储能力：本地回落仅适用于 native 的 Ready 后远端失败，不能把链禁用或未就绪表述为已保存。
- 现有链状态门控可以阻止提交；本文不声称已实现“禁用 CTA 的就地修复/下一步引导”。该体验改进若要推进，必须另立有视觉交互与控制边界复核的专题。
- 本文是文档 authority 迁移，不改变 UI、DOM、接口、状态机或协议行为。

## 里程碑

- 已完成：native entry/window、Ready 后分布式优先提交与 Web 代理的已实现合同已收敛；三个 2026-03 源三件套已删除，Git 与 `.pm` task evidence 保留追溯。
- 本轮：将上述已实现合同收敛为 stable authority，并更新默认路由。
- 后续：任何禁用态修复 CTA、字段或协议扩展均须作为新的有界任务处理。

## 风险

- 链禁用或未就绪、代理失败或本地目录不可写会阻断相应提交路径；仅 native Ready 后远端失败可写本地包。结果必须保持可诊断，且不能误报为另一条路径已成功。
- native/Web 存储能力不同；将 Web 失败误写为本地保存会造成不可恢复的用户误解。
- 该文档整合只反映已核实行为；若实现变更未同步更新，会重新造成 authority 漂移。

## 验收与追溯

- 确定性合同：`npm --prefix crates/oasis7_viewer run test:feedback-contract` 只覆盖 Viewer feedback mapping，不替代 launcher native/Web 闭环。
- launcher 行为回归按受影响实现路径执行 `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` 与 wasm check；可见变更另按 `testing-manual.md` S6 采集浏览器证据。
- 迁移追溯：三个 2026-03 源三件套已删除；使用 Git 与 `.pm` task evidence。Web parity 的现有专业专题仍按其当前路径维护。
