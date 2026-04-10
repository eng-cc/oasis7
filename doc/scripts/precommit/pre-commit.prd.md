# Pre-commit Checks（本地提交前测试脚本）

- 对应设计文档: `doc/scripts/precommit/pre-commit.design.md`
- 对应项目管理文档: `doc/scripts/precommit/pre-commit.project.md`

审计轮次: 4


## 目标
- 在本地提交前执行轻量 commit baseline，尽快反馈常见回归。
- 以单一脚本形式减少重复维护，降低遗漏风险。

## 范围
- **范围内**：执行本地提交前格式化（仅格式化已暂存 Rust 文件）与 `commit` 级别测试（文档治理 + 格式校验 + 轻量 support/viewer 套件）。
- **范围内**：`required` 继续保留核心 runtime/simulator shard；凡是需要注册或执行 builtin wasm artifact 的 runtime 闭环用例，统一下放到 `test_tier_full`。
- **范围外**：lint 或其它包的静态检查。
- **范围外**：`libp2p`/`wasmtime` 特性回归与 viewer 在线/离线联测（由 `full` 级别承担）。

## 接口 / 数据
- 脚本路径：`scripts/pre-commit.sh`
- 运行命令：`./scripts/pre-commit.sh`
- 执行内容：
  - 先通过 `git diff --cached --name-only --diff-filter=ACMR -- '*.rs'` 收集已暂存 Rust 文件，再执行 `env -u RUSTC_WRAPPER rustfmt --edition 2021 <files>`，并自动 `git add` 回暂存区。
  - 调用统一测试清单脚本：`./scripts/ci-tests.sh commit`。
    - `commit` tier 固定覆盖 `doc-governance + rust-size + fmt --check + oasis7_consensus --lib + oasis7_distfs --lib + software-safe feedback contract`。
    - `cargo test -p oasis7 --tests --features test_tier_required` 与 `cargo test -p oasis7_viewer` / `cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 都不进入默认提交路径；这些较重校验继续保留在显式 `./scripts/ci-tests.sh required` 与 CI required gate 中，本地 landing 前若需要兜底 viewer Rust 回归，也应显式执行该命令。
- 规则归属：
  - commit baseline 定义：本文件与 `scripts/ci-tests.sh`
  - required/full 覆盖命令矩阵：`doc/testing/ci/ci-test-coverage.prd.md` 与 `scripts/ci-tests.sh`
  - required/full 分层定义：`doc/testing/ci/ci-tiered-execution.prd.md`
  - case 标签定义（`test_tier_required`/`test_tier_full`）：`doc/testing/ci/ci-testcase-tiering.prd.md`

## 最小验收命令
- `./scripts/pre-commit.sh`
- `./scripts/ci-tests.sh commit`
- `./scripts/ci-tests.sh required`

## Git Hook
- **注意**：Git hooks 不会随仓库内容一并版本化；克隆到新仓库（或重新初始化 `.git`）后，默认不会自动带上 `pre-commit` hook，需要手动重新注册。
- 在仓库根目录重新注册：
```
cat > .git/hooks/pre-commit <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

./scripts/pre-commit.sh
HOOK

chmod +x .git/hooks/pre-commit
```
- 可用以下命令确认是否已注册：
```
test -x .git/hooks/pre-commit && echo "pre-commit hook installed"
```

## 失败修复
- 当 `pre-commit` 失败时，统一走 `./scripts/fix-precommit.sh`；修复流程与边界以 `doc/scripts/precommit/precommit-remediation-playbook.prd.md` 为准。

## 里程碑
- **M1**：新增本地提交前联测脚本并纳入文档说明。
- **M2**：提交前加入自动格式化时机，并在 CI 增加格式化检查。
- **M3**：补充“新仓库需重新注册 hook”文档与操作步骤。

## 风险
- **覆盖时延**：`cargo test -p oasis7 --tests --features test_tier_required` 与 viewer Rust 长跑从默认提交路径移出后，相关问题会延后到显式 `required` / CI required gate 暴露。
- **环境差异**：本地与 CI 依赖不同可能造成结果不一致。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
