# 客户端启动器跨表面受控动作设计

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md`
> 对应项目: `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.project.md`

## 设计分层

1. **共享语义层**：为配置问题、动作前置、提交中、结构化结果和恢复建议定义同源含义。
2. **平台适配层**：native 与 Web 分别决定可见字段、输入方式、存储与 transport；适配差异不得被 UI 隐藏成成功或等价声明。
3. **控制面边界**：Web 通过 Launcher 控制面转发受支持请求；控制面保留 runtime 的接受、拒绝和错误语义，而不重写结算或授权规则。
4. **反馈层**：动作表面将 executable、blocked、accepted/rejected/failed 与下一步呈现给用户，并在 in-flight 期间防止误导性的重复提交。
5. **LLM 设置适配层**：native 通过 launcher 本地配置的 `[llm]` 字段保存并读回 `api_key`、`base_url` 和 `model`；Web 通过浏览器本地存储保存并读回。两个表面都将读取、解析或写入失败作为可恢复的可见结果，不把草稿、旧值或缺省值当成已保存状态。

## 边界

- native-only 字段不进入 Web 表单或 Web required 校验；Web 专有前置仍由 Web authority 判断。
- 浏览器本地存储仅是 Web 设置适配。存储不可用必须反馈失败，不能推断安全凭据持久化。
- native 配置保存与 Web 本地读回都只是 Launcher 表面结果；它们不证明 provider 鉴权、runtime 凭据注入或跨 profile/重启持久性。secret 不进入状态快照、GUI-agent、可见错误、截图或任务证据。
- action acceptance 是提交阶段结果；最终状态继续由 runtime 及其查询/验证 authority 决定。
- 最近历史可为有界 process-static 视图，设计不得把它显示为完整或重启后仍存在的记录。

## 演进约束

新增跨表面动作必须先明确共享语义、平台差异、控制面边界、失败恢复和相应专业验证；不得以“parity”标题替代这些证据。
