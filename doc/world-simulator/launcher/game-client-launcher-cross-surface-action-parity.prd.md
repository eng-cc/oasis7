# 客户端启动器跨表面受控动作契约

本文承载 native 与 Web Launcher 在配置、设置、反馈与转账等受控动作上的当前专业边界。它不替代 runtime 的业务规则、WASM 平台契约、发行判断或历史任务证据。

- 对应设计: `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.design.md`
- 历史迁移、验证与 task 状态：GitHub task issue evidence。

## 目标

让适用 Launcher 表面真实呈现受控动作的可执行、阻断、结果和恢复语义，而不把平台适配误表述为统一实现或更强的运行时结论。

## 范围

覆盖 native/Web 的配置、设置、反馈、转账提交及其可用近期结果；不定义 runtime 业务规则、WASM ABI、结算或发布门禁。

## 当前能力边界

- 两个表面可共享动作语义、阻断原因、结构化结果和恢复入口；平台可保留不同的字段、存储和传输适配，不能据此宣称字段、组件或进程完全一致。
- native-process-only 配置字段只在适用表面出现并参与其校验；Web 仍可能有自己的可执行路径或配置前置。字段可见性不是权限、发布或可玩性结论。
- 设置、反馈与转账在当前 authority 支持时应显示 `executable`、`blocked` 或 `result`，并在失败时给出真实恢复下一步；浏览器存储失败、控制面不可达或 runtime 拒绝必须可诊断，不得伪装为成功。
- Web 控制面可代理受支持动作到 runtime；代理可达不等于授权。浏览器本地存储和控制面代理是 Launcher 适配，不是 WASM ABI、manifest、metering、permission 或 lifecycle authority。

### LLM 设置的本地存储与读回边界

- native 表面将当前 LLM 设置读写到本地 launcher 配置的 `[llm]` 字段（`api_key`、`base_url`、`model`）；保存后重载应以实际读回或明确错误呈现，空值移除仅适用字段。配置缺失、解析、读取或写入失败必须保留失败结果和修复入口，不得显示为已保存。
- Web 表面以浏览器本地存储适配当前 LLM 设置；保存后的读回只说明当前浏览器本地状态。存储不可用、读取或写入失败必须可诊断，且不应把旧值、草稿或缺省值伪装为新设置已持久化。
- 任一表面的保存/读回不证明 provider 可用、鉴权成功、控制面或 runtime 已取得凭据，亦不承诺跨浏览器 profile、隐私模式、设备、重启或清理后的持久性。`api_key` 等 secret 不得出现在 launcher 状态、GUI-agent、反馈/错误文本、截图或任务证据中；本专题不对本地存储的加密、安全性、轮换或 secret-management 作出承诺。

## 受控动作

| 动作 | 受支持输入与前置 | 可见结果 | 不作出的推断 |
| --- | --- | --- | --- |
| 配置与设置 | 仅显示当前平台适用字段；缺失或无效配置可阻断；LLM 设置按 native 配置或 Web 本地存储适配 | 保存后读回、重载或明确失败，并提供修复路径 | 不承诺跨端字段、本地持久化、provider 可用性或 secret 安全性相同 |
| 反馈 | 由当前表面和控制面支持时可提交；未就绪时应阻断 | 结构化接受、拒绝或代理失败 | 不承诺提交必达、持久账本或人工答复 |
| 转账提交 | 由当前 runtime/account/chain 前置决定；可经 Web 控制面提交 | 接受/拒绝/失败及结构化原因；接受可返回 action 标识 | submit acceptance 或 action ID 不等于 settlement、最终确认或成功 |
| 最近历史与状态 | 仅展示当前可用的进程内/接口结果 | 有界、可诊断的近期记录 | 不承诺完整、持久、重启安全或可审计总账 |

## 接口 / 数据

具体 API、字段、错误码和结果结构仍由 Launcher、runtime 与 WASM 的现有专业 authority 定义；本契约只要求其呈现不丢失 executable/blocked/result/recovery 的含义。

## 里程碑

- 已完成：四组 2026-03 跨表面动作专题的当前语义已迁移到稳定 triplet，并修复活跃入口。
- 后续：新增动作或改变平台代理、存储、输入字段或结果语义时，重新获得相应专业 owner 的验证。

## 风险

- 将 submit acceptance/action ID 解释为 settlement 或最终确认会制造错误状态承诺。
- 将浏览器存储、控制面可达或近期历史解释为 WASM authority、安全持久化或完整账本会扩大实际边界。
- 将 LLM 设置保存/读回解释为凭据已被 runtime 使用、provider 已认证，或 secret 已受安全保护，会制造超出 launcher 表现层的承诺。

## 验收与非目标

- 适用表面应能对可执行、阻断、结果和恢复作真实呈现；重复提交、配置缺失、代理失败和 runtime 拒绝不得静默吞没。
- LLM 设置应以本地保存、读回或明确失败的真实结果呈现，并避免在任何表面或诊断证据中泄露 secret。
- 具体 API、字段、错误码、runtime 结算、浏览器存储策略和测试命令由对应代码、runtime PRD、WASM authority 与验证证据维护。
- 本文不宣称 100% parity、产品级可玩性、发行/网络 readiness、性能 SLO、自动 nonce 成功或跨重启持久性。

## 追溯

本稳定分册吸收了 2026-03 的 Web 必填配置、设置/反馈、转账闭环与 transfer parity 四组历史专题；逐任务完成证据由 Git history 与 GitHub task issue evidence comments 追溯。
