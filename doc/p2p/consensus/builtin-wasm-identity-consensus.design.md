# Builtin Wasm 身份共识设计

- 对应需求文档: `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 1. 设计定位
定义 builtin wasm 工件在身份、签名、共识材料与治理绑定上的一致性设计。

## 2. 设计结构
- 身份模型：module id、artifact id、signer identity 的绑定关系。
- 共识材料：签名、门限与共识快照中的验证字段。
- 运行时接入：builtin wasm 生命周期如何接到治理与共识链路。

## 3. 关键接口 / 入口
- 工件身份字段与签名验证接口
- 共识快照/治理应用所需的身份材料
- builtin wasm 安装与激活入口

## 4. 约束与边界
- 身份信息必须可追溯到受信任签名来源。
- 共识验证与治理落地必须使用同一身份语义。
- 兼容旧工件时需提供清晰迁移边界。

## 5. 设计演进计划
- 先补齐专题 Design 与互链。
- 再按 Project 任务拆解推进实现与测试闭环。
