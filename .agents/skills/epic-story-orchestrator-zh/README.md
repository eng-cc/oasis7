# epic-story-orchestrator-zh

中文史诗背景故事编排 skill（repo-owned）。

## 目标

将长篇叙事拆解为可审计、可回写、可持续维护的资产：

- 世界观真值（world-bible）
- 人物库（character-registry）
- 时间线（timeline）
- 情节分支（plot-branches）
- 当前游戏玩法真值绑定（gameplay-canon-binding）
- 章节卡（chapter-cards）
- 一致性报告（consistency-report）
- 变更日志（canon-log）

## 当前游戏流水线边界

这个 skill 可以作为 oasis7 当前游戏背景故事生产流水线的 authoring surface，但不能单独决定游戏 canon。

当前游戏 lore 必须绑定 `PRD-GAME-012/013/014/015` 中的 trust/capability、物理尺度、间接控制 control-feeling 与 mature-world 小玩家成长线真值。章节卡和草稿必须明确 `player_leverage`、`world_change_due_to_player` 与 `release_claim_boundary`，不得把 ambient world activity 写成玩家成长，也不得借背景故事扩大 `limited playable technical preview` 的对外承诺。

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
