# Planning Self-Checklist

适用场景：

- 更新 `project.md` 后、开始实现前
- 准备把复杂任务交接给其他角色前
- 准备宣称“计划已清楚，可以开工”前

## Required Checks

- [ ] 已补 `File Structure / Affected Paths`
  - 至少列出：预计改动路径、只读依赖路径、验证入口、需要回写的正式文档路径
- [ ] 没有残留占位词
  - 重点扫描：`TBD`、`TODO`、`placeholder`、`待补`、`后续再看`
- [ ] 每条需求 / 验收点都有对应任务或验证面
  - 不能只写目标，不写落地 task、验证命令或预期结果
- [ ] 命名保持一致
  - `PRD-ID`、task slug、`.pm` task、分支 / worktree 名、文档标题、关键路径引用前后一致
- [ ] 原子步骤足够细
  - 接手人不需要额外口头补充，就能按步骤执行、验证、回写

## Optional Checks

- [ ] 已标明哪些路径是只读依赖，避免误改
- [ ] 已标明回归影响范围，避免只测 happy path
- [ ] 已标明需要哪一个 owner / reviewer 复核
