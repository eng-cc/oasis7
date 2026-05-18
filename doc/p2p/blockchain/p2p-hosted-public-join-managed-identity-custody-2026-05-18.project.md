# oasis7 hosted_public_join 托管身份 / 托管密钥与手机号邮箱登录（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] hosted-managed-identity-doc-freeze (PRD-P2P-029) [test_tier_required]: 冻结 `hosted_public_join` 的托管身份、托管密钥、手机号/邮箱登录、自托管升级和 trust boundary 文档真值，并回写模块入口映射。 Trace: .pm/tasks/task_fd98df36264944238538dea896ce4ce0.yaml
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

### 后续切片
- `runtime_engineer` + `viewer_engineer` / hosted-account-identity-broker:
  - 目标: 落地 hosted account、手机号/邮箱 OTP/magic link/passkey、`hosted_account_id` 与 `player_id` 绑定、设备识别与恢复流程。
- `runtime_engineer` + `viewer_engineer` / device-session-and-runtime-binding:
  - 目标: 用 `device_session` 替换当前浏览器 `privateKey` 持久化，打通 player-session refresh/rebind/recovery 与 runtime entity binding。
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
  - 手机号/邮箱登录入口
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
- 结论-1: 对 `hosted_public_join` 而言，“手机号/邮箱登录 + 中心化托管密钥 + 可选自托管升级”是比“让普通玩家保存公私钥”更合适的正式产品路径。
- 结论-2: 中心化 KMS 不是直接替代全部产品语义；更准确的落法是 `identity broker + custody service + sign API`，KMS/HSM 作为 custody backend 的实现选项，而不是前端/运行时直接耦合的唯一接口。
- 结论-3: 当前代码已经完成第一刀 `device_session` 收口：launcher grant 新增 `device_session_id`，viewer 不再把 hosted player `privateKey` 持久化到 `localStorage`，刷新页后只保留 `device_session` handle，并按需在页内重新生成临时 Ed25519 session key；但 hosted account、手机号/邮箱登录、custody sign API 与真正的托管签名后端仍未实现。
- 结论-4: 托管身份仅面向 player plane；node / validator / governance signer 继续沿用独立 custody/governance 专题。

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
- `rg -n "PRD-P2P-029|托管身份|托管密钥|手机号|邮箱|hosted account|signer_ref" doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前状态: active
- 下一步: 优先执行 `hosted-account-identity-broker`，把 hosted account、手机号/邮箱 OTP/magic link/passkey、`hosted_account_id -> player_id` 绑定与跨设备恢复落成代码真值；随后推进 `managed-custody-sign-api`，把高风险动作从 preview `approval_code + env signer` 迁移到正式托管签名后端。
- 最近更新: 2026-05-18
