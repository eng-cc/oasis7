# p2p 模块设计总览

审计轮次: 7

- 对应需求文档: `doc/p2p/prd.md`
- 对应项目管理文档: `doc/p2p/project.md`
- 对应文件级索引: `doc/p2p/prd.index.md`

## 1. 设计定位
`p2p` 模块的 `design.md` 负责描述网络、共识、分布式存储和节点协作的总体设计入口。

## 2. 阅读顺序
1. `doc/p2p/prd.md`
2. `doc/p2p/design.md`
3. `doc/p2p/project.md`
4. `doc/p2p/prd.index.md`
5. 下钻 `blockchain/`、`consensus/`、`distfs/`、`network/`、`node/` 等专题目录

## 3. 设计结构
- 网络层：节点发现、连接、同步、传输与非全公网 reachability 边界。
- 共识层：身份、投票、状态传播与一致性策略。
- 存储层：distfs、路径索引、复制与恢复。
- 节点层：奖励、执行、观测与治理对接。

## 4. 集成点
- `doc/world-runtime/prd.md`
- `doc/headless-runtime/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- 基础链路进入 `network/`、`consensus/`
- mixed-topology / 非全公网主链级覆盖网络进入 `network/p2p-mainnet-private-reachability-architecture-2026-04-01.*`
- 数据与复制进入 `distfs/`
- 节点执行与奖励进入 `node/`
- 区块链和生产化扩展进入 `blockchain/`、`distributed/`

## 设计目标
- 提供 `p2p` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/p2p/prd.md`
- 执行入口：`doc/p2p/project.md`
- 兼容执行入口：`doc/p2p/project.md`
- 索引入口：`doc/p2p/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy `*.project.md` 长期保留，执行入口会继续双轨并存。
