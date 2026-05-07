# task_cff572a57ece4193a864b6db1fa9a4a0 Execution Log

- task_uid: task_cff572a57ece4193a864b6db1fa9a4a0
- title: Remove repo-local oasis7 skill after public raw export
- owner_role: viewer_engineer
- worktree_hint: /home/scc/worktrees/oasis7-site-public-oasis7-skill

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-07 23:13:28 CST / viewer_engineer
- 完成内容: 删除当前 PR worktree 中整个 `.agents/skills/oasis7/` 私有 skill 目录，并将 `site/skills/oasis7.md` 改写为不依赖私有 helper 的公开 raw skill 版本；公开入口现仅保留 repo-native Local Provider 命令、bundle-first 下载说明、bridge/launcher/parity smoke 路径与链密钥安全边界。
- 完成内容: 同步回写 `doc/site/{prd,project}.md`、`doc/scripts/{prd,project}.md`、`doc/world-runtime/project.md`、`doc/world-simulator/{prd,project}.md`、`doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`、`doc/world-simulator/llm/oasis7-governance-call-surface-2026-04-26.md`、`doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.project.md` 与 `doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`，把活跃文档中的私有 skill 真值引用切到公开 raw skill、repo-native 命令或正式专题锚点，同时保留 `doc/devlog/**` 与已完成历史任务行作为历史证据。
- 验证: `./scripts/doc-governance-check.sh`、`git diff --check`。
- 遗留事项: `doc/world-simulator/project.md` 仍保留两条已完成历史任务行对旧 skill 路径的产物描述；这是历史证据，不作为当前活跃入口。
