# 会话内 BYOK 模型消费层运行手册

- 生命周期：`active`
- Owner role：`agent_engineer`
- 适用设计：[网页自带模型连接](self-supplied-model-connection.design.md)。
- 验收方法：[BYOK-01～08](../../testing/manual/self-supplied-model-connection.manual.md)。

## 当前交付边界

`oasis7_model_provider` 提供仅内存凭证、应用会话权威回调、连接 CAS、有限租约、跨连接共享的调用/token 预算、两阶段幂等检查、固定供应商传输和 Harness 适配。Launcher 的受控网页接口复用 hosted identity 的精确 Origin、CSRF、JSON 与数据库撤销校验；现有 HTML CSP/no-store 继续生效。未配置默认关闭，不显示 Key 表单。

当前支持受部署配置限定的 **非流式实验组合**：DeepSeek Chat Completions / Responses、OpenRouter Chat Completions。OpenRouter Chat 必须指定路由允许集合；OpenRouter Responses 与流式输入明确拒绝，不静默删参数或切协议。协议 fixture 不证明真实模型兼容；两家真实供应商验收仍延期。协议来源：[DeepSeek Chat](https://api-docs.deepseek.com/api/create-chat-completion/)、[DeepSeek Responses](https://api-docs.deepseek.com/api/create-response/)、[OpenRouter Responses](https://openrouter.ai/docs/api/api-reference/responses/create-responses)。

本切片没有网页登录账号到合法 Game Client executor 的解析器。页面可创建和检查模型连接，但 `executor_available=false`，activate 返回 `game_executor_unavailable`；不伪报运行。后续 #4744 接入已授权 SDK 执行器及实际运行循环后，才能验收游戏闭环。模型 Key 和登录 cookie 均不能替代 signer、任务/grant 或 Runtime fence。

## 启用前提与非敏感配置

首版仅 Unix 受控执行端：启用时进程禁用 core dump；部署还须禁请求体/APM 采样、会话重放、secret 备份及第三方页面脚本。配置中的确认字段是 operator 的部署责任声明，不是自动审计或公开发行批准。反向代理必须保留 HTTPS、精确公开 Origin 与实例亲和，并沿用 hosted identity 的可信代理配置。实例重启后 Key 全部丢失，不迁移、不从宿主环境恢复。

通过 `OASIS7_AGENT_CLIENT_CONFIG` 指向绝对路径的 TOML；该文件 **不含 Key**。`public_origin` 必须为 hosted identity 已批准的规范 HTTPS origin。示例默认关闭，实际启用前填写已审核的模型、协议与预设 revision：

```toml
enabled = false
public_origin = "https://game.example"
display_name = "受控第一方执行端"
location = "经用户确认的数据处理位置"
memory_only_controls_confirmed = false

[limits]
max_calls = 20
max_tokens = 100000
max_input_tokens = 8192
max_output_tokens = 2048
max_concurrent = 1
lease_ms = 120000
absolute_ms = 7200000
renew_interval_ms = 30000

[[presets]]
id = "reviewed-deepseek-chat-v1"
provider = "deepseek"
protocol = "chat_completions"
model = "REPLACE_WITH_REVIEWED_MODEL"
routes = []
revision = 1

[presets.limits]
max_input_bytes = 8192
max_response_bytes = 2097152
max_output_tokens = 2048
timeout_ms = 30000
```

配置与 Web capability 的 provider 标识分别为 `deepseek` / `openrouter`。OpenRouter 的 `routes` 必须为已审核的非空供应商允许集合；不接受客户端提交任意 URL、header、owner 或路由。

## 运行、限制和恢复

先确认接收方和会话隐私，再提交推理 Key；输入随提交立即清空。保存不计模型调用，不自动 probe 或运行。每次显式检查最多执行两次固定、无世界副作用的请求：工具 echo 与原 call ID 结果往返；同一个 operation ID 不重新发送。错误或结果未知保留原检查身份，先刷新状态。新的检查需要明确操作，不能把旧失败变成透明重试。

预算在同一应用会话的所有连接间共享，换 Key/新建连接不清零。发送前原子预留调用次数及完整输入/输出上界；输入按编码字节数及固定 envelope 余量作保守 token 准入。未知、部分、溢出或自相矛盾的 usage 不按零返还；该限制不是供应商账户费用硬封顶。认证、余额、限流和兼容失败不自动充值、重试或改用公共 Key。

续期有版本 CAS、滑动期限及绝对期限，不能复活已过期连接。续期改变版本并保守终结旧版 pending 检查；网页在检查进行中不续期。执行端在发送和接收结果两侧重新检查会话、连接版本、运行代、租约与绑定。Stop 永久撤销当前运行代但保留尚有效凭证；Forget/退出/撤销/到期清除凭证。已经受理的世界作业依 Runtime 原合同，不由模型连接层回滚。

回滚先关闭配置/重启受控进程，清除内存凭证并阻止新请求。不得恢复旧 Key、旧运行代或重放未知世界操作。

## 验证与未覆盖

局部自动化：

```bash
env -u RUSTC_WRAPPER cargo test -p oasis7_model_provider --lib
env -u RUSTC_WRAPPER cargo clippy -p oasis7_model_provider -p oasis7_agent_harness --all-targets -- -D warnings
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_model_client
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher model_session
npm --prefix crates/oasis7_viewer run test:ui -- software_safe_src/viewer_model_connection_client.test.js software_safe_src/model_connection_panel.test.jsx
npm --prefix crates/oasis7_viewer run build:viewer
```

自动化分别验证协议、预算、身份权威与 HTTP 路由；Chrome 桌面/窄屏使用无价值 canary 和受控 UI fixture，不能代签真实供应商、生产部署、整段 TCP 登录流程或自主游戏。实际 source/CI、独立安全评审及每项结果由实现 PR 记录，未覆盖的 BYOK-04/08 不预填 PASS。
