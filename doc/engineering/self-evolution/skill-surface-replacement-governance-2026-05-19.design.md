# oasis7：Skill Surface 替换治理（2026-05-19）设计

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`

审计轮次: 1

## 1. 设计定位

这份设计文档只回答两个问题：

1. 哪些本地 skill surface 仍应作为 repo-owned / scene-specific 资产保留。
2. 哪些 generic 或低耦合 surface 应被 replacement / retirement / defer 处理。

逐项 inventory 判定、验收口径和项目追踪已经在 PRD/project 中冻结，这里不再重复流水账。

## 2. 分桶设计

### 2.1 Keep

以下类型继续保留：

- repo-owned workflow surfaces
  - `agent-browser`
  - `bounded-brainstorming`
  - `repo-owned-workflow-router`
  - `tdd-test-writer`
- repo-owned or scenario-specific content surfaces
  - `prd`
  - `xiaohongshu`
  - `xiaohongshu-note-analyzer`
  - `gpt-image-2`
  - `humanizer-zh`

保留标准只有两个：

1. 与当前仓库 workflow / script / platform 强绑定
2. 明确承载本地场景专属能力，而不是通用方法论镜像

### 2.2 Retire

本轮已经退役的低耦合 surface：

- `documentation-writer`
- `frontend-ui-ux`
- `game-changing-features`

共同原因：

- 几乎没有 repo-specific 约束
- 与当前系统级指令、角色卡或 repo-owned workflow 表面重复
- 继续保留只会制造“存在即推荐”的误导

### 2.3 Defer

以下 generic game-skill mirror 先保持 deferred：

- `asset-optimization`
- `audio-systems`
- `game-architect`
- `game-design-theory`
- `gameplay-mechanics`
- `level-design`
- `memory-management`
- `monetization-systems`
- `optimization-performance`
- `particle-systems`
- `synchronization-algorithms`

defer 的含义不是继续推荐，而是：

- 这批 skill 的删除成本高于当前收益
- 需要先清真实引用面，再决定是否整体降为“上游跟踪清单”

## 3. Replacement Surface

- `documentation-writer` -> repo-native 文档规则 + `prd` + `humanizer-zh`
- `frontend-ui-ux` -> 系统级前端指令 + `agent-browser` + `gpt-image-2`
- `game-changing-features` -> `prd` + `game-design-theory` + `content-creation`
- `brainstorming` -> `bounded-brainstorming` + root workflow 的 bounded ideation rule
- `using-superpowers` -> `repo-owned-workflow-router` + root workflow phase order
- `writing-skills` -> repo-owned skill authoring surface、template、checklist 与 README entrypoint

## 4. 风险控制

- 被 retire 的 skill 若仍出现在角色卡或活跃文档，视为治理未收口
- defer 不等于继续推荐；后续如要再删，先清角色卡和活跃引用
- repo-owned workflow router 只能保留本地 phase-order 价值，不能回退成外部 bootstrap
- repo-owned skill authoring surface 只能补强本地 authoring discipline，不能再造第二套 workflow 真值

## 5. 使用方式

- 看正式 inventory 判定、验收与决策：读 `skill-surface-replacement-governance-2026-05-19.prd.md`
- 看当前任务与 follow-up：读 `skill-surface-replacement-governance-2026-05-19.project.md`
