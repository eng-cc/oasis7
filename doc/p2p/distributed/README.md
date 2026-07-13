# P2P 分布式运行时文档

本目录收纳已完成的分布式运行时专题。首次阅读先按问题选择一组主题；已知文件名、需要逐份追溯 PRD/design/project 时，再使用 `doc/p2p/prd.index.md`。

## 从这里开始

| 需要回答的问题 | 首读专题 | 范围与状态 |
| --- | --- | --- |
| 分布式计算、内容寻址存储与节点角色的基础边界是什么？ | `distributed-runtime.prd.md` | 基线架构与持续依赖锚点；项目记录保留已完成的阶段性演进。 |
| 为什么分布式实现不再由 `oasis7` facade 承载？ | `distributed-hard-split-phase7.prd.md` | 已完成的 crate 边界拆分、协议归位与 ABI 收敛。 |
| 以 stake、slot/epoch 和 attestation 驱动的 Head 共识如何定义？ | `distributed-pos-consensus.prd.md` | 已完成的 PoS Head 共识专题；不要将其误读为完整以太坊信标链。 |
| 生产收口的 replication、执行绑定与网络补洞约束在哪里？ | `distributed-production-runtime-gap1234568-closure.prd.md` | 已完成的 Gap 1/2/3/4/5/6/8 收口与验证记录。 |

## 阅读与维护边界

- 本 README 只做路由和主题边界，不复制四组专题的规格、验证命令或项目台账。
- 每个主题的 `.prd.md`、`.design.md`、`.project.md` 是同一专题的规格、设计和执行证据；三者均保留，避免把审计链压缩成不可追溯的摘要。
- `distributed-runtime` 是架构基线，不自动取代已完成 hard split、PoS 或 production-gap 专题的历史验收证据。
- 需要当前 P2P 模块边界或活跃执行状态时，返回 `doc/p2p/prd.md` 与 `doc/p2p/project.md`；需要精确文件检索时使用 `doc/p2p/prd.index.md`。

## 退休审计

本轮未删除本目录文件。四组 triplet 仍分别被模块 PRD、`node`/`distfs`/`network` 主题项目、精确索引或核心审计记录引用；旧审计中的“可并入主线”建议也没有形成替代文件与调用迁移的完整证据。后续只有在替代真值落盘、调用迁移完成并复核无活跃引用后，才能删除相应历史专题。
