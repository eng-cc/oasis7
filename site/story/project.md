# 《绿洲 2076》故事项目状态

## 当前任务

- [x] STORY-SITE-READER-REFRESH: 故事页阅读器与第一卷阅读版发布口径。
  - Trace: .pm/tasks/task_04e8e24fe8ea4fd69d6a0888b4bd587f.yaml
  - 范围：`site/story/index.html` 自动加载正文章节 markdown，移除旧资料分区，并将第一卷 `CH-001` 到 `CH-036` 整理为阅读版 v1.0-rc。
  - 产物文件：`site/story/index.html`、`site/assets/app.js`、`site/assets/styles.css`、`site/story/releases/volume-01-reading-version.md`、`site/story/draft/volume-01-chapter-001.md` 到 `site/story/draft/volume-01-chapter-004.md`。
  - 验收命令：`git diff --check && ./scripts/doc-governance-check.sh && python3 .agents/skills/epic-story-orchestrator-zh/tests/validate_writeback.py && bash .agents/skills/epic-story-orchestrator-zh/tests/run_smoke.sh && ./scripts/pm/workflow-lint.sh --task-uid task_04e8e24fe8ea4fd69d6a0888b4bd587f --phase current`。
- [x] STORY-VOLUME-004-CHAPTER-01-CARDS: 第四卷第一章正式章级卡。
  - Trace: .pm/tasks/task_5119bcacf5d444a8a5cdadb102f190f7.yaml
  - 范围：`TL-159` 到 `TL-166` / `CH-159` 到 `CH-166`。
  - 产物文件：`site/story/chapter-cards/volume-04-chapter-01-cards.md`。
  - 验收命令：`git diff --check && ./scripts/doc-governance-check.sh && ./scripts/pm/workflow-lint.sh --task-uid task_5119bcacf5d444a8a5cdadb102f190f7 --phase current`。
- [x] STORY-VOLUME-004-ROUTE: 第四卷公共节律路线 scaffold。
  - Trace: .pm/tasks/task_89a34f7f4edd400da91a41aee42b280d.yaml
  - 范围：`TL-159` 到 `TL-198` / `CH-159` 到 `CH-198`。
  - 产物文件：`site/story/outline/volume-04-route-scaffold.md`、`site/story/research/volume-04-route-research.md`、`site/story/chapter-cards/volume-04-route-cards.md`、`site/story/draft/volume-04-writing-positioning.md`。
  - 验收命令：`git diff --check && ./scripts/doc-governance-check.sh`。
- [x] STORY-FOLLOWUP-003-ROUTE: 第三卷后续路线 scaffold、章节卡、正文试写和 route closeout。
  - Trace: .pm/tasks/task_5c02287b562249b0b7947313822392bc.yaml
  - 范围：`TL-119` 到 `TL-158` / `CH-119` 到 `CH-158`。
  - 产物文件：`site/story/outline/volume-03-followup-route-scaffold.md`、`site/story/research/volume-03-followup-route-research.md`、`site/story/chapter-cards/volume-03-followup-*`、`site/story/draft/volume-03-followup-*`、`site/story/reviews/editorial-notes.md`。
  - 验收命令：`git diff --check && ./scripts/doc-governance-check.sh`。

## 状态

- 更新日期：2026-06-12
- 当前状态：第四卷公共节律路线已进入正式章级卡拆分；第一章 `CH-159` 到 `CH-166` 已从总章节卡草案拆为正式章节卡。
- 当前边界：第四卷第一章收在公共节律页、空白优先级栏、共享散热小刻度、温室页角、低带宽窄窗、地球端接入贴和未排序窗口夹上；不是系统成熟、资源竞争、优先级排名、第四卷发布会、温室升级或稳定上线证明。
