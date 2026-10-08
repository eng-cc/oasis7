# software_safe primary-entry diagnostics declutter evidence (2026-04-28)

## Scope

- 目标: 为 `software_safe-primary-entry-diagnostics-declutter` 提供一张可用于 PR 审阅的界面证据图，证明主入口首屏已经把 blocker / 首笔启动金提示 / 恢复指引提升为主要状态，并把 execution lane / auth / session 诊断收进折叠 surface。
- 边界: 这份证据只验证前端信息层级与 deterministic snapshot 渲染，不替代 runtime/live 正式回归。

## Capture Method

1. 使用本地静态服务暴露 `crates/oasis7_viewer/`。
2. 这张 2026-04-28 历史截图使用当时的 `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html` 版本同源加载 `software_safe.html`。该源码路径已退役；Git history 中可核验的当时最近 checked-in 版本为 `b68b4ae653`（blob SHA-256 `a37582a1b4c98dc22df8790fd7b0c283e2a16f66360c82122901603527611988`）。当前 canonical fixture 是 `crates/oasis7_viewer/viewer_first_agent_claim_evidence.html`，构建生成的兼容页为 `crates/oasis7_viewer/dist/software_safe_first_agent_claim_evidence.html`。归档截图不表示它由当前 fixture 或当前生成页产出。
3. 等待证据页注入一条带 `slot_1_auto_restricted_starter_claim_amount` 且无可选实体的 snapshot。
4. 用 Chrome headless 截图保存到仓库可追踪路径。

## Historical Commands

The commands below record the historical capture route against the then-present source page. They are provenance, not a current reproduction command.

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
- 历史证据页路径: `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html`（版本锚点见上方 Git-history provenance）
- 当前 canonical fixture successor: `crates/oasis7_viewer/viewer_first_agent_claim_evidence.html`
- 当前生成的兼容证据 route: `crates/oasis7_viewer/dist/software_safe_first_agent_claim_evidence.html`

## Notes

- 左栏故意保持 `agents=0 / locations=0`，用来验证空实体快照不会再把“先选 Agent”错当成主要提示。
- 中栏同时验证三件事: blocker 独立成卡、首个 `slot-1` auto-funding 摘要仍在正式界面内可见、诊断区默认折叠。
