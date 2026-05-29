# oasis7 环境分层与云上服务清单

审计轮次: 1

## Meta
- Owner Role: `producer_system_designer`
- Review Roles:
  - `qa_engineer`
  - `runtime_engineer`
  - `liveops_community`
- Scope: 项目级 `test/prod` 环境分层、hosted-login 云上清单、testnet/mainnet 口径边界、上线与回归流程的双环境要求
- Last Verified: 2026-05-29 Asia/Shanghai
- Related Runtime Evidence:
  - `.pm/tasks/task_a0282b0e2c51476590c5de733f0d45ef.execution.md`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
  - `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`

## 1. 适用范围
这份文档是项目级环境真值入口，用于回答三类问题：

1. 当前云上哪些机器承担测试环境、正式环境或网络层职责。
2. 新流程上线前是否具备一套可反复演练的 test lane 和一套受控 prod lane。
3. `testnet/mainnet`、`staging/production`、`dev/local` 这些词在 oasis7 内部如何对应，避免把测试服务误说成正式服务。

本文件不保存任何密钥值。所有 SMTP、Tablestore、signer、custody、API key、approval code 只允许存在于受控 secret store、部署机 env 文件或 operator 临时输入中。

## 2. 命名原则
项目默认采用三层表达，但所有对外或可变更流程至少必须具备 test/prod 两套：

| 层级 | 用途 | 是否可给外部用户 | 数据是否可重置 | 典型命名 |
| --- | --- | --- | --- | --- |
| `dev/local` | 本地开发、单机 smoke、结构验证 | 不可 | 可随时重置 | `local`, `dev`, `file backend` |
| `test/staging` | 上线前验证、受控测试用户、operator 演练 | 仅受控测试 | 可按 runbook 重置 | `test`, `staging`, `public_testnet`, `shared_devnet` |
| `prod/mainnet` | 真实玩家或正式网络入口 | 可 | 不可随意重置 | `prod`, `production`, `mainnet` |

强规则：
1. 新增外部服务时，先定义 test lane，再定义 prod lane。
2. test lane 不得复用 prod 的状态存储、secret、signer、approval code 或用户数据表。
3. prod lane 不得使用 inline OTP、server log OTP、placeholder endpoint、example manifest 或本地 file backend 作为用户主链路。
4. `public_testnet` 只能说明“公开可测试且可重置”；它不是 `mainnet`，也不承诺 mainnet 价值语义。
5. `mainnet` 只有在 genesis、bootstrap、public RPC/explorer、claims boundary、custody/signing、rollback/incident runbook 全部过 gate 后才能声明。

## 3. 当前云上清单
以下是 2026-05-29 只读核查与部署后的当前清单。

| 主机 | 当前职责 | Hosted Login | Network / Chain | 对外检查 |
| --- | --- | --- | --- | --- |
| `39.104.205.67` | 邮箱登录测试环境 | `oasis7-hosted-login-test.service`，active | 同机还有 testnet storage / triad storage / bridge 类服务 | `http://39.104.205.67:4373/` 返回 `200` |
| `39.104.204.172` | 邮箱登录正式环境；同时承载若干测试网络节点服务 | `oasis7-hosted-login-prod.service`，enabled + active | 同机还有 testnet sequencer / triad sequencer / faucet 类服务；这些不等于 mainnet | `http://39.104.204.172:4373/` 返回 `200` |

说明：
1. 主机上存在 testnet/triad 服务，不自动意味着 `public_testnet` 已达到 live candidate，也不意味着 mainnet 已上线。
2. `public_testnet` readiness 仍以 `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md` 的 seven-lane checklist 为准。
3. hosted-login 的 test/prod 两套已经分开部署，但仍建议补完整 `login/complete` 自动 smoke，以便把 OTP 收件箱验证也纳入日常 gate。

## 4. Hosted Login 环境矩阵
| 项 | 测试环境 | 正式环境 |
| --- | --- | --- |
| Host | `39.104.205.67` | `39.104.204.172` |
| Service | `oasis7-hosted-login-test.service` | `oasis7-hosted-login-prod.service` |
| Root | `/opt/oasis7/hosted-login-test` | `/opt/oasis7/hosted-login-prod` |
| Env file | `/etc/oasis7/hosted-login-test.env` | `/etc/oasis7/hosted-login-prod.env` |
| Current release | `/opt/oasis7/hosted-login-test/releases/20260529-1245-b3d684ab` | `/opt/oasis7/hosted-login-prod/releases/20260529-1450-b3d684ab` |
| Viewer port | `4373` | `4373` |
| WebSocket port | `6511` | `6511` |
| Live bind port | `6523` | `6523` |
| Store backend | `tablestore` | `tablestore` |
| Tablestore table | `oasis7_hosted_account_identity_test` | `oasis7_hosted_account_identity` |
| OTP delivery | SMTP only | SMTP only |
| Inline preview OTP | forbidden | forbidden |

Hosted-login 最小验证：
```bash
curl -fsS http://<host>:4373/

curl -fsS -X POST http://<host>:4373/api/public/hosted-account/login/start \
  -H 'Content-Type: application/json' \
  --data '{"channel":"email","handle":"<test-email>"}'
```

验收口径：
1. 根路径返回 `200`。
2. `login/start` 返回 `ok=true`。
3. `challenge.delivery_mode=smtp`。
4. 响应中没有 `preview_code`。
5. 完整 smoke 还必须用真实邮箱 OTP 调 `/api/public/hosted-account/login/complete`，并确认同一邮箱跨重启后账号映射稳定。

## 5. Network Tier 矩阵
| Tier | 环境语义 | 当前可声明状态 | Gate 文档 |
| --- | --- | --- | --- |
| `local_dev` | 本地或单机开发验证 | 可用于开发反馈，不可对外声明 | `testing-manual.md` |
| `shared_devnet` | 共享开发网络 / release train 前置环境 | 可作为 public testnet promotion 输入，不能替代 public testnet | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md` |
| `public_testnet` | 面向外部测试者、可重置、有 faucet guard 的测试网络 | 只有 seven-lane 全 pass 后才可声明 live candidate | `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md` |
| `mainnet` | 正式网络，非可随意重置的生产语义 | 未有 mainnet gate 前不得声明 | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md` |

`testnet -> mainnet` promotion 必须至少证明：
1. 候选版本、genesis、bootstrap peers 与 manifest 都不是 placeholder。
2. public RPC、explorer、faucet 都有公开可达证据。
3. reset policy 和 claims boundary 已由 `qa_engineer` / `liveops_community` 复核。
4. signer/custody/mainnet key 与 testnet key 完全隔离。
5. rollback、incident、communication runbook 可执行。

## 6. 全流程双环境要求
任何新增流程只要会被外部用户、operator、CI/CD、云服务或长期数据依赖，就必须补一行 test/prod 环境矩阵。

| 流程类型 | Test lane 必须有 | Prod lane 必须有 |
| --- | --- | --- |
| 用户登录 / 账号 | 独立测试邮箱、独立表、受控测试收件箱 smoke | 独立正式表、SMTP-only、真实 OTP complete smoke |
| 链 / 网络 | shared-devnet 或 public-testnet manifest + evidence | mainnet manifest + non-reset claims + signing/custody gate |
| 数据存储 | 测试 bucket/table/path，可清理 | 正式 bucket/table/path，备份与迁移策略 |
| Signer / Custody | 测试 key，允许演练 revoke/recovery | 正式 key，严格权限、审计与轮换 |
| CI / 打包 | PR artifact、测试部署、回归 smoke | release artifact、prod deploy、回滚点 |
| LiveOps / 对外公告 | 测试公告、preview wording、incident drill | 正式公告、claims review、支持与回滚窗口 |

新增或修改流程时，PR 描述至少回答：
1. test lane 在哪里。
2. prod lane 在哪里。
3. 哪些 secret / state / endpoint 被隔离。
4. test lane 的 smoke 命令是什么。
5. prod lane 的 smoke 命令是什么。
6. 回滚点是什么。

## 7. 变更记录要求
每次新增、移动、下线或重命名云上环境，至少回写：

1. 本文件的对应矩阵。
2. 相关 runbook 或 module README 的入口链接。
3. `.pm/tasks/<TASK-UID>.execution.md` 的部署与验证证据。
4. PR body 的环境状态摘要。

禁止项：
1. 不得在本文件、PR body、PM execution log 或公开 issue/comment 中写入密钥值。
2. 不得只部署 prod 而没有 test lane。
3. 不得把 testnet 的 `200 OK` 或单点服务 active 直接当作 mainnet readiness。
4. 不得把 `login/start` 成功误报成完整登录成功；完整登录必须包含 OTP complete。

## 8. 快速状态检查
Hosted login 测试环境：
```bash
curl -fsS -o /tmp/oasis7-hosted-login-test.html -w '%{http_code}\n' \
  http://39.104.205.67:4373/
```

Hosted login 正式环境：
```bash
curl -fsS -o /tmp/oasis7-hosted-login-prod.html -w '%{http_code}\n' \
  http://39.104.204.172:4373/
```

systemd 只读核查：
```bash
systemctl is-active oasis7-hosted-login-test.service
systemctl is-active oasis7-hosted-login-prod.service
```

云上 operator 检查时只允许输出 env key 名，不允许输出 env value。

## 9. 后续缺口
1. 为 hosted-login 增加可自动读取测试邮箱 OTP 的 `login/start -> login/complete` 云上 smoke。
2. 为 public_testnet 生成非 placeholder manifest、lanes TSV 与公开 endpoint evidence。
3. 为 mainnet readiness 增加独立环境矩阵，明确 signer/custody/genesis/bootstrap/RPC/explorer/faucet 的 test/prod 隔离。
4. 将 CI deploy job 拆成 `deploy-test` 与 `deploy-prod` 两条显式 lane，避免人工命令长期承担环境真值。
