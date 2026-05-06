# oasis7：3D 渲染物理准确性设计（尺寸对齐真实物理数据）

- 对应设计文档: `doc/world-simulator/viewer/viewer-rendering-physical-accuracy.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-rendering-physical-accuracy.project.md`

审计轮次: 5

## 1. Executive Summary
- 建立一套可落地的 3D 渲染物理口径，使 viewer 中的**尺寸、距离、光照、材质响应**与世界模拟参数保持一致。
- 保证核心对象（世界空间、碎片、Agent）在渲染层按真实量纲显示，避免“视觉尺寸正确但物理量纲漂移”。
- 提供可测试的验收标准，确保后续参数调优不会破坏物理一致性。

## 2. User Experience & Functionality

### 范围内
- `GeoPos(x_cm,y_cm,z_cm)` 到渲染坐标的量纲映射（cm→m），以及大场景精度控制策略。
- Location/Agent 的几何尺寸规范（半径、身高、边界盒）与 LOD 策略。
- 基于真实物理数据的光照、材质参数基线（小行星带环境）。
- 辐射与热状态的可视化映射（不改变模拟内核公式，仅定义 viewer 显示口径）。
- 对应配置接口、数据结构草案和回归测试口径。

### 范围外
- 不修改 `oasis7` 内核物理规则（能耗、采集、热管理等逻辑仍由 simulator 决定）。
- 不引入高精度天体力学（轨道摄动、N 体引力、相对论效应）。
- 不在本阶段引入复杂资产管线（高模扫描、程序化贴图烘焙）。

## 真实物理数据与尺寸基线

### 1) 世界与对象尺寸基线

| 对象 | 当前模拟口径 | 渲染物理口径 | 说明 |
| --- | --- | --- | --- |
| 世界空间 | `100km × 100km × 10km` | `100_000m × 100_000m × 10_000m` | 与 `WorldConfig.space` 1:1 对齐 |
| chunk | `20km × 20km × 10km` | `20_000m × 20_000m × 10_000m` | 用于空间锚点与 rebase |
| 碎片直径 | `500m ~ 10km` | `500m ~ 10_000m` | 已与 `radius_min/max_cm` 对齐 |
| Agent 身高 | `height_cm`（默认 100cm） | `1.0m`（默认） | 与机体规格保持一致 |

### 2) 小行星带环境基线（用于光照/材质）
- 太阳常数（1AU）：`1361 W/m²`（基线常数）。
- 小行星带典型半径：`2.2~3.2 AU`；默认渲染距离取 `2.5 AU`。
- 默认太阳辐照度：`E = 1361 / d² = 1361 / 2.5² ≈ 218 W/m²`。
- 真空环境默认无大气散射；环境光主要来自间接反射与星空背景。
- 碎片表面温度可视口径：`120K~260K`（仅用于热态显示，不反向驱动物理内核）。

## 渲染系统详细设计

### A. 量纲映射与精度策略

#### A1. 统一单位（强约束）
- 渲染层固定 `1 world unit = 1 meter`。
- 输入 `GeoPos` 虽仍以 `f64` 承载协议兼容，但世界状态真值要求坐标已 canonicalize 到整厘米；viewer 不得把 sub-cm 浮点残差当作额外语义。
- 转换公式：
  - `pos_m = pos_cm / 100.0`
  - `radius_m = radius_cm / 100.0`
  - `speed_mps = speed_cmps / 100.0`

#### A2. 大场景浮点精度
- CPU 侧保留 `f64` 世界坐标（米）。
- GPU 提交前应用 `floating origin`：
  - 以相机所在 chunk 中心作为 `render_origin_m`。
  - 实体提交坐标 `local_pos = world_pos_m - render_origin_m`（`f32`）。
- 触发条件：相机偏离当前 `render_origin_m` 超过 `1000m` 时重置锚点。

#### A3. 相机裁剪面
- 主相机默认：`near=0.1m`，`far=25_000m`。
- 超远对象（>25km）采用简化轮廓层（impostor/标记），避免深度精度劣化。

### B. 几何尺寸与 LOD

#### B1. 碎片几何
- 真实半径驱动基础包围体：`sphere(radius_m)`。
- 细化几何采用非均匀缩放椭球扰动，但体积误差需控制在 ±5%。
- LOD 规则按屏幕误差（SSE）切换，禁止“按固定距离硬缩放导致真实尺寸失真”。

#### B2. Agent 几何
- Agent 基准高度直接取 `body.height_cm / 100`。
- 默认 1m 机体在 5m 距离处应保持可辨识轮廓，若不足通过轮廓描边补偿，不放大真实模型尺寸。

#### B3. UI 标注尺寸
- 3D 标签使用屏幕空间渲染（billboard），但其锚点必须绑定真实几何质心。
- 标签偏移量采用米制：`offset_y = radius_m + 0.5m`。

### C. 光照与曝光

#### C1. 主光源强度
- 配置项：`stellar_distance_au`（默认 2.5）。
- 太阳辐照度：`irradiance_w_m2 = 1361 / stellar_distance_au²`。
- 渲染光照（近似）
  - `directional_illuminance_lux = irradiance_w_m2 * luminous_efficacy`
  - 默认 `luminous_efficacy = 120 lm/W`，因此默认约 `26_000 lux`。

#### C2. 曝光
- 使用固定 EV100 + 自动微调混合策略：
  - 默认 `ev100 = 13.5`。
  - 根据画面亮度分位数（P50/P95）小范围调节 ±1EV。
- 目标：在“阴影区可读 + 高亮不爆白”之间保持稳定。

### D. 材质物理参数库

| 材质 | 密度(kg/m³) | 反照率(albedo) | 粗糙度 | 金属度 | 发射率(emissivity) |
| --- | --- | --- | --- | --- | --- |
| silicate | 2800 | 0.12 | 0.82 | 0.02 | 0.92 |
| metal | 7800 | 0.55 | 0.35 | 0.95 | 0.18 |
| ice | 920 | 0.65 | 0.20 | 0.00 | 0.97 |
| carbon | 1800 | 0.06 | 0.88 | 0.05 | 0.85 |
| composite | 3200 | 0.25 | 0.60 | 0.35 | 0.70 |

- `LocationProfile.material` 直接映射到上述材质库。
- 如场景自定义了材质参数，viewer 以场景覆盖值优先。

### E. 辐射/热状态可视化

#### E1. 辐射显示换算
- 输入：`radiation_emission_per_tick`（模拟口径）。
- viewer 仅用于解释性换算：
  - `radiation_power_w = radiation_emission_per_tick * power_unit_j / time_step_s`
  - `radiation_flux_w_m2 = radiation_power_w / reference_area_m2`
- 默认 `reference_area_m2 = 1.0`，并在 UI 中标注“解释性估算”。

#### E2. 热态显示
- 输入：`heat`、`thermal_capacity`。
- 定义 `thermal_ratio = heat / max(thermal_capacity,1)`。
- 颜色映射：
  - `<=0.6` 冷色
  - `0.6~1.0` 暖色
  - `>1.0` 过热高亮闪烁

### F. 渲染数据流
1. 接收 `Snapshot/Event`（cm、tick、材质、辐射、热）。
2. 物理量纲层完成 cm→m、能量口径换算。
3. 精度层执行 `floating origin` 与相机裁剪参数同步。
4. 材质/光照层生成 PBR 参数并提交渲染。
5. UI 层输出真实尺寸与解释性物理指标。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 配置结构（草案）

```rust
pub struct ViewerPhysicalRenderConfig {
    pub enabled: bool,
    pub meters_per_unit: f32,              // 固定默认 1.0
    pub floating_origin_step_m: f64,       // 默认 1000m
    pub camera_near_m: f32,                // 默认 0.1
    pub camera_far_m: f32,                 // 默认 25_000
    pub stellar_distance_au: f32,          // 默认 2.5
    pub luminous_efficacy_lm_per_w: f32,   // 默认 120
    pub exposure_ev100: f32,               // 默认 13.5
    pub reference_radiation_area_m2: f32,  // 默认 1.0
}
```

### 数据字段约束
- 输入源仍为 `WorldSnapshot` / `WorldEvent`，不改变协议主结构。
- viewer 额外读取并消费：
  - `GeoPos`（厘米语义；进入 viewer 前已 canonicalize 到整厘米）
  - `WorldConfig.space`（世界真实尺寸）
  - `WorldConfig.physics.time_step_s / power_unit_j`
  - `LocationProfile.material / radius_cm / radiation_emission_per_tick`
  - `Agent.body.height_cm`

### 向后兼容
- 当 `enabled=false` 时，保留现有压缩尺度渲染行为。
- 升级路径：先接入只读配置，再逐步切换默认模式。

## 验收与测试口径
- 尺寸一致性：抽样 100 个对象，`render_radius_m` 与 `radius_cm/100` 误差 < 1%。
- 距离一致性：任意两点可视距离与模拟距离误差 < 0.5%。
- 光照一致性：`stellar_distance_au` 在 `2.2~3.2` 变动时，照度符合 `1/d²` 单调衰减。
- 材质一致性：同一光照下，`metal` 高镜面、`carbon` 低反照率表现稳定。
- 过热可读性：`thermal_ratio` 跨越 1.0 时可见明显状态变化。

## 5. Risks & Roadmap
- **RPA-1（文档与配置）**：完成设计文档、配置草案、参数默认值与范围。
- **RPA-2（尺寸与精度）**：实现 cm→m 与 floating origin，完成尺寸回归测试。
- **RPA-3（光照与材质）**：接入真实光照公式与材质参数库，完成截图闭环验证。
- **RPA-4（辐射/热可视化）**：接入解释性物理指标面板，补齐回归测试与文档收口。

### Technical Risks
- 大场景深度精度风险：若不做 origin rebase，100km 量级下易出现抖动与 Z-fighting。
- 物理参数语义风险：模拟内核为抽象单位，viewer 解释换算需明确“可视化口径”以免误读。
- 性能风险：真实尺度下远景对象数量多，需配套 LOD 与实例化策略。
- 美术与真实性冲突：视觉“好看”可能驱动非物理调参，需要通过配置分层（真实模式/演示模式）隔离。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
