# P2P user mode launcher UX 证据（2026-04-07）

审计轮次: 1

## Meta
- 关联专题: `P2PARCH-9`
- 关联文档:
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md`
  - `testing-manual.md`
- 责任角色: `viewer_engineer`
- 当前结论: `pass`
- 目标: 固化 launcher/viewer 对 `auto_join` / `private_safe` / `public_entry` 三档用户模式的可见性、默认推荐、底层角色映射，以及 `public_entry` 显式确认门禁与回退路径。

## 最终结论
- `oasis7_web_launcher` 已经把 chain runtime 的 P2P 推荐态透传到 `/api/state`，viewer 可以同时显示 `requested` / `recommended` / `applied` 三个层次。
- `oasis7_client_launcher` 已实现用户层三档模式 UX：
  - 配置项持久化 `chain_p2p_user_mode`
  - 显示 reachability / hole punch / relay / 探针证据
  - 将底层 `deployment_mode` / `node_role_claim` 显式映射为可读摘要
  - 对 `public_entry` 提供明确的 accept / reject 流程
- 显式选择 `public_entry` 但未确认风险时，launcher 配置校验与 runtime args 构造都会阻断启动；拒绝路径会回落到 `auto_join`。
- `test_tier_full` 证据目前覆盖 launcher/runtime 自动化与 viewer 状态面，不覆盖 dedicated AutoNAT / 真实公网入口实验室。

## 执行命令
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture --skip transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape
env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 cargo test -p oasis7_client_launcher -- --nocapture --skip transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape
env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown
./scripts/doc-governance-check.sh
git diff --check
```

## 结果
- `cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture`
  - `69 passed; 0 failed`
  - 新增覆盖:
    - `/v1/chain/status` P2P payload 读取
    - `StateSnapshot.chain_p2p_status` 序列化
    - `public_entry` 未确认时的 config / args 阻断
- `cargo test -p oasis7_client_launcher -- --nocapture`
  - `113 passed; 1 failed`
  - 失败用例:
    - `transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape`
  - 判定:
    - 该失败来自既有 `transfer_auth` 签名一致性测试，不属于本轮 P2P user-mode UX 改动面
- `cargo test -p oasis7_client_launcher -- --nocapture --skip transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape`
  - 结果:
    - 一次并行执行中出现既有 `transfer_entry::tests::build_transfer_submit_request_parses_trimmed_values` 波动失败
  - 判定:
    - 该用例依赖进程级环境变量，和包内其他 transfer/auth 测试并行时存在抢占波动，不属于本轮 P2P 改动面
- `RUST_TEST_THREADS=1 cargo test -p oasis7_client_launcher -- --nocapture --skip transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape`
  - `113 passed; 0 failed; 1 filtered out`
  - 新增覆盖:
    - `chain_p2p_user_mode` 默认值与参数透传
    - `public_entry` 显式确认门禁
    - web snapshot 对 `chain_p2p_status` 的追踪
- `cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`
  - `Finished dev profile`
  - 结果: 通过，仅存在既有 warnings
- `./scripts/doc-governance-check.sh`
  - 结果: 通过
- `git diff --check`
  - 结果: 通过

## 覆盖口径
### 1. 用户层模式可见性
- viewer 会展示:
  - 用户请求模式 `requested`
  - 探测后建议模式 `recommended`
  - 最终生效模式 `applied`
- 文案使用产品层三档语义：
  - `auto_join`
  - `private_safe`
  - `public_entry`

### 2. 底层语义对账
- viewer 会同步显示:
  - reachability 探针结果
  - hole punch / relay reservation 可用性
  - `deployment_mode`
  - `node_role_claim`
- 这保证用户只接触简化模式，同时仍能从诊断面追踪到底层正式角色语义。

### 3. `public_entry` 风险确认
- 当推荐态或用户显式选择落到 `public_entry` 时，launcher 会要求额外确认。
- `accept` 路径:
  - 写入 `chain_p2p_accept_public_entry=true`
  - 允许构建 runtime args
- `reject` 路径:
  - 清掉确认位
  - 回落到 `auto_join`
  - 保持可重试

## 当前边界
- 本证据不是 dedicated NAT / AutoNAT / 公网入口 live lab。它证明的是 launcher/viewer 已具备自动推荐、显式确认、底层映射和阻断门禁。
- 若后续要把 `public_entry` 升为更强的 release gate，还需要补真实 mixed-topology public-entry 实证，尤其是公网入口接受确认后的外部可达性取样。
- `oasis7_client_launcher` 现有 transfer/auth 测试中仍有两个与本专题无关的包级噪音:
  - 稳定失败: `transfer_auth::tests::wasm_transfer_signing_payload_matches_runtime_helper_shape`
  - 并行波动: `transfer_entry::tests::build_transfer_submit_request_parses_trimmed_values`
