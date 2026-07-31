# README 生产级缺口收口（二次）：默认 WASM 执行 + Replication RR + 分布式 Triad + 玩家节点身份（设计文档）设计

- 对应需求文档: `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.prd.md`
- 对应项目管理文档: `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.prd.md`

## 1. 设计定位
定义 README 生产级缺口二次收口设计，统一默认 WASM 执行、Replication RR、分布式 Triad 与玩家节点身份口径。

## 2. 设计结构
- 默认执行层：说明生产默认 WASM 执行路径。
- 复制拓扑层：统一 Replication RR 与分布式 Triad 说明。
- 玩家身份层：解释玩家节点身份在生产拓扑中的角色。
- README 收口层：把 Gap1/2/4/5 二次缺口统一回写。

## 3. 关键接口 / 入口
- 默认 WASM 执行口径
- Replication RR / Triad 拓扑
- 玩家节点身份说明
- README 校验项

## 4. 约束与边界
- 多个缺口口径需统一术语与主路径。
- README 不能承载实现级配置细节。
- 不在本专题扩展新的拓扑角色。

## 5. 设计演进计划
- 先冻结二次收口范围。
- 再统一执行/拓扑/身份表述。
- 最后完成 README 回写。
