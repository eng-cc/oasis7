# 游戏客户端启动器控制面与机器接口设计

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.project.md`

## 目标

launcher 的 native/Web 表现层共享 schema 和控制状态映射；`oasis7_web_launcher` 承担静态资源、状态读取和既有受控操作路由。客户端不自行重演进程或 runtime 规则，而是渲染服务端的当前状态与结果。

## 范围

本设计覆盖 launcher/Web 表现层、控制面映射与 operator HTTP-JSON 接口；不覆盖 runtime 规则、执行语义或 `DecisionProvider`。

## 接口 / 数据

1. 共享表现层：native 与 Web 共用字段、分组、文案和状态/错误映射。
2. 控制层：`/api/state` 与既有 game/chain 控制路由提供服务端快照和受控请求结果。
3. 机器层：`/api/gui-agent/capabilities`、`/state`、`/action` 组成 operator HTTP-JSON 适配面；capabilities 响应是 action-list truth。
4. 安全边界：查询/动作必须经过已声明映射；`hosted_public_join` 仅对枚举 operator 路由应用 peer-IP gate，不扩大为部署或安全 readiness 声明。

## 里程碑

- 已完成：shared schema、静态资源/控制面并存、GUI-agent capabilities/state/action 合同。
- 后续：任一实现变更以新的任务和运行中 capability discovery 响应重审。

## 风险

- 机器 action 必须是服务端枚举的严格 JSON 请求；响应始终附带最新状态，避免 UI/机器客户端各自猜测。
- 保留既有 `/api/*` 和静态资源行为，不能把 GUI-agent 变成任意代理。
- GUI-agent 是 launcher operator interface；它不实现或替代 world Agent、DecisionProvider、runtime authorization 或执行规则。
- UI/schema 变更应保持浏览器可自动化；可视化代码变更另按 `testing-manual.md` S6 做浏览器与截图验证。本次为文档迁移，无 UI 产物变更。
