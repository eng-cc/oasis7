# 客户端启动器引导配置与可用性设计（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.prd.md`
> 对应执行台账: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md`

## 表现结构

1. **语言与配置层**：现有语言状态驱动当前文案；配置预检将既有阻断项留在启动/配置表面，并保持已有高级配置入口可达。
2. **事实状态层**：launcher 忠实呈现已有 ready、未就绪、disabled、blocked/error 与 observer 状态，连同既有原因和可达动作；不得把状态卡片、日志或历史结果表示为成功。
3. **自引导层**：onboarding、任务提示、错误卡与 CTA 只编排当前已存在的动作；可跳过/重置的引导不能绕开配置、权限或 runtime 门槛。
4. **响应式层**：关键状态、阻断说明和操作入口随当前 native/Web 布局保持可读；实际信息层级、CTA 文案与窄屏验收由 game_visual_interaction_designer 定义。

## 控制与信息边界

- launcher 表现层不定义 world 规则、链角色权限、runtime 状态演化、持久化或网络/发布政策。
- `observer`、blocked、disabled 和未就绪只按当前实现的结构化含义呈现；不构成玩家访问、node admission、网络健康或 readiness 结论。
- 不新增自动修复、默认值写回、后台轮询、远端遥测、演示编排、配置画像或持久化续跑合同；历史专题中出现这些内容时仅可追溯，不能作为当前设计授权。
- 此次迁移不改变 DOM、样式、布局或交互。未来可见变更必须按 `testing-manual.md` S6 获取浏览器证据和视觉角色验收。

## 代码接点

- 当前实现入口：`crates/oasis7_client_launcher/src/{config_ui.rs,launcher_core.rs,main_app_shell.rs,self_guided*.rs,app_process.rs,app_process_web.rs}`。
- 本设计记录表现层边界，不冻结具体函数、字段、组件树、存储方式、HTTP 路径或请求频率。
