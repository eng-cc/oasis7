# oasis7 Runtime：线上模块发布合法性闭环补齐（2026-03-08）

- 对应设计文档: `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.design.md`
- 对应项目管理文档: `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md`

审计轮次: 1


## 1. Executive Summary
- Problem Statement: 当前实现已具备模块身份校验、治理提案与多 runner 对账能力，但仍存在“内置清单写死、身份签名弱化回退、本地可自签最终性证书”等断点，且生产发布路径仍未与 CI 完全解耦，无法满足去中心化场景下“所有节点仅执行合法模块”的目标态。
- Proposed Solution: 建立“线上发布清单 + 外部多签最终性 + 运行时强校验 + 节点侧可审计发布流水”四段闭环，把合法性锚点从本地构建结果迁移到治理确认的发布清单与签名证据，并明确生产发布不依赖 CI。
- Success Criteria:
  - SC-1: 生产路径下，运行时只接受“治理确认的 manifest hash + 合法 artifact 签名”模块，非法模块拦截率 100%。
  - SC-2: 生产路径下禁用本地派生 finality 证书自动 apply；`apply_proposal_with_finality` 仅接受外部阈值签名证书。
  - SC-3: builtin 模块加载主路径从仓库 `include_str!` 迁移到线上发布清单源，支持信任根版本化与轮换。
  - SC-4: 建立去中心化发布流水（提案、复构建证明、阈值签名、清单激活），生产路径不依赖 CI 服务可用性。
  - SC-5: 多平台构建差异在发布阶段可收敛为“同平台可复现 + 清单签名确认”，上线前不可复现构建必须阻断。
  - SC-6: 主 CI 不承载生产发布写入与激活；CI 仅做开发回归，生产发布由发布节点流水完成且可审计。
  - SC-7: `proof_cid` 必须指向一份可归档、可重放的 attestation proof payload；发布节点要能在 node-side 直接提交 `ModuleReleaseSubmitAttestation`，而不是依赖 CI workflow 代替生产提交流水。

## 2. User Experience & Functionality
- User Personas:
  - 链上验证者签名集（按 epoch 快照）：对模块合法性与发布生效进行阈值签名确认。
  - 节点运营者：依赖线上清单拉取合法模块，拒绝非授权模块。
  - 协议维护者：维护信任根、公钥轮换、证书格式与兼容窗口。
  - 发布节点运营者：维护可审计的提案、复构建证明与签名聚合流水。
- User Scenarios & Frequency:
  - 新模块发布：每次模块升级/新增均执行一次（高频）。
  - 紧急冻结与撤销：安全事件触发时执行（低频高优先级）。
  - 信任根轮换：按季度/半年度执行（中频）。
  - 构建漂移排查：门禁失败时执行（中频）。
- User Stories:
  - PRD-WORLD_RUNTIME-016: As a 节点运营者, I want nodes to load modules only from governance-approved online manifests, so that arbitrary local build outputs cannot enter execution path.
  - PRD-WORLD_RUNTIME-017: As a 安全评审者, I want artifact identity to require cryptographic signatures in production, so that `identity_hash` fallback cannot bypass trust policy.
  - PRD-WORLD_RUNTIME-018: As a 发布节点运营者, I want decentralized release attestations and threshold-signature finalization with full audit trail, so that production release does not depend on centralized CI services.
- Critical User Flows:
  1. Flow-OMR-001（标准发布）:
     `发布提案提交 -> 多节点同平台重建 -> 生成发布清单/identity/签名 -> 验证者签名集阈值确认（建议 >=2/3 stake 且满足最小独立节点数） -> finality 证书上链 -> 节点按 manifest hash 拉取并验证执行（主 CI 不参与签名与激活）`。
  2. Flow-OMR-002（发布阻断）:
     `任一平台构建不可复现或签名证明不足 -> 证书阈值不达标 -> 不生成发布证书 -> 运行时保持旧版本`。
  3. Flow-OMR-003（紧急撤销）:
     `发现已发布模块风险 -> 验证者签名集超阈值触发 emergency veto/brake（或由治理合约规则触发） -> 发布清单版本冻结/撤销 -> 节点拒绝目标模块`。
  4. Flow-OMR-004（信任根轮换）:
     `治理提案更新 signer set 与阈值 -> 证书验证策略切换 -> 兼容窗口结束后禁用旧 key`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 线上发布清单 | `release_id`、`manifest_hash`、`module_id`、`platform_hash_tokens`、`identity_refs`、`created_at` | 发布节点提案并广播清单 | `draft -> signed -> finalized -> active/revoked` | `module_id` 字典序稳定；platform token 按 canonical 平台序输出 | 任意节点可提案，激活需 epoch 快照内验证者阈值签名（建议 `>=2/3 stake` 且满足最小独立节点数） |
| 发布信任根快照 | `epoch_id`、`validator_set_hash`、`stake_root`、`threshold_bps`、`min_unique_signers`、`effective_height` | 治理更新并广播快照 | `draft -> pending -> effective -> retired` | stake 以 canonical 编码聚合并计算总权重；同 epoch 仅允许一个 effective 快照 | 仅治理变更可写；节点按生效高度切换 |
| artifact 身份签名 | `wasm_hash`、`source_hash`、`build_manifest_hash`、`signer_node_id`、`signature_scheme`、`artifact_signature` | 生成并校验签名，不允许生产回退 | `unsigned -> signed -> verified/rejected` | payload 固定 `modsig:ed25519:v1` | signer 必须在信任根中且达到阈值策略 |
| finality 证书 | `proposal_id`、`manifest_hash`、`consensus_height`、`epoch_id`、`threshold_bps`、`min_unique_signers`、`signatures` | 外部提交证书后执行 apply | `approved -> queued -> finalized -> applied` | 校验 `stake_signed_bps >= threshold_bps` 且 `unique_signers >= min_unique_signers`；签名集合去重 | 仅 epoch 快照内验证者签名计入阈值并可签发 |
| 运行时加载策略 | `trust_root_version`、`active_manifest_hash`、`allow_local_fallback`（prod=false） | 节点启动/热更新时校验并拉取模块 | `bootstrap -> sync -> enforce` | 先验签再加载，失败不降级到未授权字节 | 生产节点禁止本地未授权 fallback |
| 去中心化发布证据流程 | `proposal_tx`、`request_id`、`build_attestations{signer_node_id,platform,build_manifest_hash,source_hash,wasm_hash,proof_cid}`、`finality_cert`、`audit_log_cid` | 节点提交复构建证明并聚合签名；证明必须显式绑定 `request_id` 与 `wasm_hash` | `proposed -> attested -> threshold_reached -> finalized` | 证明按 `signer_node_id + platform` 去重聚合，冲突提交拒绝并保留首条审计证据 | 仅信任根内 signer 且已绑定 node identity 的证明计入阈值 |
| attestation proof payload | `proof_schema_version`、`request_id`、`signer_node_id`、`platform`、`evidence_summary`、`attached_files[]`、`payload_sha256` | 节点先打包 proof payload，再以其内容地址作为 `proof_cid` 提交 attestation | `evidence_collected -> packaged -> submitted -> archived` | `payload_sha256` 必须稳定对应 payload 内容；同一 payload 重放不得改变 `proof_cid` | payload 可由任意合规节点生成，但提交 attestation 仍需受信 node identity |
| 兼容状态机映射 | `module_release_request_id`、`release_id`、`shadow_manifest_hash`、`applied_manifest_hash`、`applied_proposal_id` | `ModuleRelease*` 事件驱动写入发布清单映射 | `requested/shadowed/approved/applied -> draft/finalized/active` | 映射关系 append-only；同 `request_id` 不可重写 | 仅治理 apply 与发布流水写入 |
- Acceptance Criteria:
  - AC-1 (PRD-WORLD_RUNTIME-016): 生产路径移除 builtin 模块清单 `include_str!` 作为主真源，改为线上发布清单驱动；本地内置仅允许作为应急 bootstrap 且默认关闭。
  - AC-2 (PRD-WORLD_RUNTIME-016): 节点在加载模块前必须同时校验 `manifest_hash`、artifact signature、signer trust root，任一失败即拒绝执行。
  - AC-3 (PRD-WORLD_RUNTIME-017): 生产配置下禁用 `identity_hash_v1` 宽松回退；仅允许 `ed25519`（或后续明确版本）签名方案。
  - AC-4 (PRD-WORLD_RUNTIME-017): `apply_proposal()` 在生产配置下不得本地生成 finality 证书；必须显式传入外部阈值签名证书。
  - AC-5 (PRD-WORLD_RUNTIME-018): 新增去中心化发布证据链路，支持提案、复构建证明、阈值签名与证据归档，且可由任意合规节点触发。
  - AC-6 (PRD-WORLD_RUNTIME-018): CI 在生产流程中不作为发布必要条件，仅承担开发期回归与兼容性校验，不拥有生产发布判定权。
  - AC-7 (PRD-WORLD_RUNTIME-018): 发布端各节点构建只要求“同平台可复现”；跨平台不同 hash 由 keyed token 表达，最终以签名清单合法性为准。
  - AC-8 (PRD-WORLD_RUNTIME-017): finality 证书校验必须绑定 `epoch_id + validator_set_hash + stake_root + threshold_bps + min_unique_signers`；仅“签名数量阈值”判定视为不达标。
  - AC-9 (PRD-WORLD_RUNTIME-017): 安装/升级/回滚/发布应用等生产路径不得调用本地自签 `apply_proposal()`；统一改为外部证书路径。
  - AC-10 (PRD-WORLD_RUNTIME-018): 现有 `ModuleReleaseRequested/Shadowed/RoleApproved/Applied` 状态机需输出到 release manifest 映射，保证迁移期可回放且可追溯。
  - AC-11 (PRD-WORLD_RUNTIME-018): 从主 CI 移除生产发布写入与激活职责（含默认模块发布写入）；主 CI 只保留 `--check` 类回归校验。
  - AC-12 (PRD-WORLD_RUNTIME-018): 为阈值验签与发布收敛提供可执行基准：`stake/epoch` 校验耗时与“2 epoch 内收敛”均需有固定测试入口（`scripts/oasis7-runtime-finality-baseline.sh`）与归档产物（`summary.md`/`summary.json`）。
  - AC-13 (PRD-WORLD_RUNTIME-018): `proposal -> attestation` 证明落盘必须包含 `signer_node_id/platform/build_manifest_hash/source_hash/wasm_hash/proof_cid`，并强校验 `wasm_hash == release request manifest.wasm_hash`，冲突重复证明拒绝。
  - AC-14 (PRD-WORLD_RUNTIME-018): `ModuleReleaseApply` 必须按当前 epoch 快照 signer 集做 attestation 阈值聚合，且仅快照内 signer 计入阈值；阈值不足不得激活 release manifest。
  - AC-15 (PRD-WORLD_RUNTIME-018): 发布运行手册必须定义并可执行分诊“证明冲突、attestation 阈值不足、manifest 不可达/回滚/漂移”三类阻断场景，并明确主 CI 仅做开发回归与对账，不承担生产发布写入/激活。
  - AC-16 (PRD-WORLD_RUNTIME-018): `oasis7_chain_runtime` 必须提供 node-side `ModuleReleaseSubmitAttestation` 提交入口，使发布节点能够直接提交 attestation action 到共识队列，而不是依赖 CI 或手工改状态。
  - AC-17 (PRD-WORLD_RUNTIME-018): proof payload 打包脚本必须能输出可归档目录与稳定 `proof_cid`，并把 `request_id/signer_node_id/platform/build_manifest_hash/source_hash/wasm_hash/builder_image_digest/container_platform/canonicalizer_version` 与 release evidence 摘要绑定到同一 payload。
- Non-Goals:
  - 不在本期引入全新加密算法（继续以 ed25519 为主）。
  - 不在本期重构模块业务 ABI 或 gameplay 逻辑。
  - 不在本期建设中心化单点构建服务；仍保持多节点提案与复核。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题为运行时/治理/发布工程链路，不涉及模型推理策略）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview:
  - 信任平面：`trust_root(epoch snapshot + validator stake + threshold_bps + min_unique_signers + version)` 由治理维护并版本化。
  - 发布平面：发布节点在 canonical 平台复构建并提交证明，聚合后组装 release manifest。
  - 共识平面：治理提案绑定 `manifest_hash`，由 epoch 快照内验证者从外部提交 finality certificate。
  - 执行平面：节点按 `active_manifest_hash` 拉取 artifact，做签名与身份校验后加载执行。
  - 兼容平面：现有 `ModuleRelease*` 流程保留为迁移接口，但必须写入 release manifest 映射并逐步切换到统一发布证据面。
- Integration Points:
  - `crates/oasis7/src/runtime/world/governance.rs`
  - `crates/oasis7/src/runtime/world/base_layer.rs`
  - `crates/oasis7/src/runtime/world/module_actions_impl_part1.rs`
  - `crates/oasis7/src/runtime/world/module_actions_impl_part2.rs`
  - `crates/oasis7/src/runtime/events.rs`
  - `crates/oasis7/src/runtime/state.rs`
  - `crates/oasis7/src/runtime/world/release_manifest.rs`
  - `crates/oasis7/src/runtime/builtin_wasm_identity_manifest.rs`
  - `crates/oasis7/src/runtime/{m1,m4,m5}_builtin_wasm_artifact.rs`
  - `crates/oasis7/src/runtime/builtin_wasm_materializer.rs`
  - `crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs`
  - `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md`
  - `doc/p2p/prd.md`（PoS 与 committed execution 当前合同）
- Edge Cases & Error Handling:
  - 线上 manifest 不可达：节点保持上一版 `active_manifest_hash`，拒绝切换到未验证新版本。
  - 证书阈值不足或签名重复：`GovernanceFinalityInvalid` 并阻断 apply。
  - 证书 signer 不在当前 epoch 快照验证者集合：`GovernanceFinalityInvalid` 并输出 signer/epoch 诊断信息。
  - 证书 `validator_set_hash`/`stake_root` 与当前 epoch 快照不一致：拒绝 apply，并输出快照期望值与证书值。
  - 证书 `threshold_bps`/`min_unique_signers` 与当前 epoch 快照不一致：拒绝 apply，并保留阈值错配审计信息。
  - 证书 `epoch_id` 与本地信任根快照不一致：拒绝 apply，并记录快照版本冲突审计事件。
  - key 轮换窗口中旧证书：在 grace period 内可验证，窗口结束后统一拒绝。
  - 复构建节点对同平台 hash 结论不一致：标记为 release fault，禁止产生活跃清单。
  - 同 `signer_node_id + platform` 的复构建证明出现不同 hash/cid：拒绝冲突写入并保留首条证明作为审计锚点。
  - attestation signer 不在当前 epoch 快照 signer 集：该证明不计入阈值聚合，若聚合不足则拒绝 `ModuleReleaseApply`。
  - 证据只停留在 CI artifact、无法形成 node-side `proof_cid`：不得视为正式发布证明，必须补 proof payload 打包与 attestation 提交。
  - 清单回滚：仅允许通过治理撤销事件推进，不允许节点本地手工回滚。
  - 生产节点误开本地 fallback：启动即告警并拒绝进入 `enforce` 状态。
  - 生产路径误调用 `apply_proposal()`：立即拒绝并输出“本地自签路径禁用”错误，禁止 silent fallback。
  - 生产路径沿用 legacy `Install/Upgrade/Rollback/ModuleReleaseApply`（未携带外部证书）必须拒绝；仅 `*WithFinality` 动作变体允许进入 apply。
  - `ModuleRelease*` 与 release manifest 映射缺失：拒绝激活并要求补齐映射证据。
  - 故障签名标准化：线上不可达/缺失回滚/identity 漂移分别输出 `builtin_release_manifest_unreachable`、`builtin_release_manifest_missing_or_rolled_back`、`builtin_release_manifest_identity_drift`，用于 runbook 与告警分诊。
  - 运行手册锚点：`testing-manual.md` 的 `S11` 必须保持与上述三类阻断信号同口径，作为值守分诊与验收入口。
  - 主 CI 误触发非 `--check` builtin manifest 写入：必须被脚本门禁拒绝（CI write-disabled），避免 CI 成为生产发布写入路径。
- Non-Functional Requirements:
  - NFR-OMR-1: 节点模块校验（manifest + identity + signature）单模块验证耗时 `p95 <= 200ms`（本地缓存命中）。
  - NFR-OMR-6: node-side attestation submit API 必须返回结构化成功/失败响应，并在无效请求、payload 编码失败、共识队列提交失败三类路径上给出可分诊错误码。
  - NFR-OMR-2: 发布证据（清单、证明签名、证书、链上高度）可追溯完整率 100%。
  - NFR-OMR-3: 信任根变更具备版本号与生效高度，所有节点在 `<= 2` 治理 epoch 内收敛。
  - NFR-OMR-4: 生产环境下未签名/伪签名模块误接纳率 0。
  - NFR-OMR-5: 生产发布路径在 CI 不可用时仍可完成，不降低阈值签名与审计约束。
  - NFR-OMR-6: `stake/epoch` finality 校验单证书耗时 `p95 <= 50ms`（100 signer 快照，热缓存命中）。
  - NFR-OMR-7: 提供固定基准入口与产物（命令、日志、报告路径），可重复验证“校验耗时”与“2 epoch 收敛”。
- Security & Privacy:
  - 强制最小权限：发布节点仅持有自身签名密钥，不共享其他验证者私钥或阈值签名分片。
  - 签名私钥不得进入仓库或构建日志；日志仅记录 signer ID 与签名摘要。
  - 节点仅信任治理下发公钥集合，未知 signer 一律拒绝。
  - 本地 deterministic seed finality signer 仅允许开发/测试环境；生产环境必须禁用。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 引入线上发布清单与外部 finality 证书强制校验，生产禁用本地自签 apply。
  - v1.1: 完成 artifact_identity 生产强制签名化与 `identity_hash_v1` 兼容窗口下线。
  - v2.0: 完成 builtin 清单主真源去仓库化、信任根轮换自动化与全链路审计报表。
- Technical Risks:
  - 风险-1: 历史清单兼容窗口过短可能影响存量节点升级。
  - 风险-2: signer 运营流程不完善会导致发布延迟。
  - 风险-3: 多节点证明与签名聚合流程复杂度上升，需补足运行手册与告警。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-016 | TASK-WORLD_RUNTIME-017/018/019/027 | `test_tier_required` | 运行时仅接受线上 manifest + 签名身份用例；`ModuleRelease* -> release manifest` 映射回放一致性校验 | 模块加载与执行合法性 |
| PRD-WORLD_RUNTIME-017 | TASK-WORLD_RUNTIME-020/021/025/026 | `test_tier_required` + `test_tier_full` | finality 证书外部化、`stake/epoch` 阈值验签、key 轮换验证；本地自签路径禁用测试 | 治理 apply 安全边界 |
| PRD-WORLD_RUNTIME-018 | TASK-WORLD_RUNTIME-022/023/024/028/029 | `test_tier_required` | 去中心化提案/复构建证明/阈值签名闭环校验 + 发布 runbook/告警策略 + 节点侧固定验收脚本（`scripts/module-release-node-acceptance.sh`）+ finality 固定基准脚本（`scripts/oasis7-runtime-finality-baseline.sh`）+ 主 CI 去发布写入 + 基准指标产物校验 | 发布门禁与工程流程 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-OMR-001 | 线上签名清单作为合法性锚点 | 继续以仓库内置清单为主真源 | 仓库基线无法覆盖线上动态发布与跨节点一致执行需求。 |
| DEC-OMR-002 | 生产禁用 `identity_hash_v1` 回退 | 长期保留回退作为默认路径 | 回退路径无法提供可追责签名保证。 |
| DEC-OMR-003 | 外部 finality 证书强制化 | 运行时本地派生证书自动 apply | 本地可伪造路径与分布式治理目标冲突。 |
| DEC-OMR-004 | 生产发布完全去 CI 依赖 | 由 CI bot 充当唯一发布入口 | 去中心化运行目标要求生产发布在无中心服务下仍可达成。 |
