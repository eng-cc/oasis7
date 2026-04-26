# oasis7 治理调用面与产品链路说明（2026-04-26）

## 目的

这份分册只解决一个问题：当前 `oasis7` 的产品主链路里，哪些治理相关动作已经有可直接调用的接口，哪些只支持观测，哪些仍然必须走专门产品流，不应被文档误写成通用 API。

## 当前产品链路

1. `oasis7-run.sh play` 或 bundle `run-game.sh` 进入产品主路径，默认仍会拉起 `oasis7_chain_runtime`，除非显式传 `--chain-disable`。
2. 玩家 authority 写入先进入 `oasis7_viewer_live` 的 `ViewerRequest` 控制面；当前正式可写类型包括 `AuthoritativeRecovery`、`AgentChat`、`PromptControl`、`GameplayAction`。
3. 若 runtime live 挂了 `chain_status_bind`，则 `GameplayAction` 不会只停留在本地 runtime 队列，而会继续转发到 chain runtime 的 `POST /v1/chain/gameplay/submit`。
4. 玩家最终看到的结果仍以 committed world sync 后的 snapshot / event 为准，而不是只看 submit 接口是否返回 `ok=true`。

## 当前可直接调用的面

### 1. 观测面

当前最稳定的治理/玩法观测入口是：

- `oasis7_pure_api_client snapshot --player-gameplay-only`

它会返回 `player_gameplay`，其中已经包含：

- `agent_claim`
- `next_claim_quote`
- `restricted_starter_claim_balance`
- `slot_1_eligible_claim_balance`
- recent gameplay feedback / blocker

这意味着“首个 claim slot 多少钱、能不能用 restricted starter claim balance、当前 owned claim 状态是什么”这些信息，已经是产品链路里的可观测真值。

### 2. Viewer authority 写入面

当前正式可直调的写入面分成三类：

- `AgentChat`
- `PromptControl::{Preview,Apply,Rollback}`
- `GameplayAction`

最稳妥的 operator 入口仍然是 `oasis7_pure_api_client`，因为它会代你生成 `PlayerAuthProof`。

### 3. Chain submit 面

当前 raw HTTP 直调面只有：

- `POST /v1/chain/gameplay/submit`

它接受的请求体是 `GameplayActionRequest`，字段包括：

- `action_id`
- `target_agent_id`
- `player_id`
- `public_key`
- `auth`

其中 `auth` 是 `PlayerAuthProof`，当前要求：

- `scheme=ed25519`
- `player_id` / `public_key` 必须和 request 对齐
- `nonce > 0`
- nonce replay 会被拒绝

注意这里不是“通用治理 submit 面”，而是“当前 gameplay action 的链上提交面”。

## 当前不应写成通用 API 的部分

### claim governance 已有规则，但没有通用 claim submit API

当前 runtime 已经有完整 claim 治理规则，包括：

- slot 1 可使用 `restricted_starter_claim_balance`
- slot 2/3 只允许 liquid balance
- activation fee / claim bond / upkeep / release cooldown / idle reclaim

但当前 active operator surface 还没有暴露下列通用入口：

- `oasis7_pure_api_client claim-agent`
- `ViewerRequest::GameplayAction` 的 `claim_agent` action id
- 与 `/v1/chain/gameplay/submit` 平行的 `claim_agent` HTTP endpoint

因此当前文档口径必须保持：

- `claim` 的 quote / eligibility / owned state 已可观测
- `claim` 不是当前 skill/operator 文档里可承诺的通用直调 submit 面
- 如果未来新增 dedicated claim endpoint/helper，必须与 skill 分册、模块主 PRD/project 一起更新

## 与产品链路的关系

当前产品链路中，治理相关调用面应按下面的层次理解：

- 产品启动链路：`oasis7-run.sh play` / `run-game.sh` / `oasis7_game_launcher`
- authority 控制链路：`ViewerRequest`
- chain 链接链路：`GameplayAction -> /v1/chain/gameplay/submit -> consensus action -> committed sync`
- claim 治理观测链路：`snapshot.player_gameplay.agent_claim`

不要把这几层混写成“点了按钮就一定直接上链”。当前仓库真值是：只有链路上显式接到 chain runtime submit 的动作，才有明确的链上提交语义；claim 当前尚未作为通用 operator API 暴露。

## 操作约束

- 玩家签名 key 与 node private key 是两套不同资产，不要混用。
- `PlayerAuthProof` 用于玩家 authority 写入；node private key 仍是链 profile 级高敏资产。
- 只需要判断 claim readiness 时，优先读 snapshot，不要伪造未文档化的 submit 调用。

## 关联入口

- skill 分册：`.agents/skills/oasis7/references/governance-call-surfaces.md`
- 模块主 PRD：`doc/world-simulator/prd.md`
- 模块项目台账：`doc/world-simulator/project.md`
