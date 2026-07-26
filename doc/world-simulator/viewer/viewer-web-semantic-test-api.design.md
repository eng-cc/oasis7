# Viewer Web 语义测试 API 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-semantic-test-api.project.md`

## 1. 设计定位
定义 Web Viewer 面向自动化的语义测试 API、观测字段与步进控制边界。

当前 implementation truth 是 `crates/oasis7_viewer/software_safe_src/legacy_core.js`；本设计不把历史 wasm/Bevy bridge 当作可继续扩展的 source layer。

## 2. 设计结构
- 语义层：暴露稳定的 UI/世界状态语义，而非脆弱 DOM 细节。
- 控制层：提供步进、查询与断言友好的测试入口。
- 观测层：输出测试所需的状态摘要与错误信号。
- profile 层：以握手后的 `controlProfile` 限制可提交动作；live 不暴露 seek/seek_event。
- 兼容层：`logicalTime` 与 `eventSeq` 是当前观测字段，`tick` 只兼容等同于 `logicalTime`；空 mailbox、reconnect 和 queued 控制不生成观测增量。

## 3. 关键接口 / 入口
- 语义测试 API 路由与返回结构
- 步进/查询/状态快照入口
- Playwright 或测试脚本消费的稳定字段

## 4. 约束与边界
- API 设计要避免与生产控制面强耦合。
- 语义字段应稳定、可版本化。
- 自动化入口不能影响正常 viewer 交互路径。
- `step` parser 必须拒绝无效 payload 而不 throw；`runSteps` 只复用同一 step-count grammar，不重新引入历史 UI DSL。
- WebSocket error/close、unsupported action、gameplay gate 与 runtime no-progress 必须保持可区分的自动化状态。

## 5. 设计演进计划
- 先完成 Design 补齐与互链回写。
- 再沿项目管理文档推进实现与验证。
