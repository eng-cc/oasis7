# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] agent-workflow-borrowing-governance (PRD-ENGINEERING-031) [test_tier_required]: 建立专题 `prd/design/project`，首批冻结 `superpowers` 的 adopted / rejected / deferred 边界，并回写 engineering 根入口、主项目、索引与 Viewer Web 后续参考口径。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] completion-claim-verification-helper (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 落地 repo-owned `scripts/pm/claim-ready.sh`，把“fresh verification before completion claims”收成可执行 helper，并同步接入 `prepare-task-pr` 推荐输出、PM close checklist、README 与 shell regression。 Trace: .pm/tasks/task_32a955cb401e4a269f72113db4fa0371.yaml

## Planned Follow-ups
- `workflow-behavior-eval-harness-followup` (`PRD-ENGINEERING-AWB-001`, target `test_tier_required + test_tier_full`): 为 `new-task-worktree -> workflow-report -> task-closeout -> prepare-task-pr -> review-thread-closeout` 建立 repo-owned agent behavior eval harness，验证主链规则在真实 agent 回合中被遵守。启动时需创建独立 `.pm` task 与 worktree。
- `viewer-visual-companion-pilot-followup` (`PRD-ENGINEERING-AWB-003/PRD-WORLD_SIMULATOR-046`, target `test_tier_required`): 在 Viewer Web 下一轮结构/视觉专题中试点 browser-based visual companion，先产出 IA/wireframe/layout compare 再切实现 task，同时保持 `agent-browser` / repo-owned UI regression 仍是正式验证面。启动时需创建独立 `.pm` task 与 worktree。
- `multi-harness-workflow-packaging-deferred` (`PRD-ENGINEERING-AWB-004`, target `test_tier_required`): 在 repo-owned workflow helpers 与 evals 稳定后，再评估是否需要为 Codex/OpenCode 等 harness 做 workflow packaging；未到该阶段前保持 deferred。若重开，必须先新建专题 task。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.project.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `testing-manual.md`

## 状态
- 更新日期: 2026-05-19
- 当前阶段: planned
- 当前任务: `workflow-behavior-eval-harness-followup`
- 阻塞项:
  - `workflow-behavior-eval-harness-followup` 仍需决定 fixture 形态与“真实 agent 行为”采样面。
  - `viewer-visual-companion-pilot-followup` 必须等下一轮明确的 Viewer Web 设计任务创建后再绑定独立 `.pm` task。
- 最新完成:
  - 已建立专题三件套，冻结 `superpowers` 的 adopted / rejected / deferred 边界，并将 adopted 项收口为三条 repo-owned follow-up。
  - 已落地 `completion-claim-verification-helper`，新增 `scripts/pm/claim-ready.sh`，并把 helper 接入 `prepare-task-pr` 推荐输出、PM close checklist、README 与 shell regression。
- 下一步:
  - 优先推进 `workflow-behavior-eval-harness-followup`，先验证当前 engineering 主链是否能被 agent 稳定执行出来。
  - Viewer 方向仅在下一轮明确结构/视觉题时，按需启动 `viewer-visual-companion-pilot-followup`。
