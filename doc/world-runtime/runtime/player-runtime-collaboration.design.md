# 玩家与外部 Runtime 持久协作系统设计

- 设计状态：target contract，待实现与独立评审；不是当前能力或发布报告。
- Canonical repository：`eng-cc/oasis7`；审读基线：`03fb58784a65de9f02a43dfeafc3e882bb04e5f2`（已合入产品 PR #4493）。
- Owner role：`runtime_engineer`；最后审读：2026-10-10；新增合同 revision：`1`。
- 产品入口：[外部 Runtime 游玩](../../product/agents-world-simulation/external-agent-runtime-play.prd.md)；[玩家与 Runtime 协作](../../product/agents-world-simulation/player-runtime-collaboration.prd.md)。
- 系统总入口：[LLM / external runtime 系统入口](../../world-simulator/llm/README.md)；写作规范：[系统设计规范](../../engineering/doc-governance/system-design-writing-standard.design.md)。

本文中的新组件、路由、记录和测试场景均为后续实施输入。`accepted` 只表示所属权威已受理，世界效果必须读取既有权威回执；文档合入不代表这些接口已经上线。

## 1. 问题、目标与非目标

Web 发布目标不能等同于 Codex/OpenClaw 已收到或世界已执行。需要一份跨设备、跨执行进程可恢复的游戏任务，明确目标、消息、审批与当前执行方，且不能产生第二个拥有世界写权的任务平台。

本设计负责 task/goal/executor fence、协作消息、授权申请、版本冲突、持久化与补送。世界动作和 grant 的最终判定仍使用现有 Runtime；模型上下文属于外部 Runtime，跨 Runtime 私有记忆无损迁移、无限期托管和把聊天解析成批准不在范围内。

## 2. 上游约束与相关角色

Runtime owner 负责世界侧 control fence、顺序、授权及恢复；Agent owner 负责实际执行器和上下文关联；Viewer owner 区分状态与来源；QA 核验迟到、重复、撤销和掉电窗口。本文是产品协作分册的技术承接，不改变 Prompt 的 accepted/applied 语义。

### 2.1 需求承接与分配表

| 上游条款 | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 排除与未覆盖范围 |
| --- | --- | --- | --- | --- |
| [AC-EXT-006](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-006) | 目标调整真正进入有效上下文，旧待决结果独立处理 | [collab-goal](#collab-goal) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-008](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-008) | 断连重启保留原世界任务且不复活失效授权 | [collab-recovery](#collab-recovery) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-COLLAB-001](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-001) | 跨设备共享任务并隔离被替换的执行方 | [collab-binding](#collab-binding) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-002](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-002) | 保存、读取、采纳、用于决策分别可验证 | [collab-goal](#collab-goal) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-003](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-003) | 计划和阶段汇报不能冒充权威成果 | [collab-content](#collab-content) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-004](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-004) | 问题/回答关联原任务和目标，旧问题不改新目标 | [collab-content](#collab-content) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-005](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-005) | 有效 owner 批准与累积预算在执行时重验 | [collab-approval](#collab-approval) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-006](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-006) | 重复消息、迟到和历史缺口安全恢复 | [collab-recovery](#collab-recovery) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-007](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-007) | 实际执行在线、停止和重新接入可区分 | [collab-binding](#collab-binding) | runtime_engineer / agent_engineer / viewer_engineer | 不证明自然语言策略正确或第三方工具已获授权 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前基线 | 目标 / 未闭合部分 |
| --- | --- | --- |
| GoalSnapshot | `crates/oasis7_agent_api/src/lib.rs` 导出目标与上下文合同；Harness PRD §9 描述 revision/digest | 承接 owner 发布版本、activation、executor read/adopt 与新决策使用，不以同一布尔值替代 |
| 生命周期 | `agent-cognition-lifecycle.design.md` 有 turn、envelope、journal、wake 与 recovery 合同 | 增加任务控制元数据及跨消息恢复；不宣布原合同所有 target 已实现 |
| 玩家控制 | Prompt 产品 PRD 约束 control-lost、stale draft 和 accepted/applied | 新协作端点必须使用同一 owner 校验和冲突规则，不绕过旧边界 |
| 权威运行 | Viewer 手册记录 chain runtime 单独管理持久 execution-world/checkpoint | 协作影响动作许可的部分必须进入相同权威写序，不能由网关内存批准 |

## 4. 边界与结构

<a id="collab-binding"></a>
### 4.1 唯一控制记录

每个 world/Agent 首期最多一个 active task 和一个 active decision executor。`TaskControl` 由当前世界权威维护，包含 task_id、owner/control revision、active_goal_revision、goal_digest、grant_revision、executor_id、executor_epoch、task_status 与最后控制提交引用。

`executor_epoch` 是 Runtime 持久单调 fence，不是客户端时间、token expiry 或内部 session ID。初次绑定、重新接管、替换及 stop 均经 owner 授权并形成新的 control revision；旧 epoch 即使仍持有未过期 token 也不能发起新副作用。不同执行进程可以共享只读历史但不能并行占有一个 Agent；内部多工具不获得独立 fence。

身份服务证明“调用者是谁”；TaskControl 证明“谁现在可以替该 Agent 作出提案”。读取历史仍须按当前 owner/audience 判定，转让后不默认向新执行方披露旧 owner 私有消息。玩家所有权转让/撤销的生效点消费现有专业合同，不重新定义资产权利。

### 4.2 两类数据，不同权威

**权威控制元数据**：目标版本/摘要引用、executor fence、grant 与预算、任务运行许可，进入 Runtime control journal/checkpoint，与世界行动按一致顺序重验。

**私有协作正文**：Prompt、计划、问题和回复使用 audience-scoped 的持久内容存储，密文/索引放在 Runtime 所属的受控存储域；世界公开快照仅保存授权可公开的承诺或不透明引用，不公开可枚举的私有正文指纹；私有 goal digest 只在受权上下文内使用，不复制明文到公共 P2P、广播事件或玩家公开世界流。首期复用现有执行世界存储代际机制，新增私有 namespace，不要求独立数据库或跨服务分布式事务。

报表、心跳和模型自报是 advisory。server 可以证明已接收，但不能据此证明模型真的理解了目标，或将自报“完成”写成生产成果。

## 5. 关键运行流程

<a id="collab-goal"></a>
### 5.1 目标发布到实际用于决策

1. 玩家读取 task snapshot，同时取得 expected goal/control revision。写入时先重验身份/控制权；control-lost 不返回新 owner 的内容或差异。
2. `PUT goal` 必须带 client_operation_id、expected revision、完整目标与允许的软约束。expected goal revision 比较 latest_goal_revision（包含已保存但尚未激活版本），task_revision 同时防止改绑竞态。旧基准 `409 goal_conflict`；保留草稿，不自动最后写覆盖。
3. 完整正文先持久化并校验 digest；提交 GoalVersionSaved 引用。目标在权威控制提交中激活后才更新 active_goal_revision；accepted/pending activation 与 active 分开返回。
4. 原执行器增量读取目标，分别提交 `received` 与 `adoption_reported`，绑定 task、revision、executor_epoch 和独立消息 ID。旧确认可以保留历史，但不前推当前版本的状态。
5. Harness 捕获该激活版本的上下文。边界记录 `context_presented`；执行器返回合法提案、等待或阻塞时记录 `decision_submitted`，世界 admission 验证时记录 `decision_context_validated`。这些只证明版本关联，不能证明模型内部语义服从；界面不得把单纯自报升级为“已实际执行”。
6. 目标再次变化使旧 context 不可用于新的副作用；旧提案必须重新取得上下文并作出新决策。已经受理的待决动作按当时及提交时有效的授权与 Runtime 规则重验，已结算成果不撤销。

自然语言偏好与可强制授权分离。预算、对象范围、风险限制和撤销必须以 typed grant/control 字段表达并由 Runtime 校验，不能寄望模型阅读一段 Prompt 来强制执行。

<a id="collab-approval"></a>
### 5.2 申请、明确批准与世界执行

申请由 executor 提出，记录 task/Agent、当前 owner revision、goal revision、executor epoch、action/proposal digest、对象/资源来源、累计额度、有效期、风险及拒绝后路径。普通文本与“好/同意”只进入消息历史，不能生成 grant。

owner 决定调用专用 decisions 端点，带 expected request revision 与幂等 ID；当前认证、控制权、目标/申请适用性先于返回差异。审批有 `open / approved / rejected / expired / invalidated`；同一申请只能进入一个终态。超时不批准；目标、owner 或执行方变化默认使该目标绑定申请 invalidated，新的申请必须重新确认。

approved 表示权威决定已提交并产生引用现有授权体系的 grant，不代表行动成功。行动 admission/commit 重新检查 grant 是否仍有效、remaining cumulative allowance、资源来源、对象范围及其他前置；同一 grant 的并发预留和消费由 Runtime 原子执行，不能把每次上限误当总上限。失败释放预留必须关联原 reservation，不能负消费或释放两次。

授权撤销/stop 不等执行器 ack：从 Runtime 的有效提交点开始拒绝新受限提案。已经提交但未结算的高后果动作按既有 Runtime 主责重新评估；API 不能承诺所有任务瞬间物理停止、所有模型费用立即停止或已结算结果回滚。游戏授权与操作系统、模型账户、第三方工具授权保持独立。

<a id="collab-content"></a>
### 5.3 汇报、问答与任务完成

Report 包含 task、目标版本、来源执行方、reported_at/received_at、phase、summary、blocker、next_actor 与 evidence_refs。服务器只校验引用存在与读取权限，不信任任意 URL 或未经回读的 receipt 字符串。UI 清楚标记“执行器汇报/估计”和服务器接收时间；不保存或展示模型内部推理链与秘密。

Question/Answer 使用稳定 question_id、message_id、in_reply_to、goal revision 和 actor。提问或回答是协作消息，不写世界。旧目标的问题被新版本替代后，迟到答案留为 stale history；要影响新任务必须有明确的新问题或目标修改，不自动搬运批准。

可机器验证的任务结束依赖绑定的 gameplay completion evaluator 与权威 receipt。任意自由文本目标无法机械判断时只能 `completion_reported` 或由 owner 显式结束委托，不能称世界已证明完成。日常请求批准不应替代 Agent 自主选择；等待期间仅允许原授权内不规避 blocker 的独立行动。

## 6. 接口与数据合同

HTTP 路由由 Game API 设计拥有；本节定义它们调用的 RuntimePort 命令与记录。所有整数 revision/seq/epoch 用 JSON 十进制字符串，类型未知、内容超限和相同消息 ID 异内容一律拒绝。成功响应包含 operation/ref 和目前权威状态，不把内存写入当已持久化。

```text
TaskSnapshotV1 = {world_binding, task_id, agent_id, owner_revision,
  task_revision, latest_goal_revision, active_goal_revision, active_goal_digest, grant_revision,
  executor_binding, task_status, activation_operation?,
  goal_delivery, latest_reports, unanswered_questions, pending_approvals,
  unsettled_operation_refs, snapshot_seq, resume_cursor, completeness}
ExecutorBindingV1 = {executor_id, executor_epoch, device_ref,
  permitted_scopes, activated_at_tick, revoked_at_tick?}
GoalDeliveryV1 = {goal_revision, executor_epoch, received_at?,
  adoption_reported_at?, context_ref?, decision_ref?, admission_ref?}
CollaborationMessageV1 = {message_id, kind, actor_ref, task_id,
  goal_revision, executor_epoch?, in_reply_to?, body,
  evidence_refs, received_at, applicability: current|stale|invalidated}
ApprovalRequestV1 = {approval_id, request_revision, task_id,
  owner_revision, goal_revision, executor_epoch, proposal_digest,
  requested_grant_scope, cumulative_limit, valid_until_tick,
  status, decision_ref?, grant_ref?}
```

端点不接受客户端填写的“世界结果 status=committed”来写回权威任务成果。`goal-receipts` 只允许 received/adoption_reported；context/decision/admission 的证据由各自受信边界记录。对没有外部观测能力的纯 Runtime 自报，保留 reported 等级，不伪造硬件或模型证明。

控制命令至少包含 `PublishGoal / BindExecutor / StopDelegation / SubmitApprovalDecision`，复用 typed owner authority。消息类命令 `PostReport / AskQuestion / AnswerQuestion / AcknowledgeGoal` 不能改 grant。每条命令都携带稳定 operation ID、expected revision 和所属世界分支；通道切换不改变身份作用域。

## 7. 状态、事务与持久化

<a id="collab-recovery"></a>
### 7.1 原子提交与恢复切点

私有正文写入新代际并 fsync，随后 Runtime control transaction 原子提交版本引用、fence/grant 更新与待发送事件。事件只从已提交记录投影；崩溃在正文写入与 control commit 之间留下不可见孤儿，安全回收不能丢失已提交引用。commit 后回复丢失通过原 operation ID 返回相同结果；不把网络投递当成事务的一部分。

单权威部署沿现有持久 world/journal 路径保证进程重启可恢复。多节点使用当前已获准的 writer/replication 合同，不能让 API 自选主节点；承诺容灾前必须验证私有正文存储与 control reference 的可用性。若元数据存在但正文不可读或校验不符，任务进入 recovery_required，新认知暂停，绝不使用空目标代替。

控制状态 `active / suspended / closed` 与执行诊断 `awaiting_executor / thinking / waiting_world / waiting_player / blocked / offline / unknown` 正交。心跳只有诊断意义；即使实例报 thinking，也不授予 epoch。wall-clock 失联计时不直接决定确定性世界提交；所有失效授权与 writer 交接经 Runtime 有序生效。

### 7.2 增量投递和缺口

Runtime 生成 task-scoped 单调 event_seq，投递至少一次。cursor 是服务端签发的不透明值，绑定 world branch、task、受众与权限 revision、位置及有效窗口；不能将全局消息总量或其他人的事件通过游标泄露。

收到消息后执行器先持久化本地 inbox/去重位点，再确认。服务端同 message_id 同内容幂等，不同内容冲突。任务 snapshot 包含一致读取切点 S：该切点的当前目标、授权、未答问题、待决批准和未结算 operation，与 `events after S` 接续。snapshot 生成时并发新消息只会出现在增量中，不会落在两者之间。

游标过期、分支/受众变化或存储窗口外返回 `cursor_reset_required`，提供允许访问的当前 snapshot 获取方式；客户端标记历史缺口，不当作“无新事件”。恢复只消费当前有效目标/批准，不按旧历史顺序重演副作用；已知世界 operation 逐项先对账。

### 7.3 停止、交接与数据寿命

Stop 的效果是持久失效当前执行许可并阻断新受限提案，保留问题、审批和回执；模型进程终止是 adapter 的另一条 best-effort 操作。同一已授权设备的重启可在当前委托明确允许续接时，经 Runtime 的 CAS 重新取得执行代际，不要求每次重启都重新人工审批，也不能延长原授权；已撤销、到期或换设备则必须重新获得有权主体确认。新 worker 只有在有效绑定与新 fence 生效后才可取得 context，旧实例的迟到操作拒绝且可审计。

终态控制记录、幂等 tombstone、审批和世界结果引用与该世界的可审计历史一致保留；事件传输日志可以压缩，不能删去 current snapshot 需要的 pending 信息。私有内容保留期、数据删除与受众规则由部署公开说明，首期传输重放窗口初值为 7 天且每任务最多 100000 项，以先到达的限制为准并在接口公布实际最早可读位置；超出时显式 snapshot reset，而不是丢掉当前问题。备份恢复必须验证世界/control generation 一致并旋转受影响凭据；旧 fence 不得因恢复较早备份重新有效；必须同时验证既有 WorldBinding 的 branch/reorg/finality 变化和身份服务的撤销状态。无法证明旧执行资格已失效时保持 RecoveryRequired，不在本地私自生成一个“更大 epoch”冒充世界权威。

## 8. 部署、安全与运行约束

读任务、订阅事件、查看报告、审批和查原操作分别鉴权，不能只在最初 WebSocket 握手验证一次。配对 token 不能签发 owner 决定；公开日志不含 private prompt、审批材料、refresh token 或本机路径。Content store 加密密钥由 Runtime operator 管理，不写公开 chain snapshot。

新目标可以在 executor 离线时保存并激活，但界面显示 awaiting_executor。skill 不负责保活，客户端不能在用户不知情时启动本机 Codex/OpenClaw。审批审计记录保留做出决定的当前身份、scope、版本和 control commit，不把 UI 勾选或普通文字当世界 grant。

## 9. 质量与容量

复用 Game API LimitsV1：goal 4 KiB，message/report 8 KiB，列表最多 200，单响应 2 MiB。每 task 首期最多 256 个未答问题加待决申请；超过返回 quota_exceeded，不丢弃未答项。保留索引是可重建投影，pending 的 source 必须持久。

恢复目标是同一完整 checkpoint 中已确认 control operation 不丢失；这是目标，不是现有线上 RPO 声明。并发验证必须含两个 owner 窗口、两个 executor、目标更新/撤销与世界 commit 交错。事件读取不能阻塞 World::step，report 洪泛不得挤占 stop/撤销的有界控制队列。

## 10. 兼容、迁移与回滚

新增协作记录使用 revision 1 独立 namespace，旧 Prompt/profile 可作为显式初始 GoalVersion 导入，保留来源与 owner 重新确认；不得把历史聊天批量变成新批准。旧 provider history 可以只读追溯，不迁移其不可信“成功”成 world receipt。

实现可复用既有 goal/journal，但必须提供新语义的完整回读，不通过 alias 默默丢掉版本。回滚关闭新命令、保留任务与操作结果查询；schema 不可读时 fail closed，不清空状态或自动回退到旧 executor。未满足持久一致性与私有存储验证前不能对外承诺跨节点恢复。

## 11. 验证设计与可追溯性

先在普通 CI 使用 fake RuntimePort/fake driver 和实际持久存储做故障注入，再以真实 Runtime + Web/native 做跨设备体验。主动扩大 scope/修改旧世界权限不属于本次文档交付。

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立 obligation 与适用条件 | 准确验证方法与场景 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-EXT-006](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-006) | [collab-goal](#collab-goal) | 目标调整真正进入有效上下文，旧待决结果独立处理 | [collab-goals](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-goals)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-008](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-008) | [collab-recovery](#collab-recovery) | 断连重启保留原世界任务且不复活失效授权 | [collab-recovery](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-recovery)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-COLLAB-001](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-001) | [collab-binding](#collab-binding) | 跨设备共享任务并隔离被替换的执行方 | [collab-bind](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-bind)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-002](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-002) | [collab-goal](#collab-goal) | 保存、读取、采纳、用于决策分别可验证 | [collab-adoption](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-adoption)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-003](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-003) | [collab-content](#collab-content) | 计划和阶段汇报不能冒充权威成果 | [collab-report](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-report)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-004](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-004) | [collab-content](#collab-content) | 问题/回答关联原任务和目标，旧问题不改新目标 | [collab-questions](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-questions)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-005](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-005) | [collab-approval](#collab-approval) | 有效 owner 批准与累积预算在执行时重验 | [collab-approval](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-approval)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-006](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-006) | [collab-recovery](#collab-recovery) | 重复消息、迟到和历史缺口安全恢复 | [collab-events](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-events)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |
| [AC-COLLAB-007](../../product/agents-world-simulation/player-runtime-collaboration.prd.md#ac-collab-007) | [collab-binding](#collab-binding) | 实际执行在线、停止和重新接入可区分 | [collab-executor](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-executor)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明自然语言策略正确或第三方工具已获授权 |

## 12. 决策、长期风险与未决问题

ADR-COLLAB-1：控制许可放在世界权威序列，私有正文单独保护；选择一致性而不是网关内存立即生效。正文投影不拥有 grant，代价是不可达时需要显式 pending/recovery。

ADR-COLLAB-2：一 Agent 一当前执行方，epoch 隔离旧实例；首期不做多控制者冲突仲裁。自然语言服从不可由 ack 证明，因此报告、上下文关联与世界效果分层展示。

Runtime owner 在实施中必须确定 private namespace 与现有 generation/checkpoint 的实际落点，并验证 restore 不回退有效 fence；身份 owner 需验证 owner 转让时私有历史可见策略。任一未就绪只阻塞相关写入/恢复承诺，不通过新建第二套数据库或管理规则掩盖缺口。
