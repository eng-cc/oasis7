# oasis7 Runtime：三节点完整监控体系（2026-04-23）

- 对应设计文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.design.md`
- 对应项目管理文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.project.md`

审计轮次: 2

## 1. Executive Summary
- Problem Statement: 当前真实三节点运维已经有 `triad snapshot`、`traffic monitor`、`node observability report`、`wasm metrics monitor` 等零散脚本，但宿主机资源、systemd 进程、链状态、流量窗口与 WASM 窗口仍需手工拼接，导致“CPU 为什么高”“哪台机先退化”“链高正常但控制面过热”这类问题缺少一次执行即可复盘的统一监控入口。
- Proposed Solution: 在仓库内冻结一套 triad 级完整监控体系，复用现有 status/traffic/wasm 能力，新增 host/process 采样与统一汇总层，并把 raw `status.json` 细分为 `host_runtime / consensus / observability / replication / storage / reward_runtime / transactions / wasm / traffic_control_plane / p2p_reachability` 等模块摘要，输出单次可审计的 `snapshot + host/process + traffic + wasm + merged summary` 证据包。
- Success Criteria:
  - SC-1: local observer + 2 ECS 节点的宿主机 CPU/load/memory/storage 与 runtime 进程 CPU/memory/threads 可被统一采样。
  - SC-2: 链状态、service health、traffic window、WASM window 继续沿用 repo 现有脚本，不引入外部监控平台依赖。
  - SC-3: triad 级 merged summary 能同时回答“链是否健康”“哪台机资源吃紧”“traffic 主要在哪条 lane”“WASM 是否退化”，并能把问题定位到具体 runtime 子模块。
  - SC-4: 输出必须机器可读，并附带 Markdown 摘要，方便 operator、QA、producer 共用同一份真值。
  - SC-5: 汇总逻辑具备 fixture 测试，不依赖真实 ECS 才能验证 summary contract。
  - SC-6: summary 必须直接输出模块级 `optimization_candidates`，避免 operator 还要手工二次阅读原始 status/traffic 数据才能判断优化方向。

## 2. User Experience & Functionality
- User Personas:
  - `runtime_engineer`: 需要快速判断问题在宿主机资源、runtime 进程还是链状态/复制控制面。
  - `qa_engineer`: 需要把 real-env triad 的健康度冻结成可复查的证据包，而不是聊天式结论。
  - `producer_system_designer`: 需要在继续扩 shared-network / web-trial 口径前，先知道当前 triad 的真实运行边界。
- User Scenarios & Frequency:
  - 每次真实环境 triad rollout 后，立即跑一次完整监控确认资源和链状态。
  - 每次用户问“哪个节点 CPU 高”“流量是否正常”“WASM 是否退化”时，直接运行完整监控，而不是手工 SSH 多台机器。
  - 每次要写 `doc/testing/evidence/*` 时，先用该脚本生成统一 summary 再回写证据。
- User Stories:
  - PRD-P2P-025-A: As a `runtime_engineer`, I want triad host/process metrics sampled together with chain status, so that node resource pressure can be tied to the same real-env window as chain health.
  - PRD-P2P-025-B: As a `qa_engineer`, I want one merged observability summary for snapshot/traffic/wasm/host metrics, so that evidence does not depend on manual cross-file stitching.
  - PRD-P2P-025-C: As a `producer_system_designer`, I want triad monitoring to surface concrete alerts like runtime CPU hot or service unhealthy, so that rollout claims stay bounded by current operating reality.
- Critical User Flows:
  1. Flow-P2P-TOS-001: `operator runs triad observability monitor -> snapshot + host monitor + traffic monitor + wasm summary all execute -> merged summary produced`
  2. Flow-P2P-TOS-002: `summary reports pass_candidate / pass_with_resource_alerts / pass_with_module_alerts / blocked -> operator reads node-level CPU/load/memory/storage + chain status deltas + module breakdown -> evidence doc references canonical output paths`
  3. Flow-P2P-TOS-003: `fixture test feeds synthetic snapshot/host/traffic/wasm/raw-status inputs -> merged summary contract stays stable without real ECS dependency`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| host/process 采样 | `cpu_cores/loadavg/mem/storage/service/runtime pid/pcpu/pmem/nlwp` | triad host monitor 周期采样三节点 | `captured -> summarized` | latest + peaks + alert heuristics | 仅 operator/QA 读取 |
| triad 完整监控总控 | `snapshot/host/traffic/wasm/report` 路径 | 一次执行串起全部子脚本 | `run -> artifacts_ready` | 默认同一 run dir 聚合 | 仅 operator 运行 |
| merged summary | `overall.status`、per-node `alerts`、`modules.*`、`optimization_candidates` | 读取 snapshot/host/traffic/wasm/raw-status 并输出统一 JSON/MD | `inputs_ready -> merged` | snapshot claim 先定底线，resource alerts 与 module alerts 再叠加 | producer/QA/runtime 共用 |
| 模块级拆分 | `host_runtime/consensus/observability/replication/storage/reward_runtime/transactions/wasm/traffic_control_plane/p2p_reachability` | 从 raw `status.json` 提取模块摘要 | `status_ready -> module_breakdown` | 每模块按独立阈值输出 `status/alerts` | operator/runtime 诊断共用 |
| optimization candidates | `severity/module/key/summary/evidence/suggested_optimizations` | 汇总热点信号并输出优化建议 | `module_breakdown -> optimization_candidates` | 结合 CPU/traffic/replication/consensus/WASM 等交叉信号 | runtime/producer 共用 |
| fixture 回归 | host fixture / observability fixture / raw status fixture | shell test 调 summary helper | `fixture -> pass/fail` | assert JSON contract 关键字段 | CI / 本地回归可读 |
- Acceptance Criteria:
  - AC-1: 仓库新增 triad host/process monitor，至少覆盖 `cpu/load/memory/storage/systemd/runtime process`。
  - AC-2: 仓库新增 triad observability monitor，总控脚本必须串起 `p2p-real-env-triad-snapshot.sh`、`p2p-real-env-host-monitor.sh`、`p2p-real-env-traffic-monitor.sh`、`oasis7-node-wasm-metrics-monitor.sh`。
  - AC-3: merged summary 必须输出 `overall.status`、node-level alerts、runtime CPU/load/memory/storage 摘要、traffic aggregate、wasm hotspot。
  - AC-4: merged summary 必须输出 per-node `modules.*`，并把 raw `status.json` 的关键模块字段冻结为稳定 contract。
  - AC-5: merged summary 必须输出 node-level 与 aggregate `optimization_candidates`，至少能直接暴露 control-plane/replication/consensus/WASM 等热点优化方向。
  - AC-6: 监控体系必须保留 repo-owned artifacts，不依赖 Prometheus/OTel/外部 TSDB。
  - AC-7: `testing-manual.md` 必须给出 canonical 命令与产物路径。
  - AC-8: fixture 回归至少覆盖 host summary、merged summary 与 raw status module breakdown 三层。
- Non-Goals:
  - 不在本阶段引入 Prometheus、Grafana、OpenTelemetry exporter。
  - 不在本阶段接入告警推送平台或长期时序库存储。
  - 不替代 shared-network / mixed-topology full-tier truth，只服务当前 triad 运维与证据收集。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题为节点运维监控体系，不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 以 repo-owned shell/python 工具为主，保持“一个控制机执行，多台节点取样，本地汇总出机器可读 summary”的结构；链状态/traffic/wasm 继续复用现有脚本，新增 host/process 采样和 triad merged summary helper，并在 helper 内解析 raw `status.json` 生成模块级诊断与优化候选。
- Integration Points:
  - `scripts/p2p-real-env-triad-snapshot.sh`
  - `scripts/p2p-real-env-traffic-monitor.sh`
  - `scripts/oasis7-node-wasm-metrics-monitor.sh`
  - `scripts/p2p-real-env-node-host-sample.sh`
  - `scripts/p2p-real-env-host-monitor.sh`
  - `scripts/p2p-real-env-host-summary.py`
  - `scripts/p2p-real-env-observability-summary.py`
  - `scripts/p2p-real-env-observability-monitor.sh`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - SSH 凭据缺失或远端不可达：脚本必须直接失败，不得伪造 triad pass。
  - runtime child pid 缺失：host summary 需要显式报 `runtime_process_missing`。
  - storage 路径不可读：storage 指标为 `null`，并在 node alerts 暴露。
  - snapshot `claim_status != pass_candidate`：merged summary 必须至少为 `blocked`，不能被资源正常覆盖掉。
  - 资源告警与链状态同时出现：merged summary 允许 `pass_with_resource_alerts`，但仅在 snapshot claim 已 pass_candidate 时成立。
  - 无 host resource 告警但模块级告警存在：merged summary 允许 `pass_with_module_alerts`，提醒 operator 当前不是宿主机资源问题而是 runtime 内部子模块异常。
- Non-Functional Requirements:
  - NFR-P2P-TOS-1: 全部 summary 产物必须为稳定 JSON contract + Markdown 摘要。
  - NFR-P2P-TOS-2: 不要求远端节点预装 repo，只通过 SSH + 标准系统命令取样。
  - NFR-P2P-TOS-3: fixture 测试必须在本地无 ECS 依赖可重复运行。
  - NFR-P2P-TOS-4: 新增脚本应保持 operator 视角最小输入，默认值直接匹配 current triad。
- Security & Privacy:
  - 不把 SSH 密码写入仓库或 summary 产物。
  - 采样输出只包含节点资源与状态，不包含私钥、完整 env 秘密或敏感业务负载。

## 5. Risks & Roadmap
- Phased Rollout:
  - Phase 1: host/process 采样 + merged summary helper 落地。
  - Phase 2: total observability wrapper 落地并接 testing manual。
  - Phase 3: real-env triad 小样本验证并回写 evidence。
- Technical Risks:
  - 风险-1: 远端系统工具输出格式差异会影响 host sample 解析，需要保持 helper 足够保守。
  - 风险-2: 若 summary 逻辑散落在多个 shell 脚本里，后续 contract 容易漂移，因此 merged summary 必须集中在单一 helper。
  - 风险-3: 若 operator 继续手工执行多个脚本，这套体系会再次退化成“有工具但无 canonical 入口”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-025-A | triad-observability-stack-T1 | `test_tier_required` | host fixture -> summary helper 输出 CPU/load/memory/storage/runtime 状态 | triad host/process contract |
| PRD-P2P-025-B | triad-observability-stack-T2 | `test_tier_required` | merged fixture + raw status fixture -> observability summary helper 输出 overall/node/module summaries | triad merged summary contract |
| PRD-P2P-025-C | triad-observability-stack-T3 | `test_tier_required` | real-env 小样本运行 + 文档/门禁检查 | triad 运维入口与 evidence 可执行性 |
| PRD-P2P-025-C | triad-module-observability-breakdown | `test_tier_required` | raw status fixture -> module breakdown + optimization candidates | triad module diagnosis contract |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-TOS-001 | 复用现有 snapshot/traffic/wasm 脚本并新增 host/merged 层 | 重写成一套独立大脚本 | 复用已有验证过的 status/traffic contract，改动面更小。 |
| DEC-P2P-TOS-002 | 保持 repo-owned 文件产物和 fixture 回归 | 直接接外部监控平台 | 当前需求是先把 triad 真实环境监控闭环补齐，而不是引入新基础设施。 |
| DEC-P2P-TOS-003 | 让 merged summary 基于 snapshot claim + resource alerts 两级判定 | 只看资源或只看链状态 | triad 健康需要同时回答“链是否通”和“资源是否快打满”。 |
