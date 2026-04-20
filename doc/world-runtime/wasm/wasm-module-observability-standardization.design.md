# oasis7 Runtime：WASM 模块标准化观测与契约测试（设计）

- 对应需求文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.project.md`

审计轮次: 1

## 目标
- 把“每个模块都能补功能和性能观测”落成可复制的工程结构，而不是流程建议。
- 让模块作者只通过 module-local spec/fixture 接入，不需要再写 bespoke runner。

## 架构分层
1. module-local spec
   - 路径固定为 `<module-dir>/observability/module_observe.json`
   - 负责声明 build 参数、subscriptions、contract cases、router probes、repeat/perf 采样参数
2. generic runner
   - `tools/wasm_module_observe`
   - 解析 spec、调用 `wasm_build_suite`、执行 executor/router、生成 summary
3. repo-owned wrapper
   - `scripts/oasis7-wasm-module-observe.sh`
   - 支持 `--spec` 与 `--manifest-path` 两种入口，降低日常使用门槛
4. template + sample
   - `_templates/module_observe.json`
   - `m1_rule_move/observability/module_observe.json`

## 数据流
1. wrapper / CLI 接收 `spec_path`
2. runner 解析 spec 并 canonicalize `manifest_path`
3. `wasm_build_suite::run_build()` 构建目标模块，产出 packaged wasm、metadata、receipt、build timing
4. runner 创建 `WasmExecutor`，按 case 构造 `ModuleCallRequest`
5. runner 逐 case 执行：
   - 编码 `ModuleCallInput` 为 CBOR
   - 调用 executor
   - 记录 wall-clock sample
   - 读取 executor/router snapshot delta
   - 对比 `expect`
6. runner 逐 router probe 执行：
   - 复用 spec 中的 `subscriptions`
   - 根据 `use_prepared` 选择 prepared/fallback 路径
   - 记录 `match` 与 router metrics delta
7. runner 输出 `summary.json` 与 `summary.md`

## Spec Schema 说明
### `module`
- `module_id`: 目标模块 ID
- `manifest_path`: 相对 spec 的模块 `Cargo.toml`
- `entrypoint`: 默认执行入口，默认 `reduce`
- `profile` / `target`: 直接透传给 build suite
- `limits`: 默认 `ModuleLimits`

### `cases[*]`
- `name`: 稳定 case 名
- `repeat`: 重复执行次数，用于采样与冷热路径区分
- `request`
  - `time/origin_kind/origin_id/stage`
  - `event_json/action_json/state_json`
  - `limits` / `entrypoint` override
- `expect`
  - `success`
  - `failure_code` / `failure_detail_substring`
  - `emit_count` / `effect_count`
  - `new_state_present`
  - `tick_lifecycle`
  - `emits[*].kind/payload_json`
  - `state_json`

### `router_probes[*]`
- `name`
- `repeat`
- `use_prepared`
- `probe.kind`
  - `event`: `event_kind + payload_json`
  - `action`: `stage + action_kind + payload_json`
- `expect_match`

## Summary Schema
### `summary.json`
- 基础信息：`module_id/spec_path/manifest_path/wasm_hash_sha256`
- build：沿用 `BuildTimingSnapshot`
- `case_results[*]`
  - `perf.runs/avg/p50/p95/max`
  - `executor_delta.*`
  - `router_delta.*`
  - `actual.success/failure_code/new_state_json/emits`
- `router_probe_results[*]`
  - `matched`
  - `use_prepared`
  - `perf.*`
  - `router_delta.*`

### `summary.md`
- 面向 PR / 人工 review 的短摘要
- 默认只保留 build timing、case perf 摘要、router probe 摘要

## 代表模块选择
- 选择 `m1_rule_move`
  - 事件路径与动作路径都简单明确
  - 有状态跟踪与 rule decision emit 两类输出
  - 可同时覆盖 executor compile/cache 路径与 router event/action probe

## 工程约束
- runner 不感知模块私有类型，只处理 generic JSON->CBOR 与通用 ABI 输出
- 任何模块差异都必须通过 spec 表达，不允许在 runner 里分模块写条件分支
- 首期产物不接入远端系统，只保留本地 summary

## 后续扩展位
- 为 m4/m5 模块补更多样例 spec
- 增加 contract template 变体，例如 event-tracker / pure-rule / gameplay-tick
- 将 summary.json 进一步接到候选 evidence 聚合工具
