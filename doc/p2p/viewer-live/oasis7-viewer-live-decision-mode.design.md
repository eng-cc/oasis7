# `oasis7_viewer_live` 决策模式设计

> 对应需求: `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.prd.md`
> 对应执行台账: `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.project.md`

## 结构

1. **解析层**：`CliOptions` 默认 `llm_mode=true`；`--llm` 写入 true，`--no-llm` 写入 false，因此重复 flag 遵循最后一次写入。
2. **配置层**：解析结果映射到 `ViewerLiveDecisionMode::{Llm, Script}`，仅配置 direct viewer live 服务。
3. **入口边界层**：launcher 的 gameplay boot 不复用 Script 作为成功回退；其无 LLM 负向路径应在启动前失败并给出 direct observer/debug 提示。
4. **控制面层**：viewer CLI 只承载 bind、Web bridge、决策模式和既有链状态/提交连接参数；release/node/runtime 选项被解析层拒绝，避免双控制面。

## 表现与验收边界

- `--no-llm` 的可见语义必须是 observer/debug only，而不是“降级后正式可玩”。
- 不改变现有页面、DOM、Web test API 或视觉流程；未来若更改 launcher 的无 LLM 表现、CTA 或玩家流程，须由视觉交互专业结论定义 S6 验收。
- 直接 Viewer 与 launcher/Web 的证据不能互相替代：前者的 Script 截图/日志不构成后者的 gameplay 或 release 证明。

## 代码接点

- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `scripts/worktree-harness.sh`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

本设计记录现有结构，不授权新增 fallback、CLI 选项、provider 策略或链控制能力。
