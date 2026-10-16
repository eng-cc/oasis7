# Viewer Pixel World Player-Readable Rendering

- 配对设计：[玩家可读渲染系统设计](viewer-pixel-world-player-readable-rendering.design.md)。
- 专业 owner：`viewer_engineer`；visual/QA 共同审读。
- 上游产品：[玩家可读的世界舞台](../../product/agents-world-simulation/player-readable-world-stage.prd.md)。
- 关联：[视觉规范](viewer-visual-design-spec-2026-06-05.design.md)、[语义定位](viewer-pixel-world-semantic-positioning.prd.md)、[Fragment LOD](viewer-pixel-world-fragment-lod.prd.md)、[大世界读模型](external-agent-world-view.design.md)。
- 历史任务：`task_b399bf37eff94c44a300c55f5db739d3` / #1294；历史实现与证据在 Git/Task 中，不能代签新合同。
- 审读：2026-10-16；基线 `a2389f437e6cedc3b870a91d33a1cdf40a00a8da`。

## 1. Executive Summary

像素世界应先回答当前目标、下一步、blocker及原行动造成的变化，而不是把模拟器数据或诊断全部铺满。当前显示派生 owner 是 Rust `host_state.rs::build_render_state`；`pixel_world_host.jsx` 消费 runtime.deriveRenderState，WASM 是唯一世界渲染内核。

本修订统一商业HUD、主体归属、来源时效和回退语义。不新增世界状态字段、第二JS renderer、3D workstream或Fragment交互。新的空间查询/分页目标由大世界系统设计负责，不能假装当前渲染输入天然是完整大世界。

## 2. User Experience & Functionality

## 目标

- 在规定的首读测试中，玩家5秒内辨认目标、下一步和主要blocker；方法见专项手册，数字不是当前实测结果。
- Agent/有效关系/原行动反馈优先于Location marker、terrain及诊断。
- 选中对象、动作主体和回执归属不混淆；无权威目标/回执时保持空或未确认。
- renderer、raw DTO、性能和测试控制默认折叠；失败时保留合法文本与恢复，不绘制替代世界。

## 范围

范围内：Rust commercial_surface派生、Solid host HUD、受支持关系表达、来源/主体/状态约束、定向回归和canonical build。范围外：世界schema、设施/生产线新增协议、dense terrain引擎、3D、正式美术资产和Shell重排。

### In Scope

当前render state保留commercial_surface的objective、next_action、active_agent_id、player_leverage、blocker、world_read及action_receipt。字段从明确的权威gameplay输入与获准显示集合派生；原函数名 `buildPixelWorldRenderState` 仅为历史名称，不再建立JS派生owner。

### Out of Scope

不新增world server/snapshot规则，不把button直接搬进地图，不新增Fragment hit region，不修改mount/update/tick/unmount及宿主事件ABI，不以本专题宣称完整产业图谱或semantic zoom。

<a id="industry-detail-requirements"></a>
### 产业图谱与空间信息层级：目标合同

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)的相关对象/blocker可读性。

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；局部承接[REQ-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)的分层可见集。

受支持产业projection应区分工厂/配方/产品/物流节点、物料/电力/数据流向、层级/阶段/状态和location/Agent/module定位；吞吐、损耗、瓶颈、拥塞、告警仅展示权威数据。支持P3分层档案时，R1~R5和bootstrap/scale/governance符号可辨，状态独立且不只依赖颜色。

World/Region/Node分别保留热区/主干流、区域关系、配方/库存，密度裁剪优先目标、相关路线和blocker，限制节点/边负载。未知与推断根因明确区分。当前agents/links/hotspots/HUD不证明完整产业图谱、吞吐流或三档semantic zoom。

## 3. User Stories

玩家能先看世界与下一步，再读原行动结果；查看B不会把A的成果贴到B上。制作人能区分玩家影响与环境活动。工程师能在现有Rust派生和host内维护同一来源，不为视觉兜底引入第二套世界真值。

## 4. Technical Specifications

## 接口 / 数据

输入包括现有 `WorldSnapshot.player_gameplay`、agents/locations和fragment blocks。Action Receipt来自权威 `player_gameplay.recent_feedback`及与原操作一致的因果字段；World Feed独立来自Runtime持久WorldEvent journal。`state.recentEvents`只是非契约ambient preview，三者不混用。

没有gameplay summary时，获准的静态world_read/ambient诊断可用于解释当前可见集合，但不得由计数合成objective、next action、player leverage、blocker、receipt或进度。world_read须说明计数是当前视点/授权范围，不把partial集合包装成世界总量。

### 4.1 Commercial Surface DTO

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)。

| 字段 | 有效语义 | 不允许的兜底 |
| --- | --- | --- |
| `objective` | 适用玩家目标；Agent私有目标另需明确绑定。 | 用场景计数/全局summary冒充选中Agent目标。 |
| `next_action` | 当前受支持推荐/下一步及明确的执行类型、主体；无法安全绑定时退为选定对象/同步/打开Command。 | 用active_agent_id或首个Agent补可执行target。 |
| `active_agent_id` | 展示关注对象，不能当作控制证明；无明确来源可为null。selection仍独立。 | 列表首项被当成玩家杠杆/已认领对象，并继续流入动作或回执。 |
| `player_leverage` | 已接受意图、执行状态和可归因结果；属于原主体/目标。 | 环境变化、其他玩家结果、视觉焦点当玩家影响。 |
| `blocker` | 当前目标/操作的明确阻塞和恢复方向。 | 任意其他实体状态作为当前失败原因。 |
| `world_read` | 当前获准显示集合的静态读数及范围。 | partial计数冒充世界全量，计数增长冒充玩家进度。 |

<a id="subject-binding-requirements"></a>
#### 主体绑定要求

Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096；承接[REQ-AGENT-STAGE-005](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-005)。

展示焦点、提交主体和回执主体分别解析。已提交对象由原operation固定，不随selection、推荐、列表顺序或重连变化。无明确操作/主体关联时不渲染该Agent的成功；有合法玩家级结果但没有Agent目标时保留玩家级，不强造Agent。当前first-agent fallback和target串联是待整改的历史实现，不再作为正确行为要求。

### 4.1.1 Action Receipt DTO/state mapping

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；承接[AC-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-001)。

`commercial_surface.action_receipt`是现有玩家因果展示锚点。权威来源为原操作的recent_feedback、execution_state、causality_kind/detail、last_world_change；本地反馈最多在等待Runtime时表达与原请求明确关联的pending，不证明世界变化。未提交草稿不是pending receipt。

| DTO field | Source mapping | Player meaning |
| --- | --- | --- |
| `present`, `state`, `confidence` | 权威反馈或明确关联原请求的等待状态。 | 有/无回执，等待/完成/阻断/拒绝/待恢复确认。 |
| `title`, `summary`, `detail` | 原行动、effect、reason、hint及权威变化。 | 尝试了什么、什么改变/未改变、原因与下一步。 |
| `target_agent_id`, `effect_kind` | 原operation/intent/action明确绑定的主体和因果类。 | 不随选择变化；无Agent的玩家级结果可为null。 |
| `delta_logical_time`, `delta_event_seq` | 权威原反馈delta，仅在合同允许的已发生阶段展示。 | 不是预估，不因accepted/queued填入世界增量。 |

accepted/submitted/queued/ack是pending；completed_advanced表示进展，completed_no_progress仍是无进展结果；blocked/rejected保留原因与恢复。recovery_required/未知结果先查原操作，不伪装失败并发新请求。缺权威回执显示 `No action receipt yet`；Recent Events/Feed不能补回执。不能用较新的acceptedIntentTarget或recommendedAction给较早回执猜归属。

### 4.1.2 World Feed v1 implementation contract

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)。

Viewer owner消费有序持久WorldEvent journal，稳定anchor为 `#viewer-world-feed`。环境事件与receipt视觉独立；缺失/loading/unavailable解释原因与恢复；gap/reorg按cursor恢复并重取snapshot。当前Recent Events/Feedback保留原名，不静默重命名或转成因果结果。

### 4.1.3 World Feed v1 additive schema (implemented)

| Field | Required contract |
| --- | --- |
| `schema_version` | `world_feed/v1`；未知版本fail closed到unavailable。 |
| `world_id` | 权威世界与cursor作用域。 |
| `reorg_epoch` | 历史epoch，变化时旧cursor失效。 |
| `cursor` | 绑定world/epoch及最后event_seq的opaque token。 |
| `events[]` | source升序；去重键 `(world_id,reorg_epoch,event_seq)`。 |
| `event_seq` | 同world/epoch内单调；保留精确整数。 |
| `kind`, `summary`, `detail` | 环境表达，不是因果证明。 |
| `receipt_ref` | nullable，只按Runtime显式因果身份提供，不能从时间/文本/delta推断。 |
| `status` | loading、ready、empty、replay、gap、unavailable。 |

reorg_epoch/event_seq在wire上使用精确非负十进制字符串；Rust为u64，已支持的legacy数值输入仍按原兼容合同处理，不能经JS Number损失精度。gap/reorg不静默拼接事件；Feed永不替代action_receipt。

### 4.2 UI Layering

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)。

布局沿[Shell当前合同](viewer-gameplay-release-experience-overhaul.prd.md#current-implementation-boundary)：一个Next Move主区，Objective/Player Leverage从属；窄屏主区第一行、上下文第二行，不恢复三张平级卡片。canvas前景为选中/目标/blocker，中层为有效agents/routes/hotspots，背景为Fragment/Location逻辑锚点，诊断折叠。

### 4.3 Player Leverage Rule

player_leverage仅从明确适用的accepted intent/原世界变化/execution cause派生。没有已接受意图时明确说明，不以“世界活跃”代替。与目标/下一步可读地相邻，不因无数据生成默认成功。

<a id="render-degradation-requirements"></a>
### 4.4 来源、降级与恢复

Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3；承接[REQ-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-002)。

几何来源和新鲜度独立；partial不是全量。读取不可用、GPU不可用、ready后图形失效、断连、撤权分别处理。生产世界只由WASM渲染；已有Rust派生DOM/fixture用于相应语义测试或明确辅助表面，不成为Rust/WASM不可用时的独立JS世界。

降级保留当前仍被授权的目标、文本、Targets、原回执/原操作回查及安全控制。图形失效时禁用旧canvas命中，恢复后重建本会话并重验当前授权/来源/选择。不因保留上下文泄漏已撤权数据，不自动提交、重放世界动作或调用模型。

## 5. Risks & Roadmap

## 里程碑

初始M1文档、M2 Rust DTO/HUD、M3定向验证为历史专题划分，不恢复旧PM义务。后续按当前PR完成主体解析、时效/回退和负例；大世界分页与原生依赖另按对应设计交付。

## 风险

旧active-agent链污染因果；全局summary伪装Agent目标；有效位置被误当有效关系；partial计数被当进展；mock loader被当Rust执行；降级形成第二renderer。产业数据不足时只展示已有明确数据，不猜设施/吞吐字段。

### Terminal implementation slices and evidence

| Check | Expected evidence | Not satisfied by |
| --- | --- | --- |
| board/HUD | 目标、Next Move、leverage与范围语义；无游戏摘要不造目标。 | ambient计数或mock截图。 |
| receipt | 无回执、queued、advanced、no-progress、blocked、unknown及主体切换。 | Feed文本、当前选择或推荐目标。 |
| feed | v1精确顺序、去重、cursor、gap/reorg与状态。 | 本地时间或文字匹配。 |
| responsive | desktop/390x844及适用tablet、键盘、触控、CJK。 | 单元/DOM结构。 |
| recovery | 初始不可用、ready后失败、撤权和原操作不重做。 | 只有GPU-disabled截图。 |

## 6. Acceptance Criteria

AC-1至5保留原专题覆盖范围，历史执行只对原候选有效；下面说明当前Rust owner和适用表面，不把历史JS叙述继续当实现合同。

- AC-1：Rust `build_render_state`派生commercial_surface，不扩世界协议；无gameplay时不合成目标/进度。
- AC-2：首屏目标、下一步、leverage可读，diagnostics折叠，布局遵守Shell。
- AC-3：已有Rust派生语义/DOM测试表面只在独立关系authority成立时显示route，terrain背景且非交互；正式WASM真实层级另验，不创造生产JS回退。
- AC-4：分层测试覆盖DTO、HUD、诊断、正确主体与无数据负例；mock host不能代签Rust。
- AC-5：适用 `test:ui -- pixel_world_host`、canonical `build:viewer`、文档检查及diff hygiene；旧 `build:software-safe`仅在真实存在的compat脚本范围使用。
- AC-6：A操作在途选择B、推荐改变、列表重排、重连，回执仍属于A原操作；无Agent绑定结果不借B或首项。
- AC-7：主体/权威反馈缺失、玩家级结果、草稿、pending/unknown和环境活动不合成Agent成功或delta。
- AC-8：位置来源/时效、partial计数、失效/撤权及旧回复不被UI抹平；高后果操作沿原authority重验。
- AC-9：renderer启动与ready后失效均可解释、安全恢复；没有第二JS世界、旧命中或重复提交。
- AC-10：5秒首读、密度、语言/输入和图形矩阵按专项手册取得相应证据，不能以构建通过代签。

AC-6/7承接REQ-AGENT-STAGE-001/005；AC-8承接002；AC-9/10在001/003/004适用范围内细化，不另建立产品权威。

## 7. Validation & Decision Record

| 范围 | 验证入口 | 证据边界 |
| --- | --- | --- |
| Rust派生/主体/因果 | `crates/pixel_world_bridge/src/host_state_tests.rs`；[PW-IDENTITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)。 | 真实Rust输出和mock host分别记录。 |
| HUD/Feed/键盘/语言 | `pixel_world_host.test.jsx`及[S6](../../testing/manual/web-ui-agent-browser-closure-manual.manual.md)。 | DOM不证明GPU/真人理解。 |
| 来源、partial与恢复 | [PW-PROJECTION](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)、[PW-RECOVERY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-recovery)。 | 全链和真实renderer另取证。 |
| 首读/密度/性能 | [PW-READABILITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-readability)、[PW-PERFORMANCE](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-performance)。 | 不把设计预算写成实测结果。 |

DEC-WS-046-COMM-1：复用现有Rust显示语义层，不为首屏扩世界协议。DEC-WS-046-COMM-2：诊断次级，世界/目的/结果优先。主体与时效修订是实施输入；本文件不记录本次CI或宣称代码已经修复，实际结果由PR/Task保存。
