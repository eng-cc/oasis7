# WASM SDK 兼容与 Wire 契约

- 专业 owner：`wasm_platform_engineer`；状态：active professional authority（目标合同不等于已实现）。
- 内容审读基线：`eng-cc/oasis7`，源提交 `9c41d57b4436f71f0cf9b481043d882cfdc551ed`；本次文档采纳任务：[Issue 4102](https://github.com/eng-cc/oasis7/issues/4102)，Task UID `task_9f4c3a6c2daa40a0ae3e2c86bdf35e02`；内容整理日期：2026-09-26。
- 独立审读与实际实现/测试候选证据由该 task evidence 记录；此处不预报审读通过、runtime capability、发布或 full proof。下方既有条款仍有效；仅显式日期/历史基线条款按其证据范围解释。

<a id="sr2-acceptance"></a>
## SR2 具体目标验收与专业承接

<a id="sr2-domain-case-1"></a>

### SDK-01 专业接受条件

输入：旧 manifests/defaults/export与target known/unknown/null codec matrix。断言：old bytes/defaults保持；strict new codec显式失败。目标字段、算法和恢复条件由 [SDK-01精确设计](wasm-sdk.design.md#sr2-sdk-contract) 承接；跨域 proof/publication obligation 由 [SDK-01外部接收器](wasm-sdk.design.md#sr2-sdk-contract) 承接。required codec/deterministic fixture 与 full actual finality/candidate artifacts 是不同层，本次未运行。

<a id="sr2-domain-case-2"></a>

### SDK-02 专业接受条件

输入：no_std+alloc / explicit std / unavailable wasm32。断言：host和module feature分隔；target缺失skipped。目标字段、算法和恢复条件由 [SDK-02精确设计](wasm-sdk.design.md#sr2-sdk-contract) 承接；跨域 proof/publication obligation 由 [SDK-02外部接收器](wasm-sdk.design.md#sr2-sdk-contract) 承接。required codec/deterministic fixture 与 full actual finality/candidate artifacts 是不同层，本次未运行。

<a id="sr2-domain-case-3"></a>

### SDK-03 专业接受条件

输入：malformed canonical bytes、fallback、multibyte/U64边界。断言：Result error不伪造empty success；既有 fallback显式。目标字段、算法和恢复条件由 [SDK-03精确设计](wasm-sdk.design.md#sr2-sdk-contract) 承接；跨域 proof/publication obligation 由 [SDK-03外部接收器](wasm-interface.md#sr2-types) 承接。required codec/deterministic fixture 与 full actual finality/candidate artifacts 是不同层，本次未运行。


既有所有成功标准、验收、limits、公式、historical identities和能力限制保留；下表逐项承接到 [精确target设计](wasm-sdk.design.md#sr2-sdk-contract)，而非以任务链接代替设计。Issue4102只承担文档合同采纳；现行实现partial与未来所需runtime/proof验证保持独立。typed字段共用 [interface types](wasm-interface.md#sr2-types)；build/signature/SDK/metrics局部结果均不证明canonical世界效果。

| 原 obligation identity（下方完整原文） | 具体承接结果 / 适用条件 | 精确设计 receiver | 验证范围 / evidence |
| --- | --- | --- | --- |
| [PRD-WORLD_RUNTIME-042原文](#sr2-obligation-prd-world_runtime-042) | &#124; PRD-WORLD_RUNTIME-042 &#124; test_tier_required &#124; SDK tests、可用时 wasm32 check、builtin wire usage scan、required-tier build &#124; SDK ABI、builtin 模块编译与 wire 兼容 &#124; | [PRD-WORLD_RUNTIME-042本域合同](wasm-sdk.design.md#sr2-sdk-contract)；下方原详细设计继续规定条款内的具体limits/policy/tooling | [partial验证入口](../../../testing-manual.md)；§11.1精确target场景；本次仅文档采纳，运行proof未执行 |

目标 acceptance 另外要求：有效 ModuleExecutionReceiptV1 仅 committed_success；canonical no-effect由runtime IntentDecisionV1区分；world branch、member_generation、decimal U64与UTF8byte bound全链一致；receipt core不包含递归blockproof。migration/state/schedule/replay精确字段由interface独占，运行发布点由lifecycle/executor独占。观测不可进入计费/世界hash；Docker构建proof与external release finality独立；SDK旧wasm-1 optional/default不变。完整DC5 attachment由GWSC/schema接收，SC9全部8cells与provider/headed proof不裁剪。


- 对应设计文档：`doc/world-runtime/wasm/wasm-sdk.design.md`
- 稳定证据入口：`doc/world-runtime/wasm/evidence.md`
- 专业权威：`wasm_platform_engineer`

## 1. 目标

为 builtin 与第三方 WASM 模块提供单一、轻量、可移植的 SDK 契约。SDK 必须默认适配 `no_std` 模块环境，并以唯一的 Canonical-CBOR wire 类型避免模块间协议漂移，同时保持既有 `wasm-1` ABI 和模块行为兼容。

## 2. 范围

### 永久契约

<a id="sr2-sdk-rule-1"></a>

1. `oasis7_wasm_sdk` 默认在非 test、未启用 `std` feature 时使用 `no_std`，需要动态分配的能力来自 `alloc`；宿主或测试只有显式启用 `std` 才可依赖标准库。
<a id="sr2-sdk-rule-2"></a>

2. `alloc`、`reduce`、`call` 导出，`LifecycleStage`、`WasmModuleLifecycle`、`dispatch_reduce`、`dispatch_call` 与 `export_wasm_module!` 保持稳定；SDK 内部收敛不得静默改变模块种类、生命周期或业务效果。
<a id="sr2-sdk-rule-3"></a>

3. `ModuleCallInput`、`ModuleContext`、`ModuleEffectIntent`、`ModuleEmit`、`ModuleOutput` 及其编码 helper 由 SDK 持有一份 wire 定义。builtin 模块只保留领域结构，不得复制一套并行协议。
<a id="sr2-sdk-rule-4"></a>

4. wire 编解码使用与 runtime `wasm-1` 一致的 Canonical CBOR 字段和默认值；输入、输出与 `output_bytes` 口径必须保持兼容。
<a id="sr2-sdk-rule-5"></a>

5. codec 失败必须以显式 `Result` 或调用点明确选择的 fallback 处理；SDK 不得把损坏输入静默转换为空输出或 `None`。
<a id="sr2-sdk-rule-6"></a>

6. 新字段或 feature 必须说明默认值、旧模块兼容性和 wasm32 构建影响。破坏性 ABI 变化不能通过 SDK 整理任务夹带进入。

## 3. 接口 / 数据

- 权威 crate：`crates/oasis7_wasm_sdk`
- wire feature：承载共享输入、上下文、effects、emits、输出与 helper。
- builtin 使用者：`crates/oasis7_builtin_wasm_modules/*`
- runtime ABI 与执行语义继续由 `wasm-interface.md`、`wasm-executor.prd.md` 和 `oasis7_wasm_abi` 拥有。
- 本文不拥有模块业务规则、hash/manifest 发布链、执行器资源限制或模块存储生命周期。

## 4. 验收

- SDK 单测覆盖生命周期 dispatch、默认分配和 wire round-trip / 失败路径。
- 在 wasm32 target 可用时验证默认 no_std 编译；target 不可用必须记录为 skipped，不能报告为通过。
- builtin 模块迁移需证明不再保留重复 wire 定义，并通过代表性 sync/check 与 required-tier 编译。
- SDK 变更必须明确向后兼容或提供迁移路径。

## 5. 风险与非声明

- `std` feature 或 serde feature 组合可能重新引入隐式宿主依赖。
- 批量复用共享 wire 类型可能暴露历史字段差异，必须逐项校验，不能仅依靠文本替换。
- 本文不宣称所有第三方模块、所有 target 或发布候选已验证。

## 6. Validation & Decision Record

SDK ABI、wire 使用和 wasm32 兼容的稳定验证入口见
[`evidence.md`](evidence.md)。任务批次、当前状态和已完成工作历史由 GitHub
task issue / Project 与 Git history 承接。

| PRD-ID | 测试层级 | 验证方法 | 回归范围 |
| --- | --- | --- | --- |
<a id="sr2-obligation-prd-world_runtime-042"></a>

| PRD-WORLD_RUNTIME-042 | `test_tier_required` | SDK tests、可用时 wasm32 check、builtin wire usage scan、required-tier build | SDK ABI、builtin 模块编译与 wire 兼容 |

| 决策 ID | 选定方案 | 否决方案 | 依据 |
| --- | --- | --- | --- |
| DEC-WR-WASM-SDK-001 | 以稳定 `wasm-sdk` 权威合并 no_std 与 wire-dedup 完成记录 | 长期保留两组三件套作为并列权威 | 两者共同约束同一 SDK 兼容面；单一权威更能防止 feature、wire 和错误语义漂移。 |
| DEC-WR-WASM-SDK-002 | codec 失败显式返回或由调用点选择 fallback | SDK 静默返回空输出/`None` | 当前结构化错误契约要求失败可观测，旧静默措辞已失效。 |
