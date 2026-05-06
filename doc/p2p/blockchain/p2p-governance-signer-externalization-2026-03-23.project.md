# oasis7 治理 signer 外部化与轮换门禁（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`

审计轮次: 2
## 任务拆解（含 PRD-ID 映射）
- [x] GOVSIGN-0 (PRD-P2P-GOVSIGN-001/002/003/004) [test_tier_required]: 新建治理 signer 外部化专题 PRD / design / project，并接入 `doc/p2p` 与 readiness 主追踪。
- [x] GOVSIGN-1 (PRD-P2P-GOVSIGN-001/002) [test_tier_required]: 盘点 finality/controller signer 当前 local seed/config 真值，冻结环境等级与 blocker。
- [x] GOVSIGN-2 (PRD-P2P-GOVSIGN-002) [test_tier_required]: 冻结两类治理 signer 的长期 source-of-truth、update authority 与禁止项。
- [x] GOVSIGN-3 (PRD-P2P-GOVSIGN-003) [test_tier_required]: 冻结 failover、rotation、revocation 与 operator ownership gate。
- [x] GOVSIGN-4 (PRD-P2P-GOVSIGN-004) [test_tier_required]: 冻结 readiness/public-claims/ceremony 对 governance signer 的前置依赖。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - `MAINNET-2`: completed as specification gate
- 选定方案:
  - governance truth target: `on-chain/world-state registry`
- 当前 blocker:
  - 默认 execution world `output/chain-runtime/viewer-live-node/reward-runtime-execution-world` 已导入 `governance.finality.v1` 与 8 个 controller slot 的 world-state registry；`chain runtime` 现在已支持在启动/恢复时优先读取该 world registry 来覆盖 validator membership / signer binding 与 controller signer policy，但这仍不等于 rotation / revocation / ceremony / QA gate 全部通过
  - finality signer 的 production signing material 仍由人工离线 custody 持有；runtime 不再把 local seed 视为 registry 存在时的真值，且默认 world 首轮真实 finality drill 已完成，但更大范围 rotation / revocation / ceremony / QA gate 仍未收口
  - controller signer policy 虽已支持由 execution world 注入 `NodeRuntime`，但真实 governance account / recipient binding、genesis ceremony 和最终 QA `pass` 仍未完成

## Transition Freeze Snapshot（public-only）
- batch id: `oasis7-governance-batch-20260323-01`
- producer decision: finality 与 treasury/controller 主槽位继续默认 `threshold_ed25519 2-of-3`；低权限运营槽位允许在 manifest 中显式声明更低 threshold，当前唯一批准特例是 restricted grant `liveops` slot 可用 `1-of-2`
- current finality signer freeze:
  - `governance.finality.v1`
  - allowed_public_keys:
    - `54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc`
    - `38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4`
    - `e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8`
- current controller signer freeze:
  - `msig.genesis.v1` 与 7 个 treasury/controller slot 的 public-only signer set 见 `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md` §3A
- note:
  - 上述信息只构成 transition ceremony snapshot，不构成最终 on-chain/world-state registry 完成态

## Execution Workstream Snapshot（2026-03-23）
- [x] 在 `WorldState` 持久化 `governance_finality_signer_registry` 与 `governance_main_token_controller_registry`
- [x] `governance_effective_finality_epoch_snapshot` 在 registry 存在时优先使用 world-state signer truth，而不是 deterministic local seed fallback
- 已完成补充：`oasis7_chain_runtime` 在 execution world 存在 `governance_finality_signer_registry` 时，会用该 world-state registry 覆盖 `NodePosConfig` 的 validator membership / signer binding，并让 replication remote writer allowlist 与 reward runtime node identity binding 继续跟随 effective config
- [x] chain runtime 启动时可从 execution world 读取 controller signer policy，并覆盖 `NodeConfig.main_token_controller_binding`
- [x] 新增 `oasis7_governance_registry_import`，可把 operator-local `public_manifest.json` 导入 execution world
- [x] 新增 `oasis7_governance_registry_audit`，可直接读取 world-state registry，输出 slot threshold / signer count / tolerated failures / manifest match 审计结果
- [x] 已用真实 `public_manifest.json` 在临时 world 目录完成 smoke import，验证 3 个 finality signer + 8 个 controller slot 可落入 world-state registry
- [x] 已将默认 world 目录 `output/chain-runtime/viewer-live-node/reward-runtime-execution-world` 导入为 world-state registry 真值
- [x] rotation / revocation / failover 的 operator/QA 执行命令链已固化到本 project 文档
- [x] 已对默认 world 目录执行 `oasis7_governance_registry_audit`，当前结果为 `overall_status=ready_for_ops_drill`
- [x] 已新增 `./scripts/governance-registry-drill.sh`，可在 clone-world 上自动产出 baseline/pass/block 三类审计产物与 `summary.json`
- [x] 已在 clone-world 对 `msig.foundation_ops.v1` 完成首轮 `pass + block` drill，证据见 `doc/testing/evidence/governance-registry-clone-world-drill-foundation-ops-2026-03-24.md`
- [x] 已在 default/live execution world 对 `msig.foundation_ops.v1` 完成首轮真实 `pass + block + restore` drill，证据见 `doc/testing/evidence/governance-registry-live-world-drill-foundation-ops-2026-03-24.md`
- [x] 已在 clone-world 对 `governance.finality.v1` 完成 `signer03 -> signer04` 的 `pass + block` drill，确认 finality rotation 需要新的 signer node id
- [x] 已在 default/live execution world 对 `governance.finality.v1` 完成首轮真实 `pass + block + restore` drill，证据见 `doc/testing/evidence/governance-registry-live-world-drill-finality-2026-03-24.md`
- [x] 已确认 finality slot 不接受“同 signer_id 换公钥”语义；真实错误签名见 `output/governance-drills/20260324-finality-live-world/logs/pass_import.stderr`
- [x] 已补 additional finality revocation coverage：`signer02 -> signer05` 在 clone-world 与 default/live execution world 均完成 `pass + block(+restore)`，证据见 `doc/testing/evidence/governance-registry-live-world-drill-finality-revocation-signer02-2026-03-24.md`
- [x] 已补 finality multi-signer loss / rejoin coverage：`signer01 + signer02` dual-loss 在 clone-world 与 default/live execution world 均被 `import_policy_reject` 拦截，restore 后 world 回到 baseline，证据见 `doc/testing/evidence/governance-registry-live-world-drill-finality-multi-loss-rejoin-2026-03-24.md`
- [x] 已补 finality non-baseline rejoin coverage：`signer02` 移除后先进入 `2-of-2 -> failover_blocked`，再通过 `signer05` rejoin 回到 `ready_for_ops_drill`，证据见 `doc/testing/evidence/governance-registry-live-world-drill-finality-rejoin-signer02-2026-03-24.md`
- [x] 已补 finality baseline rejoin coverage：`signer02` temporary offline 后直接用 baseline manifest rejoin 回到 `ready_for_ops_drill`，证据见 `doc/testing/evidence/governance-registry-live-world-drill-finality-baseline-rejoin-signer02-2026-03-24.md`
- [ ] 其余 controller slot / additional finality failover / rejoin 变体覆盖仍待继续扩展
- [ ] genesis address binding / ceremony / QA pass 仍待后续 `MAINNET-3` 收口

## Operator / QA Runbook（How-to）
1. 先审计当前 world-state registry，确认所有治理 slot 都符合 manifest 声明的 threshold；若 manifest 未显式声明，则继续按默认 `2-of-3` 且具备单 signer 故障容忍：
   - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`
2. 若要做 rotation，不直接降低 threshold，也不允许先删 signer 再观望：
   - 先在离线 custody 侧生成 replacement signer
   - controller slot 可保持原 `signer_id` 并仅替换公钥；`governance.finality.v1` 不行，必须同时换成新的 signer node id（例如 `signer03 -> signer04`）
   - 形成新的 public-only manifest；默认 slot 继续保持 `threshold=2` 与 `3` 把有效公钥，像 `liveops` 这样的低权限特例槽位则必须在 manifest 中显式写出自己的 threshold
   - 用 `oasis7_governance_registry_import` 把新 manifest 导回 target world
   - 再次执行 `oasis7_governance_registry_audit`，只有 `overall_status=ready_for_ops_drill` 才允许重启/切流
3. 若要做 revocation，按该 slot 自身 threshold 与 signer_count 是否还能维持单 signer 故障容忍分两类处理：
   - 单 signer compromise / 离岗：必须在同一次导入里完成“替换 compromised signer -> 保持该 slot 的目标 threshold/单 signer 故障容忍 -> 审计通过”
   - 两把及以上 signer 同时不可用：若导出的 finality registry 让 `signer_count < threshold`，会在 import 阶段直接命中 `GovernancePolicyInvalid`；若还能写入但失去单 signer 容忍，则在 audit 阶段记为 `failover_blocked`
   - 无论是 `import_policy_reject` 还是 `failover_blocked`，都不得宣称 governance gate 通过；需要 producer/runtime/QA 联合阻断并进入事故处理
4. failover QA 不做口头判断，只看审计结果：
   - 任一 slot `tolerated_failures=0` 或 threshold 不符，记为 `block`
   - 只有 finality slot 与全部 controller slot 都满足 `single_failure_tolerant=true`，才能记录为“可进入真实演练”
5. 证据回写最少包含：
   - 审计前 JSON
   - 新 manifest 的 batch id / slot 范围 / 公钥摘要
   - 导入后 JSON
   - QA 结论：`pass` / `block`
   - 若失败，明确失败签名属于 `threshold_mismatch`、`manifest_mismatch` 或 `single_failure_blocks_slot`
6. clone-world 首轮演练可直接复用脚本：
   - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id msig.foundation_ops.v1 --replace-signer-id signer03 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/<run_id>`
   - 若 target slot 是 `governance.finality.v1`，必须额外传 `--replacement-signer-id <new_signer_id>`，不能复用原 signer id
   - 该脚本只用于 clone-world / dry-run 证据，不替代 default/live execution world 的最终 QA 证据
7. 若要在默认 execution world 留正式 QA 证据，可直接复用 live-world 脚本：
   - `./scripts/governance-registry-live-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/<run_id>`
   - 若 temporary offline 场景需要同 signer 回归，可改用 `--pass-manifest-mode baseline`
   - 对 multi-signer loss，可额外重复传 `--block-remove-signer-id`
   - 若 block case 仍可导入，脚本会自动继续执行 `rejoin import/audit`
   - 该脚本会自动执行 backup、baseline audit、pass import/audit、block import/audit、可选 rejoin import/audit、restore import/audit，并产出 `summary.json/md`

## 依赖
- `crates/oasis7/src/runtime/world/governance.rs`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/node_runtime_core.rs`
- `crates/oasis7/src/consensus_action_payload.rs`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `testing-manual.md`

## 验收命令（本轮）
- `rg -n "deterministic local seed|controller_signer_policies|NodeConfig|externalized|failover|revocation" doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.design.md doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.project.md doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md doc/p2p/project.md`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 governance_finality_registry_roundtrip_persists_and_drives_epoch_snapshot -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime governance_registry -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 world_registry_overrides_node_controller_binding -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 import_writes_governance_registries_into_world -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 audit_report_passes_for_matching_two_of_three_registry -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 audit_report_blocks_single_failure_when_threshold_equals_signer_count -- --nocapture`
- `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir <target-world-dir> --public-manifest <operator-local-public-manifest.json>`
- `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir <target-world-dir> --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 执行状态: in_progress
- 下一步: 将真实 drill 从 `msig.foundation_ops.v1` 与当前 finality single-signer / failover / rejoin 样本继续扩到更多 controller slot；finality 侧下一步更适合转去 shared network / release train 或更复杂网络抖动场景，而不是继续堆同类单槽位样本。
- 最近更新: 2026-03-24（finality baseline rejoin signer02）
