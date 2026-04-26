# task_46ed3d5e9da64dc2a9b56ede6a5735c5 Execution Log

- task_uid: task_46ed3d5e9da64dc2a9b56ede6a5735c5
- title: homepage entry claim boundary hardening
- owner_role: viewer_engineer
- worktree_hint: /home/scc/worktrees/oasis7-site-homepage-entry-claim-boundary-hardening

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-26 11:52:48 CST / viewer_engineer
- 完成内容: 基于多角色 review 意见收紧中英首页 claim 边界，明确公开访客入口、builder 验证路径与未来平台方向分层；将 `software_safe` 固定为默认 formal Web 入口，移除首页 primary path 对 `--no-llm` 的误导性暴露，并新增 builder-only 折叠细节区。
- 完成内容: 补齐 `site/assets/styles.css` 与 `site/assets/app.js` 的 no-JS 导航渐进增强、skip-link 与边界提示样式；新增 `scripts/site-homepage-claim-check.sh`，并把该门禁接入 `.github/workflows/pages.yml`。
- 完成内容: 已通过 `./scripts/site-link-check.sh`、`./scripts/site-manual-sync-check.sh`、`./scripts/site-download-check.sh`、`./scripts/site-homepage-claim-check.sh`、`git diff --check`。
- 遗留事项: `agent-browser` 本地打开静态站时出现崩溃，未形成浏览器级截图证据；`./scripts/doc-governance-check.sh` 曾在一次 `timeout 30` 试跑中超时，需在后续若该门禁恢复异常时单独复核脚本运行状态。

## 2026-04-26 13:13:14 CST / viewer_engineer
- 完成内容: 基于截图复核继续收紧移动端布局约束，修复首屏 chip、按钮和 `pulse-panel` 行内容在窄屏下的可收缩性；补充 `overflow-x` 防护，避免视觉上再出现“页面被右侧撑出”的判断噪音。
- 完成内容: 通过 headless Chrome + CDP 以 `390px` 真移动视口复核首页，确认 `menu-button` / `lang-switch` 都在屏内，`scrollWidth` 与 `clientWidth` 同为 `390`，不再存在真实横向溢出。
- 完成内容: 修正了移动端 `pulse-panel` 纵向布局继承桌面 `flex-basis` 导致的异常留白；最新截图证据已输出到 `output/agent-browser/homepage-mobile-cdp-v2.png` 与 `output/agent-browser/homepage-desktop-v2.png`。
- 遗留事项: `agent-browser` 仍在 `page.setViewportSize: Target crashed` 阶段崩溃，因此这轮视觉验证继续使用 headless Chrome / CDP 旁路完成，后续若要恢复 viewer Web 自动截图链路，需单独排查 agent-browser 运行时环境。
