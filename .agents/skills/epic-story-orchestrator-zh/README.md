# epic-story-orchestrator-zh

中文史诗背景故事编排 skill（repo-owned）。

## 目标

将长篇叙事拆解为可审计、可回写、可持续维护的资产：

- 世界观真值（world-bible）
- 人物库（character-registry）
- 时间线（timeline）
- 情节分支（plot-branches）
- 章节卡（chapter-cards）
- 一致性报告（consistency-report）
- 变更日志（canon-log）

## 默认写回路径

`doc/game/lore/<story-slug>/`

## 本地验证

```bash
bash .agents/skills/epic-story-orchestrator-zh/tests/run_smoke.sh
```


## 严格校验

```bash
python3 .agents/skills/epic-story-orchestrator-zh/tests/validate_writeback.py
RUN_PM_LINT=1 bash .agents/skills/epic-story-orchestrator-zh/tests/run_smoke.sh
```
