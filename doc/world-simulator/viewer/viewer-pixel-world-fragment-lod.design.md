# Viewer Pixel World Fragment LOD Terrain Rendering 详细设计

- 对应需求：[Fragment LOD PRD](viewer-pixel-world-fragment-lod.prd.md)。
- Owner：`viewer_engineer`；共同审读：game_visual_interaction_designer / qa_engineer。
- 历史任务：`task_428db5366f654c5e892ac300807cb9cc` / #986，原证据在Git/Task。
- 审读基线：`a2389f437e6cedc3b870a91d33a1cdf40a00a8da`；2026-10-16。

## 1. 问题、目标与非目标

terrain提供地形/材料背景，Location保留逻辑锚点，Agent/目标/关键关系仍先读。保持现有DTO/LOD/非交互合同；不扩世界schema、block picking、体素引擎或第二JS世界，也不以地形LOD代表完整大世界探索。

<a id="目标上游与当前实现边界"></a>
## 2. 上游约束与相关角色

Runtime提供fragment/location事实；Rust host派生显示；renderer管理实体；visual审读层级；QA分别验证字段、计算、像素与真实浏览器。产品多尺度、授权搜索、native和回到任务由[大世界设计](external-agent-world-view.design.md)承接，不由本背景层包办。

### 2.1 需求承接与分配表

| 上游 requirement / product AC | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[多尺度浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | 有fragment输入时派生稳定显示键、位置、footprint和逻辑锚点，不写回世界。 | [terrain-state](#terrain-state) | Runtime事实；Rust host。 | 不证明搜索/授权/原生及产品全部AC。 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[多尺度浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | screen-space LOD保留Agent可读层级，detail仍非交互。 | [terrain-lod-boundary](#terrain-lod-boundary) | renderer/visual/QA，当前bounds/canvas/zoom。 | 不实现dense索引、体素管线或block操作。 |

<a id="current-state"></a>
## 3. 当前状态、目标状态与差距

当前派生入口为 `crates/pixel_world_bridge/src/host_state.rs::build_render_state`；`pixel_world_host.jsx`消费桥接结果，`render_fragment_visuals.rs`做实体reconciliation。原JS Host Algorithm是历史实现叙述，可从上述固定基线Git追溯，不能继续作为当前source layer。

当前关键合同是可选fragment_terrain、Location marker_role/alpha、三档LOD和非交互。密集sprite churn与真实容量仍需测量；单元阈值/DOM通过不能证明实际渲染或性能。

<a id="design-decision"></a>
## 4. 边界与结构

snapshot facts → Rust host projection → render DTO → WASM terrain reconciliation；DOM语义测试只消费同一Rust派生形状，不拥有独立世界事实。world_bounds是完整世界坐标基准，viewport/分页范围在上游适配中独立。

<a id="dto-shape"></a>
### 4.1 DTO字段

`fragment_terrain[]`保留id、location_id、pos、footprint_cm、dominant_compound、color、emphasis；locations[]保留marker_role和marker_alpha。旧snapshot缺字段由optional/default兼容，不用虚构terrain填空。

`fragment:${location.id}:${index}`是当前显示键而非持久block身份。只在同world/location输入语境下复用；重排更新实际几何，切world/撤权清缓存，不能沿index合成世界历史。

<a id="host-algorithm"></a>
## 5. 关键运行流程

Rust先验证location位置、world bounds与block集合。对有效block取主ppm成分与稳定色板，按现有origin_cm + size_cm/2计算局部中心，结合location半径/范围和既有投影得到位置并限制在世界边界，派生footprint和显示键。有terrain时location退为logic_anchor，否则保留既有primary_marker。

不得通过材料颜色/尺寸推导可采集、库存、价值、所有权或世界效果；palette是工程表达，不是最终美术或资源权威。

<a id="renderer-algorithm"></a>
### 5.1 renderer更新

屏幕footprint使用当前world-to-canvas尺度及zoom；按既有三档分类。terrain低于links/agents/hotspots，更新现有键、创建缺项、回收失效项；不因单纯animation反复重建静态背景。terrain不新增hit region，hover/selection仍沿现有合法实体。

## 6. 接口与兼容合同

保持mount/update/tick/unmount、camera_state_changed、hover_entity、select_entity及WASM反序列化兼容。host/renderer成对更新；旧projection缺新语义时明确降级，不绕回JS派生。

CSS尺寸、canvas framebuffer、DPR和camera变换必须在同一候选记录；不在LOD与命中侧混用单位。本文不改变现有helper的像素口径或阈值，改变口径时必须同步专业要求与对应负例。

<a id="terrain-state"></a>
<a id="状态数据流与兼容边界"></a>
## 7. 状态、数据流与持久化

DTO可重建，不是事务日志或材料交易证明。无fragment时保留有效Location/Agent；没有world bounds、terrain被撤销、world切换或授权丢失时不得残留旧色块。

future WorldView的partial/page先由[适配层](external-agent-world-view.design.md#view-adapter)组装明确scope，当前页未出现不直接作为全世界删除。本层回收依据合法当前显示集合；viewport淘汰不是权威对象删除，不能改写world state。

<a id="fallback-dom"></a>
### 7.1 DOM、文本降级与故障恢复

已有Rust派生DOM/fixture可以按同一world-coordinate比例显示非交互terrain，验证背景/逻辑锚点语义；它只证明该测试或明确辅助表面，不是正式GPU证据。

生产图形不可用时显示Renderer Unavailable并保留获准的目标、Targets、文本、原回执和恢复入口，不构造第二JS世界。ready后context loss清旧canvas命中，重建后重新应用当前授权显示状态；旧私有图形不能因恢复而回放。

## 8. 部署与资源

资源沿canonical Viewer build/finalize与optional-payload manifest/hash，缺资源有明确原因。无需新增部署接口、数据库、权限或世界字段。此文档变更不要求重新发布未改变的WASM；实现改动时按实际source构建并验证产物。

<a id="terrain-lod-boundary"></a>
<a id="lod设计条款部署与质量边界"></a>
## 9. LOD设计条款与质量边界

保持 `<1.5px` hidden、`<10px` background、其余detail；屏幕尺寸来自world bounds/canvas/camera zoom，不能改固定zoom值。detail加强alpha/尺寸也不授予block交互。Agent可读尺度下terrain保持低层，密集时关键主体先于背景。

dense snapshot的sprite数量/更新成本仍是容量风险，当前没有本专题单独的密集性能承诺；未来整体负载按[大世界质量预算](external-agent-world-view.design.md#view-quality)验收，不能用helper测试宣称容量或帧率改善。

## 10. 迁移与回滚

仅整理历史JS叙述时不改DTO/行为；真实host/renderer迁移保持配对版本、旧snapshot兼容和非交互。回滚不能保留不再授权的terrain；不能用旧JS路径绕过Rust/WASM失败。配对PRD AC-5的完整组合仍适用，不以历史runner问题缩减。

<a id="verification"></a>
## 11. 验证设计

host字段、LOD计算、ECS实体、软件像素和实际浏览器独立验证；固定source、fixture、world bounds、canvas/DPR、zoom、target和GPU状态。变更后的负例包含缺bounds、无fragment、block撤销/重排、换world、权限缩小和partial适配。

### 11.1 验证映射表

| 上游 requirement | 本设计条款 | 独立 obligation 与适用条件 | 验证方法 / source / scenario / candidate | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | [terrain-state](#terrain-state) | 有fragment时字段/逻辑锚点正确；无效或失效输入不残留。 |[专项手册](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)； 测试手册；`crates/pixel_world_bridge/src/host_state_tests.rs`的 `rust_host_state_derives_fragment_agent_link_and_commercial_surface`与相应负例；PW-PROJECTION，固定native Rust candidate。 | 实际fixture/DTO/退出码及清理对照。 | 样例派生不证明真实GPU、生产snapshot或完整探索。 |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | [terrain-lod-boundary](#terrain-lod-boundary) | 屏幕footprint控制三档，Agent可读层级且无block命中。 |[专项手册](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)； 测试手册；`lib_tests.rs`的 `fragment_lod_uses_screen_space_size`、`fragment_lod_keeps_blocks_background_at_agent_readable_scale`；完整AC-5按配对PRD验证执行。 | bounds/canvas/DPR/zoom、helper输出、Bevy probe/raster、browser smoke、wasm check/build分别记录。 | helper/DOM不代签像素/实际browser/native、dense性能或block交互。 |

<a id="决策风险与未决范围"></a>
## 12. 决策、风险与未决范围

地形非交互、Location逻辑锚点、Rust唯一显示派生owner、WASM正式世界renderer保持不变。工程palette、显示键、LOD是表现合同，不产生业务事实；密集负载、真实平台、图形恢复仍需对应证据。本文件不追加本次测试日记或宣称AC-5已在新候选通过。
