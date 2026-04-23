# task_c2def8d52baa4fe5a1b1df64e19a6305 Execution Log

- task_uid: task_c2def8d52baa4fe5a1b1df64e19a6305
- title: 归档三节点链状态指标部署与采样证据
- owner_role: qa_engineer
- worktree_hint: /home/scc/worktrees/oasis7-testing-triad-metrics-rollout-evidence

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-04-23 10:31:26 CST / qa_engineer
- 完成内容:
  - 新增 `doc/testing/evidence/shared-network-ecs-triad-chain-status-metrics-rollout-2026-04-23.md`，冻结本轮三节点 rollout 的 commit `8e605366`、binary sha256、release dir、same-window triad snapshot、最近 `10` 分钟 traffic window 与 `/v1/chain/status` 新增 metrics contract。
  - 复核本机 observer 与两台 ECS 的 `current` 符号链接都已切到 `8e605366-chain-status-metrics-20260423-095327`，且 `oasis7-triad-observer.service`、`oasis7-triad-sequencer.service`、`oasis7-triad-storage.service` 均为 `active`。
  - 复核 live `/v1/chain/status`：三节点都已返回 `transactions`、`consensus.recent_finality_latency`、`consensus.pending_proposal`、`consensus.pending_consensus_actions`；当前窗口 `transactions.*=0`、`pending_proposal=null`、queue / submit buffer 均为 `0`，属于“字段已上线但本窗无交易样本”，不是字段缺失。
  - 更新 `doc/testing/project.md` 与 `doc/testing/evidence/README.md`，将本任务纳入 testing 模块项目追踪与 evidence 首读入口。
- 遗留事项:
  - 若后续需要把 `transactions.confirmed_count` 与 `recent_confirmation_latency.sample_count` 采到正样本，需另行执行带真实 transfer submit 的 same-window triad 回放。
