# oasis7 环境分层与云上服务清单

审计轮次: 1

## Meta
- Owner Role: `producer_system_designer`
- Review Roles:
  - `qa_engineer`
  - `runtime_engineer`
  - `liveops_community`
- Scope: 项目级 `test/prod` 环境分层、hosted-login 云上清单、testnet/mainnet 口径边界、上线与回归流程的双环境要求
- Last Verified: 2026-07-06 Asia/Shanghai
- Related Runtime Evidence:
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
  - `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`
  - `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
  - `doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md`

## 1. 适用范围
这份文档是项目级环境真值入口，用于回答三类问题：

1. 当前云上哪些机器承担测试环境、正式环境或网络层职责。
2. 新流程上线前是否具备一套可反复演练的 test lane 和一套受控 prod lane。
3. `testnet/mainnet`、`staging/production`、`dev/local` 这些词在 oasis7 内部如何对应，避免把测试服务误说成正式服务。

本文件不保存任何密钥值。所有 SMTP、Tablestore、signer、custody、API key、approval code 只允许存在于受控 secret store、部署机 env 文件或 operator 临时输入中。

## 2. 三套环境总览
项目对内和 operator-facing 口径统一收束为三套环境：`local`、`test`、`production`。子系统可以继续使用更细的技术 tier，但必须先能映射回这三套环境。

| 项目环境 | 标准名 | 自动归入的别名 / 子系统术语 | 用途 | 当前可声明状态 | 禁止口径 |
| --- | --- | --- | --- | --- | --- |
| 本地环境 | `local` | `dev`, `local_dev`, `local_devnet`, `file backend`, `localhost`, `trusted_local_only` local-only playtest, launcher provider `local` / `local-mock` | 开发反馈、单机 smoke、本地 playtest、结构验证 | 可用于本地开发和 CI/PR 前验证；可随时重置 | 不得说成外部测试环境、线上服务、正式数据或公开网络 |
| 测试环境 | `test` | `test`, `staging`, `public_testnet`, `testnet`, hosted-login test, launcher provider `test`, Testnet Packages | 上线前验证、受控测试用户、operator 演练、可重置 public testnet rehearsal | hosted-login test 已有独立部署；`public_testnet` 当前 11 条 formal required lanes 全 pass，可声明 controlled / resettable / non-mainnet `ready_for_live_candidate`；`shared_devnet` 不再作为目标 test 子环境 | 不得把 legacy `shared_devnet` 叫成 `public_testnet`；不得把 `public_testnet` 叫成 `mainnet`、public launch、no-reset release 或 production OC settlement；不得把 skeleton / rehearsal / partial 说成 live candidate |
| 正式环境 | `production` | `prod`, `production`, `mainnet`, hosted-login prod, launcher provider `prod`, production release | 正式服务的内部运维边界；未来才承接真实玩家或正式网络入口 | 暂时还没有面向用户的正式环境；hosted-login prod 只是账号服务 production lane；`mainnet` 目前仍是 skeleton / readiness 目标，不是 live mainnet | 不得对普通用户宣称 production 环境已开放；不得使用 faucet / resettable / placeholder endpoint / local file backend；不得宣称 `mainnet_live` 或 `production_oc_settlement` |

强制解释：
1. 三套环境是项目级环境维度；`deployment_mode`、network tier、provider lane 是子系统维度，不能直接替代项目环境名。
2. `hosted_public_join` 是玩家接入 / deployment mode，不等于 production；当前可以在 local/test 内验证，但不能据此宣称面向用户的正式环境已开放。
3. `trusted_local_only` 只允许作为 local-only 调试/试玩例外，不能作为 test 或 production 用户主路径。
4. `shared_devnet` 不再作为目标 test 子环境；历史文档、脚本或证据中出现时只按 legacy/rehearsal 资产处理，不能作为新环境规划入口。
5. `public_testnet` 是 test 环境里的测试网络目标；必须具备 public RPC/explorer/guarded faucet/reset policy 并通过 readiness 后才能作为 live candidate。
6. `mainnet` 归入 production 环境，但当前只有 skeleton / readiness 目标态；不得因为 hosted-login prod 已上线就推导出 mainnet 已上线。
7. 本机暴露 `hosted_public_join` / hosted-login 形态入口时，只有在其 launcher/API/runtime 明确指向已按 formal `public_testnet` manifest / `world_id` / `chain_id` / genesis / bootstrap peers 同步的健康 testnet 节点，且 viewer / pure API 读取的是该节点的 world state 时，才能计为 test 环境中的 unified-world 候选运行测试；单独的 hosted-login 形态或本地账号 smoke 只能计为 local/test access-surface smoke。

## 3. 已收束的混乱点
| 混乱点 | 收束规则 | 依据 / 入口 |
| --- | --- | --- |
| `dev/local`、`local_devnet`、`local` 混用 | 全部归入项目 `local` 环境；只用于开发反馈、本地 smoke、本地 playtest | `testing-manual.md` 本地开发态约定；本文件三套环境总览 |
| `test`、`staging`、`testnet` 混用 | 统一归入项目 `test` 环境；具体子系统必须再说明是 hosted-login test、public_testnet 还是 provider test lane | 本文件 Hosted Login 环境矩阵与 Network Tier 矩阵 |
| `shared_devnet` 被误读成目标测试环境 | `shared_devnet` 不再作为目标 test 子环境；历史 references 只按 legacy/rehearsal 资产处理，不替代 `public_testnet` | 用户决策 2026-06-14；P2P formal network tier docs |
| `public_testnet` 被误读成正式网或 public launch | `public_testnet` 是 test 环境下的可重置、有 guard 测试网络术语；当前标准口径为 controlled / resettable / non-mainnet `ready_for_live_candidate`，不等于 public launch、mainnet、production OC settlement、public validator admission 或 no-reset release | `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`; `doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md` |
| hosted-login prod 被误读成面向用户的正式环境 | hosted-login prod 只表示账号/登录服务 production lane 已部署；当前暂时还没有面向用户的正式环境，也不代表 mainnet、生产 signer/custody 或 production OC settlement 已完成 | 用户决策 2026-06-14；本文件 Hosted Login 环境矩阵；P2P mainnet readiness docs |
| launcher provider `prod` 被误读成整项目 production ready | provider `prod` 只是 LLM/provider remote lane；当前不能据此宣称面向用户的正式环境已开放 | `scripts/run-launcher-stack.sh --agent-provider-lane` |
| `hosted_public_join` 被误读成环境名 | 它是接入模式，不是环境；在 local/test/production 中都必须显式说明所在环境和数据/secret lane | hosted-public-join managed identity docs |
| 本机 hosted-login 形态被误读成已接入 testnet 大世界 | 只有同时证明本机节点使用 formal `public_testnet` manifest / `world_id` / genesis / bootstrap peers、节点健康同步、且 hosted-login/launcher/API 指向该节点 world state，才可声明为 testnet-connected hosted entry | `p2p-public-testnet-local-observer-sync.sh`；formal public_testnet runbook |
| `trusted_local_only` 被误读成可选线上模式 | 它只能是 local-only playtest/debug 例外，且需要显式 `--allow-trusted-local-playtest` | `scripts/run-launcher-stack.sh --deployment-mode` |
| `production`、`prod`、`mainnet` 混用 | 项目环境标准名用 `production`；`mainnet` 只用于网络/链 tier，且当前不得声明 live | 本文件三套环境总览；network tier manifest / exit review |

## 4. 子系统映射表
| 子系统 / 维度 | local | test | production |
| --- | --- | --- | --- |
| Hosted Login / Account | local file backend、本地 SMTP/OTP smoke、local hosted account smoke | `oasis7-hosted-login-test.service`、`/etc/oasis7/hosted-login-test.env`、测试 Tablestore 表 | `oasis7-hosted-login-prod.service`、`/etc/oasis7/hosted-login-prod.env`、正式 Tablestore 表 |
| Network tier | `local_devnet` / local observer / local-only node | `public_testnet` governed-bootstrap evidence + 当前 11-lane all-pass controlled `ready_for_live_candidate`；`shared_devnet` 为 legacy/rehearsal 资产 | `mainnet` skeleton / readiness target；未过 gate 前不是 live |
| Launcher provider lane | `local`, `local-mock` | `test` with explicit remote URL / test provider secret | `prod` with production provider URL/secret |
| Deployment mode | `trusted_local_only` only with explicit local playtest allow; `hosted_public_join` local smoke is allowed | `hosted_public_join` with test account store, SMTP, strong-auth/custody secrets; 若用于 testnet 大世界测试，必须证明接入面指向健康 synced `public_testnet` 节点 world state | no user-facing production mode yet; production deployment mode remains future-gated |
| Data / state | temporary local files, reset anytime | resettable test tables/buckets/worlds, no prod secret reuse | non-reset production tables/buckets/worlds, audited migration/backup/rollback |
| Claims | internal dev only | controlled testing / resettable / non-mainnet | no user-facing production claim yet; mainnet claims require mainnet gates |
| Minimum smoke | local required smoke / launcher stack / hosted-account local smoke | test lane smoke plus lane evidence; public_testnet readiness script when applicable | no user-facing production smoke yet; internal production lanes still need rollback evidence |

## 5. 用户已确认的环境决策
以下决策由用户在 2026-06-14 确认，优先级高于旧专题中的临时环境规划：

1. `shared_devnet` 不再作为目标环境；测试环境只保留 `test` 总口径，网络测试目标收束到 `public_testnet`。
2. 暂时还没有面向用户的正式环境；hosted-login prod、provider prod 或 production release 只能作为内部 production lane，不构成用户可进入的正式环境。
3. 暂时没有需要加入 production 清单的用户正式入口；正式 Web 域名、provider bridge、下载渠道、支付/额度 bridge、signer/custody 或 mainnet readiness matrix 以后另行补。
4. CI/CD 先不拆 `deploy-test` / `deploy-prod`；当前文档只记录该方向，不把它作为当前要求。

遗留清理项：
1. `scripts/network-tier-public-testnet-readiness.sh` 和 public-testnet lanes template 已从新目标中移除 `shared_devnet_pass`；若旧证据或历史 runbook 继续引用 `shared_devnet`，只作为 legacy evidence 处理。
2. shared-network / shared-devnet 历史专题仍作为 provenance 保留；当前入口文档可删减重复旧口径，但 evidence-only 路径删除前必须单独评估仍被测试、证据、PRD 或脚本引用的路径。

## 6. 命名原则
项目默认采用三层表达，但所有对外或可变更流程至少必须具备 test/prod 两套：

| 层级 | 用途 | 是否可给外部用户 | 数据是否可重置 | 典型命名 |
| --- | --- | --- | --- | --- |
| `dev/local` | 本地开发、单机 smoke、结构验证 | 不可 | 可随时重置 | `local`, `dev`, `file backend` |
| `test/staging` | 上线前验证、受控测试用户、operator 演练 | 仅受控测试 | 可按 runbook 重置 | `test`, `staging`, `public_testnet` |
| `prod/mainnet` | 内部正式服务 lane 与未来正式网络入口 | 暂不可作为用户入口 | 不可随意重置 | `prod`, `production`, `mainnet` |

强规则：
1. 新增外部服务时，先定义 test lane，再定义 prod lane。
2. test lane 不得复用 prod 的状态存储、secret、signer、approval code 或用户数据表。
3. prod lane 不得使用 inline OTP、server log OTP、placeholder endpoint、example manifest 或本地 file backend 作为用户主链路。
4. `public_testnet` 只能说明“公开可测试且可重置”；它不是 `mainnet`，也不承诺 mainnet 价值语义。
5. `mainnet` 只有在 genesis、bootstrap、public RPC/explorer、claims boundary、custody/signing、rollback/incident runbook 全部过 gate 后才能声明。

## 7. 当前云上清单
以下是 2026-05-29 只读核查与部署后的云上服务清单；`public_testnet` 当前节点清单见 7.1。

| 主机 | 当前职责 | Hosted Login | Network / Chain | 对外检查 |
| --- | --- | --- | --- | --- |
| `39.104.205.67` | 邮箱登录测试环境 | `oasis7-hosted-login-test.service`，active | 同机还有 testnet storage / triad storage / bridge 类服务 | `http://39.104.205.67:4373/` 返回 `200` |
| `39.104.204.172` | 邮箱登录正式环境；同时承载若干测试网络节点服务 | `oasis7-hosted-login-prod.service`，enabled + active | 同机还有 testnet sequencer / triad sequencer / faucet 类服务；这些不等于 mainnet | `http://39.104.204.172:4373/` 返回 `200` |

说明：
1. 主机上存在 testnet/triad 服务，不自动意味着 `public_testnet` 当前 11-lane packet 仍然新鲜，也不意味着 mainnet 已上线。
2. `public_testnet` readiness 以 `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md` 的 11 条 required lanes 与 `scripts/network-tier-public-testnet-readiness.sh` 输出为准。
3. hosted-login 的 test/prod 两套已经分开部署，但仍建议补完整 `login/complete` 自动 smoke，以便把 OTP 收件箱验证也纳入日常 gate。

### 7.1 当前 public_testnet operator 节点清单
Last Verified: 2026-06-23 Asia/Shanghai, task `task_bdb48338fac544849d8c681e9a7dd441`.

当前受管 `public_testnet` 部署是五节点 fleet：两台 ECS validator、两台文档列出的 observer 机器、加本机 macOS observer。旧 `.tmp/testnet-local-node-bootstrap` 和 `.tmp/testnet-fourth-node-bootstrap` 这类 bootstrap staging 目录若没有 runtime binary、`CURRENT_VERSION` 与 service definition，不计为当前受管节点。

| node_id | 角色 | host / lane | stack root | service manager | status endpoint |
| --- | --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator / sequencer | `root@39.104.204.172` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-sequencer.service` | `http://127.0.0.1:6631/v1/chain/status` |
| `triad-testnet-storage` | validator / storage | `root@39.104.205.67` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-storage.service` | `http://127.0.0.1:6632/v1/chain/status` |
| `triad-testnet-local` | observer | Linux LAN observer | `/opt/oasis7/p2p-testnet-local` | `oasis7-testnet-observer.service` | `http://127.0.0.1:6633/v1/chain/status` |
| `triad-testnet-windows-observer` | observer | Windows observer | `C:\oasis7-deploy` | scheduled task `Oasis7Observer` | `http://127.0.0.1:5121/v1/chain/status` |
| `triad-testnet-fourth-local` | observer | macOS local observer | `$OASIS7_TESTNET_FOURTH_ROOT` | launchd `oasis7.testnet.fourth` | `http://127.0.0.1:19083/v1/chain/status` |

节点更新/补更规则：
1. CI artifact scope 必须覆盖目标平台；Linux/macOS package 不得用于 Windows observer。
2. 五节点补更时先确认或恢复 validator pair，再逐个 observer 升级和验证。
3. 若 observer 从旧高度或空状态启动后出现 `replication no connected providers`、`consensus_peer_head_unavailable` 或 `execution driver peer mismatch`，先以健康 storage/sequencer fresh state reseed observer，再验收。
4. 若 validator pair 自身出现 execution mismatch，先恢复 validator pair；不要继续用旧 validator state seed observers。
5. 最终验收必须逐节点记录 `CURRENT_VERSION`、runtime hash 或 artifact lineage、`running=true`、`last_error=null`、`readiness.status=ready`、`readiness.failed_gates=[]`、`committed_height`、`network_committed_height`、`last_execution_height`。
6. 本文件不记录任何密码、private key、token 或完整 secret env value。
7. `$OASIS7_TESTNET_FOURTH_ROOT` 是 operator 本机 macOS observer root；非归档文档不得硬编码个人 home path。

## 8. Hosted Login 环境矩阵
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

Hosted-login 形态接入 testnet 大世界的追加验证：
1. 记录本机节点的 `NETWORK_TIER_MANIFEST_PATH`、`WORLD_ID`、`CHAIN_ID`、genesis 与 bootstrap peers，且这些值来自 formal `public_testnet` manifest / governed-bootstrap evidence，而不是 local/example/placeholder 配置。
2. 记录本机节点健康与同步证据：至少包含 status/health 响应、connected peers、committed height/head 推进，以及与目标 testnet peers 的 world/head 一致性。
3. 记录 hosted-login / launcher / viewer / pure API 的 runtime/status/API endpoint 指向该本机 testnet 节点，而不是默认新建的 local execution world。
4. 若只是验证账号登录、OTP、hosted account continuity 或本地 hosted-public-join UI，不得宣称已接入 testnet 大世界；只能标为 local hosted-login smoke 或 access-surface smoke。

## 9. Network Tier 矩阵
| Tier | 环境语义 | 当前可声明状态 | Gate 文档 |
| --- | --- | --- | --- |
| `local_dev` | 本地或单机开发验证 | 可用于开发反馈，不可对外声明 | `testing-manual.md` |
| `shared_devnet` | legacy 共享开发网络 / release train 前置环境 | 不再作为目标环境；历史证据只可作为 legacy rehearsal evidence | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md` |
| `public_testnet` | 面向外部测试者、可重置、有 faucet guard 的测试网络 | 当前 11 条 formal required lanes 全 pass，可声明 controlled / resettable / non-mainnet `ready_for_live_candidate`；不得扩写为 public launch、mainnet、production OC settlement、public validator admission 或 no-reset release | `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md` |
| `mainnet` | 正式网络，非可随意重置的生产语义 | 未有 mainnet gate 前不得声明 | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md` |

`testnet -> mainnet` promotion 必须至少证明：
1. 候选版本、genesis、bootstrap peers 与 manifest 都不是 placeholder。
2. public RPC、explorer、faucet 都有公开可达证据。
3. reset policy 和 claims boundary 已由 `qa_engineer` / `liveops_community` 复核。
4. signer/custody/mainnet key 与 testnet key 完全隔离。
5. rollback、incident、communication runbook 可执行。

## 10. 全流程双环境要求
任何新增流程只要会被外部用户、operator、CI/CD、云服务或长期数据依赖，就必须补一行 test/prod 环境矩阵。

| 流程类型 | Test lane 必须有 | Prod lane 必须有 |
| --- | --- | --- |
| 用户登录 / 账号 | 独立测试邮箱、独立表、受控测试收件箱 smoke | 独立正式表、SMTP-only、真实 OTP complete smoke |
| 链 / 网络 | public-testnet manifest + evidence；legacy shared-devnet evidence 只可辅助追溯 | mainnet manifest + non-reset claims + signing/custody gate |
| Hosted-login 形态 testnet 接入 | 健康 synced `public_testnet` 本机节点 + hosted-login/launcher/API 指向该节点 world state 的证据；账号 smoke 只能补充 access surface | 未来正式用户入口必须绑定 production/mainnet readiness、strong-auth/custody、non-reset claims 与 rollback/incident gate |
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

## 11. 变更记录要求
每次新增、移动、下线或重命名云上环境，至少回写：

1. 本文件的对应矩阵。
2. 相关 runbook 或 module README 的入口链接。
3. GitHub task issue evidence comments 的部署与验证证据。
4. PR body 的环境状态摘要。

禁止项：
1. 不得在本文件、PR body、PM execution log 或公开 issue/comment 中写入密钥值。
2. 不得只部署 prod 而没有 test lane。
3. 不得把 testnet 的 `200 OK` 或单点服务 active 直接当作 mainnet readiness。
4. 不得把 `login/start` 成功误报成完整登录成功；完整登录必须包含 OTP complete。

## 12. 快速状态检查
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

## 13. 后续缺口
1. 为 hosted-login 增加可自动读取测试邮箱 OTP 的 `login/start -> login/complete` 云上 smoke。
2. 为 public_testnet 生成非 placeholder manifest、lanes TSV 与公开 endpoint evidence。
3. 为 mainnet readiness 增加独立环境矩阵，明确 signer/custody/genesis/bootstrap/RPC/explorer/faucet 的 test/prod 隔离。
4. CI deploy job 暂不拆 `deploy-test` / `deploy-prod`；若后续进入持续部署，再重新评估显式双 lane。
