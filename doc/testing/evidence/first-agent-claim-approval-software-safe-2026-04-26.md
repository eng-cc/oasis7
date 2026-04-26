# software_safe first-agent-claim approval evidence (2026-04-26)

## Scope

- 目标: 为 `PRD-WORLD_RUNTIME-040` / `PRD-WORLD_SIMULATOR-045` 补一张可用于 PR 审阅的界面证据图，证明 `software_safe` 正式玩法摘要已能展示首个 agent claim 审批状态卡。
- 边界: 这份证据只验证前台摘要卡的渲染契约，不替代 runtime/API 真链路回归。

## Capture Method

1. 使用本地静态服务暴露 `crates/oasis7_viewer/`。
2. 通过 `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html` 同源加载真实 `software_safe.html`。
3. 等待 `__AW_TEST__.injectSnapshot(...)` 注入一条 runtime-compatible `player_gameplay.agent_claim.first_agent_claim_approval_request` 快照。
4. 用 Chrome headless 截图保存到仓库内可追踪路径。

## Commands

```bash
python3 -m http.server 4275 --bind 127.0.0.1 --directory crates/oasis7_viewer
google-chrome --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1600,1400 \
  --virtual-time-budget=4000 \
  --screenshot=doc/testing/evidence/assets/first-agent-claim-approval-software-safe-2026-04-26.png \
  http://127.0.0.1:4275/software_safe_first_agent_claim_evidence.html
```

## Artifacts

- 截图: `doc/testing/evidence/assets/first-agent-claim-approval-software-safe-2026-04-26.png`
- 证据页: `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html`

## Notes

- 证据页通过 `connect=0` 显式关闭 live viewer websocket，自身只负责静态 deterministic snapshot 渲染，避免把无关的连接错误带进 PR 截图。
- 真正的 runtime/API 闭环仍由本任务已有的 Rust/API 回归覆盖：审批 request/approve/reject/claim、snapshot 回流、以及 `software_safe` contract test。
