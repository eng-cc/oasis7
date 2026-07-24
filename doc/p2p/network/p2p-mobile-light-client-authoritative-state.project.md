# P2P 移动轻客户端权威状态（项目与历史追溯）

- 对应需求文档: `doc/p2p/network/p2p-mobile-light-client-authoritative-state.prd.md`
- 对应设计文档: `doc/p2p/network/p2p-mobile-light-client-authoritative-state.design.md`

## 迁移追溯

| 历史专题 | 回填的稳定合同 | 当前证据责任 |
| --- | --- | --- |
| `p2p-mobile-light-client-authoritative-state-2026-03-06.*` | intent 签名/去重、权威批次根、三段状态、challenge/resolve、reorg recovery、snapshot/cursor 与 session-key 吊销换钥 | 当前改动由 runtime、viewer、ops 与 QA 按受影响路径验证；日期型源在本批次尚未删除，仅作追溯。 |

源文件中的历史任务与完成记录不是现时 release、public availability、network finality 或 QA 放行证据。删除源文件前，必须完成活跃引用审计和 deletion-readiness slice。

## 任务拆解

| 任务 | 验收责任 | 状态 |
| --- | --- | --- |
| MLC-AUTH-1：保持 intent、nonce、幂等和 session-key 生命周期合同与实现一致 | 定向协议/runtime 回归；旧 key 拒绝且不同载荷重放拒绝 | 持续维护 |
| MLC-AUTH-2：保持批次根、状态单调与 challenge fail-closed 边界 | 根绑定、challenge/resolve、非 final 不消费的受影响回归 | 持续维护 |
| MLC-AUTH-3：保持 snapshot/cursor/reorg 恢复合同可诊断 | 快照校验、cursor 缺口、stable batch 回退的定向证据；真实演练另由 ops/QA 记录 | 持续维护 |

## 依赖与验证边界

- 运行时协议与仲裁实现：对应 runtime/chain 专业文档和代码证据。
- 客户端最终性呈现：viewer/client 专业 authority；本文件不证明 UI 已实现或玩家流程可用。
- 真实节点恢复、拓扑和健康：blockchain ops runbook 与当前环境证据；重启仅可作为诊断或临时恢复动作，不能替代根因修复。
- 文档维护检查：`./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh && git diff --check`。

## 状态

稳定专业 authority 已建立；日期型源三件套仍待后续 deletion-readiness 处理。当前状态只表示文档治理迁移完成，不表示代码、运行环境、发布或 QA 放行完成。

本 project 记录合同归属与维护责任，不新增完成态或发布承诺。
