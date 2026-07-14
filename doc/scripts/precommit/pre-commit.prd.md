# Pre-commit Checks（本地提交前测试脚本）

- 对应设计文档: `doc/scripts/precommit/pre-commit.design.md`
- 对应项目管理文档: `doc/scripts/precommit/pre-commit.project.md`

审计轮次: 4


## 目标
- 普通本地提交不执行格式化、编译、测试、lint 或治理检查；验证由 CI required gate 与 frozen-head Pre-PR Ready 承担。
- 以单一脚本形式减少重复维护，降低遗漏风险。

## 范围
- **范围内**：保留 `scripts/pre-commit.sh` 作为已安装 legacy hook 的静默成功兼容入口。
- **范围内**：`required` 继续保留核心 runtime/simulator shard；凡是需要注册或执行 builtin wasm artifact 的 runtime 闭环用例，统一下放到 `test_tier_full`。
- **范围外**：lint 或其它包的静态检查。
- **范围外**：`libp2p`/`wasmtime` 特性回归与 viewer 在线/离线联测（由 `full` 级别承担）。

## 接口 / 数据
- 脚本路径：`scripts/pre-commit.sh`
- 运行命令：`./scripts/pre-commit.sh`
- 执行内容：静默返回成功，不读取或修改暂存区，不调用 `git`、`rustfmt`、`cargo`、`npm`、`ci-tests.sh` 或治理脚本。
- `./scripts/ci-tests.sh commit` 保留为显式诊断/开发命令，但不由普通 commit 调用，也不是 lifecycle proof。
- CI required gate 与 `claim-ready --verification-profile repository_required` 的 frozen-head Pre-PR Ready 验证保持不变。
- 规则归属：
  - 普通 commit no-op 定义：canonical workflow、本文件与 `scripts/pre-commit.sh`
  - required/full 覆盖命令矩阵：`doc/testing/ci/ci-test-coverage.prd.md` 与 `scripts/ci-tests.sh`
  - required/full 分层定义：`doc/testing/ci/ci-tiered-execution.prd.md`
  - case 标签定义（`test_tier_required`/`test_tier_full`）：`doc/testing/ci/ci-testcase-tiering.prd.md`

## 最小验收命令
- `./scripts/pre-commit.sh`
- `bash scripts/pre-commit.test.sh`
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
- legacy hook 调用该入口应始终静默成功；显式验证失败的修复流程以 `doc/scripts/precommit/precommit-remediation-playbook.prd.md` 为准。

## 里程碑
- **M1**：新增本地提交前联测脚本并纳入文档说明。
- **M2**：提交前加入自动格式化时机，并在 CI 增加格式化检查。
- **M3**：补充“新仓库需重新注册 hook”文档与操作步骤。

## 风险
- **覆盖时延**：所有回归都可能延后到 frozen-head Pre-PR Ready 或 CI required gate 暴露；不得据此重新把检查塞回普通 commit。
- **环境差异**：本地与 CI 依赖不同可能造成结果不一致。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
