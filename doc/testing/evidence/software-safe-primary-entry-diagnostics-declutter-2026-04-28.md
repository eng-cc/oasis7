# software_safe primary-entry diagnostics declutter evidence (2026-04-28)

## Scope

- 目标: 为 `software_safe-primary-entry-diagnostics-declutter` 提供一张可用于 PR 审阅的界面证据图，证明主入口首屏已经把 blocker / 审批状态 / 恢复指引提升为主要状态，并把 execution lane / auth / session 诊断收进折叠 surface。
- 边界: 这份证据只验证前端信息层级与 deterministic snapshot 渲染，不替代 runtime/live 正式回归。

## Capture Method

1. 使用本地静态服务暴露 `crates/oasis7_viewer/`。
2. 复用 `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html` 同源加载真实 `software_safe.html`。
3. 等待证据页注入一条带 `first_agent_claim_approval_request` 且无可选实体的 snapshot。
4. 用 Chrome headless 截图保存到仓库可追踪路径。

## Commands

```bash
python3 -m http.server 4275 --bind 127.0.0.1 --directory crates/oasis7_viewer
google-chrome --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1600,2200 \
  --virtual-time-budget=4000 \
  --screenshot=doc/testing/evidence/assets/software-safe-primary-entry-diagnostics-declutter-2026-04-28.png \
  http://127.0.0.1:4275/software_safe_first_agent_claim_evidence.html
```

## Artifacts

- 截图: `doc/testing/evidence/assets/software-safe-primary-entry-diagnostics-declutter-2026-04-28.png`
- 证据页: `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html`

## Notes

- 左栏故意保持 `agents=0 / locations=0`，用来验证空实体快照不会再把“先选 Agent”错当成主要提示。
- 中栏同时验证三件事: blocker 独立成卡、首个 agent claim 审批状态仍在正式摘要内可见、诊断区默认折叠。
