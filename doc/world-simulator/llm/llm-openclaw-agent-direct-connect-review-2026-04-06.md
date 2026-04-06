# Agent 直连模式设计 / 实现复核与修正计划（2026-04-06）

- owner: `producer_system_designer`
- 协作角色: `agent_engineer`、`runtime_engineer`、`viewer_engineer`、`qa_engineer`
- 关联 PRD:
  - `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.prd.md`
  - `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.prd.md`
  - `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 关联任务:
  - `TASK-WORLD_SIMULATOR-119/120/121/124/125/126/128/160/283`
  - `task_c679be21f6f74bfe9d6592e153456bcc`

## 1. 目的
- 将本轮关于 `agent_direct_connect` 当前实现状态的 review 结论落成正式文档，避免“代码已完成 / 专题已收口”的口径继续漂移。
- 明确哪些问题已经是“设计目标未兑现”的 confirmed gap，哪些只是后续优化项。
- 为后续修正建立统一排序、owner 分工与重新验收门槛。

## 2. 范围
- 覆盖 `agent_direct_connect` 当前 provider implementation=`openclaw_local_http` 的产品主链路：
  - `oasis7_client_launcher -> oasis7_game_launcher -> oasis7_viewer_live/runtime_live`
  - `DecisionProvider / OpenClawAdapter / local HTTP provider`
  - `player_parity / headless_agent / debug_viewer`
- 覆盖设计、实现、测试和对外口径之间的已确认偏差。
- 不在本文直接修改产品目标；PRD 目标态仍以现有 `PRD-WORLD_SIMULATOR-037/040` 为准。

## 3. 结论摘要
- 结论 1：`agent_direct_connect` 的 taxonomy 收口方向是正确的，`agent_access_mode / provider_impl / execution_lane / player_access_mode` 四层口径应继续保持。
- 结论 2：当前实现尚未完整兑现 `PRD-WORLD_SIMULATOR-037/040` 的关键承诺；尤其是 `player_parity` 可达性、双轨 observation 分层、provider handshake 合同与 fallback 审计链仍存在正式缺口。
- 结论 3：因此不能继续把当前状态表述为“专题已彻底 completed”；更准确的口径是“首期接通 + taxonomy 收口完成，但仍需一轮 remediation 才能宣称 dual-mode contract 已按设计落地”。

## 4. 已确认偏差

### Gap-1：client launcher 实际无法发起 `player_parity`
- Source refs:
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7_client_launcher/src/launcher_core.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`
- 设计目标:
  - `player_parity` lane 已贯通 runtime live / `oasis7_game_launcher` / `oasis7` / launcher 主链路。
- 当前实现:
  - `oasis7_client_launcher` 的 `LaunchConfig` 没有 execution mode 字段。
  - `build_launcher_args()` 仅透传 `agent_provider_mode/openclaw_base_url/openclaw_auth_token/openclaw_connect_timeout_ms/openclaw_agent_profile`，没有 `--openclaw-execution-mode`。
  - `oasis7_game_launcher` 虽支持 `--openclaw-execution-mode`，但 client launcher 无法把该参数送进去。
- 影响:
  - 从 client launcher 触发的 OpenClaw 直连实际会静默落回默认 `headless_agent`。
  - 文档中“launcher 已支持真实 `player_parity` lane”的说法对 GUI 主链路并不成立。
- 修正要求:
  - client launcher 必须显式暴露并透传 execution mode。
  - 必须新增定向回归，证明 GUI 主链路真的能把 `player_parity` 送到 runtime live sidecar。

### Gap-2：`player_parity` / `headless_agent` 目前只有 metadata 区别，没有真实 observation 分层
- Source refs:
  - `crates/oasis7/src/simulator/decision_provider.rs`
  - `crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs`
  - `crates/oasis7/src/viewer/runtime_live/mapping.rs`
  - `doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`
- 设计目标:
  - 双轨模式共享动作 contract，但观测表达必须分层；`player_parity` 只能拿玩家可感知压缩视图，`headless_agent` 可拿结构化局部拓扑与提示信息。
- 当前实现:
  - `ProviderBackedAgentBehavior::build_request()` 对两种 mode 都直接发送同一个 `Observation` 实体，只额外带 `mode/schema/environment` 等 metadata。
  - runtime live / viewer snapshot 也主要只回显 mode 标签，没有证据表明 request payload 本身发生了模式分化。
- 影响:
  - 当前的 dual-mode smoke 更接近“同一观测 + 不同标签”，而不是正式 contract 定义的双轨。
  - 现有 `player_parity vs headless_agent` 对照样本无法证明“像不像玩家在玩”，只能证明“同一个 provider 在两个 mode 标签下结果接近”。
- 修正要求:
  - 必须引入显式 observation adapter 或等价分层机制。
  - 必须补 fixture diff / schema review / negative tests，证明 `player_parity` 不泄露 headless-only 真值。

### Gap-3：launcher 默认 timeout 基线与真实 OpenClaw 延迟不一致
- Source refs:
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_openclaw_parity_bench.rs`
  - `doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`
- 设计目标:
  - local HTTP provider 错误应可恢复，不得因默认参数过于保守而稳定制造假超时。
- 当前实现:
  - client launcher 默认 `openclaw_connect_timeout_ms=200`。
  - game launcher 默认 `3000`。
  - 已有真实 smoke 证据中的 `median_latency_ms` 明显高于上述 GUI 默认值。
- 影响:
  - GUI 主链路容易把“OpenClaw 可用但较慢”误判成 `timeout` / `provider_unreachable`。
  - launcher、`oasis7` operator、parity bench 三条入口对同一 provider 的成功基线不一致。
- 修正要求:
  - 冻结统一 timeout policy，至少区分 `probe timeout` 与 `decision timeout`。
  - client launcher 默认值必须对齐当前真实试玩链路，而不是继续用 UI 级过短保守值。

### Gap-4：launcher handshake 只检查“活着没”，没有检查“兼容不兼容”
- Source refs:
  - `crates/oasis7_client_launcher/src/launcher_core.rs`
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7/src/simulator/openclaw_local_http.rs`
  - `crates/oasis7/src/bin/oasis7_openclaw_local_bridge.rs`
- 设计目标:
  - `/v1/provider/info` 的 `protocol_version/capabilities/supported_action_sets` 应用于启用前兼容性判断。
  - 错误类型需能区分 `version_mismatch`、`unsupported_agent_profile`、`health degraded` 等。
- 当前实现:
  - launcher probe 只读取 `provider_id/name/version/protocol_version` 与 `health.ok/status/queue_depth`。
  - 没有基于 `capabilities/supported_action_sets` 的显式 gating。
  - 没有在启用前做“当前 provider 是否满足 world-simulator phase-1 contract”的产品级判定。
- 影响:
  - 用户可能在 probe 成功后仍进入不可用 provider。
  - `OpenClaw Gateway 在线` 与 `当前 world-simulator provider contract 可用` 被混成同一层状态。
- 修正要求:
  - launcher probe / config 校验必须补 contract-aware gating。
  - UI 文案必须区分“服务在线”“协议兼容”“profile 可用”“当前处于 degraded”四种状态。

### Gap-5：`fallback_reason` 审计链尚未落地到真实运行产物
- Source refs:
  - `crates/oasis7/src/simulator/decision_provider.rs`
  - `crates/oasis7/src/viewer/runtime_live/mapping.rs`
  - `crates/oasis7/src/bin/oasis7_openclaw_parity_bench.rs`
  - `crates/oasis7/src/simulator/tests/decision_provider.rs`
- 设计目标:
  - 当模式降级、环境受限或改走 observer-only 路径时，runtime live / viewer / parity summary 都应保留明确 `fallback_reason`。
- 当前实现:
  - runtime live snapshot 默认把 `fallback_reason` 写成 `None`。
  - parity bench step/summary 也默认写成 `None`。
  - 目前主要只有单元测试手工设置该字段。
- 影响:
  - 现有样本无法审计“这次通过是否基于降级路径”。
  - `software_safe` / `debug_viewer` / headless 切换后的结果难以作为正式对外结论使用。
- 修正要求:
  - 将 `fallback_reason` 作为真实运行链路的 first-class output，而不是测试专用字段。
  - QA summary 与 viewer 调试面必须能直接看到该字段。

## 5. 不判为正式缺口的事项
- `agent_direct_connect` / `openclaw_local_http` / execution lane 的 taxonomy 分层本身没有发现新的设计错误；当前问题集中在落地不完整，而不是方向错误。
- local bridge 的 `sessionKey` 作用域、`unsupported_agent_profile` 与 `schema_repair_count` 等保护项已有基础实现，不属于本轮最高优先级缺口。
- `debug_viewer` 的 observer-only 定位总体成立；当前主要问题在于执行 lane 真值和 fallback 真值还不够完整。

## 6. 修正顺序

### P0：先修产品入口可达性与参数基线
- owner:
  - `viewer_engineer`
  - 联审：`agent_engineer`
- 范围:
  - client launcher 增加 OpenClaw execution mode 配置与透传。
  - 统一 launcher / `oasis7` operator / parity bench 的 timeout 基线与文案。
- 通过条件:
  - 从 client launcher 启动的 runtime live 能稳定区分并到达 `player_parity` / `headless_agent`。
  - 默认 GUI 配置不会因 200ms 级 timeout 稳定制造假失败。

### P1：补真实 dual-mode observation contract
- owner:
  - `agent_engineer`
  - 联审：`runtime_engineer`
- 范围:
  - 为 `player_parity` / `headless_agent` 引入显式 observation adapter 或等价 contract 分层。
  - 为 schema mismatch / mode mismatch 增加 hard-fail 或结构化错误。
- 通过条件:
  - 同一场景可导出两份 mode-differentiated request fixture。
  - `player_parity` negative tests 能证明 headless-only 字段不会泄露。

### P2：补 provider handshake 与观测审计链
- owner:
  - `viewer_engineer`
  - 联审：`agent_engineer`、`runtime_engineer`
- 范围:
  - launcher probe 增加 `capabilities/supported_action_sets/profile compatibility` 判断。
  - runtime live / parity bench / viewer snapshot 全链路透传 `fallback_reason`。
- 通过条件:
  - UI 能明确区分 incompatible / degraded / ready。
  - summary / snapshot / debug 面可直接读到 `fallback_reason`。

### P3：重跑 dual-mode 与 parity 证据
- owner:
  - `qa_engineer`
  - 联审：`producer_system_designer`
- 范围:
  - 重跑 `player_parity` vs `headless_agent` 真实样本。
  - 重新判断 `PRD-WORLD_SIMULATOR-040` 是否可恢复为 completed。
  - 与 `PRD-WORLD_SIMULATOR-038` 的 parity 口径重新对齐。
- 通过条件:
  - dual-mode 样本能证明“入口可达 + observation 分层 + fallback 可审计”三件事同时成立。
  - QA / producer 能重新签署“本专题已按目标态落地”。

## 7. 文档口径更新要求
- `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.project.md` 不应继续保持“completed / none”而不提 remediation。
- `doc/world-simulator/project.md` 需要新增 follow-up task，避免本轮结论只停留在 review 文本。
- 若后续修正改变了 `PRD-WORLD_SIMULATOR-037/040` 的边界或 acceptance，必须先回写对应 PRD，再推进实现。

## 8. 验收建议
- `test_tier_required`
  - client launcher execution mode 透传测试
  - dual-mode request fixture diff
  - launcher provider compatibility probe tests
  - runtime live / viewer snapshot / parity summary 的 `fallback_reason` 透传测试
- `test_tier_full`
  - 真实 OpenClaw dual-mode 重采证
  - `player_parity` 与 `headless_agent` 同 seed / 同场景对照
  - 修正后再评估是否允许重新把该专题标为 completed

## 9. 当前建议口径
- 当前可对内宣称:
  - `agent_direct_connect` 首期链路已接通。
  - taxonomy 已收口。
  - Viewer 已基本收口为 observer/debug layer。
- 当前不应继续宣称:
  - client launcher 已完整支持 `player_parity`
  - dual-mode observation contract 已按设计落地
  - fallback/degraded 审计链已经完整可用
