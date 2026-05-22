# task_1aef6daff1ef4e81bbc3a6a34531a828 Execution Log

- task_uid: task_1aef6daff1ef4e81bbc3a6a34531a828
- title: release gate web and soak hardening
- owner_role: qa_engineer
- worktree_hint: /home/scc/worktrees/oasis7-testing-release-gate-web-soak-hardening

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-22 16:49:02 CST / qa_engineer
- 完成内容: 复盘 `Release Packages` run `26276289822`（tag `v0.0.51`）的失败签名，确认 `release-gate-web` 由 `viewer-software-safe-step-regression.sh` 对未使用的 `rg` 硬依赖触发，`release-gate-soak` 则在 `restart,pause` chaos 后立刻采样，把预期恢复瞬态计入 `last_error_samples=1`、`running_false_samples=8`、`http_failure_samples=16` 并阻断发布。
- 遗留事项: 需要补脚本修复并用同 seed 的 300 秒 `triad_distributed` 样本验证。

## 2026-05-22 16:49:02 CST / qa_engineer
- 完成内容: 移除 `scripts/viewer-software-safe-step-regression.sh` 对 `rg` 的无效硬依赖；为 `scripts/p2p-longrun-soak.sh` 增加 chaos 事件后的节点恢复等待，要求目标节点恢复到 `healthz/status/balances` 可读、`running=true` 且 `last_error/load_error` 清空后再恢复采样，并把实际恢复时长记入 `chaos_exempt_secs`。本地验证已通过：`bash -n scripts/viewer-software-safe-step-regression.sh scripts/p2p-longrun-soak.sh`、`./scripts/viewer-software-safe-step-regression.sh --help >/dev/null`、`env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime`，以及 `.tmp/release_gate_p2p_v051_fix/20260522-164326` 的 300 秒同 seed S9 样本；结果为 `metric_gate=pass`，`running_false_samples=0`、`last_error_samples=0`、`status_samples_ok=342`、`balances_samples_ok=342`。
- 遗留事项: 待执行 task closeout、提交 PR、合入 `main` 后打新 tag 重新触发 release。

## 2026-05-22 16:49:02 CST / qa_engineer
- 完成内容: 补跑 `web_strict` 时继续挖出两个被 `rg` 更早遮住的 Web blocker：`release-gate-web-strict.sh` 会把 `--scenario llm_bootstrap` 透传到 `viewer-software-safe-step-regression.sh`，但 `run-game-test.sh` 之前不接受该参数；同时 `viewer-software-safe-step-regression.sh` 自身曾从正式 `step -> completed_advanced + world delta` 合约退化成“只等自然推进”，与 `doc/testing/prd.md` / `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 的 canonical contract 不一致。已修复 `run-game-test.sh` 的 `--scenario` 支持，并恢复 software-safe step 回归在无自然推进时主动发送一次 canonical `step` 再判定 formal progress。补充验证已通过：`.tmp/release_gate_web_v051_fix_rerun3/20260522-170231/release-gate-summary.md` 为 `Overall: PASS`，其中 `web_strict: passed (ok)`；`.tmp/release_gate_web_v051_fix_rerun3/20260522-170231/web_strict/software-safe-step/20260522-170251/software-safe-step-summary.md` 为 `ok=True`、`logicalTimeAdvanced=True`、`eventSeqAdvanced=True`。
- 遗留事项: 待执行 task closeout、提交 PR、合入 `main` 后打新 tag 重新触发 release。

## 2026-05-22 17:24:00 CST / qa_engineer
- 完成内容: 响应 PR `#265` review comment，修复 `scripts/run-game-test.sh` 在 `set -euo pipefail` 下对 `--scenario` 缺参时会直接 `shift 2` 的签名，改为显式输出 `error: --scenario requires a value` 并附 usage。回归验证已通过：`bash -n scripts/run-game-test.sh`，以及 `./scripts/run-game-test.sh --scenario` 现在稳定返回 `rc=1` 且输出明确错误，不再抛裸 shell 异常。
- 遗留事项: 待 follow-up commit/push，并 resolve 该条 review thread。

## 2026-05-22 20:52:42 CST / qa_engineer
- 完成内容: 响应后续 PR `#265` review comment，删除 `scripts/viewer-software-safe-step-regression.sh` 中对不存在 viewer control feedback stage `completed_timeout` 的等待分支，改为仅接受 repo 当前真实 contract：`completed_advanced` / `completed_no_progress` / `blocked`，以及正向 `logicalTime` / `eventSeq` delta。验证已通过：`bash -n scripts/viewer-software-safe-step-regression.sh`，并用 `rg -n "completed_timeout" scripts/viewer-software-safe-step-regression.sh` 确认该死分支已清除。
- 遗留事项: 待 follow-up commit/push，并 resolve 该条 review thread。
