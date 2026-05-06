# oasis7 Simulator：世界初始化（设计分册）

- 对应设计文档: `doc/world-simulator/scenario/world-initialization.design.md`
- 对应项目管理文档: `doc/world-simulator/scenario/world-initialization.project.md`

审计轮次: 5

本分册描述世界初始化（World Initialization）的最小实现，用于在不依赖外部输入的情况下生成可运行的初始世界状态。

## 1. Executive Summary
- 提供**确定性**的世界初始化流程：同一配置与种子产生一致的世界模型。
- 支持最小可运行世界：原点地点 + 初始 Agent + 可选小行星带碎片。
- 初始化产物可直接用于 `WorldKernel`，并保持时间与事件日志语义清晰。

## 2. User Experience & Functionality

### In Scope（V1）
- `WorldInitConfig`：初始化参数（seed、原点地点、自定义地点、小行星带碎片生成、初始 Agent、初始电力设施）。
- 原点地点创建：默认位于空间中心，可配置位置与画像与资源存量。
- 自定义地点创建：显式指定地点列表，可配置位置/画像/初始资源。
- 小行星带碎片生成：复用 `generate_fragments`，并支持单独 seed 偏移。
- 初始 Agent 生成：按数量与前缀批量创建，按 seed 从可用地点中确定性随机出生，可配置初始资源。
- 初始电力设施：支持发电设施的预置与校验。
- 初始化输出：生成 `WorldModel`，并提供 `WorldKernel` 便捷构造。
- 基础校验：越界位置、ID 冲突、出生地点不存在、负资源、设施参数非法等。

### Out of Scope（V1 不做）
- 初始设施/资产/合约/交易等复杂社会系统初始化。
- 多阵营/多区域的高级场景模板。
- 大规模地图分片与并行生成。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 关键结构
- `WorldInitConfig`
  - `seed: u64`
  - `origin: OriginLocationConfig`
  - `locations: Vec<LocationSeedConfig>`
  - `asteroid_fragment: AsteroidFragmentInitConfig`（小行星带碎片生成配置）
  - `agents: AgentSpawnConfig`
  - `power_plants: Vec<PowerPlantSeedConfig>`
- `WorldScenario`
  - `minimal`：原点 + 1 Agent，不启用小行星带碎片
  - `two_bases`：原点 + 2 个基地地点 + 2 Agent，不启用小行星带碎片
  - `power_bootstrap`：原点 + 1 Agent + 基础发电设施
  - `resource_bootstrap`：原点 + 1 Agent + 初始电力/硬件/数据库存
  - `twin_region_bootstrap`：双区域 + 双 Agent + 基础发电与资源库存
  - `triad_region_bootstrap`：三区域 + 三 Agent + 分层电力与资源库存
  - `triad_p2p_bootstrap`：三 P2P 节点 + 3 Agent（Agent 出生地点由 seed 随机）
  - `asteroid_fragment_bootstrap`：小行星带碎片开启 + 原点 + 1 Agent + 发电设施与基础资源
  - `asteroid_fragment_twin_region_bootstrap`：小行星带碎片开启 + 双区域 + 双 Agent + 基础电力与资源
  - `asteroid_fragment_triad_region_bootstrap`：小行星带碎片开启 + 三区域 + 三 Agent + 基础电力与资源
  - 场景定义文件：`crates/oasis7/scenarios/*.json`（编译期嵌入）

### 场景使用建议
- `minimal`：用于测试核心流程或最小单 Agent 回归。
- `two_bases`：用于验证多地点移动与基础互动。
- `power_bootstrap`：用于验证发电设施路径。
- `resource_bootstrap`：用于验证资源库存与交易/消耗逻辑。
- `twin_region_bootstrap` / `triad_region_bootstrap`：用于验证多区域资源/设施差异与运输成本。
- `triad_p2p_bootstrap`：用于验证 P2P 多节点拓扑与节点内 Agent 初始分布。
- `asteroid_fragment_bootstrap`：用于验证小行星带碎片/辐射采集与热管理路径。
- `asteroid_fragment_twin_region_bootstrap`：用于验证小行星带碎片下的多区域资源/设施差异与运输成本。
- `asteroid_fragment_triad_region_bootstrap`：用于验证小行星带碎片下的三方资源/设施差异与协作/竞争。

### 场景别名
- `two_bases`：`two-bases`
- `power_bootstrap`：`bootstrap`
- `resource_bootstrap`：`resources`
- `twin_region_bootstrap`：`twin-regions`
- `triad_region_bootstrap`：`triad-regions`
- `triad_p2p_bootstrap`：`triad-p2p` / `p2p-triad`
- `asteroid_fragment_bootstrap`：`asteroid_fragment`
- `asteroid_fragment_twin_region_bootstrap`：`asteroid-fragment-regions`
- `asteroid_fragment_triad_region_bootstrap`：`asteroid-fragment-triad`

### 小行星带碎片种子策略
- 初始化使用 `seed + asteroid_fragment.seed_offset` 生成小行星带碎片，确保与其他随机源解耦。
- 各场景为小行星带碎片设置不同的 `seed_offset`，避免同一 seed 下的碎片分布完全一致。
- 新增场景时应选择未使用过的 `seed_offset`，保持分布差异可追踪。
- `WorldInitReport.asteroid_fragment_seed` 会记录实际使用的小行星带碎片种子，便于回放与比对。

### 场景 ID 稳定性
- 现有场景 ID 视为稳定接口，不应随意改名或删除。
- 新增场景应保持命名风格一致，并补充别名与稳定性测试。
 - 场景文件名需与场景 ID 一致（`{id}.json`）。
- `OriginLocationConfig`
  - `enabled: bool`
  - `location_id/name/profile`
  - `pos: Option<GeoPos>`（空则取空间中心；`GeoPos` 直接使用整数厘米）
  - `resources: ResourceStock`
- `LocationSeedConfig`
  - `location_id/name/profile`
  - `pos: Option<GeoPos>`（空则取空间中心；`GeoPos` 直接使用整数厘米）
  - `resources: ResourceStock`
- `AsteroidFragmentInitConfig`
  - `enabled: bool`
  - `seed_offset: u64`
  - `min_fragment_spacing_cm: Option<i64>`（可选，覆盖小行星碎片最小间距）
- `AgentSpawnConfig`
  - `count: usize`
  - `id_prefix: String`
  - `start_index: u32`
  - `resources: ResourceStock`
- `PowerPlantSeedConfig`
  - `facility_id/location_id/owner`
  - `capacity_per_tick/fuel_cost_per_pu/maintenance_cost`
  - `efficiency/degradation`
- `WorldInitReport`
  - 统计创建数量（locations/agents）与使用的 seed
- `WorldInitError`
  - 越界位置、ID 冲突、出生地点缺失、负资源等错误

### 初始化语义
- 初始化在**时间 0** 完成：
  - `WorldKernel.time = 0`
  - `journal` 为空（初始化视为“世界既成事实”）。
- 生成过程使用 `WorldConfig` 的 `space/asteroid_fragment/power` 配置。
- 任意初始化入口写入 `WorldModel` 时，位置坐标都必须已经满足 `SPACE_UNIT_CM=1` 的整数厘米合同；不接受也不持久化 sub-cm 坐标。
- 生成顺序：origin → custom locations → asteroid fragments → agents → facilities（确保依赖对象已存在）。
- 场景模板通过 `WorldInitConfig::from_scenario(scenario, config)` 生成初始化配置。
- 场景文件由 `WorldScenario` 加载并转换为 `WorldInitConfig`。

### 示例工具
- `oasis7_init_demo`：命令行示例，按场景生成世界并输出统计摘要。

### 使用示例
- 运行示例工具：`env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_init_demo -- minimal`
- 查看场景列表：`env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_init_demo -- --help`
- 运行自定义场景文件：`env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_init_demo -- --scenario-file path/to/scenario.json`
- 示例输出包含每个地点的资源摘要（electricity/hardware/data）。
- 示例输出包含每个 Agent 的资源摘要（electricity/hardware/data）。
- 示例输出包含估算的小行星带碎片数量（asteroid_fragment_fragments）。
- 示例输出包含每个地点的设施统计（power_plants）。
- `--summary-only` 可仅输出概要统计（隐藏详细列表）。

## 5. Risks & Roadmap
- **I1**：定义初始化配置与报表结构，输出 `WorldModel`。
- **I2**：接入 `WorldKernel` 便捷构造与基础校验；补充单元测试。
- **I3**：扩展初始化模板（资源/多据点）与更多可配置项。
- **I4**：扩展初始化模板（发电设施）与更多校验/测试。

### Technical Risks
- 默认小行星带碎片生成可能导致初始化耗时波动（需可配置关闭）。
- 依赖浮点随机数的确定性：需确保同平台可复现。
- 初始化不写入事件日志，可能影响“完整事件回放”场景。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
