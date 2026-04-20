# oasis7 Runtime：WASM 模块标准化观测与契约测试

- 对应设计文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.design.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 现有 WASM observability 已能回答 build/executor/router 的全局热点，但仍缺少“按模块重复使用”的标准入口。每次想验证某个模块的功能契约与性能数据，owner 仍要临时拼 request/fixture/script，导致新模块接入成本高、口径不一致。
- Proposed Solution: 引入模块本地 `module_observe.json` 规范、通用 `wasm_module_observe` runner、repo-owned wrapper script 与模板/示例模块，让每个 WASM 模块只需提供 module-local spec/fixture，即可统一完成 `build -> contract assertions -> executor/router timing delta -> summary`。
- Success Criteria:
  - SC-1: 新增标准 observe spec schema，至少覆盖模块 build 配置、功能 case、router probe、契约期望与 repeat/perf 采样参数。
  - SC-2: repo 内提供通用 runner 与 wrapper script，不再要求每个模块自写一套专用 perf/contract 脚本。
  - SC-3: 至少一个 builtin wasm 模块通过该体系跑通真实 build + execute + router probe，并产出 machine-readable json 与 markdown summary。
  - SC-4: 新模块接入默认只新增 module-local spec/fixture，runner 不需要为单模块写 bespoke 逻辑。
  - SC-5: 标准化体系输出必须保留现有 build timing、executor cache delta 与 router prepared/fallback delta，避免再回退到临时日志。

## 2. User Experience & Functionality
- User Personas:
  - `wasm_platform_engineer`: 需要为每个模块快速补齐功能契约与性能采样，不想重复写执行胶水。
  - `runtime_engineer`: 需要把模块级热点和全局 `/v1/chain/status.wasm` 观测面接起来，判断某个模块是否异常。
  - `qa_engineer`: 需要固定、可回放的 contract/perf 产物，而不是一次性的本地命令截图。
  - builtin / future module 作者: 需要一套模板，新增模块时顺手补观测与契约测试，而不是事后补洞。
- User Scenarios & Frequency:
  - 新模块接入：每次新增 builtin / published wasm 模块时执行，补齐标准 observe spec。
  - 契约回归：每次模块逻辑、订阅过滤或输出结构变化时执行，确认功能与性能边界未漂移。
  - 热点排查：当某个模块疑似慢或行为异常时执行，获取模块级 build/call/router delta 摘要。
  - 模板复制：每次基于 `_templates` 新建模块时执行，作为默认工程步骤的一部分。
- User Stories:
  - PRD-WORLD_RUNTIME-037: As a `wasm_platform_engineer` / `qa_engineer`, I want every wasm module to expose a standard local observe spec that a shared runner can execute, so that new modules automatically gain comparable contract and performance evidence.
- Critical User Flows:
  1. `模块作者创建/复制 module_observe.json -> 填 module_id / manifest_path / subscriptions / cases / router probes`
  2. `repo-owned wrapper script 定位 spec -> 通用 runner 调 build suite 构建 canonical wasm -> 创建 executor -> 跑 contract cases`
  3. `runner 对每个 case 收集 wall-clock samples、executor delta、router delta -> 对比 expect`
  4. `runner 对 router probes 验证 prepared/fallback match 语义 -> 输出 json/md summary`
  5. `QA / owner 读取 summary 或将产物接入 PR / candidate evidence，不再手工拼临时说明`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| module-local observe spec | `schema_version`、`module.module_id`、`manifest_path`、`entrypoint`、`limits`、`subscriptions`、`cases`、`router_probes` | runner 解析 spec，按相对路径定位模块 manifest 与 module-local fixture | `draft -> runnable -> verified` | `manifest_path` 相对 spec 解析；case/probe `repeat >= 1` | 模块作者维护，runner 只读 |
| contract case | `request.time/origin/stage/event_json/action_json/state_json` + `expect.success/failure_code/emit_count/effect_count/new_state_present/emits/state_json` | runner 统一编码 `ModuleCallInput`、调用 executor、断言输出 | `pending -> executed -> passed/failed` | request JSON 统一编码为 CBOR；成功/失败断言互斥 | runner 只在本地观测环境执行 |
| perf sampling | `repeat`、每次 call wall-clock、executor metrics delta | 同一 case 连续执行 N 次并聚合 `avg/p50/p95/max` | `cold -> warmed -> summarized` | 采用 case 窗口内 delta，避免读全局累计值误判 | 仅本地 summary 可写 |
| router probe | `probe.kind`、`event_kind/action_kind`、`payload_json`、`use_prepared`、`expect_match` | runner 复用模块 subscriptions 验证 prepared 与 fallback 匹配结果 | `configured -> matched/mismatched` | 记录 `prepared_hits` / `parse_fallbacks` / `match_ms_total` delta | 仅本地 summary 可写 |
| wrapper script | `--spec`、`--manifest-path`、`--out-dir` | 推断 spec 路径并转发给 runner | `inferred -> executed -> summarized` | `--manifest-path` 默认推断 `<module-dir>/observability/module_observe.json` | repo-owned script |
| sample/template | `_templates/module_observe.json`、代表模块 `observability/module_observe.json` | 模块作者复制模板，代表模块提供真值参考 | `template -> copied -> customized` | 默认模板不内置模块私有逻辑；代表模块提供真实断言样本 | repo 内模板公开可读 |
- Acceptance Criteria:
  - AC-1: repo 内必须新增通用 `wasm_module_observe` runner，统一消费 module-local observe spec。
  - AC-2: repo 内必须新增 wrapper script，支持通过 `--spec` 或 `--manifest-path` 启动标准观测。
  - AC-3: observe spec 至少覆盖 build 参数、contract case、expectation、router probe 与 repeat/perf 采样配置。
  - AC-4: 代表模块必须通过该 runner 跑通真实 build + execute + router probe，并产出 summary json/md。
  - AC-5: case summary 必须保留 executor `compile_misses/memory_cache_hits/call_wall_ms_buckets` 等 delta，router probe summary 必须保留 `prepared_hits/parse_fallbacks` 等 delta。
  - AC-6: 新模块接入默认只新增 module-local spec/fixture，不允许为了单模块在 runner 里加入 bespoke case 逻辑分支。
  - AC-7: 模板路径必须固定且可复制，至少提供 `_templates/module_observe.json`。
  - AC-8: 文档必须把该体系回写到 `world-runtime` 根 PRD / project / README / prd.index，形成正式入口。
- Non-Goals:
  - 不在首期把所有 builtin 模块一次性补齐 observe spec。
  - 不在首期引入远端 metrics backend、trace exporter 或 PR auto-upload。
  - 不在首期替代 runtime 全局 `/v1/chain/status.wasm` 观测面；本专题是模块级补充入口。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `tools/wasm_build_suite`、`oasis7_wasm_executor`、`oasis7_wasm_router`、新建 `tools/wasm_module_observe`、repo-owned wrapper script。
- Evaluation Strategy: 以“是否能在不写 bespoke runner 逻辑的前提下，对模块输出 contract + perf 证据”为主，验证代表模块与模板复制路径。

## 4. Technical Specifications
- Architecture Overview: 标准化体系采用 module-local spec 驱动。runner 先调用 `wasm_build_suite` 构建目标模块，再以 `oasis7_wasm_executor` 执行 case，以 `oasis7_wasm_router` 跑 subscriptions probe，最后对比 expect 并输出 summary json/md。性能口径统一复用已有 build timing、executor cumulative snapshot 与 router cumulative snapshot 的 delta。
- Integration Points:
  - `tools/wasm_module_observe/Cargo.toml`
  - `tools/wasm_module_observe/src/lib.rs`
  - `tools/wasm_module_observe/src/main.rs`
  - `scripts/oasis7-wasm-module-observe.sh`
  - `crates/oasis7_builtin_wasm_modules/_templates/module_observe.json`
  - `crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json`
  - `tools/wasm_build_suite/src/lib.rs`
  - `crates/oasis7_wasm_executor/src/lib.rs`
  - `crates/oasis7_wasm_router/src/lib.rs`
  - `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- Edge Cases & Error Handling:
  - spec 相对路径无效：runner 必须返回结构化错误，指出缺失的 manifest/spec。
  - case 断言失败：runner 必须停止并返回明确的 case 名、期望值与实际值差异。
  - repeat 为 0：spec 解析阶段直接拒绝，不进入 build/execute。
  - 失败 case：必须支持按 `failure_code` / detail substring 断言，而不是只支持 success path。
  - state/output 无法按 JSON-CBOR 解码：若 spec 要求 `state_json` 对比，则 runner 必须报错而不是静默忽略。
  - router prepared/fallback 分叉：summary 必须区分 `use_prepared=true/false`，不能把 parse-fallback 与 prepared-hit 混成一个计数。
  - wasm32 target 不可用：测试入口允许 skip，但 runner 本身必须返回真实 build 失败信息。
  - 模块极快导致 `0ms`：允许 wall-clock 为 `0ms`，但仍需输出 repeat、bucket delta 与 compile/cache 路径。
- Non-Functional Requirements:
  - NFR-1: 标准化 runner 不得把模块特例写死在代码里；模块差异必须通过 spec 表达。
  - NFR-2: 产物必须同时输出 machine-readable json 与人可读 markdown。
  - NFR-3: runner 本地采样不得修改模块 deterministic output；所有 timing 仍只存在本地观测层。
  - NFR-4: 标准模板必须允许未来模块仅通过复制并改 spec 即可接入，避免新增脚本/测试框架分叉。
  - NFR-5: summary 默认输出应保持 bounded，避免把逐 trace 原始 payload 全量转储为常态产物。
- Security & Privacy: runner 只处理模块本地 fixture，不默认输出敏感 payload dump；summary 只保留断言需要的 bounded 输出与 metrics delta。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 提供 spec schema、通用 runner、wrapper script、模板与一个代表模块样例。
  - v1.1: 为更多 builtin 模块补齐 observe spec，并沉淀模块分类模板。
  - v2.0: 将模块级 observe summary 纳入候选 evidence / release gate 汇总入口。
- Technical Risks:
  - 风险-1: 若 spec 表达力不足，模块作者可能继续回退到 bespoke 脚本。
  - 风险-2: 若 summary 输出过多原始数据，会把“标准化”变成新的高基数噪音源。
  - 风险-3: 若 executor/router delta 读取方式不统一，模块级 summary 与全局 observability 会给出冲突解释。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-037 | `task_20b6ee42182247ccbebe6a6a2c2db469` | `test_tier_required` | `cargo test --manifest-path tools/wasm_module_observe/Cargo.toml`、`cargo run --manifest-path tools/wasm_module_observe/Cargo.toml -- observe --spec crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json`、`bash -n scripts/oasis7-wasm-module-observe.sh`、`doc-governance-check`、`git diff --check` | wasm 模块级 contract/perf 证据标准化、未来新模块接入路径 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WMOS-001 | 模块作者提供 module-local spec，runner 统一执行 | 每个模块自写测试脚本/bench harness | 统一 schema 才能保持长期可比性并降低接入成本。 |
| DEC-WMOS-002 | 复用 build/executor/router 现有 metrics delta | 为模块级 runner 重新定义一套平行 perf 指标 | 现有 observability 已是全局真值，模块级体系应复用而不是分叉。 |
| DEC-WMOS-003 | 先用模板 + 一个代表模块示例收口 | 首期要求所有 builtin 模块一次性补齐 | 先证明标准入口可用，再逐模块扩面，风险更低。 |
