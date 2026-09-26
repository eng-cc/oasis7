# WASM SDK 兼容与 Wire 设计

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
| [SDK-01](wasm-sdk.prd.md#sr2-domain-case-1) | 输入 旧 manifests/defaults/export与target known/unknown/null codec matrix；断言 old bytes/defaults保持；strict new codec显式失败 | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | runtime/QA；[SDK-01外部receiver](wasm-sdk.design.md#sr2-sdk-contract) | 当前localfixture不能证明full finality/provider/headed |
| [SDK-02](wasm-sdk.prd.md#sr2-domain-case-2) | 输入 no_std+alloc / explicit std / unavailable wasm32；断言 host和module feature分隔；target缺失skipped | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | runtime/QA；[SDK-02外部receiver](wasm-sdk.design.md#sr2-sdk-contract) | 当前localfixture不能证明full finality/provider/headed |
| [SDK-03](wasm-sdk.prd.md#sr2-domain-case-3) | 输入 malformed canonical bytes、fallback、multibyte/U64边界；断言 Result error不伪造empty success；既有 fallback显式 | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | runtime/QA；[SDK-03外部receiver](wasm-interface.md#sr2-types) | 当前localfixture不能证明full finality/provider/headed |
| [PRD-WORLD_RUNTIME-042](wasm-sdk.prd.md#sr2-obligation-prd-world_runtime-042) | &#124; PRD-WORLD_RUNTIME-042 &#124; test_tier_required &#124; SDK tests、可用时 wasm32 check、builtin wire usage scan、required-tier build &#124; SDK ABI、builtin 模块编译与 wire 兼容 &#124; | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SDK rule 1](wasm-sdk.prd.md#sr2-sdk-rule-1) | `oasis7_wasm_sdk` 默认在非 test、未启用 `std` feature 时使用 `no_std`，需要动态分配的能力来自 `alloc`；宿主或测试只有显式启用 `std` 才可依赖标准库。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |
| [SDK rule 2](wasm-sdk.prd.md#sr2-sdk-rule-2) | `alloc`、`reduce`、`call` 导出，`LifecycleStage`、`WasmModuleLifecycle`、`dispatch_reduce`、`dispatch_call` 与 `export_wasm_module!` 保持稳定；SDK 内部收敛不得静默改变模块种类、生命周期或业务效果。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |
| [SDK rule 3](wasm-sdk.prd.md#sr2-sdk-rule-3) | `ModuleCallInput`、`ModuleContext`、`ModuleEffectIntent`、`ModuleEmit`、`ModuleOutput` 及其编码 helper 由 SDK 持有一份 wire 定义。builtin 模块只保留领域结构，不得复制一套并行协议。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |
| [SDK rule 4](wasm-sdk.prd.md#sr2-sdk-rule-4) | wire 编解码使用与 runtime `wasm-1` 一致的 Canonical CBOR 字段和默认值；输入、输出与 `output_bytes` 口径必须保持兼容。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |
| [SDK rule 5](wasm-sdk.prd.md#sr2-sdk-rule-5) | codec 失败必须以显式 `Result` 或调用点明确选择的 fallback 处理；SDK 不得把损坏输入静默转换为空输出或 `None`。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |
| [SDK rule 6](wasm-sdk.prd.md#sr2-sdk-rule-6) | 新字段或 feature 必须说明默认值、旧模块兼容性和 wasm32 构建影响。破坏性 ABI 变化不能通过 SDK 整理任务夹带进入。 | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | ABI/runtime owns receipt finality；builtin owns显式fallback | 不改变wasm-1业务/lifecycle，不宣称wasm32可用 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前状态 / 基线 | 目标 | 差距与证据边界 |
| --- | --- | --- | --- |
| wasm-1 foundation | 原有代码/契约保留，源commit见身份块 | 兼容consumer及精确target contracts | 原有tests为located，不是本次run |
| receipt / migration / gate | interface仍声明target或partial | `sr2-sdk-contract` 与runtime authority相连 | 代码、full finality/provider/headed证明不在本任务 |
| 文档与schema | Issue4102采纳目标合同 | 语义/结构/anchors检查 | 文档通过不意味着runtime pass或SC9/DC5闭合 |

## 4. 边界与结构

输入声明和provider response为advisory，host/runtime负责authority/admission；sandbox仅确定性计算。build证明字节来源、SDK证明codec、metrics证明local诊断，各自不能签署world finality。ContentRef必须fetch/hash/length验证，再由相应authority验trust；current caches与local signature不能越过生产许可。

## 5. 关键运行流程

先绑定candidate/world branch/window与exact manifest/artifact/schema；strict decode和limits preflight后验证authority。成功只进入有权的下一阶段，世界效果须canonical commit receipt。缺字段、版本、proof、兼容或head continuity即停止，不隐式fallback或重试另一身份。重试同request hash幂等，changed hash拒绝；恢复重读当前canonical边界，历史receipt按记录版本重放。

## 6. 接口与数据合同

<a id="sr2-sdk-contract"></a>
### 6.1 版本化 consumer codec 合同

SDK 接收端分开 legacy `wasm-1` wire 与 interface 的 [target records](wasm-interface.md#sr2-types)。不把 target receipt/migration 强行加进旧 ModuleContext/ModuleOutput，也不让 SDK 生成 finality。旧 manifest optional fields 与 serde default、tick 缺省 suspend、effect/emit semantic order 均保留。

| 输入 | decode / encode 判定 | compatibility / failure |
| --- | --- | --- |
| 旧 wasm-1、无新增 optional 字段 | 原 wire owner/types/default；bytes round-trip 与 output_bytes 口径不变 | lifecycle exports/dispatch 与 no_std+alloc 不变 |
| 新 target records / 已知 type literal | strict field+enum+canonical decoding；U64 decimal JSON checked 转 CBOR；UTF8 byte bound | 验证 payload shape 不代表 proof 信任；runtime 接收器验 finality |
| 未知 target version / enum / null / field | explicit codec Result error；不得 nearest-schema fallback | 未提交 state/effects/charges |
| 非 canonical map / malformed bytes | byte-exact reencode equality，不偷偷 normalize authenticated input | builtin 必须显式选择原有 fallback；不返回伪空 success |
| 缺 wasm32 target | 明确 skipped 或失败 | 禁止将 host std check 当 no_std WASM pass |

目标存储 codec 仅保存 bytes/refs，authority 与 replay ownership 归 runtime；SDK 无持久化 winner/activation 状态。升级先建立旧 fixtures，再分别启用新 consumer，逐项比较 export、defaults、wire byte lengths 和 domain IDs；回滚到当前 wasm-1 consumer，不重写历史 target receipt bytes。`std` 仅 host/test 显式启用；wire/serde feature 不能引入隐式 std。输入长度先验证再 alloc，使用现行 ModuleLimits；codec error bounded，不输出 secret payload。


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
| [SDK-01](wasm-sdk.prd.md#sr2-domain-case-1) | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | old bytes/defaults保持；strict new codec显式失败 | [现有partial source](../../../testing-manual.md)；target case `SDK-01` 输入 旧 manifests/defaults/export与target known/unknown/null codec matrix；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [SDK-02](wasm-sdk.prd.md#sr2-domain-case-2) | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | host和module feature分隔；target缺失skipped | [现有partial source](../../../testing-manual.md)；target case `SDK-02` 输入 no_std+alloc / explicit std / unavailable wasm32；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [SDK-03](wasm-sdk.prd.md#sr2-domain-case-3) | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | Result error不伪造empty success；既有 fallback显式 | [现有partial source](../../../testing-manual.md)；target case `SDK-03` 输入 malformed canonical bytes、fallback、multibyte/U64边界；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [PRD-WORLD_RUNTIME-042](wasm-sdk.prd.md#sr2-obligation-prd-world_runtime-042) | [本域合同](wasm-sdk.design.md#sr2-sdk-contract) | &#124; PRD-WORLD_RUNTIME-042 &#124; test_tier_required &#124; SDK tests、可用时 wasm32 check、builtin wire usage scan、required-tier build &#124; SDK ABI、builtin 模块编译与 wire 兼容 &#124; | [现有partial source](../../../testing-manual.md)；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SDK rule 1](wasm-sdk.prd.md#sr2-sdk-rule-1) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | `oasis7_wasm_sdk` 默认在非 test、未启用 `std` feature 时使用 `no_std`，需要动态分配的能力来自 `alloc`；宿主或测试只有显式启用 `std` 才可依赖标准库。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |
| [SDK rule 2](wasm-sdk.prd.md#sr2-sdk-rule-2) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | `alloc`、`reduce`、`call` 导出，`LifecycleStage`、`WasmModuleLifecycle`、`dispatch_reduce`、`dispatch_call` 与 `export_wasm_module!` 保持稳定；SDK 内部收敛不得静默改变模块种类、生命周期或业务效果。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |
| [SDK rule 3](wasm-sdk.prd.md#sr2-sdk-rule-3) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | `ModuleCallInput`、`ModuleContext`、`ModuleEffectIntent`、`ModuleEmit`、`ModuleOutput` 及其编码 helper 由 SDK 持有一份 wire 定义。builtin 模块只保留领域结构，不得复制一套并行协议。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |
| [SDK rule 4](wasm-sdk.prd.md#sr2-sdk-rule-4) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | wire 编解码使用与 runtime `wasm-1` 一致的 Canonical CBOR 字段和默认值；输入、输出与 `output_bytes` 口径必须保持兼容。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |
| [SDK rule 5](wasm-sdk.prd.md#sr2-sdk-rule-5) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | codec 失败必须以显式 `Result` 或调用点明确选择的 fallback 处理；SDK 不得把损坏输入静默转换为空输出或 `None`。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |
| [SDK rule 6](wasm-sdk.prd.md#sr2-sdk-rule-6) | [SDK consumer codec](wasm-sdk.design.md#sr2-sdk-contract) | 新字段或 feature 必须说明默认值、旧模块兼容性和 wasm32 构建影响。破坏性 ABI 变化不能通过 SDK 整理任务夹带进入。 | [SDK partial tests](../../../testing-manual.md)；SDK-01/02/03逐项旧fixture/new strict codec/feature target，required；target不可用skipped | 实际SDK implementation evidence记录candidate/features/target/exit；Issue4102仅文档 | 未执行；host check不能代wasm32 |

现有partial/unrun来源：`crates/oasis7_wasm_executor/src/tests.rs`（output/fuel/memory/cache），`crates/oasis7_wasm_abi/tests/open_module_commands.rs`（legacy/envelope），`crates/oasis7/src/runtime/world/module_runtime_tests.rs`（metrics/due-preflight），`scripts/oasis7-node-wasm-metrics-monitor.test.sh`（window summary），`scripts/ci-verify-m1-wasm-summaries.py`（build对账）。这些不是新target receipt/migration/DC5已覆盖证据。GWSC [negative matrix](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-qa-negative-matrix) 接收全部17QA cases，[runtime scenarios](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-runtime-scenarios) 接收10组runtime；本页domain rows为具体输入/assertion分配。

## 12. 决策、长期风险与未决问题

选择单authority typedtarget而非重复wire，是为避免scope/hash/版本漂移。已知风险是implementation缺失、proof不可读、U64/UTF8投影误读、递归hash与历史误作新候选证据；WASM/runtime/QA分别按本页case和runtimereceiver解除。触发器为新ABI/schedule/schema、trust epoch、activation或consumer变更；需重新冻结合同与专业联审。SC9八fullcells及activeLLM/provider API parity、真实desktop+narrow/headed证据保持产品/QAauthority，不因本页文档完成消除。


- 对应需求文档：`doc/world-runtime/wasm/wasm-sdk.prd.md`
- 稳定证据入口：`doc/world-runtime/wasm/evidence.md`

## 1. 设计原则

本页只定义 SDK 的稳定 wire/ABI 契约；实施任务、状态和历史验收由 GitHub
task issue / Project 与 Git history 追溯。可重复执行的验证入口见
[`evidence.md`](evidence.md)。

- 核心 SDK 默认 `no_std`，宿主便利能力通过显式 feature 隔离。
- wire schema 只有一个 owner；模块代码复用类型并只实现领域逻辑。
- ABI 稳定优先于内部去重，任何字段变更都必须保留 serde/default 兼容。
- 编解码错误是接口结果，不是空业务结果。

## 2. 分层

| 层 | 权威入口 | 要求 |
| --- | --- | --- |
| 生命周期与导出 | `crates/oasis7_wasm_sdk/src/lib.rs` | 保持 `alloc/reduce/call`、lifecycle trait、dispatch 和 export macro 稳定。 |
| Wire schema | `oasis7_wasm_sdk::wire` | 统一 input/context/effect/emit/output 类型与 Canonical-CBOR helper。 |
| Builtin 模块 | `crates/oasis7_builtin_wasm_modules/*` | 复用 SDK wire 类型；不得复制协议结构；codec fallback 必须显式。 |
| Runtime 接收端 | `oasis7_wasm_abi` 与 executor/runtime | 校验 ABI、schema、limits 与输出；不由 SDK 文档重定义。 |

## 3. Feature 与构建模型

```text
default module build
  -> no_std + alloc
  -> stable lifecycle/export surface
  -> optional wire feature
  -> Canonical CBOR encode/decode

test or host tooling
  -> explicit std feature
```

- `cfg_attr` 必须使普通模块构建不隐式依赖 `std`。
- feature 组合需由 Cargo metadata/check 覆盖，避免 check-cfg 漂移。
- wasm32 target 缺失时验证应 fail clearly 或显式 skipped，不得产生虚假成功。

## 4. Wire 演进

- 增加字段时使用兼容默认值，并同时更新 runtime schema/evidence。
- 删除或改变字段语义属于 ABI 迁移，需独立兼容方案。
- helper 返回 `Result`；调用点若需要兼容 fallback，必须记录选择及其业务含义。
- `ModuleOutput` 默认值、effect/emit 顺序和 `output_bytes` 计算保持确定。

## 5. 验证图

- SDK unit tests：生命周期、分配、wire round-trip、损坏输入。
- wasm32 check：仅在 target 可用时作为通过证据。
- builtin scan/build：没有重复协议定义，代表性模块 sync/check 通过。
- required-tier：证明 runtime 与 builtin 消费面仍可编译。

## 6. 演进边界

SDK 文档不拥有 sandbox limits、artifact integrity、module storage、治理许可或玩家规则。相关变化必须回到 executor、module lifecycle/storage 与产品治理权威。
