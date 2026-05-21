# task_8cccaa2362df47eab30b9eb52b7ddf6c Execution Log

- task_uid: task_8cccaa2362df47eab30b9eb52b7ddf6c
- title: hosted account tablestore backend
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-p2p-hosted-account-tablestore

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-19 18:26:16 CST / runtime_engineer
- 完成内容: 为 `oasis7_game_launcher` 的 hosted account 登录 server 新增 `HostedAccountStoreBackend`，把原先单机 JSON 持久化抽成 `Disabled/File/Tablestore` 三态；接入 `aliyun-tablestore-rs`，支持 `OASIS7_HOSTED_ACCOUNT_STORE_BACKEND=auto|file|tablestore`、`OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` 与 `ALIYUN_OTS_*` 回退配置、自动建表、`factor`/`meta` 双 row bucket，以及基于序列自增的稳定 `hosted_account_id -> player_id` 持久化。
- 完成内容: 将 `hosted_account_identity.rs` 改为通过 backend 抽象持久化 verified login；保留本地文件 backend 作为开发 fallback，不改变现有邮箱 OTP、challenge delivery mode、`device_session + player_session` 换发协议。
- 完成内容: 已补 hosted account backend 配置单测与 hosted 身份相关回归，并回写 `doc/p2p/project.md` 与本专题 PRD/design/project，明确 hosted 部署可以把账户注册表落到 Aliyun Tablestore。
- 遗留事项: 仍缺真实环境下的 SMTP + Tablestore 组合 smoke、恢复/冻结 runbook，以及后续 `managed-custody-sign-api` 正式托管签名后端。

## 2026-05-20 18:18:00 CST / runtime_engineer
- 完成内容: 修正 `aliyun-tablestore-rs` 初始化参数，启动时从 endpoint 解析并显式传入 `instance_name` 与 `region`，避免真实环境下实例名为空导致的 `OTSParameterInvalid`；同时把建表探测从 `ListTable` 收窄为 `DescribeTable`，降低实例 ACL 需要开放的权限面。
- 完成内容: 已在 ECS 上完成两段 live smoke。第一段确认 SMTP 凭据可真实发信；第二段在新的 VPC endpoint `https://oasis7.cn-huhehaote.vpc.tablestore.aliyuncs.com` 上验证 hosted account 持久化，首次启动允许由 `OTSObjectNotExist` 进入自动建表，随后同一邮箱 `cc@ncuhome.tech` 在 launcher 重启前后两次登录均返回同一个 `hosted_account_id=oasis-account-00000001` 与 `player_id=hosted-player-account-00000001`。
- 完成内容: 已把 hosted operator runbook 补充为 MVP 运维真值，新增邮箱登录 + Tablestore 的最小部署要求、默认表名、常见失败签名与跨重启 smoke 步骤，避免后续 operator 只看设计文档而遗漏实际云侧约束。
- 遗留事项: 目前只证明 hosted identity MVP 主链路可用；SMTP 新实例长期稳定性、并发/风控冻结恢复、监控告警以及 `managed-custody-sign-api` 仍待后续专题收口。
