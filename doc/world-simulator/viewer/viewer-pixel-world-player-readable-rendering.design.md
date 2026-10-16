# Viewer Pixel World Player-Readable Rendering 详细设计

- 配对要求：[玩家可读渲染 PRD](viewer-pixel-world-player-readable-rendering.prd.md)。
- Owner：`viewer_engineer`；视觉/QA共同审读。
- 审读：2026-10-16；基线 `a2389f437e6cedc3b870a91d33a1cdf40a00a8da`。
- 历史任务：`task_b399bf37eff94c44a300c55f5db739d3` / #1294；历史证据从Git/Task追溯。
- 本修订定义待实施的主体隔离与降级合同，保留既有HUD/Feed语义，不宣称新行为已经实现。

## 1. 问题、目标与非目标

像素世界由当前Rust显示投影提供HUD/board，不能让展示兜底污染动作或回执归属。目标是保持世界优先、有效关系可读、单一因果锚点与合法恢复；不扩世界schema、WASM宿主ABI、Shell布局或动作权限，不建立第二JS世界。

## 2. 上游约束与相关角色

产品 owns 玩家承诺；Shell owns Player/Director、dock、Search/Quote与布局；Runtime/Agent owns gameplay、权限、原操作和结果；Rust host owns DTO派生；JS host owns表现/交互；QA分别验证Rust、mock host、WASM、真实浏览器。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[Commercial DTO](viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | 现有gameplay派生HUD，无数据不造目标/进度。 | [dto-shape](#dto-shape) | Runtime/gameplay；Rust host。 | mock loader不证明Rust或世界执行。 |
| Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096；[主体绑定](viewer-pixel-world-player-readable-rendering.prd.md#subject-binding-requirements) | 焦点、提交、回执主体独立，原操作不被选择改写。 | [subject-binding](#subject-binding) | Runtime原操作关联、owner授权。 | 不在Viewer实现新admission。 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[Receipt](viewer-pixel-world-player-readable-rendering.prd.md#411-action-receipt-dtostate-mapping) | pending/advanced/no-progress/blocked/unknown/无回执分别解释。 | [action-receipt-mapping](#action-receipt-mapping) | 原feedback/operation/receipt。 | 环境事件不代签因果。 |
| Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3；[来源与降级](viewer-pixel-world-player-readable-rendering.prd.md#render-degradation-requirements) | 来源/时效独立、partial/失效/撤权不抹平。 | [display-provenance](#display-provenance)、[fallback-route-rendering](#fallback-route-rendering) | 授权数据客户端；WASM/host。 | 旧DTO不能表达时需降级，不假装current。 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[UI层级](viewer-pixel-world-player-readable-rendering.prd.md#42-ui-layering) | 单一Next Move主区、从属上下文及窄屏顺序。 | [ui-layout](#ui-layout) | Shell/visual authority。 | 不新增布局或证明完整S6。 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[Feed合同](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | 有序/去重/精确整数、cursor恢复、环境/因果隔离。 | [world-feed-visual-handoff-implemented](#world-feed-visual-handoff-implemented) | Runtime journal、transport、Viewer。 | 当前recentEvents不是此Feed。 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[产业目标](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | 有权威数据时节点、流、blocker及推断/未知可读。 | [industry-detail-target](#industry-detail-target) | Runtime产业projection/visual。 | 现有routes/HUD不等于完整产业图谱。 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[产业分层](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | World/Region/Node可见集和目标定位有界。 | [industry-scale-target](#industry-scale-target) | 空间投影/Viewer/QA。 | 不单独证明semantic zoom或完整探索。 |

## 3. 当前状态、目标状态与差距

<a id="current-state"></a>
### Current State

`pixel_world_host.jsx`将有界snapshot输入交给Rust `host_state.rs::build_render_state`，消费world_bounds、locations、fragment_terrain、agents、links、hotspots、selection和commercial_surface。WASM绘制世界，focus/right-panel兼容hooks与Targets的 `#entity-search`仍存在；独立 `world_feed/v1`由Runtime journal提供。

当前 `build_commercial_surface`会沿recommended target、accepted target、selection、first visible agent解析active_agent_id，并可用于next_action兜底；`build_action_receipt`也接收该值作为target回退。此为待核实整改的静态源码事实，不再作为正确主体语义，也不据此断言已经发生越权执行。

<a id="design-decision"></a>
### Design Decision

保留现有字段与Rust派生owner，分离内部主体解析函数，移除从展示焦点流向业务target的隐式路径。来源/时效不能表达时抑制不诚实动态图形并提供合法文本；不新增JS世界投影或重做Shell。

## 4. 边界与结构

<a id="dto-shape"></a>
### DTO Shape

commercial_surface继续包含objective.title/detail/progress_percent、next_action.label/detail/target_agent_id/execute_kind、active_agent_id、player_leverage.state/summary/detail、blocker、world_read及action_receipt。没有权威gameplay时不凭数组计数合成这些业务字段；world_read单独说明可见范围和partial。

内部可增加类型化结果，不必新增世界或WASM协议字段：

```text
DisplayFocus = explicit selection | supported recommendation | none
ActionBinding = explicit authorized subject + action/goal context | unavailable
ReceiptBinding = original-operation agent binding | authorized player scope | unknown
```

这些是设计类型，不声称源码已有同名类型。将焦点和原操作绑定分别传递，避免使用一个可选字符串承担三种意义。JS只消费Rust结果与已有独立授权UI状态，不另写buildTestRustRenderState作生产派生。

<a id="subject-binding"></a>
### 主体解析与不可变关联

1. `active_agent_id`最多是展示关注对象；无明确来源时null。默认构图与认领/可控标签分开，不把first visible agent当玩家杠杆。
2. `next_action.target_agent_id`只来自当前适用、明确主体的推荐动作/指导上下文。若原推荐无主体或无法授权绑定，execute保持不可用，提供选择/同步/Command入口；不能 `.or_else(active_agent_id)`。
3. 回执先按原operation与其受权feedback关联，再读取明确主体。仅有较新的acceptedIntentTarget、recommendedAction或当前selection不足以给旧回执归属。
4. 合法玩家级结果可保留target_agent_id=null，并明确玩家级；无法确认结果属于当前玩家/操作时不进入成功展示。
5. A在途切换B时，A的原结果仍关联A；B详情只读自身绑定结果。重复引用原receipt不产生第二效果，搜索/排序/重新连接不改变关联。

实现建议在 `host_state.rs`拆出独立解析helper，让 `build_action_receipt`不再接受展示active_agent_id作为fallback参数；调用处依赖明确的原操作关联。缺少上游关联时先fail closed并记录缺口，不新增客户端猜测或伪造Runtime字段。真正补充上游字段必须单独同步其authority及版本兼容。

## 5. 关键运行流程

<a id="host-algorithm"></a>
### Host Algorithm

Rust先解析获准世界显示集合与selection，再独立解析gameplay目标/下一步/原操作主体，构建receipt与world_read，最后附加commercial_surface。位置、关系、活动、Intent和因果分别验证，不按“有位置/有活动”推导关联。缺业务摘要时可以仍绘制获准世界，但不制造玩家目标。

<a id="action-receipt-mapping"></a>
### Action Receipt Mapping

以权威 `recent_feedback`及与同一原操作关联的execution_state、causality_kind/detail、last_world_change为来源。本地请求反馈仅能表达身份匹配的等待，不把local_pending草稿当accepted。

| 状态 | 表达 | 不允许 |
| --- | --- | --- |
| 无权威回执 | No action receipt yet。 | 从Feed/计数补效果。 |
| accepted/submitted/queued/ack | pending，显示原对象/原目标和等待原因。 | 完成文案或世界delta。 |
| completed_advanced | 原操作的权威进展。 | 使用别人的产出或当前选择补主体。 |
| completed_no_progress | 完成但无进展，保留原因/下一步。 | 统一成成功奖励。 |
| blocked/rejected | 原因与修正/重新确认路径。 | 静默改参数重投。 |
| recovery_required/unknown | 原结果待恢复确认，回查同一operation。 | 显示普通失败并创建新键。 |

`delta_logical_time/delta_event_seq`仅按原专业合同在有权威世界证据的阶段展示；缺值保持未知，不从动画时长/本地时间推算。confidence描述来源确定性，不把推测升成世界完成。

<a id="display-provenance"></a>
### 来源、完整性与权限

消费[大世界适配](external-agent-world-view.design.md#view-adapter)的独立几何/时效、cut、audience和partial侧带状态。旧renderer无法区分last-known/current时，不把陈旧精确坐标以普通当前marker输出；使用明确陈旧的受支持样式，否则文本降级。query页缺对象不等于删除，world_read不是全世界总量。

主体或audience变更先清旧私有图形/DOM/hover/搜索结果，再接收新数据。保留上下文仅限仍被授权披露的内容。显示current不是写许可，高后果动作仍走原Game API/Runtime重验。

## 6. 接口与数据合同

保留 `createPixelWorldBridge`、mount/update/tick/unmount、camera_state_changed、hover_entity、select_entity合同。无需为主体修正更改WASM ABI；不能把DTO里某个target存在当权限证明。JSON精确tick/revision按已有API处理，不能Number化再参与排序/身份。

<a id="ui-layout"></a>
### UI Layout

- `pixel-world-host__summary`：简短世界/目标，不重复长文。
- `pixel-world-command-strip`：Next Move主区及Objective/Player Leverage从属；窄屏主区第一行。
- `pixel-world-canvas`：terrain、有效关系、实体和选中/目标/blocker层次。
- `pixel-world-render-diagnostics`：折叠的renderer、camera、counts、raw DTO和测试控制。

<a id="terminal-shell-relationship"></a>
### Terminal shell relationship

[Shell PRD](viewer-gameplay-release-experience-overhaul.prd.md#current-implementation-boundary)拥有Player/Director、dock、console、Search/Quote和布局。这里不复活三张平级卡片、Next Move第二行或常驻Feed列，也不新增第二产品模式。

## 7. 状态、持久化与恢复

显示状态可重建，原operation与receipt由Runtime/API持久保存。页面重载/选中变化不重做已提交操作。renderer/network/query/task/executor状态独立，renderer失败不表示世界停止。

<a id="fallback-route-rendering"></a>
### Fallback Route Rendering

该旧anchor保留给历史引用，但必须按表区分表面；不再笼统要求“WASM不可用时DOM画一个世界”。

| 表面 | 有效合同 | 不证明/不允许 |
| --- | --- | --- |
| 正式WASM世界 | 使用Rust派生且获准、来源有效的关系，route在terrain与前景实体之间。 | location_id/邻近不产生关系；terrain/route不新增动作权限。 |
| Rust派生DOM语义测试或明确辅助表面 | 沿同一坐标投影验证已有关系、层级、非交互；固定fixture/source。 | 不证明GPU，也不是独立JS renderer或正式图形ready。 |
| 无GPU但仍可读取合法数据 | 目标/Targets/原回执/文本/安全控制与明确Renderer Unavailable。 | 不用DOM重建一个“正常世界”。 |
| Rust派生/资源不可用 | 既有独立授权UI仍可显示合法文本/原结果，解释图形依赖。 | 不以JS重算世界或复用旧私有图形兜底。 |

ready后图形失效需停止旧命中、区分旧帧/当前状态，重建本渲染会话后重新应用授权视图及camera；仅probe成功不算恢复。遵循[恢复状态机](external-agent-world-view.design.md#view-recovery)，不重置授权、任务或原operation。

<a id="world-feed-visual-handoff-implemented"></a>
### World Feed visual handoff (implemented)

`world_feed/v1`在fullscreen Player中是顶部边缘的紧凑折叠ambient chip，按需展开为有界面板；移动/平板可为安全区调整位置，但不成为常驻首屏列或success receipt。

source order与去重键为 `(world_id,reorg_epoch,event_seq)`，wire epoch/seq为精确非负十进制字符串，按精确比较器处理，旧数值只沿已支持兼容；不经JS Number损精度或按本地时间排序。

| Feed state | Visual treatment | Recovery rule |
| --- | --- | --- |
| loading | 预留安静框/短文案。 | 保留合法board/receipt。 |
| empty | 明确无事件。 | 不补事件、不隐藏原因。 |
| ready | 环境列表。 | receipt_ref仅来自Runtime明确身份。 |
| replay | 明确重放。 | 有界历史，不表达新收益。 |
| gap/reorg | 警告状态。 | 重取cursor/snapshot，不拼未知片段。 |
| unavailable | 明确原因。 | 提供适用恢复入口。 |

当前Recent Events/Feedback不重命名成Feed；Runtime没有明确关联时receipt_ref=null。`#viewer-world-feed`的源码/生成锚点保持。

<a id="player-visual-feedback-invariants-2026-09"></a>
### Player visual feedback invariants (2026-09)

移动Selected chip在安全区可换行；触控/键盘选择仍关联正确实体。LIVE仅表示连接且Feed ready，REPLAY/NO EVENTS/GAP/UNAVAILABLE各自有标记，均不代表玩家成功。展开Feed不能遮挡Next Move/Action Receipt。

Hotspot是只读检查：键盘/触控可读kind/label，说明保持到Escape或显式关闭并恢复焦点，不自动提交玩法动作。长Next Move/blocker/恢复文案在预留区域换行或内部滚动，不挤走主要操作。Escape/IME顺序沿页面模块设计。

## 8. 部署、安全与兼容

canonical viewer.js及dist compat alias由同一build/finalize生成，optional WASM遵守manifest/hash。源码与生成产物分别验证；本次文档不改构建策略。正式release和显式visual-test产物不互相代签；不把测试API开到release以绕过实际UI。

日志、DOM probe和截图只在授权测试范围取证，不包含token、模型Key、其他账号私有数据。native与Web复用同一显示合同，但逐平台实际验收。

## 9. 质量、产业分层与容量

<a id="industry-detail-target"></a>
### 产业关系与细节层级目标

未来受支持projection驱动一致的文本与舞台，包含节点、流类型/方向、吞吐/损耗、层级/阶段/状态和定位。R1~R5及bootstrap/scale/governance仅在数据支持时展示；根因推断与确认事实分开，缺字段明确未知。不用已有HUD/routes证明完整产业图谱。

<a id="industry-scale-target"></a>
### 产业空间层级目标

World/Region/Node分别优先热区/主干流、区域关系、配方/库存，保持目标定位与节点/边负载有界。复用受支持资产，不恢复旧图谱模块；数据扩展由Runtime authority另行承接。

当前目标/选中/blocker/关键路线优先于背景，重叠对象需可访问候选入口。容量、帧/输入/冷启动预算复用[view-quality](external-agent-world-view.design.md#view-quality)，不在此建立第二份数值真值。

## 10. 迁移与回滚

先给原主体切换/无绑定/玩家级结果补失败用例，再拆Rust解析helper和删除业务target兜底，随后更新host与同候选产物。时效/partial表达与WorldView适配分开验证，不能靠放宽现有关系authority使测试通过。

回滚到仍满足来源/归属/权限合同的构建；不兼容时只保留文本和原操作恢复，不恢复错误主体兜底，不改世界历史或pending。旧函数/JS算法可从固定基线Git读取，不再混作当前owner。

## 11. 验证设计

<a id="verification"></a>
### Verification

现有 `host_state_tests.rs`、`pixel_world_host.test.jsx`和相关定向测试为入口；新增case须能使旧fallback失败。mock loader的buildTestRustRenderState只证明mock host，不得当作Rust/WASM执行。软件raster、真实GPU、浏览器输入和真人理解分别留证。

### 11.1 验证映射表

| 上游 requirement / acceptance | 本设计条款 | 独立 obligation 与条件 | 方法 / test或manual source / scenario / environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Commercial DTO](viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | [dto-shape](#dto-shape) | 无gameplay不造目标/进度。 | `host_state_tests.rs`真实Rust；[PW-IDENTITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)空输入case，固定candidate。 | 输入和DTO断言、Rust退出码。 | mock host不证明Rust。 |
| [主体绑定](viewer-pixel-world-player-readable-rendering.prd.md#subject-binding-requirements) | [subject-binding](#subject-binding) | A在途切B/列表变化/原目标缺失。 | [PW-IDENTITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)；Rust与host分别回归。 | 原operation/target与新selection对照；旧行为负例失败。 | 未运行不宣称bug已修复。 |
| [Receipt](viewer-pixel-world-player-readable-rendering.prd.md#411-action-receipt-dtostate-mapping) | [action-receipt-mapping](#action-receipt-mapping) | pending/advanced/no-progress/unknown/无回执。 | 原Rust/host receipt tests及[PW-FRESHNESS](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-freshness)原操作恢复；实际S6同候选。 | 状态、delta、原结果和恢复记录。 | UI状态不证明世界效果。 |
| [来源与降级](viewer-pixel-world-player-readable-rendering.prd.md#render-degradation-requirements) | [display-provenance](#display-provenance)、[fallback-route-rendering](#fallback-route-rendering) | last-known不变current、partial不误删、撤权不留图形。 | [PW-PROJECTION](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)、[PW-RECOVERY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-recovery)；确定性+实际GPU。 | 页/DTO/权限、旧命中清理、恢复前后。 | GPU-disabled不证明ready后恢复。 |
| [UI层级](viewer-pixel-world-player-readable-rendering.prd.md#42-ui-layering) | [ui-layout](#ui-layout) | Shell主次及窄屏/输入不退化。 | `pixel_world_host.test.jsx`与[PW-ACCESSIBILITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-accessibility)、[PW-READABILITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-readability)。 | 同候选desktop/narrow截图、焦点、回答。 | DOM/模型审读不代签真人。 |
| [Feed合同](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | [world-feed-visual-handoff-implemented](#world-feed-visual-handoff-implemented) | 顺序/精度/去重/恢复及ambient隔离。 | 现有world_feed_state/transport/panel测试与[PW-FRESHNESS](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-freshness)，固定source/fixture。 | 精确序列、cursor、状态和截图。 | recentEvents不能替代journal Feed。 |
| [产业目标](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | [industry-detail-target](#industry-detail-target) | 仅有来源的节点/流/状态/根因。 | [产业验收](viewer-manual.manual.md#industry-detail-acceptance)，实际受支持projection/同候选headed。 | 原数据、未知/推断负例和截图。 | 不证明完整产业数据已提供。 |
| [产业分层](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | [industry-scale-target](#industry-scale-target) | 分层可见集、负载和目标定位。 | 产业验收及[PW-PERFORMANCE](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-performance)。 | 远/中/近可见集与实际质量数据。 | helper阈值不证明semantic zoom。 |

## 12. 决策、风险与未决范围

<a id="follow-up-brainstorm"></a>
### Follow-up Brainstorm

`viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md`保留候选command-lens/production-pulse/pixel-moment思路，不是第二Shell authority。新数据/交互须同步真实专业合同，不复活旧PM流程。

<a id="边界决策与风险"></a>
### 边界、决策与风险

保留世界主舞台、单一Next Move与原因果锚点，明确Rust owner与host消费。关键风险是上游缺原操作关联、旧字段无法表达来源时效、DOM/raster被误作真实GPU，以及将历史实现进度误作当前通过。缺口在实际PR/Task跟踪，不能由本文声明关闭。
