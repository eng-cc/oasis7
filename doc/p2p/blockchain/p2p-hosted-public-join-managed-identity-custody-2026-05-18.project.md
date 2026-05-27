# oasis7 hosted_public_join 托管身份 / 托管密钥与邮箱登录（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] hosted-managed-identity-doc-freeze (PRD-P2P-029) [test_tier_required]: 冻结 `hosted_public_join` 的托管身份、托管密钥、邮箱登录、自托管升级和 trust boundary 文档真值，并回写模块入口映射。 Trace: .pm/tasks/task_fd98df36264944238538dea896ce4ce0.yaml
- [x] hosted-browser-device-session-recovery (PRD-P2P-029) [test_tier_required]: 清退 `hosted_public_join` 浏览器 `localStorage privateKey` 持久化，引入 `device_session_id` contract，并把 hosted player-session 恢复链路改成“持久化 device session handle + 页内临时 Ed25519 会话 key”。 Trace: .pm/tasks/task_584da7818a9d42e6aae5894512413102.yaml
  - 产物文件:
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
    - `crates/oasis7_viewer/software_safe_src/legacy_core.js`
    - `crates/oasis7_viewer/software_safe_src/main.test.jsx`
    - `crates/oasis7_viewer/software_safe.js`
    - `crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
    - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
    - `doc/p2p/project.md`
    - `.pm/tasks/task_584da7818a9d42e6aae5894512413102.execution.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_player_session_ -- --nocapture`
    - `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
    - `npm --prefix crates/oasis7_viewer run test:ui`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] hosted-account-identity-broker-server (PRD-P2P-029) [test_tier_required]: 在 `oasis7_game_launcher` 的 public HTTP 面落地中心化 hosted account 登录 server，提供邮箱 login challenge、稳定 `hosted_account_id -> player_id` 映射持久化、验证后换发 `device_session + player_session`，并把 viewer hosted onboarding 改成 email + OTP 表单。 Trace: .pm/tasks/task_b837ca5ee1b34439a9c581ad6ab87a64.yaml
  - 产物文件:
    - `crates/oasis7/Cargo.toml`
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher/static_http.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
    - `crates/oasis7/src/hosted_access.rs`
    - `crates/oasis7_viewer/software_safe_src/legacy_core.js`
    - `crates/oasis7_viewer/software_safe_src/main.jsx`
    - `crates/oasis7_viewer/software_safe_src/main.test.jsx`
    - `crates/oasis7_viewer/software_safe.js`
    - `crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
    - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
    - `doc/p2p/project.md`
    - `.pm/tasks/task_b837ca5ee1b34439a9c581ad6ab87a64.execution.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_ -- --nocapture`
    - `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
    - `npm --prefix crates/oasis7_viewer run test:ui`
    - `npm --prefix crates/oasis7_viewer run build:software-safe`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] hosted-account-tablestore-backend (PRD-P2P-029) [test_tier_required]: 将 `oasis7_game_launcher` 的 hosted account 服务端持久化抽成 `file/tablestore` 双 backend，支持 `OASIS7_HOSTED_ACCOUNT_STORE_BACKEND=auto|file|tablestore`、`OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` / `ALIYUN_OTS_*` 配置、自动建表，并把 `hosted_account_id -> player_id` 映射迁移到可选的 Aliyun Tablestore 托管存储。 Trace: .pm/tasks/task_8cccaa2362df47eab30b9eb52b7ddf6c.yaml
  - 产物文件:
    - `Cargo.lock`
    - `crates/oasis7/Cargo.toml`
    - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity_tests.rs`
    - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_store_backend.rs`
    - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`
    - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md`
    - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
    - `doc/p2p/project.md`
    - `.pm/tasks/task_8cccaa2362df47eab30b9eb52b7ddf6c.execution.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_account_identity -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_ -- --nocapture`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] hosted-account-env-tiering (PRD-P2P-029-G) [test_tier_required]: 冻结 hosted account 中心化服务的 `dev/staging/production` 环境边界，明确 SMTP、account store、strong-auth/custody secrets、风控参数、对外 claims 与最小验证命令不得跨层混用，并把 operator runbook 收口为分环境执行清单。 Trace: .pm/tasks/task_ad5cbac95aa54e26a9fa7d7558380750.yaml
  - File Structure / Affected Paths:
    - 正式回写:
      - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`
      - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
      - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
      - `doc/p2p/project.md`
      - `.pm/tasks/task_ad5cbac95aa54e26a9fa7d7558380750.execution.md`
    - 只读依赖:
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity.rs`
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_store_backend.rs`
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_strong_auth.rs`
  - 原子步骤:
    1. 将 `dev/staging/production` 的环境定义、允许项、禁止项和 claims 边界回写 PRD。
       - 验证命令: `rg -n "SC-8|Environment Tiering Contract|NFR-P2P-029-7|PRD-P2P-029-G" doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`
       - 预期结果: PRD 明确分层，不再只描述 SMTP/Tablestore 可用性。
    2. 将 operator runbook 收口为分环境最小配置、禁止 shortcut 和 promotion gate。
       - 验证命令: `rg -n "5B\. 分环境执行清单|dev 环境|staging 环境|production 环境|promotion gate" doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
       - 预期结果: runbook 能回答“测试环境和正式环境怎么分、不能混什么、升生产前要验什么”。
    3. 同步模块 project 与 execution log，保留 task trace 和 fresh verification 入口。
       - 验证命令: `rg -n "hosted-account-env-tiering|task_ad5cbac95aa54e26a9fa7d7558380750" doc/p2p/project.md .pm/tasks/task_ad5cbac95aa54e26a9fa7d7558380750.execution.md`
       - 预期结果: 模块追踪和 task 过程可回溯。
  - 验收命令 (`test_tier_required`):
    - `rg -n "SC-8|Environment Tiering Contract|NFR-P2P-029-7|PRD-P2P-029-G|hosted-account-env-tiering|5B\. 分环境执行清单|promotion gate" doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md doc/p2p/project.md .pm/tasks/task_ad5cbac95aa54e26a9fa7d7558380750.execution.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] hosted-account-staging-automation (PRD-P2P-029-G) [test_tier_required]: 新增 repo-owned `scripts/hosted-account-staging-smoke.sh`，把 hosted account 的本地 required smoke 与 staging `smtp + store continuity` live smoke 收成同一条自动化入口，并把命令统一回写到 operator runbook 与模块 project。 Trace: .pm/tasks/task_f445927d10234bada7bb7058a1d2f5d0.yaml
  - File Structure / Affected Paths:
    - 正式回写:
      - `scripts/hosted-account-staging-smoke.sh`
      - `scripts/ci-tests.sh`
      - `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
      - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
      - `doc/p2p/project.md`
      - `.pm/tasks/task_f445927d10234bada7bb7058a1d2f5d0.execution.md`
    - 只读依赖:
      - `crates/oasis7/src/bin/oasis7_game_launcher/static_http.rs`
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity.rs`
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_store_backend.rs`
      - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
  - 原子步骤:
    1. 新增 hosted account smoke 脚本，自动完成 `login/start -> login/complete -> player-session/release -> launcher restart -> stable account continuity`。
       - 验证命令: `bash ./scripts/hosted-account-staging-smoke.sh --mode local`
       - 预期结果: 本地 `smtp + file backend + otp-fetch-command` smoke 通过，并生成 summary artifact。
    2. 将本地 smoke 接入 repo-owned required 自动化，同时保留 staging `smtp + otp-fetch-command` 入口。
       - 验证命令: `bash -n scripts/hosted-account-staging-smoke.sh && rg -n "hosted account local smoke|OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE" scripts/ci-tests.sh`
       - 预期结果: `./scripts/ci-tests.sh required` 拥有稳定的本地 hosted account e2e smoke，而 staging 继续复用同一脚本。
    3. 回写 operator runbook 和 hosted account project，明确 staging 自动化链路与证据边界。
       - 验证命令: `rg -n "hosted-account-staging-smoke.sh|staging 自动化链路|ci-tests.sh required" doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md doc/p2p/project.md`
       - 预期结果: 文档能够直接回答“repo-owned 自动化链路怎么跑”。
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/hosted-account-staging-smoke.sh`
    - `bash ./scripts/hosted-account-staging-smoke.sh --mode local`
    - `rg -n "hosted account local smoke|OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE" scripts/ci-tests.sh`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`

### 后续切片
- `runtime_engineer` / managed-custody-sign-api:
  - 目标: 建立 `signer_ref`、custody sign API、runtime 验签与审计记录，替代当前 preview `approval_code + env signer` 的长期方案。
- `runtime_engineer` + `viewer_engineer` + `qa_engineer` / step-up-auth-and-risk-policy:
  - 目标: 为 `prompt_control_apply/rollback/main_token_transfer` 等动作接入 step-up auth、风险策略与结构化拒绝。
- `runtime_engineer` + `viewer_engineer` / external-wallet-bind-and-transfer-out:
  - 目标: 落地 external wallet bind、托管退出、transfer-out cooldown 与 custody mode 切换。
- `qa_engineer` + `liveops_community` / qa-abuse-and-liveops-runbook:
  - 目标: 建立 hosted account abuse suite、账户冻结/恢复/runbook、事故模板与 claims boundary。

## 角色拆解
### hosted-account-identity-broker / runtime_engineer + viewer_engineer
- 输入:
  - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
  - `crates/oasis7_viewer/software_safe.js`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- 输出:
  - hosted account contract
  - 邮箱登录入口
  - `hosted_account_id -> player_id` 绑定规则
- 完成定义:
  - 不输入裸公私钥也能完成 hosted player login
  - 同一账户换设备可恢复，不靠旧私钥文件

### device-session-and-runtime-binding / runtime_engineer + viewer_engineer
- 输入:
  - hosted account contract
  - 当前 `localStorage` 持久化 hosted player private key 路径
- 输出:
  - `device_session` 数据模型
  - 浏览器安全存储替换方案
  - runtime rebind / recover / revoke 流程
- 完成定义:
  - hosted 浏览器不再把长期 player signer 私钥写入 `localStorage`
  - 断线重连与 runtime 恢复依赖设备会话，不依赖 legacy bootstrap

### managed-custody-sign-api / runtime_engineer
- 输入:
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - 当前 preview `hosted_strong_auth` 真值
- 输出:
  - `signer_ref`
  - sign API
  - custody audit contract
- 完成定义:
  - `main_token_transfer` 有 hosted 目标态，不再只有 `blocked_until_strong_auth`
  - runtime 只信任 sign proof，不信任浏览器自报托管私钥

### step-up-auth-and-risk-policy / runtime_engineer + viewer_engineer + qa_engineer
- 输入:
  - sign API
  - high-risk action matrix
- 输出:
  - step-up auth UX
  - 风控状态机
  - 结构化错误码
- 完成定义:
  - 高风险动作都能落到 `requested/challenged/approved/denied` 等可审计状态
  - 无法再用普通 player session 穿透到资产或高风险 creator action

### external-wallet-bind-and-transfer-out / runtime_engineer + viewer_engineer
- 输入:
  - hosted account 与 managed signer contract
- 输出:
  - external wallet bind
  - transfer-out request/cooldown
  - custody mode switch 规则
- 完成定义:
  - 托管不是永久锁定；用户有显式自托管升级路径
  - 迁移过程中不会让 managed 与 self-custody 对同一动作并发出签

### qa-abuse-and-liveops-runbook / qa_engineer + liveops_community
- 输入:
  - 登录、设备会话、sign API、step-up 与 transfer-out 方案
- 输出:
  - abuse suite
  - recovery/freeze/revoke runbook
  - 对外 claims 与 incident 模板
- 完成定义:
  - 盗号、设备丢失、重复绑定、OTP 滥刷、风控冻结、托管退出失败都能给出 block/pass 结论

## 当前结论
- 结论-1: 对 `hosted_public_join` 而言，“邮箱登录 + 中心化托管密钥 + 可选自托管升级”是比“让普通玩家保存公私钥”更合适的正式产品路径。
- 结论-2: 中心化 KMS 不是直接替代全部产品语义；更准确的落法是 `identity broker + custody service + sign API`，KMS/HSM 作为 custody backend 的实现选项，而不是前端/运行时直接耦合的唯一接口。
- 结论-3: 当前代码已经完成两刀 hosted identity 基线：其一是 `device_session` 收口，viewer 不再把 hosted player `privateKey` 持久化到 `localStorage`；其二是中心化 hosted account 登录 server，`oasis7_game_launcher` 现已提供 email login challenge、稳定 `hosted_account_id -> player_id` 持久化和登录后换发 `device_session + player_session` 的 public route，viewer 也已改成 hosted account 登录表单。
- 结论-4: 当前登录投递已具备真实邮件链路：server 面向用户固定使用 `smtp` challenge delivery lane，不再提供部署期 delivery mode 开关；`smtp` 通过 `OASIS7_HOSTED_LOGIN_SMTP_*` 环境变量加载配置，默认可对接 Aliyun DirectMail `smtpdm.aliyun.com:465`；同时 OTP start 路径已补 resend cooldown、短窗/长窗配额与 `retry_after_seconds` 反馈，避免前端只能盲目重试。
- 结论-5: 当前 hosted account registry 已支持 Aliyun Tablestore 托管存储；服务端通过 `HostedAccountStoreBackend` 在 `file` 与 `tablestore` 之间切换，默认 `auto` 模式下会在检测到 `OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` 或 `ALIYUN_OTS_*` 后自动启用 Tablestore。
- 结论-5A: 2026-05-20 已在 ECS 上完成一次真实 VPC Tablestore smoke：`https://oasis7.cn-huhehaote.vpc.tablestore.aliyuncs.com` 可从部署机直连，`AUTO_CREATE=true` 时首次启动允许由 `OTSObjectNotExist` 进入自动建表；同一邮箱在 launcher 重启前后两次登录均返回同一个 `hosted_account_id` / `player_id`，证明 hosted identity MVP 的“邮箱登录 + 服务端持久化恢复”主链路已经跑通。
- 结论-5B: 当前已补 repo-owned `scripts/hosted-account-staging-smoke.sh`。同一脚本默认使用 `smtp`，并要求 `--otp-fetch-command <cmd>` 读取云上投递的真实 OTP；本地可搭配 file backend 验证 account continuity，staging 可直接验证真实 OTP 与跨重启 account continuity，不再需要临时拼散命令。
- 结论-6: 托管身份仅面向 player plane；node / validator / governance signer 继续沿用独立 custody/governance 专题。
- 结论-7: hosted account 服务从现在起必须按 `dev/staging/production` 分层执行；环境分层的最小真值不是“不同 URL”，而是 SMTP、account store、strong-auth/custody secret、风控阈值和对外 claims 的独立隔离。

## 依赖
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
- `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/hosted_strong_auth.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs`
- `crates/oasis7_viewer/software_safe.js`
- `testing-manual.md`

## 验收命令（本轮文档冻结）
- `rg -n "PRD-P2P-029|托管身份|托管密钥|邮箱|hosted account|signer_ref" doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前状态: active
- 下一步: repo-owned hosted account smoke 已补齐；接下来优先补 staging 的 revoke/recovery operator drill 证据与自动化封装，再推进 `managed-custody-sign-api`，把高风险动作从 preview `approval_code + env signer` 迁移到正式托管签名后端。
- 最近更新: 2026-05-23
