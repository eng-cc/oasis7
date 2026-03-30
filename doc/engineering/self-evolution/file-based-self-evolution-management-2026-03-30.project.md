# oasis7：自我进化文件化项目管理（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-074 (PRD-ENGINEERING-SE-001/004/006) [test_tier_required]: 建立 `self-evolution` 专题 `prd/design/project`，并同步回写 engineering 根入口、主项目、索引与 devlog。
- [ ] TASK-ENGINEERING-075 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 建立 `.pm/` 目录骨架、role registry、task registry 与模板文件，明确首批 7 角色及扩容规则。
- [ ] TASK-ENGINEERING-076 (PRD-ENGINEERING-SE-002/003/004) [test_tier_required]: 落地 signal inbox 与 `promote-signal` 基础链路，优先覆盖 `qa_engineer` 与 `liveops_community`。
- [ ] TASK-ENGINEERING-077 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 落地 role memory / backlog 文件格式、`superseded` 生命周期与 lint 规则。
- [ ] TASK-ENGINEERING-078 (PRD-ENGINEERING-SE-001/004) [test_tier_required]: 落地 `stage/gate` 文件与 `stage-report` 汇总入口，为制作人阶段评审提供 canonical 输入。
- [ ] TASK-ENGINEERING-079 (PRD-ENGINEERING-SE-002/003/006) [test_tier_required] + [test_tier_full]: 建立 `devlog -> signal -> memory/task -> stage report` required-tier 验证链路，并评估角色扩容与多 worktree 并发回归。
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-SE-005/006) [test_tier_required]: 建立长期 memory 自建专题 `prd/design/project`，将 role memory schema、promotion 与 superseded 规则从总专题中单独冻结。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `doc/devlog/YYYY-MM-DD.md`
- `testing-manual.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- 仓库根目录 `.pm/`（待建）
- `scripts/pm/*.sh`（待建）

## 状态
- 更新日期: 2026-03-30
- 当前阶段: planned
- 当前任务: `TASK-ENGINEERING-075`
- 阻塞项:
  - 当前仓库尚无 `.pm/` 运行态目录与统一 task/memory schema。
  - signal promotion、role registry 和 stage report 仍未落地脚本化入口。
- 最新完成:
  - `TASK-ENGINEERING-074`：已建立 `self-evolution` 专题三件套，并将文件化项目管理目标态正式挂入 engineering 根入口、主项目、索引与 devlog。
  - `TASK-ENGINEERING-080`：已将长期 memory 从总专题里拆成独立子专题，单独冻结 active/superseded schema、promotion 规则和 memory 脚本契约。
- 下一步:
  - 先执行 `TASK-ENGINEERING-075`，建立 `.pm/` 骨架和模板；
  - 再执行 `TASK-ENGINEERING-076/077`，优先把 `qa_engineer` / `liveops_community` 的信号和 backlog 跑通；
  - 最后执行 `TASK-ENGINEERING-078/079`，将 stage/gate 汇总和 required-tier 验证纳入治理闭环。
