# Hosted 多凭证账号验收手册

本手册只验证 `viewer / hosted_public_join / identity v2`。虚拟认证器、loopback 云协议和账号登录均不代签真实硬件、真实云事务、邀请资格、资产 custody 或完整首局可玩性。

## 候选与配置

记录候选 Git OID、构建来源、RP ID、精确 HTTPS origin、身份 namespace、外部 instance epoch、世界 ID、存储后端和浏览器版本。新注册默认关闭，原生密钥保持关闭。生产 RP 不从 Host、转发头或世界 URL 推导。

启动 v2 至少配置：

- `OASIS7_HOSTED_IDENTITY_V2_ENABLED=1`、`OASIS7_HOSTED_IDENTITY_SERVICE_ID`、`OASIS7_HOSTED_IDENTITY_ENVIRONMENT`。
- `OASIS7_HOSTED_WEBAUTHN_RP_ID`、`OASIS7_HOSTED_WEBAUTHN_ORIGINS`、`OASIS7_HOSTED_IDENTITY_INSTANCE_EPOCH`。
- 独立的 `OASIS7_HOSTED_IDENTITY_SESSION_KEY` 和授权 issuer；私钥只进入身份服务，运行节点只接收授权公钥。
- file 后端使用新 `.sqlite` 路径；Tablestore 使用显式 v2 表、instance、AK 或 STS 配置，并确认本地事务能力。不得自动回旧表、file 或空库。
- 启用邮箱或导入历史邮箱时配置稳定的 `OASIS7_HOSTED_IDENTITY_EMAIL_KEY` 与 key ID；SMTP 缺失只关闭邮件通道。
- `OASIS7_HOSTED_IDENTITY_TRUST_LOOPBACK_PROXY=1` 仅适用于受控 loopback 代理；代理覆盖 `X-Real-IP`，向世界 WebSocket 转发时清除账号 Cookie。直连请求不信任转发来源。
- `OASIS7_HOSTED_WORLD_ADMISSION_MODE=open_preview` 明确限定技术预览；不能据此宣称邀请码已原子消费。正式邀请资格仍须其独立 authority。

所有 secret 通过受限部署注入；不贴到日志、截图、PR、命令行参数或浏览器。数据库和备份使用受限权限。关闭 v2 后不得回到仅邮箱 writer 服务已创建的纯公钥账号。

<a id="contract-suite"></a>
## 契约与协议回归

首次编译准备共享 target。正式记录 canonical 命令和完整结果：

```bash
./scripts/cargo-dev.sh --prepare-target
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher --no-default-features hosted_identity:: -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --no-default-features hosted_authorization -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher --no-default-features identity_request_security_tests
```

对应源码：身份 `hosted_identity/{store/tests.rs,session_tests.rs,credential_contract_tests.rs,email.rs,recovery.rs,webauthn.rs,world_tests.rs}`、签名授权 `viewer/auth/hosted_authorization_tests.rs`。型别化领域 fixture 只证明事务和权限状态机，不证明未调用的认证器或云服务。

必须观察：旧 ID 和 player 映射保留；失败不发布部分写入；挑战五次失败耗尽；冻结不被认证解开；共享凭证下单独退出会话不删凭证；撤销凭证失效全部派生会话；安全 epoch 和外部 epoch 阻止旧 session；最后凭证保护及策略版本重新核对；第 101 条历史记录仍可分页和定向撤销。

Tablestore loopback 测试证明签名请求、事务 ID、回滚、未知 commit receipt 和分页协议。上线声明该后端前，必须在隔离真实表运行同一事务场景，确认真实 Start/Commit/Abort、分区锁、故障及表能力。没有实测就保留 readiness 阻断。

<a id="virtual-authenticator"></a>
## Chromium 虚拟认证器

本地 disposable HTTPS fixture 使用 RP `localhost`、origin `https://localhost:8449`，只监听 loopback。测试使用临时证书和隔离数据库。测试信任临时证书仅限本地 fixture，公开站点必须通过正常 TLS 校验。

```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher --no-default-features hosted_passkey_virtual_authenticator_fixture -- --ignored --nocapture
node scripts/hosted-passkey-virtual-authenticator-smoke.mjs --url https://localhost:8449/
```

第二条命令要求受控本地 TLS 代理转发到 fixture 的 `127.0.0.1:4179`。fixture 最长运行十分钟，停止文件位于系统临时目录的 `oasis7-passkey-virtual-fixture/stop`。不得将 fixture 端口或测试认证器公开。

脚本通过真实 `navigator.credentials.create/get` 和两个独立虚拟认证器验证 resident/UV required、注册、清 Cookie 后原账号登录、响应重试不重发 Cookie、备用凭证、撤销后旧凭证签名被拒、CSRF 和退出登录。测试不保存或打印私钥、Cookie、OTP 或完整标准响应。UI 取消和未知提交须保留原尝试；只有服务器确认取消、耗尽或未提交后才新建仪式。

## 真实诊断 runtime 授权合同

同候选构建 `oasis7_viewer_live` 与 launcher 测试 binary 后，可运行隔离协议验证：

```bash
node scripts/hosted-passkey-world-runtime-smoke.mjs \
  --runtime-bin <same-candidate-viewer-live> \
  --fixture-bin <same-candidate-launcher-test-binary> \
  --url https://localhost:8449/
```

脚本先确认 4179/5023 空闲，启动只监听 loopback 的 `minimal --no-llm` 诊断 runtime 与真实身份 HTTP handler。沿用上一节受控 TLS 代理；测试密钥、数据库和日志仅留在新建的 0700 临时目录，环境密钥文件为 0600。结果记录实际 binary SHA256；不得把旧 binary 的记录改成新候选通过。

必须观察真实注册 ack、超过 35 秒的续期、停止续期后的授权过期、第二设备仅登录不接管、明确接管隔离旧 key、旧 release 不影响新控制者、当前 release 提升 epoch 并撤销 runtime 控制。脚本有界关闭自己启动的进程；可选 `--hold-ms` 最多 180000，只用于浏览器采样。

此诊断世界关闭玩法，不使用付费模型，也不证明 DevLocal 链集成或正式 Skill 自主游玩。完整 Agent 游戏验收仍按 [自测手册](agent-runtime-self-playtest.manual.md) 选择场景、独立派工并核对权威世界结果。

<a id="real-device"></a>
## 真实设备、可访问性与邮箱

按配对 PRD 支持矩阵逐项记录 macOS Safari/Chrome、Windows Edge/Chrome、iOS Safari、Android Chrome 和一个支持用户验证的硬件安全密钥。验证主动点击、取消、PIN、第二凭证、清 Cookie、跨设备、撤销、受限环境替代入口、键盘和读屏状态。虚拟认证器不替代该矩阵。

使用操作者掌控的邮箱直接在 UI 输入验证码；不在聊天、日志或截图传播验证码。分别关闭 SMTP、新注册、恢复通道，验证已有 Passkey 登录继续可用。邮箱登录只建立账号 session，不把世界满员时的 OTP 放回待用状态。旧邮箱账号首次绑定保持原 ID；曾有公钥的账号不能用邮箱首次升级旁路。

恢复验证须绑定同一个预先登记候选，等待从 SMTP accepted 时刻计时，未成熟邮箱不能恢复；批准不跳过最终同邮箱 OTP 与候选断言。并发取消/完成只产生一个终态，旧凭证、旧 epoch 和旧会话不能复活。默认 clock health 为阻断；启用自动完成前确认可信 UTC，并验证本地墙钟跳变保护。

<a id="world-control"></a>
## 世界控制与撤销时限

两台设备分别登录同一账号，第二台只登录不得占 slot 或替换第一台。明确接管须提交用户确认和刚读取的 runtime epoch；候选注册失败保留旧控制，结果未知查询原尝试。旧 device/epoch 的 refresh/release 必须拒绝，重复已完成 release 保持幂等。

撤销凭证、退出账号 session、完成恢复和冻结后，持续打开的 WebSocket 仍必须在不超过 60 秒内拒绝新受认证操作。关闭身份服务、漏推和陈旧缓存不得延长期限；签名授权最长 30 秒，来源读取时刻约束其期限。纯登录和这些恢复动作均不得修改资产 signer 或开放 operator/node/governance 权限。

<a id="release-gates"></a>
## 迁移、性能与放量

旧 writer 必须实际停止并撤去写权限，生成一致性来源快照，再显式导入和逐条核对账号、因子、player 映射。operator 的 fenced/history 声明不等于外部撤销历史已经恢复。灾备恢复先隔离，对账撤销/恢复终态，再单调提升外部 epoch；旧 RP、旧 signer 或缺历史的 credential 不能因可读备份自动恢复。

两种对外宣称支持的后端分别完成 10,000 账号、30,000 凭证、20 并发、5 次/秒持续十分钟的认证/登记负载，并记录硬件、P95、错误率及隔离限流 profile。不能用 store-only 或 loopback 协议耗时代替完整有效认证。首次用户观察至少 20 人，账号创建和世界等待分开计时。

在真实设备、真实后端、撤销时限、迁移、性能和恢复证据完成前，保留未验收边界。`open_preview`、开发构建成功或虚拟认证器成功均不升级根 README 的公开 claim。
