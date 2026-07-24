# DistFS 子域入口

## 从这里开始
- 想确认 DistFS 在 P2P/consensus/execution/observer 组合后的整体闭环、测试层级与 claim boundary：先读 `testing-manual.md#s9a链上大世界状态底座自闭环`；本页只负责 DistFS 子域入口。
- 想理解 DistFS production hardening 的当前合同：先读 `distfs-production-hardening.prd.md`，再按需进入同名 design/project。
- 想查分布式韧性与自愈边界：先读 `distfs-distributed-resilience.prd.md`；其中 NodeRuntime 轮询接线仍作为受限的增量能力，保留“缺依赖跳过、单轮失败不阻断 tick”的边界。
- 想查路径索引、标准文件 IO、builtin wasm storage/API：直接按专题名进入对应 PRD / design / project 三件套。

## 阅读面边界
- 本页只做 DistFS 子域分流，不复制 `doc/p2p/prd.index.md` 的完整三件套长表。
- DistFS 专题的 blob closure、复制、恢复或 self-healing green 结果只能作为链上大世界状态底座的存储层证据；模块级闭环以 S9A 的 `module_required / module_full / integration_required / release_full` 分层为准。
- `distfs-production-hardening` 是 production-hardening 的唯一当前专业权威，收敛历史 Phase 1-9 的本地 CAS/索引完整性与 storage challenge 调度合同。原 phase 文件名仅保留在历史 audit/review 文字和 Git history 中，不作为当前入口或 readiness 依据。
- distributed-resilience 是异构 provider、无单机完整依赖与自愈组的默认主入口；轮询与 NodeRuntime 接线只承载受限增量差异。
- 完整文件级检索仍回到 `doc/p2p/prd.index.md`。

## 专题入口
| 组 | 默认入口 | 边界 |
| --- | --- | --- |
| DistFS production hardening | `distfs-production-hardening.prd.md` | 收敛 Phase 1-9 的历史完成态；不替代节点存储奖励池结算、跨节点 challenge/proof 协议或 production/readiness 证据。 |
| DistFS distributed resilience | `distfs-distributed-resilience.prd.md` | 异构 provider 选择、无单机完整依赖、自愈轮询与受限 NodeRuntime 接线统一收敛；NodeRuntime 仅在依赖齐备时执行，单轮失败不阻断 tick。 |

## 维护规则
- 新增 DistFS 专题时，先判断是否属于既有专业权威；历史完成态应语义吸收到对应当前专题并保留 provenance，不新建默认 phase 入口。
- 如果新增内容引入新的通用边界，应先拆出新的主入口，再更新本页分流。
