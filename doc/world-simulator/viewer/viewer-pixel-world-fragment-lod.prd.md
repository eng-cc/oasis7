# Viewer Pixel World Fragment LOD Terrain Rendering

- 对应设计：[Fragment LOD设计](viewer-pixel-world-fragment-lod.design.md)。
- Owner：`viewer_engineer`；visual/QA共同审读。
- 产品来源：[玩家可读的世界舞台](../../product/agents-world-simulation/player-readable-world-stage.prd.md)。
- 关联：[语义定位](viewer-pixel-world-semantic-positioning.prd.md)、[视觉规范](viewer-visual-design-spec-2026-06-05.design.md)。
- 历史任务：`task_428db5366f654c5e892ac300807cb9cc` / #986；原证据从Git/Task追溯。

## 1. Executive Summary

Fragment terrain提供世界地表/材料背景，Location保留逻辑锚点。Agent、目标、有效关系、blocker和原结果优先；地形细节不创造交互或资源权威。

当前派生owner是Rust `host_state.rs::build_render_state`，JS host消费结果；旧 `buildPixelWorldRenderState` 是历史名称。WASM是正式世界渲染内核，rendered DOM只在已有Rust派生语义测试或明确辅助表面范围使用，不是WASM失效时的第二JS世界。

## 2. User Experience & Functionality

## 目标

地形在Agent可读尺度降为背景，足够大的屏幕投影才增加细节；Location弱化但仍支持既有选择/关系上下文。没有地形数据不伪造地形，渲染失效不把空画布解释为空世界。

## 范围

范围内为Rust DTO、terrain sprite/reconcile、屏幕LOD、既有DOM语义测试与分层验证。范围外为世界snapshot字段、正式block交互、体素切割/资产、quadtree picking、dense terrain索引或3D工具链恢复。

### In Scope

从既有 `location.fragment_profile.blocks.blocks` 派生fragment_terrain，含所属location、位置、footprint、主成分与稳定工程色板。terrain在grid之上、关键关系/Agent/hotspot之下，Location降为logic_anchor。detail只加强视觉，不为后续交互预先授予命中/操作资格。

### Out of Scope

不新增world server字段、binary asset format、六视图体素管线或直接采矿/建造；不把本专题LOD当完整大世界semantic zoom或密集容量承诺。

## 3. User Stories

玩家能读到Agent所在世界的地形背景，又不会被色块淹没目标。工程师通过可选DTO保持旧snapshot兼容。QA分别验证派生、LOD计算、ECS、像素与真实浏览器，不用一个helper单测代签全部。

## 4. Technical Specifications

## 接口 / 数据

输入为现有 `WorldSnapshot.model.locations[*].fragment_profile.blocks.blocks`。输出fragment_terrain使用serde optional/default兼容；无fragment_profile时保留获准Location/Agent/关系/热点，不合成terrain。

### 4.1 Fragment Terrain DTO

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；仅承接[REQ-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)的背景空间表达。

- `id`：现有 `fragment:${location.id}:${index}` 显示键；不是持久世界block身份。
- `location_id`：所属逻辑锚点。
- `pos`：现有location位置与block局部中心经过既有世界投影得到的位置。
- `footprint_cm`：现有block size的平面footprint口径。
- `dominant_compound`：有效ppm最大项；缺失/未知有明确工程回退色，不代表资源可采集。
- `color`：按compound的确定性RGB映射，非最终美术资产。
- location设置 `marker_role=logic_anchor`、较弱marker_alpha与size hint。

ID只在同world/location输入上下文有意义；换world或权限失效清相关缓存。block重排不得把index显示键作为权威连续历史；几何与内容按当前输入更新。world_bounds必须是世界基准而非当前分页包围盒。

### 4.2 Screen-Space LOD

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；局部承接[REQ-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)。

根据footprint_cm、world_bounds、canvas尺寸与camera zoom计算屏幕尺寸，保持现有 `<1.5px` hidden、`<10px` background、其余detail。不能替换成固定zoom门槛。测试记录CSS viewport、framebuffer、DPR和helper实际输入，避免不同像素单位混用。

background无block边框/命中，detail仍不参与选择。密度变化先保留Agent/目标/关键关系；terrain变细不授予新操作。

### 4.3 Layering

grid最低，terrain为背景，Location为弱逻辑锚点，合法links/agents/hotspots及任务提示保持前景。无fragment时不清除仍有效对象；无世界边界或地形撤销时不能留下上一快照孤立色块。

### 4.4 部分输入与降级

未来WorldView的分页/partial先经[大世界适配](external-agent-world-view.design.md#view-adapter)合并为明确scope的显示输入；本renderer不把当前页缺block解释成全世界删除。LOD不会解决授权、分页或全量世界传输。

正式renderer失效沿[降级合同](viewer-pixel-world-player-readable-rendering.prd.md#render-degradation-requirements)保留合法文本/Targets/原回执并明确unavailable；不由JS重建terrain、关系或旧私有世界。

## 5. Risks & Roadmap

## 里程碑

初始M1文档/RED、M2 Rust/terrain显示、M3分层验证是历史专题划分，不恢复旧项目流程义务。当前改动按实际PR和适用测试记录。

## 风险

密集block可能增加sprite/reconcile成本，必须实测，不能从单测推导性能；工程palette可替换但不能暗示不存在的资源/交互；稀疏或缺fragment时诚实回退。wasm-target单测runner异常不允许跳过真实渲染层证据或缩减AC-5。

## 6. Acceptance Criteria

- AC-1：Rust从既有location fragment blocks派生fragment_terrain，字段和来源正确。
- AC-2：有terrain的Location为弱logic_anchor，Agent/任务仍可读。
- AC-3：WASM接受并绘制terrain，保持mount/update/tick/unmount、hover/select和camera合同。
- AC-4：按屏幕footprint分类，覆盖hidden/background/detail阈值与实际canvas/zoom组合。
- AC-5：Host UI tests、native pixel_world_bridge Rust tests、Bevy render-probe、Bevy pixel regression、browser visual smoke、wasm cargo check、build和doc/diff hygiene均按适用完整组合取证；helper/DOM不代签像素和真实浏览器。

无效bounds、删除/重排block、换world/撤权、partial输入与恢复是上述状态/兼容义务的负例；历史AC不因文档整理被取消。

<a id="verification"></a>
## 7. Validation & Decision Record

| 范围 | 验证方法 | 证据边界 |
| --- | --- | --- |
| DTO/LOD/reconcile | `env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge --lib`；host_state_tests/lib_tests与既有terrain测试。 | 固定fixture/source/target，真实Rust与mock分开。 |
| Host和构建 | `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host`；canonical build:viewer。 | DOM和生成物不证明GPU。 |
| 实际渲染 | `./scripts/viewer-pixel-world-bevy-render-probe.sh`、`./scripts/viewer-pixel-world-bevy-pixel-regression.sh`、`./scripts/viewer-pixel-world-fragment-visual-smoke.sh`。 | 实际source、bounds/canvas/DPR/zoom、像素和browser分别记录。 |
| WASM/文档 | `env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`、doc-governance及diff检查。 | 不以构建成功代签产品探索。 |
| 跨范围/恢复 | [PW-PROJECTION](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)、[PW-RECOVERY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-recovery)。 | 不扩大为block交互或密集世界已通过。 |

实际结果保留在对应PR/Task；本要求不自报本轮执行成功。
