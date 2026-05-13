# viewer wasm-only runtime proof (2026-05-13)

## Scope

- 目标: 为 PR #217 补一张真实 Viewer 页面截图，证明当前 `pixel-world` 嵌入舞台仍能在真实 launcher/live 栈下正常渲染，同时本轮去掉 JS renderer fallback 后主入口仍可加载、连接并显示世界舞台。
- 边界: 这份证据只补“真实页面截图”与当前页面可见状态，不替代 `required-gate`、repo-owned loader/host 测试或更长玩法回归。

## Capture Method

1. 在当前 task worktree 内启动隔离 harness：`./scripts/worktree-harness.sh up --smoke-timeout 30`
2. 使用 harness 暴露的真实页面 URL：
   - `http://127.0.0.1:46520/?ws=ws://127.0.0.1:46521&test_api=1&locale=zh`
3. 通过 `agent-browser` 打开页面并等待 `networkidle`。
4. 把截图保存到仓库证据路径。

## Artifact

- 截图: `doc/world-simulator/viewer/evidence/viewer-wasm-only-runtime-proof-2026-05-13.png`

## Notes

- 本轮截图来自真实 `launcher + viewer live + web bridge` 栈，不是静态 mock 页面。
- 截图采样时 worktree harness 状态为 `ready`，并已完成一次 `__AW_TEST__` smoke。
