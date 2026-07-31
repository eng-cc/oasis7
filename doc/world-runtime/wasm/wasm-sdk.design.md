# WASM SDK 兼容与 Wire 设计

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
