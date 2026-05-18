# p2p PRD

审计轮次: 13

## 目标
- 建立 p2p 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 p2p 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 p2p 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/p2p/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/p2p/prd.md`
- 项目管理入口: `doc/p2p/project.md`
- 文件级索引: `doc/p2p/prd.index.md`
- 追踪主键: `PRD-P2P-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 网络、共识、DistFS 与节点激励相关设计迭代频繁，缺少统一 PRD 导致跨子系统改动难以同时满足可用性、安全性与可审计性。
- Proposed Solution: 以 p2p PRD 统一定义分布式系统的目标拓扑、共识约束、存储策略、奖励机制与发布门禁。
- Success Criteria:
  - SC-1: P2P 关键改动 100% 映射到 PRD-P2P-ID。
  - SC-2: 多节点在线长跑套件按计划执行并形成可追溯结果。
  - SC-3: 共识与存储链路关键失败模式具备回归测试覆盖。
  - SC-4: 发行前完成网络/共识/DistFS 三线联合验收。
  - SC-5: 移动端轻客户端路径可在不运行本地权威模拟器前提下稳定接入。
  - SC-6: PoS slot/epoch 在多节点间由统一时间公式驱动，允许漏槽但不出现时间语义倒退。
  - SC-7: PoS 支持槽内 logical tick 相位门控与动态节拍调度，实现可配置 `tick/slot` 语义。
  - SC-8: `oasis7_chain_runtime/oasis7_game_launcher/oasis7_web_launcher/oasis7_client_launcher/scripts` 控制面参数与状态口径与 PoS 时间锚定语义一致，不再将 `node_tick_ms` 误解为出块时间。
  - SC-9: 清理残留时序语义偏差（`tick_count` 观测命名、`oasis7_viewer_live` 旧控制面假设、`world-rule` 时间模型描述），保证规范/实现/运维口径一致。
  - SC-10: runtime/game/web/client launcher 默认 PoS 时间参数与文档一致，默认启动即满足“slot 时钟锚定 + 轮询语义解耦”口径。
  - SC-11: runtime/game/web/client launcher 与 longrun 脚本默认参数统一为 `slot_duration_ms=12000`、`ticks_per_slot=10`、`proposal_tick_phase=9`，满足“12s 出块、每块 10 tick”基线。
  - SC-12: `oasis7_viewer_live` 对外 CLI 收敛为纯观察服务，不再接受 `--release-config` 与 `--node-*` 控制面参数；误传时必须显式拒绝并提示改用 `oasis7_chain_runtime`。
  - SC-13: `oasis7_viewer_live` 移除 legacy 参数兼容层，不再接受 `--runtime-world` 等历史别名；代码库中不再保留未接入生产入口的旧 CLI 解析路径。
  - SC-14: 历史 PRD/project 文档中的 `oasis7_viewer_live` 旧文件路径完成替换，不再指向已删除的 `src/bin/oasis7_viewer_live/` 子目录文件。
  - SC-15: 主链 Token 创世分配与早期贡献奖励口径具备可审计分桶、低流通边界、单人直持上限与贡献制发放约束，能够直接映射到现有 runtime 创世/金库机制。
  - SC-16: hosted world 网页远程接入具备明确的 `public player plane / private control plane / signer plane` 分层、`guest/player/strong-auth` 授权梯度、公开 join admission control 与 `gui-agent` surface split 策略，且浏览器不再被视为可持有 host 节点长期私钥的受信环境。
  - SC-17: p2p 模块具备一份 public-chain-grade 的“非全公网依赖”覆盖网络目标态，明确 `public/hybrid/private/relay_only/validator_hidden` 多部署模式、`validator core/sentry/relay` 角色分离，以及 `peer record + discovery + reachability + traffic lanes` 的统一框架边界。
  - SC-18: 当前链上代币的正式产品命名、runtime `main_token.symbol` / ticker 与公钥派生账户前缀已统一迁移到“绿洲币 / Oasis Coin” / `OC` / `oc:pk:`；对外 API、viewer/client、脚本与测试不得再把 `AWT` / `awt:pk:` 当作现行真值。
  - SC-19: 当前本机 + 2 ECS real-env triad 必须具备一条可审计的“三节点等权 validator”落地路径，明确 validator set、signer binding、static bootstrap、same-window snapshot evidence 与 residual legacy service naming 的边界；当 execution world 已存在 `governance_finality_signer_registry` 时，节点启动/恢复必须优先从该 world-state registry 恢复 validator membership 与 signer binding，而不是继续把 role-separated `observer + sequencer + storage` 或 operator-local env 当成唯一真值拓扑。
  - SC-20: `oasis7` 可以通过独立部署的 bridge-service，把已确认的 `OC` 充值映射为 LetAI Run OpenAPI 的用户额度与动态项目 `token_key`，同时冻结“只支持 one-way service-credit bridge，不是公开兑换所、不是 AMM、也不支持自动提现回 OC”的对外口径。
  - SC-21: `hosted_public_join` 必须具备一条对普通玩家友好的正式身份路径：默认允许邮箱 hosted login、托管 player signer 与后续自托管升级；浏览器不再被要求作为长期玩家私钥保管面。
  - SC-22: validator / finality signer 必须具备一条正式的治理准入流程，至少覆盖 `apply -> approved_candidate -> probation_ready -> active -> rotate/revoke`，并明确 world-state registry 才是正式激活真值；`--node-validator*` 与 operator-local env 只能作为 bootstrap 或显式运维覆盖。
  - SC-23: p2p 模块必须具备一套正式的公共主链式网络分层机制，明确 `local_devnet -> shared_devnet -> public_testnet -> mainnet` 的 tier 边界、manifest 真值与 claims/promotion 规则，避免把 shared release-train 与正式 testnet/mainnet 继续混写。
  - SC-24: `public_testnet` 必须具备一条 repo-owned readiness review 路径，能把“只有 skeleton manifest”与“具备 live candidate evidence”明确区分，避免把 placeholder endpoint 或模板证据误报为可部署结论。
  - SC-25: `public_testnet` 必须具备一份 repo-owned live-candidate companion checklist，把 seven-lane readiness gate、最小 evidence、canonical 命令与允许/禁止 claims 固定成单一执行入口，避免 producer/liveops/QA 对“还差哪些”各说各话。

## 2. User Experience & Functionality
- User Personas:
  - 协议工程师：需要明确网络与共识边界。
  - 节点运营者：需要稳定部署和可观测运行信号。
  - 安全评审者：需要签名、治理、资产流转的可审计证据。
  - 移动端玩家：需要低算力设备可持续在线并获得正确最终性反馈。
  - 制作人与金库治理维护者：需要在创世前冻结主链 Token 分配结构，避免过早流通、单人过度控盘或“玩就发币”的错误口径。
  - hosted world host / operator：需要把可公开分享的 join URL 与私有世界控制面拆开，避免分享试玩地址时连控制权一起暴露。
  - hosted world 远程玩家：需要通过网页先建立 session 再游玩，而不是直接继承 host 节点 signer。
  - 私网 / 家宽 / 企业内网节点运营者：需要在没有公网 IP 的前提下，仍能以正式角色加入网络、同步状态或通过 sentry/relay 参与主链。
  - validator 候选运营者：需要明确提交什么材料、何时从候选转为 active validator，以及哪些 signer/控制权限并不对外开放。
- User Scenarios & Frequency:
  - 协议演进评审：每次共识或网络协议改动前执行。
  - 多节点长跑：按周执行并记录稳定性与恢复结果。
  - 发行前联合验收：每个候选版本执行一次三线联测。
  - 安全审计复核：关键资产链路改动后立即触发。
  - 轻客户端接入验收：每次移动端协议调整后执行输入/最终性/重连验证。
  - 创世发行前评审：每次准备冻结 Token 创世配置或早期奖励口径时执行一次。
  - hosted world 架构复核：每次准备让“玩家部署服务给其他玩家通过网页进入”进入公开测试前执行一次。
- User Stories:
  - PRD-P2P-001: As a 协议工程师, I want explicit protocol boundaries, so that multi-crate changes remain coherent.
  - PRD-P2P-002: As a 节点运营者, I want reliable longrun validation, so that production confidence increases.
  - PRD-P2P-003: As a 安全评审者, I want auditable cryptographic and governance flows, so that risk is controlled.
  - PRD-P2P-004: As a 移动端玩家, I want intent-only light client access, so that low-end devices can still participate fairly.
  - PRD-P2P-005: As a 协议工程师, I want slot/epoch to be wall-clock driven, so that block time semantics remain stable across restart and lag.
  - PRD-P2P-006: As a 协议工程师, I want slot-internal tick-phase pacing, so that proposal cadence can follow configured `ticks_per_slot`.
  - PRD-P2P-007: As a 节点运营者, I want runtime/launcher/scripts to expose anchored slot-clock parameters explicitly, so that block-time tuning is deterministic and auditable.
  - PRD-P2P-008: As a 协议工程师, I want cross-doc and status field naming to disambiguate worker polling vs consensus ticks, so that observability and operations avoid semantic drift.
  - PRD-P2P-009: As a 节点运营者, I want sane default PoS timing values and uniform validation wording, so that default startup already follows anchored block-time semantics without hidden overrides.
  - PRD-P2P-010: As a 发布维护者, I want `oasis7_viewer_live` to reject legacy release/node control-plane flags, so that chain control is unambiguously hosted by `oasis7_chain_runtime`.
  - PRD-P2P-011: As a 发布维护者, I want legacy compatibility aliases removed from `oasis7_viewer_live`, so that CLI semantics are single-source and there is no dead parser path.
  - PRD-P2P-012: As a 维护者, I want historical docs to reference current source layout, so that reviewers do not chase deleted paths during audit or regression.
  - PRD-P2P-013: As a producer_system_designer, I want one frozen genesis allocation and early-contribution reward policy, so that oasis7 can issue token with low circulation, auditable control boundaries and no accidental play-to-earn framing.
  - PRD-P2P-014: As a producer_system_designer, I want one explicit cryptographic security baseline verdict, so that oasis7 does not overclaim “mainstream public-chain-grade security” before transaction authorization, signer custody and genesis control are actually ready.
  - PRD-P2P-015: As a `runtime_engineer`, I want public main-token transfer submit to require signed account authorization, so that the first exposed asset surface no longer trusts unsigned `from_account_id` fields.
  - PRD-P2P-016: As a producer_system_designer, I want the remaining post-STRAUTH signer custody, governance signer and genesis ceremony blockers turned into formal readiness gates, so that oasis7 can truthfully stay in preview while still having one executable path toward a later mainnet-grade re-evaluation.
  - PRD-P2P-017: As a producer_system_designer, I want one explicit production signer custody / keystore baseline, so that preview bootstrap signers are clearly separated from production signer ownership and release policy.
  - PRD-P2P-018: As a producer_system_designer, I want one explicit governance signer externalization baseline, so that deterministic local seed and local config signer truth are clearly separated from long-term production governance truth.
  - PRD-P2P-019: As a producer_system_designer, I want one explicit genesis freeze/ceremony/QA gate, so that logic-frozen but still-unbound genesis parameters cannot be mistaken for mint readiness.
  - PRD-P2P-020: As a producer_system_designer, I want one final public claims policy re-evaluation after MAINNET readiness planning, so that outward language stays aligned with execution reality rather than spec completeness.
  - PRD-P2P-021: As a producer_system_designer, I want one explicit benchmark against mainstream public-chain testing systems, so that oasis7 testing maturity is judged by layered evidence rather than isolated green checks.
  - PRD-P2P-022: As a producer_system_designer, I want one explicit shared network / release train minimum model, so that oasis7 can turn `L5` from a known gap into an executable workstream without overclaiming it is already in place.
  - PRD-P2P-023: As a producer_system_designer, I want one explicit hosted-world player access and session-auth model, so that one player部署服务给另一个玩家通过网页进入时，不会再把 host control-plane、shared `gui-agent` control surface 与 node signer 暴露给浏览器。
  - PRD-P2P-024: As a producer_system_designer, I want one public-chain-grade private-reachability P2P architecture, so that oasis7 不再把“所有正式节点都要有公网 IP”当成默认前提，并能在 mixed-topology 现实下继续对标公共主链。
  - PRD-P2P-025: As a producer_system_designer, I want one canonical triad observability stack, so that 当前 real-env triad（物理上为本机 + 2 ECS，runtime 上已收口为 three_equal_validator，历史 service label 仅作兼容别名）的真实运行状态可以在同一轮监控里同时回答资源、链状态、流量、WASM 健康，并进一步定位到具体 runtime 子模块和优化热点。
  - PRD-P2P-026: As a producer_system_designer, I want the live triad to support a three-equal-validator topology, so that the local node is no longer a permanent observer exception and triad semantics can match “three peer-equal validators” when operations explicitly choose that mode.
  - PRD-P2P-027: As a producer_system_designer, I want one canonical one-way `OC -> LetAI Run OpenAPI quota/token_key` bridge model, so that oasis7 可以把当前主链 Token 用作受控的 AI 服务额度充值资产，同时不误滑成公开兑换所、浏览器热钱包或双向提现承诺。
  - PRD-P2P-028: As a producer_system_designer, I want one formal public-chain-style network-tier mechanism, so that oasis7 can stop treating `shared_devnet`、`public_testnet` and `mainnet` as informal aliases and instead promote networks through explicit manifest + gate truth.
  - PRD-P2P-029: As a producer_system_designer, I want one explicit hosted-public-join managed identity / custody model, so that普通玩家可以用邮箱登录游戏，而不是被迫管理裸公私钥，同时 hosted player signer、step-up auth 与自托管升级路径都保持在可审计边界内。
- Critical User Flows:
  1. Flow-P2P-001: `网络拓扑变更 -> 共识联调 -> DistFS 同步 -> 节点状态一致性验证`
  2. Flow-P2P-002: `执行 S9/S10 长跑 -> 采集故障与恢复数据 -> 输出收敛报告`
  3. Flow-P2P-003: `资产/签名链路变更 -> 审计检查 -> 安全门禁 -> 发布判定`
  4. Flow-P2P-004: `手机端提交签名 intent -> 权威模拟执行 -> 链上承诺/挑战 -> 客户端 final 确认`
  5. Flow-P2P-005: `节点读取 wall-clock -> 计算 slot/epoch -> 允许漏槽推进 -> 拒绝未来槽/过旧槽提案`
  6. Flow-P2P-006: `节点按 wall-clock 计算 logical tick/phase -> 相位命中才提案 -> runtime 动态等待下一 tick 边界`
  7. Flow-P2P-007: `运维配置 slot_duration/ticks_per_slot/proposal_phase -> runtime/game/web/client launcher 统一生效 -> status/soak 输出可观测并用于门禁`
  8. Flow-P2P-008: 状态接口/手册/PRD 同步更新 -> `tick_count` 明确为 worker poll 指标 -> 采样脚本以共识 slot/tick/height 为主
  9. Flow-P2P-009: `默认启动 runtime/game/web/client launcher -> 使用统一默认 slot_duration/ticks_per_slot -> 文档/帮助/校验文案一致呈现 poll vs slot 语义`
  10. Flow-P2P-010: `用户误传 oasis7_viewer_live --release-config/--node-* -> CLI 显式拒绝并给出替代入口 -> 文档与示例迁移到 oasis7_chain_runtime`
  11. Flow-P2P-011: `用户误传 oasis7_viewer_live 任意 legacy 参数（含 --runtime-world） -> CLI 明确拒绝并输出迁移入口 -> 测试与手册口径一致`
  12. Flow-P2P-012: `执行历史文档巡检 -> 替换已删除源码路径到当前入口路径 -> 文档门禁 + grep 零残留校验`
  13. Flow-P2P-013: `制作人冻结创世分配表 -> runtime 映射创世桶与 vesting 参数 -> liveops/QA 审核早期贡献奖励边界 -> 创世配置与低流通门禁共同放行`
  14. Flow-P2P-014: `盘点签名/地址/交易授权/keystore/治理 signer/创世控制真值 -> 形成 red/yellow/green 矩阵 -> producer 输出总 verdict 与 P0 blocker`
  15. Flow-P2P-015: `客户端为 transfer submit 构造 canonical payload 并签名 -> runtime 验签并比对 oc:pk 账户绑定 -> 通过后才进入余额/nonce 预检与 consensus submit`
  16. Flow-P2P-016: `operator 运行 triad 完整监控 -> snapshot/host/traffic/wasm 产物全部落盘 -> merged summary 输出 overall status + node-level alerts -> evidence doc 引用 canonical 输出路径`
  17. Flow-P2P-017: `STRAUTH-3 收口后复盘剩余安全缺口 -> 冻结 MAINNET-1~4 readiness gate -> signer custody/governance signer/genesis ceremony 逐项过门禁 -> producer 再决定是否重评阶段`
  18. Flow-P2P-018: `盘点 node/viewer/governance signer 来源 -> 冻结 preview-only bootstrap 与 production target backend 边界 -> rotation/revocation/audit policy 入门禁`
  19. Flow-P2P-019: `盘点 finality/controller signer 真值 -> 冻结 externalized source-of-truth 与 operator ownership -> 候选 validator 提交 node identity/finality signer/public manifest -> candidate/probation 审核 -> activation 生效后再写入 world-state registry -> failover/rotation/revocation 进入治理门禁`
  20. Flow-P2P-020: `读取 genesis freeze sheet -> 绑定 slot/bucket 真值 -> 执行 ceremony checklist -> QA 审核 evidence bundle -> 决定是否允许 mint-ready 口径`
  21. Flow-P2P-021: `读取 MAINNET-1~3 当前状态 -> 判断哪些仅为 spec gate、哪些已 execution complete -> 冻结 claim allowlist/denylist 与未来升级条件`
  22. Flow-P2P-022: `读取 testing-manual 与安全/readiness 专题 -> 映射 oasis7 当前测试层 -> 对照主流公链 testing benchmark -> 冻结 gap matrix 与下一步验证优先级`
  23. Flow-P2P-023: `host 启动 hosted world -> public join 先过 admission control -> 远程访客建 guest/player session -> runtime 按 capability 绑定实体与动作 -> `gui-agent` 仅走 player-safe split surface -> 资产/治理类动作再升级 strong auth`
  24. Flow-P2P-024: `用户绑定 bridge 身份 -> bridge-service 分配唯一 deposit route -> 用户通过受信转账面支付 OC -> bridge watcher 等待确认并写入 bridge_ledger -> LetAI OpenAPI 执行 user upsert / project+token_key / topup / query verification -> operator 对账与异常收口`
  25. Flow-P2P-025: `访客从 guest 升级到邮箱 hosted account -> identity broker 恢复账户与 signer_ref -> session broker 签发 device/player session -> 高风险动作再经 step-up + custody sign -> 如需退出托管则走 external wallet bind / transfer-out`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 网络与共识协同 | 节点ID、轮次、提交高度、延迟 | 启动联测并比对共识结果 | `joining -> syncing -> committed` | 高度/轮次单调递增 | 仅授权节点参与共识 |
| DistFS 复制 | 文件ID、副本状态、同步延迟 | 触发复制并校验完整性 | `queued -> replicating -> verified` | 优先关键数据副本 | 节点需满足存储策略 |
| 长跑与恢复 | 失败类型、恢复动作、恢复时长 | 注入故障并执行恢复流程 | `stable -> degraded -> recovered` | 按故障等级排序处理 | 运维/评审可操作恢复流程 |
| 轻客户端权威状态 | `intent(tick/seq/sig)`、`state_root`、`finality_state` | 手机端只上报 intent，接收 delta/proof 并展示最终性 | `pending -> confirmed -> final` | 按 tick 排序，重复 seq 幂等去重 | 权威状态仅由模拟节点提交，客户端无写权限 |
| PoS 固定时间槽 | `genesis_unix_ms`、`slot_duration_ms`、`epoch_length_slots`、`last_observed_slot`、`missed_slot_count` | 每次 tick 按真实时间换算 slot；仅在 `next_slot <= current_slot` 时允许提案 | `pending -> committed/rejected`（槽位单调） | `current_slot=floor((now-genesis)/slot_duration)`；`epoch=slot/epoch_length_slots` | 仅验证者可提案/投票；未来槽消息拒绝 |
| PoS 槽内 tick 节拍 | `ticks_per_slot`、`tick_phase`、`proposal_tick_phase`、`last_observed_tick`、`missed_tick_count` | 仅在命中提案相位时触发提案；worker 按下一 logical tick 边界动态调度 | `idle -> proposing`（相位门控） | `logical_tick=floor((now-genesis)*ticks_per_slot/slot_duration)`；`phase=tick%ticks_per_slot` | 节拍公式全节点一致；本地调度可回退固定间隔 |
| PoS 控制面参数对齐 | `node_tick_ms`（轮询）+ `slot_duration_ms`、`ticks_per_slot`、`proposal_tick_phase`、`adaptive_tick_scheduler_enabled`、`slot_clock_genesis_unix_ms`、`max_past_slot_lag` | runtime/game/web/client launcher/scripts 显式暴露并校验参数，状态接口回显观测字段 | `configured -> running -> audited` | `node_tick_ms` 不参与出块时间计算，仅作为 worker 轮询/回退间隔 | 运维可配置；非法值必须启动前拒绝 |
| 时序语义残留收敛 | `worker_poll_count`、`consensus.last_observed_tick`、`consensus.committed_height`、`slot_duration_ms` | 更新状态字段命名/文档叙述并修复过时控制面假设 | `legacy -> aligned` | 轮询指标与共识指标分离，不混用同一“tick”语义 | 运维/QA 只读；配置前必须通过校验 |
| Viewer 控制面边界收敛 | `oasis7_viewer_live` CLI（`--bind`/`--web-bind`/`--llm`/`--no-llm`） | 仅保留观察服务参数；误传 `--release-config`、`--runtime-world`、`--node-*` 与其他 legacy 控制面参数直接拒绝 | `legacy_mixed -> observer_only_strict` | CLI 白名单固定；错误信息必须包含迁移目标 `oasis7_chain_runtime` | 运行链控制面仅限受信运维入口 |
| Token 创世分配与低流通 | `display_name_cn`、`display_name_en`、`symbol`、`account_prefix`、`initial_supply`、`allocation_bps`、`bucket_id`、`recipient`、`cliff_epochs`、`linear_unlock_epochs`、`founder_direct_cap_bps`、`circulation_cap_bps` | 冻结正式命名、创世分配表、创世绝对总量、设定 vesting 与金库控制边界、审计早期贡献奖励准入，并统一 runtime/account 派生真值 | `draft -> named -> symbol_migrated -> supply_frozen -> auditable` | 正式产品名固定为“绿洲币 / Oasis Coin”；当前 `symbol=OC`、`account_prefix=oc:pk:`；当前 `initial_supply=10,000,000,000 OC`；分配总和必须 `10000 bps`；项目战略控制目标 `5000 bps`；协议奖励池 `3500 bps`；单人直持硬上限 `1500 bps` | 制作人定义口径；runtime 按创世配置落地；liveops/QA 仅可在已定义边界内执行与验收 |
| 密码学安全基线评估 | `primitive_status`、`transaction_auth_status`、`account_model_status`、`key_custody_status`、`governance_signer_status`、`genesis_control_status`、`overall_verdict` | 盘点代码/文档真值并输出 system-level verdict；若 blocker 未清零则拒绝高级安全口径 | `unknown -> inventoried -> verdict_frozen` | 只要资产动作缺统一签名交易模型，整体必须保持 `not_mainnet_grade` | `producer_system_designer` 拍板，`runtime_engineer`/`qa_engineer` 联审 |
| 主链 Token 签名交易鉴权 | `from_account_id/to_account_id/amount/nonce/public_key/signature` | runtime 先验签并校验 `oc:pk:` 账户绑定，再进入既有余额/nonce 预检与 consensus submit | `unsigned_surface -> transfer_signed_surface` | transfer submit 必须带固定版本签名；`from_account_id` 必须等于 `oc:pk:<public_key_hex>`；其他资产动作仍待后续专题 | `runtime_engineer` 牵头实现，`viewer_engineer`/`qa_engineer` 跟进客户端与回归 |
| 主流公链测试体系对标 | `layer_id/current_coverage/evidence_paths/gap_status/next_action` | 将 oasis7 suites/evidence 对位到主流公链测试分层，并冻结缺口矩阵与执行优先级 | `draft -> mapped -> prioritized` | 若缺共享网络、真实 drill 证据或 fuzz/property gate，则不得宣称“主流公链级测试成熟度” | `producer_system_designer` 拍板，`qa_engineer` 联审 |
| Validator / finality signer 治理准入 | `candidate_id/node_id/finality_signer_public_key/operator_owner/public_manifest/activation_epoch/admission_status` | 受理申请、审核 reachability/registry/failover 准入条件，并在 activation 生效后把候选节点写入正式 validator truth | `applied -> approved_candidate -> probation_ready -> active -> rotated/revoked` | 只有 world-state registry 生效后才算正式 validator；`--node-validator*` 与本地 env 改动不算长期 admission 完成态 | `producer_system_designer` 拍板，`runtime_engineer`/`qa_engineer` 联审 |
| Hosted world 玩家接入与 session auth | `deployment_mode/session_id/session_level/capability_set/control_plane_scope/strong_auth_state/admission_policy/player_safe_agent_surface` | 将网页远程玩家面、host 控制面与 signer plane 分层；签发 guest/player session，并对敏感动作要求 strong auth | `specified_not_implemented -> trusted_local_only -> hosted_ready` | 只要浏览器仍依赖 `node.private_key` bootstrap、可命中 host 控制面路由或未冻结 admission / `gui-agent` split，就不得判为 hosted-ready | `producer_system_designer` 拍板，`runtime_engineer`/`viewer_engineer`/`qa_engineer`/`liveops_community` 联审 |
| Hosted public join 托管身份与托管密钥 | `hosted_account_id/player_id/device_session_id/signer_ref/custody_mode/step_up_state/transfer_out_state` | 为公开 join 玩家提供邮箱登录、托管 signer、step-up auth 与自托管升级路径 | `guest_only -> account_verified -> managed_custody_active -> self_custody_bound/transferred_out` | 只要 hosted player 仍要求用户保存裸私钥、浏览器仍长期持有托管 signer 或 `main_token_transfer` 没有正式 custody lane，就不得宣称“任意新用户默认可安全登录并长期使用” | `producer_system_designer` 拍板，`runtime_engineer`/`viewer_engineer`/`qa_engineer`/`liveops_community` 联审 |
- 三线联合验收清单（TASK-P2P-002）:
| 线别 | 必跑命令（基线） | 联合验收门禁 | 阻断条件（任一命中即 fail） | 证据产物 |
| --- | --- | --- | --- | --- |
| 网络线（net） | `env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib`；`env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib` | `./scripts/release-gate.sh --dry-run` + S9 发布档位命令（见 `testing-manual.md`） | `oasis7_net` 单测失败；S9 `metric_gate.status != pass`；`consensus_hash_consistent != true` | `release-gate-summary.md`、S9 `summary.json/timeline.csv` |
| 共识线（consensus） | `env -u RUSTC_WRAPPER cargo test -p oasis7_consensus --lib`；`env -u RUSTC_WRAPPER cargo test -p oasis7_node --lib` | S9 + S10 发布档位命令（见 `testing-manual.md`） | 共识/节点单测失败；S9 或 S10 `overall_status/run.status != ok`；`consensus_hash_mismatch_count > 0` | S9/S10 `summary.json`、`failures.md`（若失败） |
| 存储线（DistFS） | `env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib` | S9 发布档位命令（含 `--max-distfs-failure-ratio 0.1`） | DistFS 单测失败；`distfs_failure_ratio` 超阈值；反馈/复制不一致无法闭环 | S9 `summary.json`、`feedback_events.log`、`chaos_events.log` |
- Acceptance Criteria:
  - AC-1: p2p PRD 覆盖网络、共识、存储、激励四条主线。
  - AC-2: p2p project 文档任务项明确映射 PRD-P2P-ID。
  - AC-3: 与 `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md` 等设计文档口径一致。
  - AC-4: S9/S10 相关测试套件在 testing 手册中有对应条目。
  - AC-5: 轻客户端专题需求落盘并映射到独立任务链（`TASK-P2P-MLC-*`）。
  - AC-6: `node-pos-slot-clock-real-time-2026-03-07` 专题文档落盘并映射任务链 `TASK-P2P-008`。
  - AC-7: `node-pos-subslot-tick-pacing-2026-03-07` 专题文档落盘并映射任务链 `TASK-P2P-009`。
  - AC-8: 三线联合验收清单明确给出“基线命令 + 发布门禁阈值 + 阻断条件 + 证据产物”，可直接用于发行前检查。
  - AC-9: S9/S10 长跑结果模板与缺陷闭环模板完成定义，失败运行必须能映射到 `incident_id -> 修复任务 -> 回归证据`。
  - AC-10: 发行门禁分布式质量指标（S9/S10）具备“阈值 + 数据源 + 阻断策略 + 责任归属”映射，并与 `release-gate` 脚本参数一致。
  - AC-11: `node-pos-time-anchor-control-plane-alignment-2026-03-07` 专题文档落盘并映射任务链 `TASK-P2P-010`，覆盖 runtime/game/web/client launcher/scripts 与状态接口口径对齐。
  - AC-12: 残留语义项完成收敛：`world-rule` 时间模型、launcher `chain_node_tick_ms` 校验文案、`/v1/chain/status` 轮询字段命名、viewer/manual/site 与 `oasis7_viewer_live` 实际 CLI 能力保持一致。
  - AC-13: `oasis7_chain_runtime/oasis7_game_launcher/oasis7_web_launcher/oasis7_client_launcher` 默认 `slot_duration_ms` 与文档基线一致；`oasis7_web_launcher` 校验文案明确 `chain_node_tick_ms` 为 poll interval 语义。
  - AC-14: `oasis7_chain_runtime/oasis7_game_launcher/oasis7_web_launcher/oasis7_client_launcher/oasis7_viewer_live/p2p-longrun/s10` 默认 `slot_duration_ms/ticks_per_slot/proposal_tick_phase` 与“12s/10/9”基线一致，相关默认值断言与手册同步更新。
  - AC-15: `oasis7_viewer_live` 解析层移除 `--release-config` 与 `--node-*` 参数能力；定向测试覆盖“误传 legacy 参数 -> 启动失败 + 替代提示”路径。
  - AC-16: `oasis7_viewer_live` 进一步移除 `--runtime-world` 兼容别名与旧 split CLI 路径，定向测试覆盖 `--release-config/--runtime-world/--node-*` 拒绝行为。
  - AC-17: 历史文档中 `oasis7_viewer_live` 子目录旧路径完成迁移（对齐 `oasis7_viewer_live.rs` 与 `oasis7_chain_runtime/*` 现行布局），文档门禁通过。
  - AC-18: `doc/p2p/**` 仍可读历史专题的首行标题必须统一使用 `oasis7 Runtime` 或 `oasis7` 品牌；旧 `oasis7*` 标题仅允许保留在正文历史上下文、证据原文与兼容说明中。
  - AC-19: `mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22` 专题文档落盘并映射任务链 `TASK-P2P-031`，明确 `10000 bps` 创世分配表、项目战略控制 `5000 bps`、协议奖励池 `3500 bps`、单人直持目标/上限、低流通边界与“贡献制奖励而非 P2E”口径。
  - AC-20: `p2p-mainnet-crypto-security-baseline-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-032`，明确当前整体 verdict 为 `not_mainnet_grade`，固定交易授权、keystore、治理 signer 与创世控制 blocker，并给出 mainnet-ready 路线图。
  - AC-21: `mainchain-token-signed-transaction-authorization-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-033`；`POST /v1/chain/transfer/submit` 必须新增 `public_key/signature` 鉴权、绑定 `oc:pk:<public_key_hex>` 并完成 required 回归。
  - AC-22: `p2p-mainnet-grade-readiness-hardening-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-034`，明确当前阶段只可称为 `limited playable technical preview` + `crypto-hardened preview`，并冻结 `MAINNET-1~4` readiness gate。
  - AC-23: `p2p-production-signer-custody-keystore-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-035`，明确 `config.toml` 明文 key、HTML 私钥注入与 env 私钥 bootstrap 只属于 preview-only signer path，不得作为 production custody 完成态。
  - AC-24: `p2p-governance-signer-externalization-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-036`，明确 governance registry 优先、deterministic local seed / `NodeConfig` local fallback 只属于 preview/local truth，不得作为 production governance truth，并冻结 validator / finality signer 的治理准入目标流程。
  - AC-25: `p2p-genesis-freeze-ceremony-qa-gate-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-037`，明确 `logic_frozen_address_binding_pending`、`TBD_BEFORE_MINT`、`pending_binding` 与 `ready_pending_address_binding` 都属于 mint-ready blocker。
  - AC-26: `p2p-mainnet-public-claims-policy-2026-03-23` 专题文档落盘并映射任务链 `TASK-P2P-038`，明确 `MAINNET-1~3` 当前仅完成 spec gate、整体 verdict 仍为 `not_mainnet_grade`，并冻结 allowlist/denylist 与 future upgrade conditions。
  - AC-27: `p2p-mainstream-public-chain-testing-benchmark-2026-03-24` 专题文档落盘并映射任务链 `TASK-P2P-039`，明确主流公链测试分层模型、oasis7 当前映射、`fuzz/property` 与 `shared network/release train` 缺口，以及真实 governance drill 证据的当前优先级。
  - AC-28: `p2p-shared-network-release-train-minimum-2026-03-24` 专题文档落盘并映射任务链 `TASK-P2P-040`，明确 `shared_devnet/staging/canary` 三层最小轨道、`release_candidate_bundle` 真值、promotion/freeze/rollback 规则、liveops runbook 入口与当前 `specified_not_executed` 结论。
  - AC-29: `p2p-hosted-world-player-access-and-session-auth-2026-03-25` 专题文档落盘并映射任务链 `TASK-P2P-041`，明确 hosted world 的 `public player plane / private control plane / signer plane`、`guest/player/strong-auth` 会话梯度、`gui-agent` surface split、public join admission control，以及“无需 invite-only 也不能把长期 signer 暴露给浏览器”的边界。
  - AC-30: `p2p-mainnet-private-reachability-architecture-2026-04-01` 专题文档落盘并映射任务链 `TASK-P2P-043`，明确 `public/hybrid/private/relay_only/validator_hidden` 部署模式、`validator core/sentry/relay/full-storage/observer-light` 角色边界、`peer record + discovery + reachability + traffic lanes` 框架，以及 mixed-topology 下的 anti-eclipse / relay budget / claims gate。
  - AC-31: `TASK-P2P-045` 必须把当前链上代币的正式产品名冻结为“绿洲币 / Oasis Coin”，作为后续 runtime 符号与账户派生迁移的前置口径。
  - AC-32: `TASK-P2P-046` 必须把当前链上代币的 runtime `main_token.symbol`、公钥派生账户前缀与签名鉴权前缀统一迁移到 `OC` / `oc:pk:`，并同步 API、viewer/client、liveops、脚本、测试与模块入口文档，不再把 `AWT` / `awt:pk:` 当作现行真值。
  - AC-33: `TASK-P2P-047` 必须把当前链上代币的创世 `initial_supply` 冻结为 `10,000,000,000 OC`，并把 7 个 bucket 的绝对分配额、首年外部释放绝对边界与 formal freeze sheet 的 supply gate 同步回写到 token 专题与模块执行台账。
  - AC-34: `triad-observability-stack` 必须把 real-env triad 的 host/process、chain status、traffic window、wasm window 收敛到统一 repo-owned 监控入口，并在 `testing-manual.md` 冻结 canonical 命令与产物路径。
  - AC-35: `triad-three-equal-validator-topology` 必须把当前 real-env triad 从“本机 observer + 两台云端 validator”提升为“三节点等权 validator”可审计基线，至少覆盖：`3` 个 validator 的 stake/signer binding、local 节点不再以 observer-only 角色运行、repo-owned snapshot/manual 不再把 `partial_with_observer_blocker` 当成唯一有效 claim、same-window evidence 对 legacy service label 与真实 runtime role 的区分，以及 `oasis7_chain_runtime` 在 execution world 已落盘 `governance_finality_signer_registry` 时会优先用该 world-state registry 恢复 validator membership / signer binding；`--node-validator*` 只保留为 bootstrap 或显式运维覆盖。
  - AC-36: `mainchain-token-newapi-quota-bridge-2026-05-06` 专题文档落盘并映射任务链，明确 `one-way OC -> LetAI Run OpenAPI quota`、bridge-service 独立部署、唯一入账映射、`bridge_ledger` 幂等对账、动态 project/`token_key`、query verification 与 manual review 风控，以及“不支持自动提现/不承诺公开兑换所”边界。
  - AC-37: `p2p-formal-network-tiers-testnet-mechanism-2026-05-14` 专题文档与 repo-owned skeleton 必须落盘并映射任务链 `formal-network-tiers-testnet-mechanism (PRD-P2P-028)`，明确 `local_devnet/shared_devnet/public_testnet/mainnet` 四层模型、`network_tier_manifest` 字段集合、`public_testnet` 的 public RPC/explorer/faucet/reset 语义，以及 `mainnet` 的 `no faucet + frozen reset + MAINNET-1~4` gate。
  - AC-38: `p2p-hosted-public-join-managed-identity-custody-2026-05-18` 专题文档必须落盘并映射任务链 `hosted-managed-identity-doc-freeze (PRD-P2P-029)`，明确 hosted account、邮箱登录、`signer_ref`、device session、step-up auth、托管退出与“默认不让玩家管理裸私钥”的正式产品边界。
  - AC-39: `public_testnet` 必须具备 repo-owned readiness review 入口，至少能基于 manifest + lane evidence 输出 `specified_skeleton_only|partial|block|ready_for_live_candidate`，并对 placeholder endpoint / 缺失 candidate bundle / 缺 lane evidence 保持阻断。
  - AC-40: `p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md` 必须作为 companion runbook 落盘并映射任务链 `formal-public-testnet-live-candidate-checklist (PRD-P2P-028)`，至少冻结 seven-lane owner/evidence/check 命令/claim boundary 与当前 `specified_skeleton_only` 边界。
- Non-Goals:
  - 不在本 PRD 细化 viewer UI 交互。
  - 不替代 runtime 内核的模块执行细节设计。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 长跑脚本、链路探针、反馈注入、共识日志分析工具。
- Evaluation Strategy: 以在线稳定时长、分叉恢复成功率、反馈链路可用性、错误收敛时间评估。

## 4. Technical Specifications
- Architecture Overview: p2p 模块负责 `oasis7_net`/`oasis7_consensus`/`oasis7_distfs` 与 node 侧分布式运行协同，强调一致性与故障恢复。
- Integration Points:
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
  - `doc/p2p/distributed/distributed-hard-split-phase7.prd.md`
  - `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md`
  - `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
  - `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`
  - `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
  - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
  - `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
  - `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
  - `world-rule.md`
  - `doc/world-simulator/viewer/viewer-manual.md`
  - `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`
  - `oasis7_viewer_live.release.example.toml`
  - `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md`
  - `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 节点掉线：共识链路需在节点恢复后自动重同步并验证状态。
  - 网络分区：检测分区后阻断不安全提交并等待合并恢复。
  - 轻客户端弱网：启用低频增量+关键帧同步并保持最终性状态不倒退。
  - 空副本：DistFS 副本不足时触发补副本任务并记录告警。
  - 超时：共识轮次超时后执行回退/重试策略。
  - 并发冲突：同高度多提交候选按共识规则拒绝冲突分支。
  - 数据损坏：校验失败副本立即隔离并重建。
  - 时钟回拨/漂移：wall-clock 出现回拨时禁止 slot 倒退；超阈值漂移进入拒绝或告警路径。
  - 大跨度漏槽：节点恢复后按当前 wall-clock 对齐 slot，并累加漏槽计数，不补历史空块。
  - 控制面兼容：保留 `node_tick_ms` 时必须明确其“轮询/回退间隔”语义，避免误用为 slot/block 时间。
  - 旧参数误用：`oasis7_viewer_live` 若收到 `--release-config` 或任意 `--node-*` 参数，必须立即失败并输出“请改用 oasis7_chain_runtime”。
  - 兼容别名误用：`oasis7_viewer_live` 若收到 `--runtime-world`，必须立即失败并输出“请直接使用纯 viewer 参数”。
  - 创世控盘越界：若单人直持或项目直接液态份额超过 PRD 上限，则创世配置不得冻结。
  - 奖励语义漂移：若 early-player reward 被描述为“登录就发”或“时长挖矿”，则必须退回为 contribution-based 口径重审。
  - 安全等级误判：若局部 `ed25519`/allowlist 能力被误写成“整体已达主流公链安全”，则必须退回并以系统级交易授权/托管/治理真值重审。
  - 阶段误升级：若 `STRAUTH-3` 已完成但生产级 keystore、治理 signer 外部化或创世 ceremony 仍未通过，就把安全阶段升级为 `mainnet-grade`，必须直接阻断并回到 readiness gate 检查。
  - hosted world 误暴露：若 public join URL 仍可命中 world 启停、链控制或 GUI operator action 等管理接口，则必须直接判定为架构越界而非部署细节问题。
  - 浏览器 signer 泄露：若 HTML bootstrap、JS 全局对象或任意 public API 仍返回长期 signer 私钥、seed 或等价真值，则 hosted-world 路径必须直接阻断。
  - 私网节点离网：若家宽 / NAT / CGNAT 节点因缺少公网入站而被默认判定为不可参与，则必须回到覆盖网络架构重审，而不是继续追加静态 peer 补丁。
  - relay 单点依赖：若 private/validator_hidden 节点只剩单一 relay-domain 路径，必须直接降级 verdict，不得继续声称已具备 public-chain-grade mixed-topology。
  - 拓扑安全退化：若 active peer set 集中于单一 operator、ASN 或 `/24`，则必须触发 anti-eclipse 阻断，而不是只要“能连上”就放行。
  - 权限混层：若 guest/player session 在没有强鉴权的情况下能执行资产转账、治理或高风险 prompt/control，则必须回退到 hosted-world 权限设计审查。
  - admission 失控：若 public join 在没有 `max_guest/max_player/rate_limit/world_full_policy` 的情况下无界签发 session，则必须回退到 hosted-world admission 设计审查。
  - bridge 错配：若 bridge-service 无法把一笔 `OC` 入账唯一映射到一个 LetAI `platform_user_id`、`platform_project_id`、`external_order_id` 或 `token_key`，则必须停在 `manual_review`，不得把共享收款账户上的模糊入账自动折算成 quota。
  - credit 半成功：若链上入账已确认、`bridge_ledger` 已写入，但 LetAI OpenAPI 的 user/project/topup/query 任一步失败，则必须保留幂等重试键并阻止重复入账，不得靠人工重复发放覆盖。
- Non-Functional Requirements:
  - NFR-P2P-1: 多节点长跑稳定性指标持续达标并可追溯。
  - NFR-P2P-2: 共识提交与复制链路关键失败模式覆盖率 100%。
  - NFR-P2P-3: 节点异常恢复流程具备标准化操作与证据产物。
  - NFR-P2P-4: 资产与签名链路审计记录完整率 100%。
  - NFR-P2P-5: 协议演进不得破坏既有网络兼容性基线。
  - NFR-P2P-6: 手机轻客户端路径必须可验证最终性，且不要求端侧权威模拟。
  - NFR-P2P-7: slot 计算在重启前后保持单调一致；槽位倒退容忍度为 0（仅允许漏槽）。
  - NFR-P2P-8: 在启用 `ticks_per_slot` 时，logical tick/phase 计算跨节点一致，提案节拍可观测且可回归验证。
  - NFR-P2P-9: S9/S10 若出现失败，必须在同一审计轮次内沉淀 `incident_id/root_cause/fix_commit/regression_command` 四元组证据。
  - NFR-P2P-10: 分布式发布门禁不得接受 `insufficient_data` 作为通过结果；S9/S10 指标门禁结果必须显式为 `pass`。
  - NFR-P2P-11: 控制面参数命名与状态字段在 runtime/game/web/client launcher/scripts 上保持一致，避免语义分叉导致错误调参。
  - NFR-P2P-12: 指标命名必须区分“worker poll”与“consensus tick”；任何对外接口不得将二者混称为同一 tick 语义。
  - NFR-P2P-13: `oasis7_viewer_live` CLI 帮助与错误文案中不得再出现 release/node 控制面入口，避免与 `oasis7_chain_runtime` 控制平面重复。
  - NFR-P2P-14: `oasis7_viewer_live` 仅保留一个生效的 CLI 解析实现；仓内不得存在与生产入口分叉的 legacy 参数解析代码路径。
  - NFR-P2P-15: 模块文档中的源码路径引用必须可解析到当前仓库存在文件，避免审计与回归排障时出现失效链接。
  - NFR-P2P-16: Token 创世分配表必须满足 `sum(allocation_bps)=10000`、项目战略控制目标 `5000 bps`、单人直持硬上限 `1500 bps`、创世液态流通硬上限 `500 bps`，且首 12 个月非团队外部释放目标 `100~200 bps`、硬上限 `500 bps`。
  - NFR-P2P-17: 在资产动作签名交易模型、生产级 keystore、治理 signer 外部化与创世 slot 真实绑定完成前，`oasis7` 不得宣称“对标主流公链安全”或 `mainnet-grade`。
  - NFR-P2P-18: 公开 `transfer submit` 面不得存在无签名旁路；请求级鉴权必须在余额/nonce 预检之前完成，且 `oasis7_web_launcher` 代理结构与 runtime 保持同一字段集合。
  - NFR-P2P-19: 在 `MAINNET-1~4` readiness gate 全部通过前，公开口径最多只能使用 `limited playable technical preview` 与 `crypto-hardened preview`，不得升级到 `production mint ready`。
  - NFR-P2P-20: 生产 signer custody 未外部化前，任何本地 `config.toml`、HTML bootstrap 或长期 env 私钥路径都不得进入 production release allowlist。
  - NFR-P2P-21: 生产 governance truth 未外部化前，任何 deterministic local seed、单机 `NodeConfig` signer policy、手工 env 改动或 `--node-validator*` 参数注入都不得进入 production governance allowlist 或被视为正式 validator admission 完成态。
  - NFR-P2P-22: 在 genesis slot/bucket 真值、ceremony evidence bundle 与 QA `pass` 完成前，任何 `mint_ready` 或 `production mint ready` 口径都不得进入 public claims allowlist。
  - NFR-P2P-23: 在 `MAINNET-1~3` 仍停留于 spec gate 而 execution blockers 未清零时，任何高于 `crypto-hardened preview` 的 public claims 都必须被 denylist 拒绝。
  - NFR-P2P-24: 在 `shared_devnet/staging/canary` 仍未形成正式 shared-network evidence 前，任何 `release train established`、`shared network validated` 或“对标主流公链测试成熟度已完成”的表述都必须被 denylist 拒绝。
  - NFR-P2P-25: hosted world public player plane 在任何 HTML/JS/bootstrap/API 响应中都不得暴露长期 signer 私钥、seed 或等价真值。
  - NFR-P2P-26: hosted world public join origin 默认不得暴露 world start/stop、chain start/stop 或 operator-only GUI action 入口；能力不足时前后端都必须拒绝。
  - NFR-P2P-27: hosted world public join 必须具备有界 admission control，至少冻结 `max_guest_sessions/max_player_sessions/issue_rate_limit/world_full_policy`，且超限时返回结构化拒绝。
  - NFR-P2P-28: `doc/p2p/**` 活跃文档、token 专题、模块入口与 runtime/account 相关实现提到当前链上代币时，必须统一使用“绿洲币 / Oasis Coin” / `OC` / `oc:pk:` 作为现行真值；`AWT` / `awt:pk:` 仅允许保留在明确标注的历史语境或兼容说明中。
  - NFR-P2P-29: `OC -> LetAI Run OpenAPI` bridge 必须坚持 `one-way service-credit only`、独立 bridge-service、唯一入账映射、`bridge_ledger` 幂等对账、动态 project/`token_key` 持久化与 operator-review fallback；在公开钱包体系、生产级 custody、双向兑回与价格发现机制缺失前，任何“公开兑换所”“自动提现”“浏览器直连热钱包充值”口径都不得进入 allowlist。
  - NFR-P2P-30: `hosted_public_join` 的正式产品路径必须允许邮箱 hosted login、服务端托管 player signer 与后续自托管升级；在浏览器仍长期持有托管私钥、账户恢复仍依赖手抄私钥或高风险动作仍没有正式 step-up + custody sign lane 之前，不得声称“任意新用户都已有安全可用的 hosted account + wallet”。
- Security & Privacy: 需保证节点身份、签名、账本与反馈数据链路的完整性；所有关键动作必须具备可审计记录。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化网络/共识/存储统一设计基线。
  - v1.1: 补齐在线长跑失败模式和恢复手册。
  - v2.0: 建立分布式质量趋势看板（稳定性、时延、恢复、失败率）。
- Technical Risks:
  - 风险-1: 多子系统并行演进带来接口漂移。
  - 风险-2: 长跑测试覆盖不足导致线上异常暴露滞后。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-001 | TASK-P2P-001/002/005 | `test_tier_required` | 网络/共识/存储联合验收清单检查 | 协议边界与跨 crate 兼容 |
| PRD-P2P-002 | TASK-P2P-002/003/005 | `test_tier_required` + `test_tier_full` | S9/S10 长跑与恢复演练 | 多节点稳定性与故障恢复 |
| PRD-P2P-003 | TASK-P2P-003/004/005 | `test_tier_full` | 签名与治理链路审计检查 | 资产安全与发布风险控制 |
| PRD-P2P-004 | TASK-P2P-006/007 | `test_tier_required` + `test_tier_full` | 轻客户端 intent/finality/challenge/reconnect 闭环验证 | 移动端接入、公平性与可用性 |
| PRD-P2P-005 | TASK-P2P-008 | `test_tier_required` + `test_tier_full` | 固定时间槽单调性/漏槽/重启恢复/未来槽拒绝回归 | 共识时间语义、提案与投票窗口 |
| PRD-P2P-006 | TASK-P2P-009 | `test_tier_required` + `test_tier_full` | 槽内 tick 相位门控、动态调度等待与跨节点节拍回归 | 共识提案节奏、runtime 调度与可观测 |
| PRD-P2P-007 | TASK-P2P-010 | `test_tier_required` + `test_tier_full` | runtime/game/web/client launcher/scripts 参数映射、状态观测与兼容回归 | 控制面调参、运维门禁与观测一致性 |
| PRD-P2P-008 | TASK-P2P-011 | `test_tier_required` | 状态字段语义对齐、launcher 校验文案回归、文档一致性检查 | 运维观测、发行手册与参数治理 |
| PRD-P2P-009 | TASK-P2P-012 | `test_tier_required` | 默认参数一致性、launcher/web 校验文案与手册口径回归 | 默认启动行为、控制面配置与运维认知一致性 |
| PRD-P2P-009 | TASK-P2P-013 | `test_tier_required` | 默认值切换到 `12s/10/9` 并回归 CLI/脚本/文档口径 | 时间锚定基线一致性与默认运行节拍 |
| PRD-P2P-010 | TASK-P2P-014 | `test_tier_required` | `oasis7_viewer_live` legacy 参数拒绝、帮助文案收敛与文档/示例迁移回归 | Viewer/chain 控制面边界一致性 |
| PRD-P2P-011 | TASK-P2P-015 | `test_tier_required` | `oasis7_viewer_live` 删除 `--runtime-world` 兼容别名、移除旧 split CLI 路径并回归手册/测试口径 | CLI 单一事实源与维护成本收敛 |
| PRD-P2P-012 | TASK-P2P-016/018 | `test_tier_required` | 历史文档旧路径替换、历史专题标题零残留校验 + 文档门禁（过程日志除外） | 文档可追溯性与维护效率 |
| PRD-P2P-013 | TASK-P2P-031 | `test_tier_required` | 创世分配专题 PRD/project/design 建档、模块入口映射、文档门禁与差异检查 | Token 创世口径、低流通边界与早期贡献奖励策略 |
| PRD-P2P-013 | TASK-P2P-045 | `test_tier_required` | 正式命名冻结、symbol/ticker 边界说明、模块入口与 token 专题口径同步、文档门禁与差异检查 | 链上代币 public naming 与后续迁移前置边界 |
| PRD-P2P-013/015 | TASK-P2P-046 | `test_tier_required` | `OC` / `oc:pk:` 迁移、签名鉴权前缀同步、API/viewer/client/liveops/tests/docs 回写、文档门禁与差异检查 | 链上代币 runtime/account 当前真值统一 |
| PRD-P2P-013 | TASK-P2P-047 | `test_tier_required` | `10,000,000,000 OC` 创世总量冻结、bucket 绝对分配额/首年释放边界回写、formal freeze sheet supply gate 更新、文档门禁与差异检查 | 链上代币创世绝对总量与低流通绝对边界 |
| PRD-P2P-014 | TASK-P2P-032 | `test_tier_required` | 密码学安全基线专题 PRD/project/design 建档、代码真值盘点、模块入口映射、文档门禁与差异检查 | 安全口径、mainnet-ready blocker 与优先级治理 |
| PRD-P2P-015 | TASK-P2P-033 | `test_tier_required` | 签名交易鉴权专题 PRD/project/design 建档、transfer submit 鉴权实现、control-plane schema 同步与定向回归 | 主链 Token 首个公开资产面签名化 |
| PRD-P2P-016 | TASK-P2P-034 | `test_tier_required` | mainnet-grade readiness 硬化专题 PRD/project/design 建档、剩余 P1/P2 gate 冻结、模块入口映射与文档门禁 | signer custody、治理 signer、创世 ceremony 与 public claims gate |
| PRD-P2P-017 | TASK-P2P-035 | `test_tier_required` | 生产级 signer custody / keystore 专题 PRD/project/design 建档、signer inventory、环境门禁与 readiness project 回写 | signer source boundary、rotation/revocation/audit 与 release policy |
| PRD-P2P-018 | TASK-P2P-036 | `test_tier_required` | 治理 signer 外部化专题 PRD/project/design 建档、governance signer inventory、source-of-truth 门禁、validator/finality signer admission target workflow 与 readiness project 回写 | governance truth、validator admission、failover/rotation/revocation 与 operator ownership |
| PRD-P2P-019 | TASK-P2P-037 | `test_tier_required` + `test_tier_full` | 创世 freeze/ceremony/QA gate 专题 PRD/project/design 建档、freeze sheet blocker 冻结、QA evidence bundle 与 claim gate 回写 | mint readiness、创世执行与对外口径 |
| PRD-P2P-020 | TASK-P2P-038 | `test_tier_required` | public claims policy 复评专题 PRD/project/design 建档、allowlist/denylist 冻结、future upgrade condition 与 readiness 完结回写 | 对外口径、阶段复评与后续升级条件 |
| PRD-P2P-021 | TASK-P2P-039 | `test_tier_required` | 主流公链测试体系 benchmark 专题 PRD/project/design 建档、testing-manual 映射、gap matrix 与执行优先级冻结 | 测试成熟度口径、QA 证据体系与后续 hardening 排序 |
| PRD-P2P-022 | TASK-P2P-040 | `test_tier_required` | shared network / release train minimum 专题 PRD/project/design/runbook 建档、three-track model、candidate bundle、claims gate 与 `testing-manual` 入口冻结 | shared-network 执行模型、release train 口径与后续 rehearsal 排序 |
| PRD-P2P-023 | TASK-P2P-041 | `test_tier_required` | hosted-world player access / session-auth 专题 PRD/project/design 建档、plane split、session ladder、`gui-agent` split、admission control、sensitive-action capability 与 claims boundary 冻结 | hosted web multiplayer 边界、浏览器 signer 暴露风险与后续实现排序 |
| PRD-P2P-024 | TASK-P2P-043 | `test_tier_required` | 非全公网覆盖网络专题 PRD/project/design 建档、deployment mode / role model / peer record / reachability / traffic lanes 与 claims gate 冻结 | mixed-topology 网络边界、私网节点参与能力与后续框架拆解排序 |
| PRD-P2P-025 | triad-observability-stack | `test_tier_required` | triad host/process monitor、merged observability summary、testing manual 入口、fixture 回归与 real-env smoke | 当前 real-env triad（本机 + 2 ECS，runtime 已为 three_equal_validator）的 canonical 运维监控入口，并显式区分 legacy service label 与真实 runtime role |
| PRD-P2P-026 | triad-three-equal-validator-topology | `test_tier_required` | live triad validator-set/signer/bootstrap 改造、same-window snapshot evidence、testing manual claim 口径更新与 legacy service label 边界说明 | 三节点等权 validator 拓扑、live 运维真值与 mixed-topology 历史边界 |
| PRD-P2P-027 | mainchain-token-newapi-quota-bridge-proposal | `test_tier_required` | `OC -> LetAI Run OpenAPI quota` 专题 PRD/design/project 建档、one-way bridge boundary、独立 bridge-service、唯一入账映射、`bridge_ledger` 状态机、动态 project/`token_key`、query verification 与 operator risk gate 冻结 | 链上资产到 AI 服务内部额度的受控桥接口径与后续实现排序 |
| PRD-P2P-028 | formal-network-tiers-testnet-mechanism | `test_tier_required` | 正式网络分层 / testnet 机制专题 PRD/design/project 建档、`network_tier_manifest` 脚本+smoke+example manifests、`testing-manual` 入口、current verdict 冻结与 `public_testnet` live-candidate checklist companion runbook | 公共主链式 `shared_devnet/public_testnet/mainnet` 分层口径、manifest 真值、live-candidate checklist 与后续 runtime/liveops 接线排序 |
| PRD-P2P-029 | hosted-managed-identity-doc-freeze | `test_tier_required` | hosted-public-join 托管身份 / 托管密钥专题 PRD/design/project 建档、hosted account/device session/`signer_ref`/step-up auth/self-custody upgrade 边界冻结、模块入口映射与文档门禁 | 普通玩家 hosted onboarding、player custody 产品边界与后续实现排序 |
- S9/S10 长跑结果模板（TASK-P2P-003）:
| 字段 | 说明 | 来源 |
| --- | --- | --- |
| `suite` | `S9` 或 `S10` | `testing-manual.md` 套件定义 |
| `run_id` | 本次运行唯一标识（目录名/时间戳） | `.tmp/p2p_longrun/*` 或 `.tmp/s10_game_longrun/*` |
| `profile` | `soak_smoke/soak_endurance/soak_release` 或 S10 对应档位 | 执行命令 |
| `gate_status` | `pass/fail` | `summary.json` |
| `key_metrics` | `lag_p95/distfs_failure_ratio/consensus_hash_mismatch_count` 等 | `summary.json` |
| `evidence_paths` | `summary.json/timeline.csv/failures.md` | 产物目录 |
- S9/S10 缺陷闭环模板（TASK-P2P-003）:
| 字段 | 填写要求 | 闭环判定 |
| --- | --- | --- |
| `incident_id` | `S9-YYYYMMDD-xxx` 或 `S10-YYYYMMDD-xxx` | 与失败运行一一对应 |
| `symptom` | 失败现象 + 首个告警指标 | 可定位到日志行或指标项 |
| `root_cause` | 技术根因（网络/共识/存储/配置） | 具备可复现实验步骤 |
| `fix_task` | 对应任务 ID（如 `TASK-P2P-00x-*`） | 任务文档可追踪 |
| `fix_commit` | 修复提交 SHA | commit 可检索 |
| `regression_command` | 至少 1 条定向回归 + 1 条长跑复验命令 | 命令可执行且结果通过 |
| `closure_note` | 风险评估与是否阻断发布 | 发布门禁结论一致 |
- 发行门禁分布式质量指标映射（TASK-P2P-004）:
| 指标 | 数据源 | 发布阈值（2026-03-07） | 阻断策略 | 执行责任 |
| --- | --- | --- | --- | --- |
| `S9.topologies[].metric_gate.status` | S9 `summary.json` | 必须为 `pass` | 任何拓扑 `fail/insufficient_data` 直接阻断 | 发行值班工程师 |
| `S9.topologies[].metrics.consensus_hash_consistent` | S9 `summary.json` | 必须为 `true` | 任意 `false` 直接阻断并拉起共识排障 | 共识 owner |
| `S9.topologies[].metrics.consensus_hash_mismatch_count` | S9 `summary.json` | 必须为 `0` | 非 0 直接阻断并要求补 `consensus_hash_mismatch.tsv` 分析 | 共识 owner |
| `S9.topologies[].metrics.lag_p95` | S9 `summary.json` | `<= 50`（由 `--max-lag-p95 50` 注入） | 超阈值阻断并进入网络退化复盘 | 网络 owner |
| `S9.topologies[].metrics.distfs_failure_ratio` | S9 `summary.json` | `<= 0.1`（由 `--max-distfs-failure-ratio 0.1` 注入） | 超阈值阻断并进入 DistFS 复制链路修复 | DistFS owner |
| `S10.run.metric_gate.status` | S10 `summary.json` | 必须为 `pass` | `fail/insufficient_data` 直接阻断 | 发行值班工程师 |
| `S10.run.status` | S10 `summary.json` | 必须为 `ok` | 非 `ok` 直接阻断并要求 `failures.md` | 发行值班工程师 |
| `S10.run.metrics.lag_p95` | S10 `summary.json` | `<= 50`（由 `--max-lag-p95 50` 注入） | 超阈值阻断并回退到性能/网络专项 | 网络 owner |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-001 | 网络/共识/DistFS 统一验收 | 子系统独立验收 | 可降低跨链路隐性回归风险。 |
| DEC-P2P-002 | 长跑结果进入发布门禁 | 仅开发阶段抽样运行 | 发布质量依赖真实长稳证据。 |
| DEC-P2P-003 | 关键动作全链路审计 | 仅关键节点日志 | 审计深度不足会放大安全风险。 |
| DEC-P2P-004 | 移动端采用轻客户端+链下权威模拟 | 手机端参与权威模拟 | 移动端资源受限，权威性和实时性需分层保障。 |
| DEC-P2P-005 | PoS slot 按 wall-clock 统一公式驱动 | 继续本地 tick 自增 slot | 可消除重启/负载抖动造成的时间语义漂移。 |
| DEC-P2P-006 | PoS 增加槽内 tick 相位门控与动态调度 | 仅保留固定 `tick_interval` 与 slot 门控 | 需要稳定落地 `10 tick/slot` 节奏并降低固定 sleep 漂移。 |
| DEC-P2P-007 | 三线联合验收采用“子系统单测基线 + S9/S10 长跑门禁”双层收口 | 仅保留单测或仅保留长跑 | 单一层级无法覆盖“确定性回归 + 长时退化”双维风险。 |
| DEC-P2P-008 | 分布式质量指标按 `release-gate.sh` 参数固化为“硬阻断” | 发布前人工主观评估放行 | 降低人工判断漂移，确保门禁可复现。 |
| DEC-P2P-009 | 将 `node_tick_ms` 定义为 worker 轮询/回退间隔，并显式暴露 slot-clock 参数 | 继续用 `node_tick_ms` 承担出块时间语义 | 减少运维误配与观测误读，保证时间锚定语义可操作。 |
| DEC-P2P-010 | 在状态与文档中显式区分 `worker_poll_count` 与共识 tick/height 指标 | 继续沿用 `tick_count` 作为泛化进度字段 | 避免“轮询次数=出块推进”的误读，降低误判与误调参风险。 |
| DEC-P2P-011 | 统一 runtime/game/web/client launcher 默认 `slot_duration_ms` 为文档基线值，并收敛校验文案为 poll interval 语义 | 继续维持 `slot_duration_ms=1` 且允许文案混用 tick/block 语义 | 减少“默认启动即偏离锚定口径”的隐性配置风险，降低运维误读。 |
| DEC-P2P-012 | 默认 PoS 时间参数采用 `slot_duration_ms=12000`、`ticks_per_slot=10`、`proposal_tick_phase=9` | 保持 `200/1/0` 等压测导向默认组合 | 与“12s 出块、每块 10 tick”设计口径一致，默认体验与协议基线对齐。 |
| DEC-P2P-013 | `oasis7_viewer_live` 移除 `--release-config` 与 `--node-*` 控制面参数，仅保留观察服务 CLI | 继续在 viewer 保留 release/node 控制面兼容入口 | 避免控制面双入口造成运维误配，统一由 `oasis7_chain_runtime` 承担链参数与节点生命周期。 |
| DEC-P2P-014 | `oasis7_viewer_live` 删除 `--runtime-world` 兼容别名与 legacy split CLI 代码，保留单一生产入口 `oasis7_viewer_live.rs` | 继续保留兼容别名和未接入入口的旧解析代码 | 避免“文档/测试改了但真实入口不生效”的双轨风险，降低后续维护和误判成本。 |
| DEC-P2P-015 | 统一将历史文档中的 `oasis7_viewer_live` 旧文件路径替换为当前源码布局路径（`oasis7_viewer_live.rs` / `oasis7_chain_runtime/*`） | 保留旧路径并依赖读者自行映射 | 降低审计误导与排障成本，确保文档可直接定位现行实现。 |
| DEC-P2P-016 | 先冻结“项目战略控制 50% + 协议奖励池 35% + 低流通 + 贡献制奖励”口径，再决定具体创世账户与执行节奏 | 先广泛发币或直接采用开放式 play-to-earn | 当前阶段仍是 `limited playable technical preview`，需要先守住低流通、可审计与反滥用边界。 |
| DEC-P2P-017 | hosted world 采用 `public player plane / private control plane / signer plane` + `guest/player/strong-auth` 梯度 | 继续把 join/control/signer 混在单一 web bootstrap 里，或用 invite-only 代替安全边界 | hosted world 的核心问题是信任面混层，不先拆平面和能力就无法安全支持“一个玩家部署、另一个玩家通过网页进入”。 |
| DEC-P2P-018 | 先冻结当前链上代币的正式产品名为“绿洲币 / Oasis Coin”，再单开专题迁移 runtime/account 真值 | 在未评审 API/UI/兼容性影响前直接顺手改 runtime symbol / account prefix | 产品名、symbol、链上字段和客户端展示面属于不同治理层；先冻结 public naming，才能把后续 runtime 改动收成独立可审计任务。 |
| DEC-P2P-019 | 在独立迁移专题中，把当前链上代币的 runtime symbol、公钥派生账户前缀与签名鉴权前缀统一切到 `OC` / `oc:pk:` | 继续让 `AWT` / `awt:pk:` 作为现行真值，或只改产品名不改 runtime/account | 当前 public naming 已冻结，继续双轨会让 API、viewer/client、liveops 与审计口径长期分叉；需要一次把 runtime/account 当前真值收口。 |
