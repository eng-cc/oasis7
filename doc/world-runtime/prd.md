# world-runtime PRD

审计轮次: 7

## 目标
- 建立 world-runtime 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 world-runtime 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 world-runtime 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/world-runtime/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/world-runtime/prd.md`
- 项目管理入口: `doc/world-runtime/project.md`
- 根级兼容执行入口: `doc/world-runtime.project.md`
- 文件级索引: `doc/world-runtime/prd.index.md`
- 追踪主键: `PRD-WORLD_RUNTIME-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: world runtime 涉及确定性执行、事件溯源、WASM 扩展、治理、审计与运行态持久化等核心能力，若缺少统一设计入口，跨阶段改动容易引发一致性、安全与存储体积失控回归。
- Proposed Solution: 以 world-runtime PRD 统一定义内核能力边界、WASM 运行约束、治理流程、数值语义、存储治理与验证标准。
- Success Criteria:
  - SC-1: runtime 关键改动具备 PRD-WORLD_RUNTIME-ID 映射与测试证据。
  - SC-2: 确定性回放与事件审计链路保持可复现。
  - SC-3: WASM 沙箱与接口变更具备兼容性与安全校验记录。
  - SC-4: 数值语义硬化议题持续收敛并形成阶段性里程碑。
  - SC-5: 运行态持久化具备可观测的存储预算、保留策略与恢复验证，不再在默认链路中无界增长。
  - SC-6: retention policy 保留范围内的目标高度必须具备可验证的 replay contract，可由 checkpoint + canonical log 重建并校验 `execution_state_root`。
  - SC-7: 面向 Viewer / QA 的运行时测试钩子必须显式 env-gated，并输出可回放的标准世界事件，避免测试态捷径泄漏到默认产品路径。
  - SC-8: provider `player_parity` / `headless_agent` 共用同一 runtime 权威动作校验，且 mode/schema/environment/fixture/replay 元数据可稳定写入 request、summary 与 benchmark 产物。
  - SC-9: WASM 构建与发布链路必须通过 pinned Docker canonical builder 收敛为单一 publish hash，保证 `builder_image_digest/source_hash/build_manifest_hash/wasm_hash` 可追溯，且执行层默认只认 canonical binary。
  - SC-10: 生产运行入口必须默认启用 release security policy，关闭 builtin manifest fallback、本地 identity hash 签名、本地 finality signing 与 runtime source compile，保证“只认 canonical binary”不是测试态约定而是产品默认路径。
  - SC-11: `TASK-WORLD_RUNTIME-043` 收口时必须归档真实跨宿主 Docker canonical evidence，至少覆盖 `linux-x86_64` 与一条 Docker-capable `darwin-arm64` 证据输入，不能以 Linux-only gate 宣称跨宿主 closure。
  - SC-12: `doc/world-runtime/**` 仍可读运行时专题标题统一使用 `oasis7 Runtime` 品牌，不再在模块入口与历史专题中混用 `oasis7 Runtime` 标题。
  - SC-13: WASM 构建、同步、CI summary 与 builder image 的 operator env key 必须统一使用 `OASIS7_WASM_*`；repo-owned 脚本、容器镜像与 build suite 不得再接受任何旧品牌前缀作为有效运行入口。
  - SC-14: builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 必须统一使用 `OASIS7_BUILTIN_WASM_*`；运行时取件/抓取/回退链路不得再接受任何旧品牌前缀作为有效运行入口。
  - SC-15: `compile_module_artifact_from_source` 及其 source package 限额/超时控制必须统一使用 `OASIS7_MODULE_SOURCE_*`；dev/test source compile 路径、simulator/runtime 回归与沙箱环境隔离断言不得再接受任何旧品牌前缀作为有效运行入口。
  - SC-16: `doc/world-runtime/project.md` 等模块主入口中的当前 cargo 回归命令、crate 路径与产物文件清单必须统一使用 `oasis7*` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在历史证据、兼容说明或负向测试语义中。
  - SC-17: `oasis7_chain_runtime` 的 `/v1/chain/status` 必须显式暴露节点网络流量观测快照，至少区分 `udp_gossip` 与 `libp2p_replication` 两条链路，并标明统计范围是否包含 transport/control-plane 开销。
  - SC-18: `fetch-commit` gap-sync 请求必须对最近刚返回“缺少 handler / 不支持该协议”签名的 `ErrUnsupported`、`ErrNotFound`、`Timeout` 或连接缺口的 peer 做短时协议级退避；业务语义层的 `ErrUnsupported` 不计入该退避条件，以避免在真实 triad 中对同一无效目标反复发起 libp2p 请求。
  - SC-19: `libp2p_net` peer discovery 路径中的 `get_local_peer_record`、`get_cached_peer_record` 与 `get_cached_discovery_peers` 请求必须对同一 peer 保持短时协议级冷却，压制 DHT/routing/rendezvous/connection-established 事件造成的高频重复取件，同时保留单次 cached-peer-record 请求链里的多 proxy fallback。
  - SC-20: chain-linked `oasis7_viewer_live` 必须默认被动跟随 `/v1/chain/status.consensus.committed_height` 对应的 execution world，不需要显式 `Play` 才能消费 committed action；若轮询没有新的 committed action 或没有新增 world event，则不得把该轮询记成逻辑 world progression。

## 2. User Experience & Functionality
- User Personas:
  - 运行时架构师：需要控制可信边界与模块化演进。
  - 模块开发者：需要稳定 ABI/执行语义与治理流程。
  - 审计与安全评审者：需要完整可追溯的事件与收据链路。
  - QA / 发布运维：需要可预测的运行态磁盘预算、恢复能力与状态指标。
  - 发布节点运营者 / 构建审计者：需要区分哪些校验属于 Docker canonical build evidence，哪些属于线上发布合法性。
- User Scenarios & Frequency:
  - 运行时语义评审：每次核心行为改动前执行，确认确定性与兼容边界。
  - WASM 接口变更：每个接口变更至少进行一次兼容核验与回放验证。
  - 治理事件审计：发布前执行，检查关键治理事件链路完整性。
  - 安全回归复核：按周执行，验证沙箱、签名、权限约束无回退。
  - 运行态存储复核：每次持久化 / 启动器 / 链路改动后执行，确认 footprint、GC 与恢复能力符合预算。
  - WASM 构建/发布复核：每次涉及 builder image、canonicalizer、manifest/identity、source compile 或 release materializer 时执行，确认 Docker canonical hash 与社会层审计字段保持一致。
- User Stories:
  - PRD-WORLD_RUNTIME-001: As a 架构师, I want deterministic world execution semantics, so that replay and audit remain trustworthy.
  - PRD-WORLD_RUNTIME-002: As a 模块开发者, I want stable WASM interfaces and lifecycle governance, so that upgrades are safe.
  - PRD-WORLD_RUNTIME-003: As a 安全评审者, I want explicit security and receipt guarantees, so that critical risks are controlled.
  - PRD-WORLD_RUNTIME-013: As a Runtime 维护者, I want bounded execution-state retention and sidecar GC, so that默认运行链路的磁盘占用受控。
  - PRD-WORLD_RUNTIME-014: As a QA / 审计维护者, I want GC 后仍可 latest-state 恢复并保留检查点取证能力, so that体积优化不破坏恢复与审计。
  - PRD-WORLD_RUNTIME-015: As a 发布工程师, I want profile-based storage policies and metrics, so that dev/release/soak 能执行不同磁盘预算。
  - PRD-WORLD_RUNTIME-019: As a Runtime 维护者 / Viewer / QA, I want factory production blocked/resumed/completed state to be explicit and replayable, so that early industrial onboarding can explain why production advanced or stalled.
  - PRD-WORLD_RUNTIME-020: As a `wasm_platform_engineer`, I want publishable WASM to be built only inside a pinned Docker builder image, so that host platform differences stop influencing release hashes.
  - PRD-WORLD_RUNTIME-021: As a 模块发布者 / 发布节点运营者, I want build receipt and release evidence to bind `builder_image_digest + source_hash + build_manifest_hash + wasm_hash`, so that binary trust can be socially verified without relying on host-native builds.
  - PRD-WORLD_RUNTIME-022: As a `runtime_engineer` / `qa_engineer`, I want runtime to consume only Docker-canonical binaries and production source compile to leave the runtime hot path, so that build drift is blocked before execution.
  - PRD-WORLD_RUNTIME-023: As a `runtime_engineer`, I want production-facing runtime entrypoints to default to hardened `ReleaseSecurityPolicy`, so that `no-fallback / no-local-signing / no-runtime-source-compile` is enforced by construction instead of call-site convention.
  - PRD-WORLD_RUNTIME-024: As a `runtime_engineer` / `qa_engineer`, I want `apply_domain_event*` replay/apply paths to return structured `WorldError` for invariant breaks, so that corrupted journal / migration drift is diagnosable without panic-killing recovery or preflight.
  - PRD-WORLD_RUNTIME-025: As a `runtime_engineer`, I want oversized runtime hotpath files split below the 1200-line governance ceiling, so that determinism, replay, and rule changes stop depending on multi-kiloline match blocks.
  - PRD-WORLD_RUNTIME-026: As a `runtime_engineer` / `qa_engineer` / 节点运营者, I want `/v1/chain/status` to expose network traffic counters for `udp_gossip` and `libp2p_replication`, so that real traffic spikes can be attributed to a concrete lane instead of only host-level bandwidth totals.
  - PRD-WORLD_RUNTIME-027: As a `runtime_engineer` / 节点运营者, I want a persistent triad traffic sampler to convert cumulative `/v1/chain/status.traffic` counters into recent-window deltas, so that questions like "last 10 minutes per node" can be answered without waiting for a process restart.
  - PRD-WORLD_RUNTIME-028: As a 节点运营者, I want traffic monitoring to be an env-gated node startup capability, so that local recent-window history can be enabled per node without hand-running an external triad script.
  - PRD-WORLD_RUNTIME-029: As a `runtime_engineer` / 节点运营者, I want `fetch-commit` retries to short-circuit peers that just returned protocol-unavailable, not-found, or timeout signatures, so that gap-sync traffic waste drops without relaxing replication correctness.
  - PRD-WORLD_RUNTIME-030: As a `runtime_engineer` / 节点运营者, I want peer-record/discovery requests to remember the same target peer for a short cooldown window, so that repeated DHT/routing/rendezvous triggers stop reissuing `get_local_peer_record` / `get_cached_peer_record` / `get_cached_discovery_peers` requests immediately after the previous attempt cleared.
  - PRD-WORLD_RUNTIME-031: As a `runtime_engineer` / `viewer_engineer`, I want chain-linked viewer runtime to passively follow committed execution-world progress from `oasis7_chain_runtime`, so that the player no longer needs explicit `Play` just to consume committed chain actions and empty polls do not masquerade as world advancement.
  - PRD-WORLD_RUNTIME-032: As a `runtime_engineer` / 节点运营者, I want storage challenge `fetch-blob` probes to reuse a short-lived success cache for recently verified content hashes, so that triad nodes stop re-fetching the same reachable blob every commit while still rechecking new blobs and expiring old proofs quickly.
  - PRD-WORLD_RUNTIME-033: As a `runtime_engineer` / `viewer_engineer`, I want chain-linked viewer gameplay actions to submit through `oasis7_chain_runtime` and only become visible after committed execution-world sync, so that player controls follow the same consensus-backed path as chain observation instead of mutating the local viewer runtime optimistically.
  - PRD-WORLD_RUNTIME-034: As a `runtime_engineer` / 节点运营者, I want repeated successful `fetch-commit` requests with the same deterministic payload to reuse a short-lived local success cache, so that followers stop re-asking the network for the same already-found commit during tight gap-sync loops without hiding not-found or timeout recovery.
- Critical User Flows:
  1. Flow-WR-001: `提交 runtime 变更 -> 执行回放一致性验证 -> 对比事件链 -> 输出兼容结论`
  2. Flow-WR-002: `WASM 模块注册/升级 -> 生命周期治理校验 -> 沙箱执行 -> 审计事件归档`
  3. Flow-WR-003: `安全异常发现 -> 回溯 receipt -> 定位策略缺口 -> 补回归与发布阻断`
  4. Flow-WR-004: `运行一段时间 -> 采集 storage metrics -> 执行 retention / GC -> 重启恢复 -> 对比 latest state 与审计链`
  5. Flow-WR-005: `选择 retention policy 保留的目标高度 -> 定位 checkpoint -> 回放 canonical log -> 校验 execution_state_root -> 输出 replay 结论`
  6. Flow-WR-006: `源码/manifest 变更 -> pinned Docker builder image 构建 canonical packaged wasm -> canonical hash/identity/release evidence -> DistFS/release manifest -> runtime 仅按 binary hash 装载执行`
  7. Flow-WR-007: `viewer 连接 chain-linked runtime -> 轮询 /v1/chain/status -> committed_height 增加时重载 execution world -> 仅在出现新 event / logical time 变化时向 viewer 发出 snapshot/events`
  8. Flow-WR-008: `viewer 触发 gameplay_action -> POST /v1/chain/gameplay/submit -> chain runtime 校验 auth/nonce 并入 consensus queue -> committed execution world 落盘 -> viewer 仅在下一次 chain sync 后观察到结果`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 确定性执行与回放 | action/event 序列、snapshot、replay 差异 | 执行回放并比较关键状态 | `pending -> replaying -> matched/mismatched` | 按 tick 与 event id 有序比较 | 仅运行时维护者可调整基线 |
| WASM 生命周期治理 | 模块ID、版本、哈希、策略 | 注册/升级/停用流程带审计输出 | `register -> activate -> deactivate -> upgrade`（治理动作） | 版本与策略双约束 | 未授权模块不得激活 |
| 审计与收据链路 | effect、receipt、签名、cause | 导出审计记录并验证签名 | `emitted -> signed -> verified/rejected` | 按事件时间与重要级别检索 | 安全评审者可查看完整链路 |
| 运行态存储治理 | `storage_profile`、snapshot/journal refs、GC 结果、目录字节数 | 启动时加载策略，运行时发布指标并执行 retention / GC | `configured -> active -> degraded/failed` | latest head 永远 pin；checkpoint 按高度排序；metrics 按目录聚合 | 仅运行时维护者 / 发布配置可放宽预算 |
| 回放契约治理 | `canonical_log`、`checkpoint_anchor`、`retained_heights`、`execution_state_root` | 对保留范围内目标高度执行 replay 验证 | `requested -> replaying -> matched/mismatched` | 以 checkpoint + canonical log 为重建基准 | QA / 审计维护者可读取结果 |
| WASM Docker 确定性构建与工件治理 | `builder_image_digest`、`source_hash`、`build_manifest_hash`、`wasm_hash`、`canonicalizer_version`、`canonical_token` | 统一 Docker canonical build、manifest/identity、DistFS 与 runtime binary-only 消费 | `source -> container-built -> canonicalized -> manifested -> verified/executed` | 发布级只允许一个 canonical hash；宿主差异不得进入发布 hash 空间 | `wasm_platform_engineer` 定义 builder image；CI 仅校验；生产发布不由 CI 写入 |
| 历史标题治理 | 标题前缀、专题路径、历史命名说明 | 将仍可读 runtime/module/governance/wasm/testing 专题标题切到 `oasis7 Runtime` | `legacy_title -> current_title -> audited` | 先改主入口/治理/运行时专题，再改更深历史专题 | `producer_system_designer` 定口径，`runtime_engineer` 承接 |
| chain-linked gameplay 提交 | `GameplayActionRequest`、auth proof、nonce、consensus `action_id`、`committed_height` | viewer action 通过 `/v1/chain/gameplay/submit` 入链，提交成功只返回 consensus action id，不直接改本地 world | `requested -> submitted -> committed_visible/rejected` | viewer 只在 committed execution world 产生新 logical time/event 后更新快照 | 需要 viewer auth proof 且 chain runtime 必须拒绝 nonce replay |
- Acceptance Criteria:
  - AC-1: world-runtime PRD 覆盖内核、WASM、治理、安全四条主线。
  - AC-2: world-runtime project 文档任务映射 PRD-ID 并维护状态。
  - AC-3: 与 `doc/world-runtime/runtime/runtime-integration.md`、`doc/world-runtime/wasm/wasm-interface.md` 等分册一致。
  - AC-4: 关键行为变更同步更新测试方案与执行记录。
  - AC-5: 内置 WASM 工件 `sha256` 清单与 identity manifest 保持一致，CI 不得出现 hash token 漂移。
  - AC-6: 运行态存储治理具备专题 PRD / project、预算口径、恢复验证与测试映射，默认链路不得出现无界磁盘增长。
  - AC-7: 运行态持久化专题必须明确 replay contract、canonical log 与 checkpoint 语义，并通过 retained-height replay 测试验证。
  - AC-8: WASM deterministic pipeline 专题必须明确 Docker builder image、canonicalizer version、single canonical publish hash、identity/release evidence、source compile 外移或 gated 与 runtime binary-only 消费边界，并具备独立任务与验证映射。
  - AC-9: 生产 runtime / node 入口必须有可验证的 release policy 绑定证据，证明 fallback / 本地签名 / runtime source compile 默认关闭；若仅测试调用 `enable_production_release_policy()`，不得视为收口。
  - AC-10: 发布候选的 wasm determinism evidence 必须显式区分“当前稳定 gate”和“最终跨宿主 gate”；当 GitHub-hosted runner 缺 Docker daemon 时，必须补外部 Docker-capable macOS evidence，不能把 `linux-x86_64` 单宿主结果等同于跨宿主 closure。
  - AC-11: `doc/world-runtime/**` 仍可读专题标题统一使用 `oasis7 Runtime` 品牌；旧 `oasis7 Runtime` 仅允许保留在正文历史上下文、实现兼容说明与证据原文中。
  - AC-12: `scripts/build-wasm-module.sh`、`scripts/sync-m1-builtin-wasm-artifacts.sh`、`scripts/ci-m1-wasm-summary.sh`、`tools/wasm_build_suite` 与 `docker/wasm-builder/Dockerfile` 必须只读取或写入 `OASIS7_WASM_*`；错误提示、usage、容器注入 env 与 build receipt 元数据采集不得再接受任何旧品牌前缀作为有效运行入口。
  - AC-13: `runtime/builtin_wasm_materializer`、`runtime/m{1,4,5}_builtin_wasm_artifact`、`runtime/world/release_manifest` 及对应测试必须只读取 `OASIS7_BUILTIN_WASM_DISTFS_ROOT`、`OASIS7_BUILTIN_WASM_FETCHER`、`OASIS7_BUILTIN_WASM_FETCH_URLS`、`OASIS7_BUILTIN_WASM_COMPILER`、`OASIS7_BUILTIN_WASM_FETCH_TIMEOUT_MS`；builtin wasm 取件、抓取、编译 fallback 与 release manifest 生产策略故障签名必须证明旧品牌前缀已失效。
  - AC-14: `runtime/module_source_compiler` 与 `runtime/simulator` 对应回归必须只读取 `OASIS7_MODULE_SOURCE_COMPILER`、`OASIS7_MODULE_SOURCE_MAX_FILES`、`OASIS7_MODULE_SOURCE_MAX_FILE_BYTES`、`OASIS7_MODULE_SOURCE_MAX_TOTAL_BYTES`、`OASIS7_MODULE_SOURCE_COMPILE_TIMEOUT_MS`；source compile 成功、旧 alias 已移除与 sandbox env 隔离断言必须覆盖当前前缀。
  - AC-15: `/v1/chain/status` 必须输出 `traffic.udp_gossip` 与 `traffic.libp2p_replication` 两组快照；其中 UDP gossip 至少提供按消息种类聚合的入/出站 datagram 与 payload bytes，libp2p replication 至少提供 gossip/request/response 的入/出站 payload counters、按 topic/protocol 聚合明细，以及 `scope`/排除项说明。
  - AC-16: `doc/world-runtime/project.md` 中当前 `cargo test -p` 命令、crate 路径与产物清单必须写为 `oasis7` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在历史证据、兼容说明或负向测试输入中。
  - AC-17: `World::new()` / `RuntimeWorld::new()` 所服务的生产或默认运行入口不得依赖额外 `enable_production_release_policy()` 调用才能满足 hardened policy；若某入口必须放宽，必须以显式 dev/test 配置进入并留下验证证据。
  - AC-18: `state.apply_domain_event*`、preflight preview 与恢复链路中不得因“prechecked / must be handled”类假设触发 panic；同类异常必须落为可断言的 `WorldError`，并有损坏事件回归样本覆盖。
  - AC-19: `action_to_event_*`、`apply_domain_event_*`、`state.rs` 与其他 runtime 热路径 Rust 文件不得超过 1200 行；拆分后需保留现有 determinism / replay / persistence 回归覆盖，不得以降低测试强度换取拆分完成。
  - AC-20: `oasis7_chain_runtime` 默认注入的 loopback replication fallback 仅可为 replication / feedback 提供本地兜底，不得在已配置 UDP gossip 的多机部署中抢占 PoS consensus 广播；显式共享 replication network 的 network-consensus 路径必须继续可用。
  - AC-21: repo 内必须提供可复用的 triad traffic monitor 脚本，默认面向本机 observer + 两台 ECS 节点，能抓取 `/v1/chain/status` 并写入持久化 history。
  - AC-22: triad traffic monitor 必须支持最近 N 分钟窗口汇总，至少输出每节点的 UDP gossip / libp2p replication totals delta、top `by_kind` / `by_topic` / `by_protocol` 贡献项，以及 committed/network heights 与 recent error counters 的窗口变化。
  - AC-23: triad traffic monitor 的窗口基线选择必须识别 `observed_since_unix_ms` 变化并在进程重启或 counter reset 时自动缩短覆盖窗口，而不是跨 reset 计算负值或伪增量。
  - AC-24: triad traffic monitor 的持久化 history 不得无限增长；脚本必须只保留“最近窗口 + buffer”范围内的样本，并以 NDJSON 流式读取/裁剪 history，而不是整文件 `read_text` 后一次性解码。
  - AC-25: repo 内必须提供节点启动壳的正式源码来源，至少覆盖当前 `/opt/oasis7/p2p-triad{,-local}/bin/start-node.sh` 所使用的 `node.env -> runtime CLI` 装配逻辑，并允许新增 env-gated sidecar 能力而不再依赖 `/opt` 上手改。
  - AC-26: 节点本地 traffic monitor 必须能通过 `node.env` 功能开关启停，默认对单节点本地 `/v1/chain/status` 进行周期采样，输出持久化 history 与最近 N 分钟 summary；节点与 triad monitor 的窗口汇总必须复用共享 helper，且本地 history 也要做 bounded retention。
  - AC-27: 节点启动壳在开启 traffic monitor 时必须与 runtime 共享生命周期，runtime 退出或 service 停止时不能留下长期孤儿 monitor 进程；若开关开启但 monitor 脚本缺失，启动必须显式失败而不是静默跳过。
  - AC-28: `libp2p_replication_network` 必须仅对 `fetch-commit` 请求中的 missing-handler/unsupported-protocol `ErrUnsupported` 签名、`ErrNotFound`、`request failed: Timeout` 与连接缺口类错误触发短时 peer cooldown，并通过定向回归证明立即重试会被抑制、窗口过后可恢复请求、非 `fetch-commit` 协议与通用业务错误（包括泛化业务态 `ErrUnsupported`）不受影响。
  - AC-29: `libp2p_net` 必须仅对 peer-record/discovery 路径中的 `get_local_peer_record`、`get_cached_peer_record` 与 `get_cached_discovery_peers` 触发 peer-scoped、protocol-scoped 短时冷却；定向回归需证明同一 peer 在窗口内不会被立刻重复请求、窗口过后可恢复请求、断连会清理对应 peer 冷却、且 cached-peer-record 在单次请求链中的 fallback proxy 仍可继续尝试。
  - AC-30: storage challenge gate 对近期已验证成功的 `content_hash` 必须提供短窗口 success cache，只允许在缓存过期、命中新 hash、或本地 blob 缺失时重新发起 `fetch-blob` 网络探测；定向回归需证明连续两次 gate 调用不会对同一已验证 blob 重复发网请求，同时缓存过期后仍会恢复真实探测。
  - AC-31: chain-linked `gameplay_action` 必须通过 `oasis7_chain_runtime` 的 `/v1/chain/gameplay/submit` 进入 consensus queue；提交路径必须复用 viewer auth proof、拒绝 nonce replay、返回 consensus `action_id` 作为提交回执，并证明 viewer 本地 world 在提交时不会立即变更，只会在 committed execution world sync 后观察到新工厂/配方结果。
  - AC-32: `libp2p_replication_network` 必须仅对 `fetch-commit` 的成功响应建立短时、本地、payload-scoped success cache；定向回归需证明相同 payload 的立即重复成功请求不会再次发网、窗口过后会恢复真实请求、且 `found=false` 响应不会进入 success cache。
- Non-Goals:
  - 不在本 PRD 中展开每个阶段的实现代码细节。
  - 不替代 p2p 网络拓扑或 site 发布策略设计。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: WASM 执行与治理测试、审计导出工具、数值语义回归套件。
- Evaluation Strategy: 以回放一致性、治理事件完整度、沙箱安全回归数、数值语义缺陷收敛率评估。

## 4. Technical Specifications
- Architecture Overview: world-runtime 模块是系统可信执行基座，负责世界状态演化、模块扩展执行与治理审计，向上游 simulator/game 与下游 p2p 提供稳定语义。
- Integration Points:
  - `doc/world-runtime/runtime/runtime-integration.md`
  - `doc/world-runtime/wasm/wasm-interface.md`
  - `doc/world-runtime/wasm/wasm-executor.prd.md`
  - `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
  - `doc/world-runtime/governance/governance-events.md`
  - `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md`
  - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
  - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
  - `doc/world-runtime/testing/testing.md`
- Edge Cases & Error Handling:
  - 回放不一致：立即标记高风险阻断并输出差异快照。
  - 接口超时/失败：WASM 执行异常需返回结构化错误而非 panic。
  - 空事件流：空输入需稳定返回，无副作用写入。
  - chain-linked 空轮询：`/v1/chain/status` 轮询若未观察到新的 `committed_height`，或新的 execution world 未产生新增 event / logical time，则 viewer 只能保持当前快照，不得伪造“自动推进中”的 world 进度。
  - 权限不足：未授权模块请求直接拒绝并记录审计事件。
  - 并发冲突：治理操作并发时按版本序列化处理，拒绝乱序变更。
  - 数据异常：receipt 校验失败时不得推进状态并触发安全告警。
  - 存储异常：GC/保存中断时必须保留 latest recoverable head，禁止“先删后写”导致不可恢复状态。
  - 构建漂移：若同一 Docker builder 在不同宿主上产出不同 canonical hash，或 receipt/identity 不一致，必须在进入 runtime 执行前被 gate 阻断。
  - 生产入口未启用 release security policy：必须在发布验证中被标记为 `no-go`，因为此时 builtin manifest fallback / 本地签名 / runtime source compile 仍可能留在热路径。
  - 默认 loopback replication fallback 抢占 consensus 广播：若节点已配置 UDP gossip peers，则该 fallback 不得阻止 proposal / attestation / commit 继续经 gossip 交换，否则多机 PoS 会表现为各自推进高度但 `known_peer_heads=0`。
  - GitHub-hosted macOS runner 无 Docker daemon：允许将 CI 临时收敛为 Linux-only stable gate，但必须把跨宿主 canonical evidence 标记为未完成，并要求外部 Docker-capable macOS summary/import 继续补证。
  - 执行器初始化失败：WASM executor / SDK wire 初始化或解码失败必须向宿主或调用者返回结构化错误，不得在平台层静默吞错。
- Non-Functional Requirements:
  - NFR-WR-1: 同一输入回放结果一致率 100%。
  - NFR-WR-2: 关键治理事件审计链路完整率 100%。
  - NFR-WR-3: WASM 接口变更需保持向后兼容或明确破坏性声明。
  - NFR-WR-4: 安全相关回归在 full 层级覆盖率达到目标阈值并持续跟踪。
  - NFR-WR-5: 核心运行时异常可在 30 分钟内完成初步定位。
  - NFR-WR-6: 默认开发/启动器 profile 必须定义明确磁盘预算、保留窗口与 metrics 输出，不得依赖手工清目录维持可用性。
  - NFR-WR-7: retention policy 保留范围内的 replay success rate 必须为 `100%`，且重建 `execution_state_root` 与原记录一致。
  - NFR-WR-8: 同一 commit、同一 Docker builder image digest 下的 canonical packaged wasm hash 可复现率必须为 `100%`，且 drift 失败输出必须定位到 `module_id/builder_image_digest/expected/actual`。
  - NFR-WR-9: production runtime / node 默认策略必须保证 `allow_builtin_manifest_fallback=false`、`allow_identity_hash_signature=false`、`allow_local_finality_signing=false`、`allow_runtime_source_compile=false`；任何放宽都必须通过显式 dev/test 配置进入。
  - NFR-WR-10: `TASK-WORLD_RUNTIME-043` 完成前，发布文档与 gate 摘要必须明确标示“Linux-only stable gate”与“cross-host evidence pending”的区别，避免误报 closure。
  - NFR-WR-11: 网络流量观测快照必须在线程间安全可读、随节点生命周期累积，并在 payload 中明确标注统计范围，避免将逻辑 payload 计数误报成完整 wire-bandwidth 真值。
  - NFR-WR-12: `fetch-commit` 失败退避必须保持 peer-scoped、protocol-scoped 且自动过期，默认只压制短时高频重试，不得把业务层永久不兼容或其他协议的错误扩大成全局节点隔离。
  - NFR-WR-13: peer-record/discovery 请求冷却必须局限于同 peer 的单协议短窗抑制，默认只减少重复取件噪音，不得把一次 cached-peer-record cache miss 扩大成长期 peer 隔离，也不得阻断同一请求链中的备选 proxy fallback。
  - NFR-WR-14: storage challenge `fetch-blob` success cache 必须只复用短窗口内的正向验证结果，默认只减少最近重复取件噪音，不得把过期网络可达性证明长期当真，也不得掩盖本地 blob 缺失等硬失败。
  - NFR-WR-15: chain-linked gameplay submit 必须保持“提交成功不等于本地 world 立即可见”的 committed-only 可见性边界，viewer 只能在 committed execution world sync 之后反映新状态，不得因本地 optimistic mutation 伪造链上结果。
  - NFR-WR-16: `fetch-commit` success cache 必须只复用近期、相同 deterministic request payload 的正向响应，不得缓存 `found=false` 或协议错误，也不得把 peer-scoped 失败冷却扩大成全局成功假象。
- Security & Privacy: 强制最小权限、签名校验、审计留痕；禁止未授权模块绕过规则层直接修改世界状态。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化 runtime 模块设计基线与边界约束。
  - v1.1: 补齐 WASM 生命周期与治理流程的跨模块验收清单。
  - v2.0: 建立运行时质量趋势报表（确定性、安全、性能、治理事件完整度）。
- Technical Risks:
  - 风险-1: 运行时复杂度提升导致验证成本增加。
  - 风险-2: ABI/治理策略变更引发兼容性断裂。
  - 风险-3（2026-03-18 记录，2026-03-31 复核，P0 未收口）: `TASK-WORLD_RUNTIME-043` 当前只有 GitHub-hosted `linux-x86_64` stable gate；本地 `runtime_engineer` 复核环境为 `Linux x86_64 + Docker(linux/x86_64)`，只能验证导入/打包/proof 工具链，仍无法产出真实 Docker-capable `darwin-arm64` full-tier evidence，因此跨宿主 closure 继续保持外部 live 证据阻塞态。
  - 风险-4（2026-03-18 记录，2026-03-31 已收口）: `ReleaseSecurityPolicy` 的生产禁 fallback / 禁本地签名 / 禁 runtime source compile 不再依赖额外 `enable_production_release_policy()` 约定；production-facing `chain runtime execution world` 装载、`reward runtime worker`、`viewer runtime_live` bootstrap 与 `governance_registry_import` 新建/加载路径均已默认绑定 hardened policy。
  - 风险-5（2026-03-31 已收口）: `apply_domain_event_gameplay*` 中 replay / preflight / 恢复链路残留的 panic-style `expect(...)` 已改为结构化 `WorldError`，损坏事件与缺失 actor 回归已补齐，不再把状态漂移故障降级为 panic。
  - 风险-6（2026-03-31 已收口）: runtime 热路径超限文件已按语义边界拆分，`action_to_event_core.rs` 与 `apply_domain_event_main_token.rs` 均已回到治理线内，并通过编译/定向回归验证“拆文件不改语义”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-001 | TASK-WORLD_RUNTIME-001/002/005/049 | `test_tier_required` + `test_tier_full` | 回放一致性、核心边界验收清单校验 | 世界状态演化与确定性语义 |
| PRD-WORLD_RUNTIME-002 | TASK-WORLD_RUNTIME-002/003/005/006/054 | `test_tier_required` | WASM 接口兼容性检查、治理流程测试、builtin wasm `sha256` 与 identity 清单一致性校验、初始化失败结构化错误回归 | 模块升级、工件治理与生命周期稳定性 |
| PRD-WORLD_RUNTIME-003 | TASK-WORLD_RUNTIME-003/004/005 | `test_tier_full` | 收据签名校验、安全回归抽样 | 审计可信性与安全边界 |
| PRD-WORLD_RUNTIME-013 | TASK-WORLD_RUNTIME-030/031/032/034 | `test_tier_required` | retention / GC / footprint budget 回归 | execution bridge、execution world、CAS 持久化 |
| PRD-WORLD_RUNTIME-014 | TASK-WORLD_RUNTIME-030/031/032/033/034 | `test_tier_required` + `test_tier_full` | latest-state restart、checkpoint replay、GC fail-safe、checkpoint 保留验证 | 恢复能力与审计链路 |
| PRD-WORLD_RUNTIME-015 | TASK-WORLD_RUNTIME-030/032/033/034 | `test_tier_required` | storage profile / metrics / archive read / launcher 脚本透传验证 | 发布链路可观测性与环境预算治理 |
| PRD-WORLD_RUNTIME-019 | TASK-WORLD_RUNTIME-038 | `test_tier_required` | 工厂生产阻塞/恢复/完成状态回归、事件历史可解释性断言 | 前期工业引导、Viewer 工业反馈、QA playability 解释链 |
| PRD-WORLD_RUNTIME-020 | TASK-WORLD_RUNTIME-041/042 | `test_tier_required` | Docker builder image、containerized canonical packaging、single canonical token 验证 | wasm publish build entry、容器环境收敛 |
| PRD-WORLD_RUNTIME-021 | TASK-WORLD_RUNTIME-041/042/043 | `test_tier_required` | build receipt / identity / release evidence 绑定验证 | 工件治理、发布证据与社会层可验证性 |
| PRD-WORLD_RUNTIME-022 | TASK-WORLD_RUNTIME-041/043/044 | `test_tier_required` + `test_tier_full` | multi-runner Docker compare、source compile 外移或 gated、runtime binary-only policy 验证；production entry 必须补 release policy 绑定证据 | build drift 阻断、执行前合法性与源码包发布边界 |
| PRD-WORLD_RUNTIME-023 | TASK-WORLD_RUNTIME-054 | `test_tier_required` | 生产入口 policy 绑定审计、默认构造路径巡检、release profile 回归 | runtime 默认安全边界、发布入口一致性 |
| PRD-WORLD_RUNTIME-024 | TASK-WORLD_RUNTIME-055 | `test_tier_required` | 损坏事件 / 缺失 actor 回归、preflight/replay 错误语义断言 | 恢复链路、事件预演、结构化故障定位 |
| PRD-WORLD_RUNTIME-025 | TASK-WORLD_RUNTIME-056 | `test_tier_required` + `test_tier_full` | 热路径文件长度治理、拆分后 determinism / replay / persistence 回归 | 规则演进可维护性、运行时热路径复杂度 |
| PRD-WORLD_RUNTIME-026 | TASK-WORLD_RUNTIME-061 | `test_tier_required` | `/v1/chain/status` 流量快照、UDP gossip / libp2p replication counters、范围说明与定向回归 | 节点带宽归因、运行态网络可观测性 |
| PRD-WORLD_RUNTIME-027 | task_370ce55ed73a490797055403164e8f41 | `test_tier_required` | triad traffic history 持久化、最近 N 分钟窗口汇总、`observed_since_unix_ms` reset-aware baseline 选择、live triad 短窗口采样验证 | 节点最近窗口流量归因、triad 运营监控可回答性 |
| PRD-WORLD_RUNTIME-028 | task_7863b156fce3484481310b33a263cc7c | `test_tier_required` | repo-owned node start wrapper、env-gated local traffic monitor、dry-run CLI 装配验证、单节点 live status 采样验证 | 节点级监控开关、真实环境启动入口可维护性 |
| PRD-WORLD_RUNTIME-029 | task_df0a42e3efea4806bb3f41245c1ef4d5 | `test_tier_required` | `fetch-commit` peer cooldown 定向回归、协议级范围约束断言、`doc-governance-check` 与 `git diff --check` | libp2p 复制流量浪费收口、真实 triad gap-sync 降噪 |
| PRD-WORLD_RUNTIME-030 | task_a28db8372d864bde9a9c5ea508bd7824 | `test_tier_required` | `oasis7_net` peer-record/discovery cooldown 定向回归、fallback proxy 连续性断言、`doc-governance-check` 与 `git diff --check` | peer-record/discovery 请求流量降噪、真实 triad 连续触发抑制 |
| PRD-WORLD_RUNTIME-031 | task_c1149e15fef14f12925182a03f37e546 | `test_tier_required` | `oasis7_viewer_live` chain-linked 被动跟随回归、不开 `Play` 的 committed world 同步、空轮询不推进断言 | viewer live 与 chain runtime 的逻辑 world progress 一致性 |
| PRD-WORLD_RUNTIME-032 | task_53b1918a361445f5bf678bcf525abc5c | `test_tier_required` | storage challenge `fetch-blob` success cache 定向回归、缓存过期恢复探测断言、`doc-governance-check` 与 `git diff --check` | triad `fetch-blob` 重复成功拉取降噪、sequencer↔storage 热点流量收口 |
| PRD-WORLD_RUNTIME-033 | task_dd49ad3480d14922993ceb3acf2555c6 | `test_tier_required` | `/v1/chain/gameplay/submit` handler 回归、viewer chain-linked gameplay submit 回归、`cargo check`、`git diff --check` | viewer gameplay action 与 chain runtime committed world 的一致性闭环 |
| PRD-WORLD_RUNTIME-034 | task_5b736236fdf5404099ef1d1aec37beb1 | `test_tier_required` | `fetch-commit` success cache 定向回归、缓存过期恢复请求断言、`found=false` 不缓存断言、`doc-governance-check` 与 `git diff --check` | triad `fetch-commit` 重复成功拉取降噪、gap-sync 紧环路请求收口 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WR-001 | 以确定性回放作为运行时验收核心 | 仅执行结果正确即可 | 可追溯和审计需求要求强确定性。 |
| DEC-WR-002 | WASM 生命周期走治理流程 | 模块直接热替换 | 无治理热替换难以保证安全与一致性。 |
| DEC-WR-003 | 安全事件必须输出可验证 receipt | 仅日志文本记录 | 签名收据可支撑事后审计与取证。 |
