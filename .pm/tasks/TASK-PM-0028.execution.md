# TASK-PM-0028 Execution Log

- task_id: TASK-PM-0028
- title: Implement P2PARCH-3 deployment mode and role policy substrate
- owner_role: runtime_engineer
- worktree_hint: oasis7-p2p-p2parch-3-deployment-role-policy

## 2026-04-02 16:59:19 CST / runtime_engineer
- 完成内容: 为 `P2PARCH-3` 落 deployment mode / role policy substrate。`PeerRecord` 新增显式 `deployment_mode`，并把 `node_role` 升级为 canonical network role 语义；runtime 新增 `NodeNetworkPolicy`，把共识 `NodeRole` 与 P2P `deployment_mode/node_role_claim` 分离，默认映射为 `sequencer -> validator_core`、`storage -> full_storage`、`observer -> observer_light`，同时允许 observer runtime 显式声明 `sentry/relay`。peer record 与 runtime config 现在都会拒绝无效 deployment-role 组合，以及 `private/relay_only/validator_hidden` 发布 direct public surface。
- 完成内容: `oasis7_chain_runtime` CLI 新增 `--p2p-deployment-mode` / `--p2p-node-role`，default peer record / replication network config 会从新 policy 生成显式 role claim 与 reachability；discovery 面对旧 `sequencer/storage/observer` peer record label 仍保持兼容解析。
- 完成内容: 验证已通过 `env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node --lib`。
- 遗留事项: 补做 manual diff review、PM close、commit 与标准化 landing；`P2PARCH-4` 的 lane/QoS substrate 与 `P2PARCH-6` mixed-topology evidence 仍待后续任务推进。

## 2026-04-02 17:02:37 CST / runtime_engineer
- 完成内容: 受当前会话 delegation 约束限制，未启独立 subagent；已对 `crates/oasis7_proto` / `crates/oasis7_node` / `crates/oasis7_net` / `crates/oasis7_chain_runtime` / 专题 `project.md` diff 做人工 review 代替。未发现会破坏现有 libp2p transport substrate 或 node runtime 语义的阻断性回归；本轮改动仅把 role/deployment policy 正式抬升到 config/schema 层，没有冒进到 lane 或 mixed-topology claim。
- 完成内容: 收口门禁已通过 `git diff --check`、`./scripts/doc-governance-check.sh`、`./scripts/pm/lint.sh`。
- 遗留事项: 执行 PM close、迁移任务到 `done`、提交 commit 并标准化 landing 到本地 `main`。
