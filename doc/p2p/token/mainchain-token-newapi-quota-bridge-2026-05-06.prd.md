# oasis7 主链 Token 到 New API 内部额度桥接方案（2026-05-06）

- 对应设计文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `oasis7` 当前已经有 `OC`、`oc:pk:` 与签名化 transfer submit，但还没有一条正式、可审计、不会误滑成“公开兑换所”的 `OC -> AI 服务额度` 路径。若直接把链上代币、浏览器转账、`New API` 站内余额和公开换汇混在一起，容易把 preview-grade 资产面误包装成生产级钱包或双向交易能力。
- Proposed Solution: 冻结一版 producer-owned 的 `one-way OC -> New API quota` bridge PRD，要求通过独立部署的 bridge-service，把已确认的 `OC` 入账映射为 `New API` 的内部 quota / redeem credit；同时明确该能力只是 operator-managed service-credit bridge，不是公开兑换所、不是 AMM，也不支持自动提现回 `OC`。
- Success Criteria:
  - SC-1: MVP 口径明确为 `one-way OC -> New API internal quota/redeem credit`，`OC <- quota` 兑回、自动提现、做市、价格发现与公开交易所语义全部排除在外。
  - SC-2: 每一笔成功 credit 都必须可追溯到 `bridge_deposit_id -> chain tx / deposit account -> bridge_ledger row -> New API credit mutation / redeem credit issuance`，幂等对账完整率 `100%`。
  - SC-3: bridge-service 必须独立于 `oasis7_chain_runtime`、`oasis7_web_launcher` 与 `New API` 本体部署，并冻结独立的 custody、watcher、credit adapter 与 operator review 权限边界。
  - SC-4: 系统必须支持“唯一入账映射”策略：同一笔 `OC` 入账在进入自动 credit 前，必须能唯一绑定到一个 `New API` 用户、一个 bridge order 或一个 redeem credit。
  - SC-5: 对外 claim 必须明确写成“limited preview operator-managed service-credit bridge”；任何 `公开兑换所`、`浏览器热钱包充值`、`双向提现` 或“链上代币已可直接购买模型服务并自由兑回”的说法都不得进入 allowlist。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要冻结“这到底是服务额度桥，还是交易所/钱包产品”的边界，避免对外承诺越界。
  - `runtime_engineer`：需要知道桥接能力依赖哪些现有链上 truth，哪些不应直接下沉进 runtime。
  - bridge operator：需要运维独立 bridge-service、custody、watcher、对账与异常处理。
  - `New API` operator：需要把 bridge credit 接到 `New API` 的内部 quota 或 redeem credit 入口，而不是改造上游模型结算逻辑。
  - invited user / contributor：需要把 `OC` 充值为可消费的 AI 服务额度，而不是理解成可随时双向提现的交易资产。
  - `qa_engineer` / `liveops_community`：需要验证账本一致性、错误签名、重复 credit、错绑账户与对外文案边界。
- User Scenarios & Frequency:
  - invited user top-up：每次需要补充 `New API` 使用额度时触发。
  - operator reconciliation：每日或每批次核对 `OC` 入账与 `New API` credit 发放。
  - pricing review：每次调整 `OC -> quota` 定价表、活动补贴或 invite 策略时触发。
  - security / claims review：每次准备公开该能力、扩展自助面或升级资产口径时触发。
- User Stories:
  - PRD-P2P-TBRIDGE-001: As a `producer_system_designer`, I want one canonical one-way bridge boundary, so that `OC` 的服务额度充值不会被误解成公开交易所或双向兑回承诺。
  - PRD-P2P-TBRIDGE-002: As a bridge operator, I want every deposit to map to one unique beneficiary and one idempotent ledger row, so that confirmed `OC` 入账不会重复 credit 或错发给别人。
  - PRD-P2P-TBRIDGE-003: As a `New API` operator, I want a single credit adapter contract, so that链上入账确认后可以稳定落到 `New API` 的 quota / redeem credit，而不需要改动上游模型结算。
  - PRD-P2P-TBRIDGE-004: As a `qa_engineer`, I want stable anomaly states and manual-review gates, so that underpay / overpay / unknown user / adapter failure 都不会静默变成错误额度。
- Critical User Flows:
  1. Flow-TBRIDGE-001: `用户先登录 bridge portal 并绑定 New API 用户身份 -> bridge-service 分配唯一 bridge deposit account 或 order -> 用户在受信转账面向该账户转入 OC`
  2. Flow-TBRIDGE-002: `chain watcher 观察到新入账 -> bridge_ledger 记录 detected/pending_confirmations -> 达到确认窗口后转为 confirmed -> credit adapter 向 New API 发放 quota / redeem credit -> ledger 转为 credited/reconciled`
  3. Flow-TBRIDGE-003: `入账无法唯一匹配用户、订单或金额策略 -> ledger 转为 manual_review -> operator 审核后才允许补发、拆分、退回或关闭`
  4. Flow-TBRIDGE-004: `credit adapter 第一次调用失败 -> bridge_ledger 保留 idempotency key -> 重试 worker 继续补发 -> 直到 credited 或转为 manual_review`
  5. Flow-TBRIDGE-005: `外部提出“既然能充值，是否也能提现 / 公开交易” -> 对照 PRD 检查 one-way 边界 -> 若没有生产级 custody、公开钱包体系与双向风控，则直接驳回`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 用户绑定 | `bridge_user_id`、`newapi_user_ref`、`oasis_sender_account_id`、`status` | 用户完成登录并绑定 `New API` 身份与可用的 `OC` 付款来源 | `unbound -> pending_verification -> active/rejected` | 同一个活跃绑定在同一时刻只能指向一个默认 `New API` beneficiary | 仅 bridge user 自己可发起；operator 可停用或重绑 |
| 入账路由 | `deposit_route_type`、`deposit_account_id`、`order_id`、`expires_at` | bridge-service 分配唯一 deposit account 或 order | `draft -> issued -> expired/settled` | 自动 credit 前必须能把入账唯一映射到一个活跃 route | 仅 bridge-service 生成；operator 可手动作废 |
| 链上充值检测 | `chain_tx_id`、`to_account_id`、`amount_oc`、`observed_at`、`confirmations` | watcher 轮询或订阅链上 truth，记录到 `bridge_ledger` | `detected -> pending_confirmations -> confirmed/rejected` | 只有达到确认窗口且未命中 anomaly 才允许进入 credit 阶段 | 只读链上；不得修改 runtime 链状态 |
| `bridge_ledger` 对账 | `bridge_deposit_id`、`beneficiary_ref`、`pricing_version`、`credit_units`、`idempotency_key`、`state` | 记录 bridge 全流程状态并驱动重试/审计 | `detected -> confirmed -> crediting -> credited -> reconciled`，或 `manual_review/failed/closed` | `idempotency_key` 必须对同一笔入账稳定；任何终态不得重复 credit | 仅 bridge-service worker 和 operator 可写 |
| New API credit adapter | `target_type=quota|redeem_credit`、`target_ref`、`adapter_response` | 调用 `New API` 管理侧入口，写入站内额度或生成兑换 credit | `ready_to_credit -> crediting -> credited/failed` | 精确入口可随部署版本变化，但调用契约必须支持幂等与审计回写 | 仅 bridge-service 服务账号可调用 |
| 定价策略 | `pricing_version`、`oc_amount`、`credit_units`、`bonus_units`、`effective_at` | 按冻结定价表把 `OC` 折算成 `New API` credit | `scheduled -> active -> retired` | 每笔入账只能命中一个定价版本；禁止“先猜测汇率、后人工覆盖” | 仅 producer/operator 联合审批更新 |
| 手工审查 | `review_reason`、`operator_note`、`resolution` | 处理 underpay / overpay / unknown binding / adapter fail / chain anomaly | `manual_review -> resolved/closed` | 未经 resolution 不得直接 credit；若需要补偿或回滚，必须写审计备注 | 仅 operator / finance owner 可处理 |
- Acceptance Criteria:
  - AC-1: 专题必须明确 MVP 只支持 `one-way OC -> New API internal quota/redeem credit`，并把提现、双向兑换、AMM、公开交易所全部列为 non-goal。
  - AC-2: bridge-service 必须被定义为独立部署单元，不得要求把桥逻辑直接并入 `oasis7_chain_runtime`、`oasis7_web_launcher` 或 `New API` 本体。
  - AC-3: 自动 credit 前必须有“唯一入账映射”规则；共享收款账户上的模糊入账不得直接进入自动发放。
  - AC-4: 专题必须定义 `bridge_ledger` 的最小状态机、幂等键和审计字段，覆盖 `detected/pending_confirmations/confirmed/crediting/credited/reconciled/manual_review/failed/closed`。
  - AC-5: 专题必须明确 `New API` 侧使用的是内部 quota / redeem credit 语义，而不是链上资产语义。
  - AC-6: 若 `credit adapter` 调用失败，系统必须保留幂等重试键并进入重试或 `manual_review`，不得重复 credit。
  - AC-7: 对外文案必须明确这是一条 `limited preview operator-managed service-credit bridge`；“公开兑换所”“浏览器热钱包充值”“自动提现回 OC”等表述必须被列入 denylist。
  - AC-8: 专题必须给出后续 implementation 任务拆解、验证层级与 owner role，而不是停留在概念讨论。
- Non-Goals:
  - 不实现 `OC <- New API quota` 自动兑回。
  - 不实现链上 AMM、order book、公开做市或价格发现。
  - 不把 bridge-service 做进 hosted public player plane 或浏览器 HTML/bootstrap。
  - 不在本轮实现 production-grade keystore、HSM/KMS、法币支付、税务或合规结算。
  - 不要求改造上游模型提供商结算逻辑；`New API` 只承担内部 credit 消耗层。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: bridge-service 需要对接 `New API` 的管理侧 quota / redeem credit 写入口；具体 HTTP path 或内部 admin adapter 应以实际部署版本为准。该专题不要求 AI 模型推理质量评测，目标是让链上入账可以稳定落到 AI 服务内部额度。
- Evaluation Strategy: 以 credit 正确率、重复 credit 为 0、人工审查率、credit 延迟、对账成功率与 claim 边界正确率评估。

## 4. Technical Specifications
- Architecture Overview: 架构采用独立 bridge-service。它包含 `binding API`、`deposit route allocator`、`chain watcher`、`bridge_ledger store`、`pricing engine`、`New API credit adapter` 与 `reconciliation worker`。`oasis7` 侧继续负责链上资产真值与签名化 transfer submit；`New API` 侧继续负责站内额度消费；bridge-service 只做“已确认 `OC` 入账 -> 内部 quota / redeem credit”映射，不持有上游模型消费结算逻辑。
- Integration Points:
  - `crates/oasis7/src/runtime/main_token.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/server.rs`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
  - `New API` 管理侧 quota / redeem credit 写入口（精确接口路径依赖实际部署版本，implementation 前需以 active deployment 复核）
- Edge Cases & Error Handling:
  - 若用户没有先完成 `New API` 身份绑定，就试图直接转 `OC` 到 bridge deposit account，则系统必须进入 `manual_review`，不得猜测 beneficiary。
  - 若同一用户同时持有多个未过期 deposit route，bridge-service 必须保证 route 级唯一性，避免一笔入账命中多个订单。
  - 若用户 underpay，默认进入 `manual_review`；不得把不足额自动折算成任意 bonus 规则。
  - 若用户 overpay，默认进入 `manual_review`；不得把超额部分静默吞掉或自动新建第二笔 credit。
  - 若链上观察到重复事件、重试回放或 watcher 重启，`bridge_ledger` 必须以稳定 `chain_tx_id + route/order + output tuple` 做幂等去重。
  - 若 `New API` adapter 超时或返回 5xx，bridge-service 必须转入 `crediting_retry_pending` 等价状态并保留审计记录，不得重复发放。
  - 若链状态出现异常、确认窗口被撤回，bridge-service 必须停留在 `pending_confirmations` 或 `manual_review`，不得抢跑 credit。
  - 若 producer 后续要求公开自助网页充值，而资产面仍依赖浏览器长期 signer、共享收款地址或不成熟钱包体系，则提案必须退回。
- Non-Functional Requirements:
  - NFR-TBRIDGE-1: `bridge_ledger` 审计字段完整率 `100%`，至少包含 `bridge_deposit_id/chain_tx_id/beneficiary/pricing_version/credit_units/idempotency_key/state/operator_note`。
  - NFR-TBRIDGE-2: 对同一链上入账，自动 credit 次数必须恒为 `<= 1`；重复 watcher 事件、worker 重试或 adapter 超时不得导致双发。
  - NFR-TBRIDGE-3: bridge-service 必须支持配置化确认窗口，并把该值写入审计输出；未达到确认阈值的入账不得 credit。
  - NFR-TBRIDGE-4: 任何自动 credit 路径都必须建立在唯一 beneficiary 绑定上；共享收款 + 模糊匹配不允许进入自动通道。
  - NFR-TBRIDGE-5: bridge-service 的 custody 只允许留在受控服务端环境，不得进入 HTML/bootstrap/public JS 或 hosted public player plane。
  - NFR-TBRIDGE-6: 对外 claim 只能使用 `limited preview operator-managed service-credit bridge`；`公开兑换所`、`自动提现`、`双向锚定`、`生产级钱包充值` 等表述命中次数必须为 `0`。
  - NFR-TBRIDGE-7: implementation 前必须重新确认 active `New API` 部署版本的 admin/quota 写入口；文档里不得把未验证的第三方接口路径写成当前真值。
- Security & Privacy: bridge-service 持有自己的 bridge custody 与 `New API` 管理侧 credential，必须与 `oasis7` runtime、网页 public plane 和用户浏览器完全隔离。用户侧只暴露必要的 deposit account / order、beneficiary 引用与 credit 结果；不得在文档或公共接口中暴露服务端私钥、管理 token 或链下 operator secret。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: operator-managed `one-way OC -> New API quota` bridge，支持用户绑定、唯一入账路由、确认窗口、`bridge_ledger`、credit adapter 与 manual-review fallback。
  - v1.1: 增加 operator dashboard、pricing version 管理、批量 reconciliation 报表与 redeem credit 发放模式。
  - v2.0: 在 `oasis7` 的 custody、wallet、public claims 与 `New API` adapter 都更稳定后，再评估是否扩展更强的自助化能力；仍不默认承诺双向提现。
- Technical Risks:
  - 风险-1: 当前 `oc:pk:` 仍是 runtime 内部账户派生语义，不是成熟外部钱包体系；若过早包装成“任何人都能自助充值”，会高估可用性与安全等级。
  - 风险-2: `New API` 管理侧写入口可能随部署版本变化；若实现前不做版本锁定，credit adapter 容易漂移。
  - 风险-3: 若没有唯一入账映射，所有共享收款策略都会把对账风险推给人工，放大错发概率。
  - 风险-4: 若把 bridge-service 和浏览器 public plane 混层，会把链上资产充值误做成 hosted signer 暴露问题。
  - 风险-5: 若对外把 service credit bridge 说成“代币兑换所”，会引入不必要的合规、定价和信任预期压力。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-TBRIDGE-001 | BRIDGE-0/1 | `test_tier_required` | 专题 PRD/design/project 建档、模块入口回写、one-way bridge claim 与 non-goal 冻结 | 资产口径、对外 claim 与 owner 边界 |
| PRD-P2P-TBRIDGE-002 | BRIDGE-1/2/3 | `test_tier_required` | `bridge_ledger` 状态机、唯一入账映射、确认窗口、幂等键与 anomaly 流程设计评审 | 账本正确性、重复 credit 风险与 operator 对账 |
| PRD-P2P-TBRIDGE-003 | BRIDGE-3/4 | `test_tier_required` | `New API` credit adapter 契约、pricing version、manual-review fallback 与 admin credential 隔离评审 | `New API` 额度发放、adapter 漂移与安全边界 |
| PRD-P2P-TBRIDGE-004 | BRIDGE-2/4/5 | `test_tier_required` | underpay/overpay/unknown user/adapter fail/chain anomaly 手工审查路径与 runbook | liveops / QA 异常收口与审计证据 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-TBRIDGE-001 | 采用 `one-way OC -> New API internal quota/redeem credit` | 直接承诺双向兑换或 `OC <- quota` 兑回 | 当前目标是受控服务额度充值，不是交易所产品；双向兑回会显著放大 custody、合规与价格发现难度。 |
| DEC-TBRIDGE-002 | bridge-service 独立部署 | 直接写进 `oasis7_chain_runtime`、`oasis7_web_launcher` 或 `New API` fork | 该能力同时跨链上资产、服务端 custody、`New API` admin credit，必须独立隔离信任边界与失败模型。 |
| DEC-TBRIDGE-003 | 自动 credit 依赖唯一入账映射 | 使用共享收款地址 + 人工猜测 beneficiary | 没有唯一映射就没有可审计的自动 credit 闭环，后续会持续依赖人工拆账。 |
| DEC-TBRIDGE-004 | `New API` 侧使用内部 quota / redeem credit 语义 | 试图让 `New API` 直接承载链上资产余额 | `New API` 擅长的是站内额度和模型路由，不是链上资产账本。 |
| DEC-TBRIDGE-005 | 先冻结 manual-review fallback 和 claim denylist | 先开放公开自助入口，异常以后再补风控 | 资产桥接的最常见故障不是 happy path，而是错绑、重复、超时与 claim 误导；必须先定义异常终态。 |
