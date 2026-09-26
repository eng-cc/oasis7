# oasis7 Runtime：WASM 可观测性与耗时指标设计

- 专业 owner：`wasm_platform_engineer`；状态：active professional authority（目标合同不等于已实现）。
- 内容审读基线：`eng-cc/oasis7`，源提交 `9c41d57b4436f71f0cf9b481043d882cfdc551ed`；本次文档采纳任务：[Issue 4102](https://github.com/eng-cc/oasis7/issues/4102)，Task UID `task_9f4c3a6c2daa40a0ae3e2c86bdf35e02`；内容整理日期：2026-09-26。
- 独立审读与实际实现/测试候选证据由该 task evidence 记录；此处不预报审读通过、runtime capability、发布或 full proof。下方既有条款仍有效；仅显式日期/历史基线条款按其证据范围解释。

## 1. 问题、目标与非目标

本次把既有专业义务承接为可执行的 target contract 与验证设计。成功结果是精确身份、边界、失败和恢复算法能被消费者读取；不改变 ABI/code/经济价格，不宣称目标已落地。下方原有细节、limits、公式、ID 与历史 evidence 原序保留并继续约束其适用范围。

## 2. 上游约束与相关角色

产品端到端 authority 为 [deterministic world execution](../../product/world-infrastructure/deterministic-world-execution.prd.md) 与 [distributed state availability](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md)；专业细化由 paired PRD、runtime/module 与 P2P 合同拥有。WASM owns codec/artifact/metering，runtime owns staged publication/replay，QA owns same-candidate acceptance，consumer owners own real projection。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [OBS-01](wasm-observability-timing-metrics.prd.md#sr2-domain-case-1) | 输入 restart窗口/reset、counter回退、zero traffic；断言 checked delta缩窗、reset=true、无fabricated p95 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | runtime/QA；[OBS-01外部receiver](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | 当前localfixture不能证明full finality/provider/headed |
| [OBS-02](wasm-observability-timing-metrics.prd.md#sr2-domain-case-2) | 输入 poisoned metrics lock、top11、large/default payload；断言 degraded而主调用继续；top10+truncated；64/128KiB budgets | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | runtime/QA；[OBS-02外部receiver](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | 当前localfixture不能证明full finality/provider/headed |
| [OBS-03](wasm-observability-timing-metrics.prd.md#sr2-domain-case-3) | 输入 dry_run无timing、repeat0、private payload、timing变化；断言 显式missing/error；无secret；execution ID/charge不变 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | runtime/QA；[OBS-03外部receiver](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | 当前localfixture不能证明full finality/provider/headed |
| [SC-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-1) | SC-1: /v1/chain/status.wasm 必须稳定暴露 build/executor/router 三组 machine-readable 指标，并与节点生命周期累计同步。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-2) | SC-2: 执行器必须区分 memory cache hit、disk cache hit、compile miss 三条路径，并提供对应的 wall-clock timing 聚合。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-3) | SC-3: build suite 必须输出 total_build_wall_ms 与至少 3 个阶段耗时字段，确保 canonical build 不再只有 hash/size 而无 cost 证据。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-4) | SC-4: router 必须提供 prepared subscription 与 parse-each-time 的 timing 对比入口，保证 TASK-WORLD_RUNTIME-060 的优化收益能持续复核。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-5) | SC-5: 指标设计不得把 trace_id、原始 payload 或无界 module_id 明细直接写入默认 status payload，默认口径需满足 bounded cardinality 与 deterministic isolation。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [PRD-WORLD_RUNTIME-036](wasm-observability-timing-metrics.prd.md#sr2-obligation-prd-world_runtime-036) | PRD-WORLD_RUNTIME-036: As a wasm_platform_engineer / runtime_engineer / qa_engineer, I want the WASM build, executor, and router paths to emit bounded cumulative timing metrics and status snapshots, so that hotspot attribution no longer depends on ad hoc logs or ignored local perf probes. | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-1) | AC-1: tools/wasm_build_suite 必须为 canonical build 输出 total_build_wall_ms 与阶段耗时字段，且 dry-run 与真实构建在 schema 上可区分。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-2) | AC-2: oasis7_wasm_executor 必须区分 memory-cache hit、disk-cache hit、compile miss 三类路径，并为 compile/deserialize/instantiate/entrypoint/decode 提供累计耗时或固定 bucket。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-3) | AC-3: oasis7_wasm_router 必须输出 subscription prepare 与 filter match 的 timing/counter 指标，至少覆盖 prepared-hit、parse-fallback 与 regex-compile 三类信号。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-4) | AC-4: /v1/chain/status 必须新增 wasm section，并沿用 storage/traffic 的共享 snapshot 语义，不得把 timing 指标直接写入世界状态或共识数据。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-5) | AC-5: 默认 status payload 不得包含 trace_id、原始 input/output bytes、无界 module_id -> metrics map；若需要模块级明细，必须限制为 top-N 或显式 allowlist。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-6](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-6) | AC-6: 节点或 status lock 降级时，degraded_reason 必须显式说明 WASM metrics 不可用，但模块执行本身不得因此失败。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-7](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-7) | AC-7: repo 内必须提供可复用的 summary 入口，把 cumulative snapshot 转成窗口 delta、bucket-derived p50/p95 与热点摘要。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-8](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-8) | AC-8: 观测设计必须与现有 TASK-WORLD_RUNTIME-060 perf probe、chain-status-traffic-metrics status snapshot 模式、runtime-release-gate-metrics-template 兼容，不得再引入一套平行口径。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-1) | NFR-1: 默认 /v1/chain/status.wasm payload 必须保持 bounded cardinality，未启用模块级 top-N 时不得随模块总数线性增长。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-2) | NFR-2: 观测写入不得改变 deterministic execution 输出；所有 timing/metrics 仅限本地观测层，不得进入共识数据、world state 或 replay contract。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-3) | NFR-3: 默认状态下，WASM metrics instrumentation 对现有 release perf probe 的额外 wall-clock 开销目标不高于 10%。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-4) | NFR-4: status payload 在默认配置下必须可在单次请求中稳定序列化，建议预算 <=64 KiB；开启 bounded top-N 明细后建议预算 <=128 KiB。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-5) | NFR-5: 计时字段必须统一使用毫秒或微秒语义并显式标注单位，禁止不同子系统混用无单位整数。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [NFR-6](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-6) | NFR-6: 模块级明细若存在，默认上限不得超过 top 10，且不得包含原始 payload 内容。 | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [PRD-WORLD_RUNTIME-037](wasm-observability-timing-metrics.prd.md#sr2-obligation-prd-world_runtime-037) | &#124; PRD-WORLD_RUNTIME-037 &#124; wasm-module-observability-standardization &#124; test_tier_required &#124; observe runner tests、代表模块 spec、wrapper shell check、JSON/Markdown summary &#124; 模块级 contract/perf 证据与新模块接入 &#124; | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前状态 / 基线 | 目标 | 差距与证据边界 |
| --- | --- | --- | --- |
| wasm-1 foundation | 原有代码/契约保留，源commit见身份块 | 兼容consumer及精确target contracts | 原有tests为located，不是本次run |
| receipt / migration / gate | interface仍声明target或partial | `sr2-metrics-contract` 与runtime authority相连 | 代码、full finality/provider/headed证明不在本任务 |
| 文档与schema | Issue4102采纳目标合同 | 语义/结构/anchors检查 | 文档通过不意味着runtime pass或SC9/DC5闭合 |

## 4. 边界与结构

输入声明和provider response为advisory，host/runtime负责authority/admission；sandbox仅确定性计算。build证明字节来源、SDK证明codec、metrics证明local诊断，各自不能签署world finality。ContentRef必须fetch/hash/length验证，再由相应authority验trust；current caches与local signature不能越过生产许可。

## 5. 关键运行流程

先绑定candidate/world branch/window与exact manifest/artifact/schema；strict decode和limits preflight后验证authority。成功只进入有权的下一阶段，世界效果须canonical commit receipt。缺字段、版本、proof、兼容或head continuity即停止，不隐式fallback或重试另一身份。重试同request hash幂等，changed hash拒绝；恢复重读当前canonical边界，历史receipt按记录版本重放。

## 6. 接口与数据合同

<a id="sr2-metrics-contract"></a>
### 6.1 本地 observation window 合同

WasmObservationWindowV1 是 target diagnostic projection：`{schema_version:"WasmObservationWindowV1",node_id:Id,process_window_id:Id,observed_since_unix_ms:U64,sample_start_unix_ms:U64,sample_end_unix_ms:U64,metrics_available:bool,degraded_reason?:Id,window_reset_detected:bool,truncated:bool,delta_calls:U64,delta_compile_ms:U64,delta_timeout_count:U64,executor_p50_call_ms?:U64,executor_p95_call_ms?:U64,router_p50_match_ms?:U64,router_p95_match_ms?:U64,top_hotspots:[{module_id:Id,match_ms:U64}]}`。新 projection 的 U64 JSON 为 decimal strings，现行 metrics endpoint number format 不在本文偷偷改变。所有值是毫秒/计数的本地诊断，不进入 execution/receipt/manifest consensus core 或计费。

同 node/process 窗口以 cumulative snapshot 求 checked delta；observed_since 改变或 counter 回退即缩窗且标 reset，不跨 restart 求负 delta。start<=end；no traffic 样本不给 percentile；bucket-derived percentile 只能从同窗口固定 buckets 计算且注明 bucket 精度，不能伪造精确 latency。metrics lock poisoned/unavailable 返回 bounded degraded reason，执行主路径继续；top_hotspots<=10，裁剪显式 truncated，禁止 raw payload/trace/keys/auth proof。默认 status<=64KiB、top-N<=128KiB、instrumentation overhead目标<=10% 沿用 PRD NFR，不用本次未运行宣称达标。

当前已有 code locations 为 build suite timing、executor/router shared snapshots、status_payload.wasm 和 monitor/module_observe。它们仅 located implementation anchors：`tools/wasm_build_suite/src/lib.rs`、`crates/oasis7_wasm_executor/src/lib.rs`、`crates/oasis7_wasm_router/src/lib.rs`、`crates/oasis7/src/bin/oasis7_chain_runtime/status_payload.rs`、`scripts/oasis7-node-wasm-metrics-monitor.test.sh`、`tools/wasm_module_observe`；本次未执行。module observe contract/perf 不等于 functional/release/DC5 proof。观测 restart/reset、zero traffic、degraded lock、top11 clipping、dry_run missing timing、repeat0 rejection、changed locator equal bytes 必须各有 fixture。新字段 consumer 显式版本协商；rollback 原 reader 不能静默误解新版本，也不得影响执行状态。


## 7. 状态、事务与持久化

本域记录不能把 accepted/built/decoded/observed 当 applied。SDK/build/metrics 的本地输出不拥有世界事务；interface定义receipt core，runtime staged commit拥有state/effects/charges/registry/journal/winner同一发布点。失败全体不发布，crash在prepublish与postpublish由durable receipt/winner/index恢复，禁止重跑已commit WASM。实际prototype atomic seam仅bounded prepared proposal，不能推导完整事务。snapshot保留原manifest/artifact/schedule/lineage与历史bytes。

## 8. 部署、安全与运行约束

生产binary-only及external finality仍由release/runtime policy；无Docker/wasm32/provider环境不能伪报通过。modules无clock/random/I/O，host注入caller/time/target；namespace/capability/AUTHZ约束全部仍有效。计数、字节和费用checked bounds-before-allocation/mutation，limit来源为现行ModuleLimits与各paired PRD；diagnostics不得泄露raw payload/keys/proof。

## 9. 质量与容量

| 环境 / 输入 | 预期响应 | 阈值来源 | 验证入口 / 当前范围 |
| --- | --- | --- | --- |
| required canonical fixture / malformed bytes、integer overflow、wrongscope | structured rejection且不发布state/effect/charge | interface类型、现行limits及paired PRD | §11.1各case；target设计未运行 |
| full实际canonical环境 / restart、activation、proof撤销 | durable replay/identity与no-new-effect gate一致 | runtime/GWSC各精确receiver | samecandidate artifacts mandatory，未证明 |
| 现行资源/perf输入 | 不扩大既有budget，不用localtiming计费 | 下方原有limits、NFR与perf方案 | 既有tests/scripts仅located partial |

## 10. 兼容、迁移与回滚

保留legacy serde defaults、wire exports、工件不可覆盖与历史bytes。新target先固定合同再逐consumer启用，unknown version拒绝；当前ABI不变。回滚到身份块源码/旧consumer，仅影响新请求；已canonical commit的事件、charges、receipt不可撤销或改价。迁移须explicit from/to descriptor及governed artifact；缺proof停在旧版，不能以代码合入或健康恢复启用。

## 11. 验证设计与可追溯性

Issue4102只交付文档合同采纳与local checks；实际实现、runtime required/full、provider、headful、production proof分别在真实实现task evidence记录candidate/source/integration/tested tree与环境，缺失保持未证明，不能用FutureIssue占位代替本页receiver。全DC5验证由 [GWSC receiver](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-dc5-attachment) 与其 [schema](../../testing/schemas/world-execution-state-sync-attachment.schema.json) 拥有。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [OBS-01](wasm-observability-timing-metrics.prd.md#sr2-domain-case-1) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | checked delta缩窗、reset=true、无fabricated p95 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；target case `OBS-01` 输入 restart窗口/reset、counter回退、zero traffic；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [OBS-02](wasm-observability-timing-metrics.prd.md#sr2-domain-case-2) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | degraded而主调用继续；top10+truncated；64/128KiB budgets | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；target case `OBS-02` 输入 poisoned metrics lock、top11、large/default payload；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [OBS-03](wasm-observability-timing-metrics.prd.md#sr2-domain-case-3) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | 显式missing/error；无secret；execution ID/charge不变 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；target case `OBS-03` 输入 dry_run无timing、repeat0、private payload、timing变化；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [SC-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-1) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | SC-1: /v1/chain/status.wasm 必须稳定暴露 build/executor/router 三组 machine-readable 指标，并与节点生命周期累计同步。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-2) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | SC-2: 执行器必须区分 memory cache hit、disk cache hit、compile miss 三条路径，并提供对应的 wall-clock timing 聚合。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-3) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | SC-3: build suite 必须输出 total_build_wall_ms 与至少 3 个阶段耗时字段，确保 canonical build 不再只有 hash/size 而无 cost 证据。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-4) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | SC-4: router 必须提供 prepared subscription 与 parse-each-time 的 timing 对比入口，保证 TASK-WORLD_RUNTIME-060 的优化收益能持续复核。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-sc-5) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | SC-5: 指标设计不得把 trace_id、原始 payload 或无界 module_id 明细直接写入默认 status payload，默认口径需满足 bounded cardinality 与 deterministic isolation。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [PRD-WORLD_RUNTIME-036](wasm-observability-timing-metrics.prd.md#sr2-obligation-prd-world_runtime-036) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | PRD-WORLD_RUNTIME-036: As a wasm_platform_engineer / runtime_engineer / qa_engineer, I want the WASM build, executor, and router paths to emit bounded cumulative timing metrics and status snapshots, so that hotspot attribution no longer depends on ad hoc logs or ignored local perf probes. | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-1) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-1: tools/wasm_build_suite 必须为 canonical build 输出 total_build_wall_ms 与阶段耗时字段，且 dry-run 与真实构建在 schema 上可区分。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-2) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-2: oasis7_wasm_executor 必须区分 memory-cache hit、disk-cache hit、compile miss 三类路径，并为 compile/deserialize/instantiate/entrypoint/decode 提供累计耗时或固定 bucket。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-3) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-3: oasis7_wasm_router 必须输出 subscription prepare 与 filter match 的 timing/counter 指标，至少覆盖 prepared-hit、parse-fallback 与 regex-compile 三类信号。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-4) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-4: /v1/chain/status 必须新增 wasm section，并沿用 storage/traffic 的共享 snapshot 语义，不得把 timing 指标直接写入世界状态或共识数据。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-5) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-5: 默认 status payload 不得包含 trace_id、原始 input/output bytes、无界 module_id -> metrics map；若需要模块级明细，必须限制为 top-N 或显式 allowlist。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-6](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-6) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-6: 节点或 status lock 降级时，degraded_reason 必须显式说明 WASM metrics 不可用，但模块执行本身不得因此失败。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-7](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-7) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-7: repo 内必须提供可复用的 summary 入口，把 cumulative snapshot 转成窗口 delta、bucket-derived p50/p95 与热点摘要。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-8](wasm-observability-timing-metrics.prd.md#sr2-obligation-ac-8) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | AC-8: 观测设计必须与现有 TASK-WORLD_RUNTIME-060 perf probe、chain-status-traffic-metrics status snapshot 模式、runtime-release-gate-metrics-template 兼容，不得再引入一套平行口径。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-1](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-1) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-1: 默认 /v1/chain/status.wasm payload 必须保持 bounded cardinality，未启用模块级 top-N 时不得随模块总数线性增长。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-2](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-2) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-2: 观测写入不得改变 deterministic execution 输出；所有 timing/metrics 仅限本地观测层，不得进入共识数据、world state 或 replay contract。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-3](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-3) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-3: 默认状态下，WASM metrics instrumentation 对现有 release perf probe 的额外 wall-clock 开销目标不高于 10%。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-4](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-4) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-4: status payload 在默认配置下必须可在单次请求中稳定序列化，建议预算 <=64 KiB；开启 bounded top-N 明细后建议预算 <=128 KiB。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-5](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-5) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-5: 计时字段必须统一使用毫秒或微秒语义并显式标注单位，禁止不同子系统混用无单位整数。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [NFR-6](wasm-observability-timing-metrics.prd.md#sr2-obligation-nfr-6) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | NFR-6: 模块级明细若存在，默认上限不得超过 top 10，且不得包含原始 payload 内容。 | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [PRD-WORLD_RUNTIME-037](wasm-observability-timing-metrics.prd.md#sr2-obligation-prd-world_runtime-037) | [本域合同](wasm-observability-timing-metrics.design.md#sr2-metrics-contract) | &#124; PRD-WORLD_RUNTIME-037 &#124; wasm-module-observability-standardization &#124; test_tier_required &#124; observe runner tests、代表模块 spec、wrapper shell check、JSON/Markdown summary &#124; 模块级 contract/perf 证据与新模块接入 &#124; | [现有partial source](../../../scripts/oasis7-node-wasm-metrics-monitor.test.sh)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |

现有partial/unrun来源：`crates/oasis7_wasm_executor/src/tests.rs`（output/fuel/memory/cache），`crates/oasis7_wasm_abi/tests/open_module_commands.rs`（legacy/envelope），`crates/oasis7/src/runtime/world/module_runtime_tests.rs`（metrics/due-preflight），`scripts/oasis7-node-wasm-metrics-monitor.test.sh`（window summary），`scripts/ci-verify-m1-wasm-summaries.py`（build对账）。这些不是新target receipt/migration/DC5已覆盖证据。GWSC [negative matrix](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-qa-negative-matrix) 接收全部17QA cases，[runtime scenarios](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-runtime-scenarios) 接收10组runtime；本页domain rows为具体输入/assertion分配。

## 12. 决策、长期风险与未决问题

选择单authority typedtarget而非重复wire，是为避免scope/hash/版本漂移。已知风险是implementation缺失、proof不可读、U64/UTF8投影误读、递归hash与历史误作新候选证据；WASM/runtime/QA分别按本页case和runtimereceiver解除。触发器为新ABI/schedule/schema、trust epoch、activation或consumer变更；需重新冻结合同与专业联审。SC9八fullcells及activeLLM/provider API parity、真实desktop+narrow/headed证据保持产品/QAauthority，不因本页文档完成消除。


- 对应需求文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- 稳定证据入口: `doc/world-runtime/wasm/evidence.md`

审计轮次: 1

## 1. 设计定位
本设计把 WASM 体系里“只能靠 ignored perf probe 和临时日志回答”的性能问题，收敛为正式 runtime 观测面。目标不是替代本地 benchmark，而是提供一条长期可复用的 `build -> executor -> router -> status -> summary` 证据链，让 release candidate、CI incident 与节点侧热点都用同一套字段归因。

实时样本、release candidate 结论和任务推进状态不属于本设计；它们由 GitHub
task issue / Project 维护。稳定 schema、边界和验证入口由本文、PRD 与
[`evidence.md`](evidence.md)共同承接。

## 2. 现状盘点

**历史基线限定：** 下表是原观测专题启动时的缺口盘点，原文按历史事实保留；不是2026-09-26当前absence断言。当前已存在的shared snapshot/status/monitor/module-observe代码locators及本次未运行边界见 [当前内容基线](#sr2-metrics-contract)。

| 层级 | 当前实现事实 | 观测缺口 |
| --- | --- | --- |
| build suite | `tools/wasm_build_suite` 已输出 `source_hash/build_manifest_hash/wasm_hash/wasm_size_bytes` | 没有 `build_wall_ms/cargo_build_ms/canonicalize_ms`，无法判断 canonical build 成本 |
| executor | `compile_module_cached()` 已天然区分 memory hit、disk hit、compile miss | 这些路径没有累计 timing/counter snapshot，只有零散 `Instant` 与失败码 |
| router | 本地 perf probe 已能测 `parse_each_time -> prepared_once` | 结果只停留在 ignored test `eprintln!`，没有持续观测 surface |
| status | `/v1/chain/status` 已有 `storage` 与 `traffic` 共享 snapshot 模式 | 还没有 `wasm` section，无法在节点侧看到 executor/router/build 热点 |
| summary | 现有 triad traffic monitor 已能把 cumulative counter 转窗口 delta | wasm 还没有对应 summary 入口，无法持续输出 p50/p95/top hotspot |

## 3. 设计原则
- 原则-1：指标必须复用既有 `shared snapshot -> status payload -> external summary` 模式，不再平行创造另一套日志协议。
- 原则-2：wall-clock timing 只属于本地观测层，绝不能进入 deterministic world state、event log 或共识数据。
- 原则-3：默认 surface 只暴露 bounded counters、sums、buckets；模块级明细必须是 top-N 或 allowlist，而非全量 map。
- 原则-4：先解决“归因可回答”，再追求极致精细度；首期优先覆盖 build、compile/cache、executor call、router match 四类热点。

## 4. 目标态架构

```text
tools/wasm_build_suite
  -> build timing metadata / receipt
           |
           v
oasis7_wasm_executor -----+
  -> cache/compile/call   |
           |              |
           v              |
oasis7_wasm_router        |
  -> prepare/match timing |
           |              |
           +------> shared wasm metrics snapshot
                              |
                              v
                 /v1/chain/status.wasm
                              |
                              v
             repo-owned window summary script
                              |
                              v
                 summary.md / summary.json
```

关键变化：
- build receipt 从“只有 hash/size”升级为“hash/size + timing”。
- executor/router 从“一次性 perf probe”升级为“进程级累计 snapshot”。
- `/v1/chain/status` 从“storage/traffic only”升级为“storage/traffic/wasm 三面并列”。

## 5. 详细设计

### 5.1 数据面分层
- build 层：
  - 写入 `metadata.json` / `build-receipt.json`
  - 适合记录单次构建耗时
- runtime 层：
  - 维护 `SharedWasmMetricsSnapshot`
  - 适合记录节点生命周期内累计 counters/sums/buckets
- summary 层：
  - 读取 status samples
  - 适合计算窗口 delta、bucket 派生 p50/p95、top hotspot

这三层明确分工，避免“单次构建耗时”和“进程累计执行耗时”混在一个 schema 里。

### 5.2 Build Timing Contract
`tools/wasm_build_suite` 新增 timing 字段：
- `total_build_wall_ms`
- `cargo_build_ms`
- `canonicalize_ms`
- `hash_ms`
- `receipt_write_ms`
- 可选：`metadata_write_ms`

约束：
- 字段必须进入 metadata 与 receipt 的 machine-readable schema。
- `dry_run=true` 时允许部分字段为空，但必须显式区分，不得伪造 `0ms success`。
- timing 单位统一使用毫秒。

### 5.3 Runtime Snapshot Contract
新增 `WasmMetricsSnapshot`，建议与 `StorageMetricsSnapshot` 风格对齐：
- `observed_since_unix_ms`
- `metrics_available`
- `degraded_reason`
- `build`: 最近一次构建或导入的 bounded timing 摘要
- `executor`: 累计 counters/sums/buckets
- `router`: 累计 counters/sums/buckets

其中 `executor` 至少包括：
- `calls_total`
- `memory_cache_hits`
- `disk_cache_hits`
- `compile_misses`
- `failure_by_code`
- `compile_ms_total`
- `deserialize_ms_total`
- `instantiate_ms_total`
- `entrypoint_call_ms_total`
- `decode_ms_total`
- `call_wall_ms_buckets`

其中 `router` 至少包括：
- `prepare_subscriptions_ms_total`
- `match_filters_ms_total`
- `regex_compile_ms_total`
- `prepared_hits`
- `parse_fallbacks`
- `match_wall_ms_buckets`

### 5.4 Executor Instrumentation Points
`oasis7_wasm_executor` 的关键埋点：
- `compile_module_cached()`
  - memory cache lookup
  - disk cache load
  - deserialize compiled artifact
  - compile miss
  - serialize/store compiled artifact
- `call()`
  - instantiate
  - input write
  - entrypoint call
  - output read
  - output decode
  - total wall clock

设计约束：
- timing 记录在本地变量结束后统一更新 snapshot，减少锁持有时间。
- 失败路径也要记 timing 与 failure code，避免只统计成功样本。
- snapshot 更新必须使用短临界区；锁失败时仅打 `degraded_reason`，不影响主执行流程。

### 5.5 Router Instrumentation Points
`oasis7_wasm_router` 的关键埋点：
- `prepare_subscriptions()`
- `prepare_subscription_filters()`
- regex compile cache miss
- `prepared_module_subscribes_to_event/action()`

设计重点：
- router 默认不输出全量 `module_id -> timing` 明细。
- 若后续需要模块级热点，建议先提供 bounded `top_modules_by_match_ms`，并限制为 `top 10`。

### 5.6 Status Payload Exposure
`oasis7_chain_runtime` 新增：
- `status_payload.rs` 中的 `wasm: WasmMetricsSnapshot`
- 构建/刷新 `snapshot_wasm_metrics(...)`

语义与现有 storage/traffic 保持一致：
- 状态接口只读
- 返回 cumulative snapshot
- 外部脚本自行计算窗口 delta

不做的事：
- 不在 status 接口直接输出 p95
- 不在 status 接口直接输出全量原始时序
- 不在 status 接口暴露未裁剪的模块级 map

### 5.7 External Summary Script
新增 repo-owned summary 入口，职责是：
- 读取多次 `/v1/chain/status` 采样
- 识别 `observed_since_unix_ms` reset
- 计算 delta/s
- 从固定 buckets 派生 p50/p95
- 输出 `summary.md/json`

推荐最少输出：
- `delta_calls`
- `delta_compile_ms`
- `delta_timeout_count`
- `executor_p50/p95_call_ms`
- `router_p50/p95_match_ms`
- `top_hotspots`
- `degraded_reason`

### 5.8 Cardinality 与 Payload Guardrails
默认 guardrails：
- 禁止 `trace_id`
- 禁止原始 input/output bytes
- 禁止全量 `module_id -> metrics`
- 允许：
  - 全局累计 counters
  - 固定 buckets
  - bounded top-N

若后续需要更细维度：
- 必须显式 env-gated
- 必须有 payload budget
- 必须写明裁剪策略

## 6. 与现有体系的关系
- 与 `TASK-WORLD_RUNTIME-060` 的关系：
  - 不是替代 perf probe
  - 而是把其结论沉淀为长期可复核字段
- 与 `chain-status-traffic-metrics` 的关系：
  - 直接复用 shared snapshot + status payload 设计模式
- 与 `runtime-release-gate-metrics-template` 的关系：
  - 后续可把 `wasm.build/executor/router` 作为 runtime 候选指标的一部分

## 7. 回退与降级策略
- 若 metrics 初始化失败：
  - `metrics_available=false`
  - 写 `degraded_reason`
  - 继续允许 module 执行
- 若 status payload 裁剪命中：
  - 显式输出 `truncated=true` 或等价说明
- 若 summary 脚本遇到 reset：
  - 自动缩窗
  - 在 summary 中标记 `window_reset_detected=true`

## 8. 实施顺序建议
1. build suite timing schema
2. executor snapshot
3. status payload `wasm` section
4. summary 脚本
5. router snapshot 与 bounded top-N

原因：
- build/executor 是最高价值且埋点最明确的两段。
- status payload 需要先有统一 schema。
- router 明细容易受 cardinality 影响，放在后半段更稳。

## 9. Module-local observe 层

```text
module_observe.json
  -> generic wasm_module_observe runner
  -> wasm_build_suite
  -> executor contract cases + metrics delta
  -> prepared/fallback router probes + metrics delta
  -> summary.json + summary.md
```

- spec 路径固定在模块 `observability/` 下，manifest 相对 spec 解析。
- cases 统一表达 request、expect、repeat 与 failure assertions；router probes 统一表达 prepared/fallback 和 match expectation。
- runner 只处理 generic JSON -> Canonical CBOR 与共享 ABI 输出，不知道模块私有类型。
- `repeat` 至少为 1；assertion、path 或 decode 失败返回结构化错误并停止对应运行。
- summary 同时保留 build timing、case wall-clock、executor/cache delta 与 router delta，但默认不包含 raw payload。
- 模板与 `m1_rule_move` 样例提供接入真值；扩展更多模块只新增 spec/fixture，不修改 runner 分支。

## 10. 权威边界

- 全局 status/window 适合节点与候选热点归因；module-local observe 适合单模块 contract/perf 验证，两者互补。
- instrumentation 降级不得阻断模块执行，也不得代签模块功能、产品闭环或 release readiness。
- 所有 timing 保持本地观测属性和 bounded cardinality。
