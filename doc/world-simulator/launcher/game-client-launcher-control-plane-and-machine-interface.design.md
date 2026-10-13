# 游戏客户端启动器控制面与机器接口设计

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`
- 历史迁移、验证与 task 状态：GitHub task issue evidence。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [控制动作生命周期](game-client-launcher-control-plane-and-machine-interface.prd.md#launcher-action-lifecycle) | 既有 operator Start/Stop、启动输入与预检分别保留准确结果和恢复 | [动作与输入设计](#launcher-action-lifecycle-design) | Launcher / 平台进程与脚本 owner / QA | 不新增 hosted 本地链、信号协议或真实模型探测 |

## 目标

launcher 的 native/Web 表现层共享 schema 和控制状态映射；`oasis7_web_launcher` 承担静态资源、状态读取和既有受控操作路由。客户端不自行重演进程或 runtime 规则，而是渲染服务端的当前状态与结果。

## 范围

本设计覆盖 launcher/Web 表现层、控制面映射与 operator HTTP-JSON 接口；不覆盖 runtime 规则、执行语义或 `DecisionProvider`。

## 接口 / 数据

1. 共享表现层：native 与 Web 共用字段、分组、文案和状态/错误映射。
2. 控制层：`/api/state` 与既有 game/chain 控制路由提供服务端快照和受控请求结果；停止 UI 统一呈现用户停止和窗口关闭所触发的有界优雅停止、终止回退或失败诊断，不将请求结果扩展为 runtime 收敛结论。
3. 机器层：`/api/gui-agent/capabilities`、`/state`、`/action` 组成 operator HTTP-JSON 适配面；capabilities 响应是 action-list truth。
4. 安全边界：查询/动作必须经过已声明映射；`hosted_public_join` 仅对枚举 operator 路由应用 peer-IP gate，不扩大为部署或安全 readiness 声明。
5. Web 诊断层：初始化、浏览器环境、状态读取或控制面失败必须映射为可见 blocked/error，而不是 ready 或静默崩溃；具体浏览器 clock、轮询、WASM lifecycle 与 runtime session 仍由 `game-client-launcher-runtime-session-continuity.prd.md` 及对应专业 authority 定义。

<a id="launcher-action-lifecycle-design"></a>
## 控制动作与启动输入设计

本节承接[控制动作生命周期要求](game-client-launcher-control-plane-and-machine-interface.prd.md#launcher-action-lifecycle)。
不增加 action endpoint、持久化控制任务或 hosted 本地链能力；以下待修项按实际实现 PR 验证。

### 停止和服务退出

沿用同步 stop 响应，使用动作专用的端到端等待预算，不让普通状态查询的短超时覆盖停止。
预算覆盖当前 `GRACEFUL_STOP_TIMEOUT_MS`、有界终止/wait 与传输余量；客户端收到该动作
终态才显示完成。响应丢失或预算耗尽时结束本次网络等待，显示结果未知并通过现有 state 重新核实，不能
把超时解释成进程已停或停止必定失败。后续真实 snapshot 可以恢复准确状态，不自动重发
非幂等控制操作。服务端停止回退同样有界，失败时保留可管理句柄与具体诊断。

accept loop 区分正常停止、可恢复错误与不可恢复错误。不可恢复错误先保存主错误，再退出
循环进入统一收尾；不得从循环内部直接返回绕过 game/chain stop/wait。收尾逐一尝试现有
托管对象，记录各自结果，保留主错误；没有 child 的路径不制造清理成功证据。已声明的
托管子进程之外，不新增全局 kill、进程发现或 Windows graceful signaling 语义。

基准 `5e930fa` 的 native `app_process.rs` HTTP 超时为 1500ms，服务端优雅等待为 4000ms；
`server.rs` 的非 WouldBlock accept 错误直接返回，确有预算/收尾缺口。`start_chain_process`
已有先拒绝 running 再变更的 guard，须保留并覆盖 GUI-agent 与 HTTP 两入口，不为本次设计
重新实现已存在的分支。

### 解析与启动前验证

1. 路由在既有访问检查之后，把 parse/validation 失败映射成当前 `ApiResponse`；错误分支同样
   读取准确快照和写完整 HTTP 响应。不得把 parse error 只写服务端日志后断开连接，也不把
   非 2xx 统一变成无内容的客户端 QueryFailed。无效输入不提交新配置、不 spawn。
2. CLI parser 保留“用户是否显式提供静态目录”的来源信息，resolver 按 explicit→environment→
   existing fallback 选择。比较字符串是否等于默认值不能判断来源；空环境按未设置处理。
   不增加候选目录搜索或让有效显式路径受错误环境值覆盖。
3. `run-launcher-stack.sh` 的 provider preflight 解析 info/health 后按现有 provider 健康合同
   校验必需字段类型、协议/能力及 healthy 语义，再生成成功记录；网络、JSON、结构或语义
   失败返回非零，并且不进入 Launcher 启动分支。不把健康探针升级为真实推理或工具调用。
4. `run-producer-playtest.sh` 在启动 child 前解析最终 metadata 参数并规范化一次，父级读取
   和子级写入使用同一值。沿用子入口当前重复参数规则，父级不得保留另一默认值；若选择
   不支持某类参数则启动前清晰拒绝。打开浏览器只消费本次 metadata 的 ready URL。

| 独立项 | 最小有效验证 | 不可替代的范围 |
| --- | --- | --- |
| L1 慢停止 | 真实 HTTP handler＋可控 child：快停、约 2 秒退出、约 4 秒 graceful 回退、真无响应；native transport 读取真实终态并保留有界未知状态 | Windows active-child 单独运行；普通 Unix/mock/no-op 不代签 |
| L2 服务异常 | 已托管 child 后注入 accept failure，核 stop/wait、PID/端口退出；低 FD/短连接峰值、正常 SIGINT、无 child 与可恢复退避 | 连接数量限制不证明 teardown；不纳入 detached shell 转发 |
| L3 拒绝 Start | 已有可存活 child 后从 GUI-agent 与 `/api/chain/start` 提交 disabled/hosted/invalid-mode，拒绝后同 PID/配置仍可 Stop；合法重复启动/空态另测 | 只测空 ServiceState 不证明保留原句柄 |
| L4 非法 payload | 两个 Start 路由的坏 JSON/字段类型/未知 tier，断言完整可解码响应、invalid_request、原 state 和零 spawn，另测有效输入 | parser 单测不证明完整 HTTP；不改 chunked/429 |
| L6 显式 static-dir | 两目录有效及坏环境目录下比较省略、显式 web、./web、绝对路径、空环境；Windows wrapper 参数透传 | 不声称所有目录解析或打包均已验证 |
| L7 健康语义 | mock HTTP 200 healthy/ok=false/degraded/缺字段/错类型，加网络/JSON 失败；断言退出码、未启动和成功产物未生成 | 不需要真实模型；preflight pass 不等于玩法 pass |
| L8 metadata | 受控 stack/browser fixture 覆盖默认、绝对/相对/含空格/重复参数与非 headed 直接 exec，核父子实际读写路径 | 不需要真实 provider 或公开发布 |

实际测试文件优先复用 `oasis7_web_launcher/control_plane/tests.rs`、`oasis7_web_launcher_tests.rs`、
`oasis7_client_launcher` 的 process tests、`oasis7_game_launcher/viewer_static_dir_tests.rs` 和
相关 launcher/producer shell tests；新增回归绑定实际代码，不在本文虚构函数或通过结果。
可见状态变化按 S6 验证，原控制权限和 runtime/session 专业合同保持不变。

## 里程碑

- 已完成：shared schema、静态资源/控制面并存、GUI-agent capabilities/state/action 合同。
- 后续：任一实现变更以新的任务和运行中 capability discovery 响应重审。

## 风险

- 机器 action 必须是服务端枚举的严格 JSON 请求；响应始终附带最新状态，避免 UI/机器客户端各自猜测。
- 保留既有 `/api/*` 和静态资源行为，不能把 GUI-agent 变成任意代理。
- GUI-agent 是 launcher operator interface；它不实现或替代 world Agent、DecisionProvider、runtime authorization 或执行规则。
- 控制面只呈现 stop 请求和快照结果；chain/runtime/session、execution-world recovery 及 WASM 兼容性不在此设计中重演。
- UI/schema 变更应保持浏览器可自动化；可视化代码变更另按 `testing-manual.md` S6 做浏览器与截图验证。本次为文档迁移，无 UI 产物变更。

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 验证方法、test/manual source 与环境 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [控制动作生命周期](game-client-launcher-control-plane-and-machine-interface.prd.md#launcher-action-lifecycle) | [动作与输入设计](#launcher-action-lifecycle-design) | 上表 L1/L2/L3/L4/L6/L7/L8 分别验收，正常与失败结果保持 | [测试手册](../../../testing-manual.md) 的 launcher 定向测试与适用 S6；复用上列实际 process/HTTP/CLI 测试接点，按上表注入、真实 child、Windows 和受控脚本 fixture 选择；planned | 各实现 PR 的当前候选、真实命令和结果 | 本次未运行；mock/Unix 不代替 Windows active child，preflight 不证明玩法 |
