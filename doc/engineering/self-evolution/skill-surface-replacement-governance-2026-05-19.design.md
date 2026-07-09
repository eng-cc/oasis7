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
  - `skills/xiaohongshu-note-analyzer` (non-default library reference)
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

### 2.3 Generic Game Mirror Follow-up Decision

2026-06-20 已收口原 `defer` 桶。治理结论不再是继续观察，而是按真实引用面和 repo-specific 增量拆成三类：

#### 2.3.1 Retired to upstream tracking

以下本地 skill surface 退役为上游跟踪清单，不再作为 `.agents/skills/` 本地可触发 skill：

- `asset-optimization`
- `audio-systems`
- `monetization-systems`

共同原因：

- 无当前角色卡强绑定
- 缺少 oasis7 专属资产/音频/商业化实现契约
- 入口与 supporting files 主要是通用方法论镜像，继续保留会制造“存在即推荐”的误导

未来若出现正式资产管线、音频系统或商业化/购买/合规 PRD，应由对应专业角色重新提出 repo-owned skill，而不是直接恢复旧镜像。

#### 2.3.2 Maintain with trigger narrowing

以下 skill 保留，但必须压缩入口并绑定专业角色/当前 repo truth：

- `game-design-theory`
- `synchronization-algorithms`

治理要求：

- 入口必须是短触发面，不保留大段通用教材
- 结论必须归属对应专业角色 slice
- generic reference 只能作为按需 supporting material，不能替代 oasis7 证据

#### 2.3.3 Preserve as domain-triggered non-default

以下 skill 保留为 domain-triggered non-default surface：

- `level-design`
- `particle-systems`

治理要求：

- 只在匹配场景通过 TPM routing 或专业 slice 触发
- 不能作为默认 workflow phase
- 未被 entrypoint 明确承接的 placeholder scripts/assets/old guides 应退役

原 deferred 桶中 `game-architect`、`gameplay-mechanics`、`memory-management`、`optimization-performance` 已有明确 workflow/professional-role 绑定或维护理由，后续按各自维护项处理，不再属于本轮 watch 决策。

## 3. Replacement Surface

- `documentation-writer` -> repo-native 文档规则 + `prd` + `humanizer-zh`
- `frontend-ui-ux` -> 系统级前端指令 + `agent-browser` + `gpt-image-2`
- `game-changing-features` -> `prd` + `game-design-theory` + `content-creation`
- `brainstorming` -> `bounded-brainstorming` + root workflow 的 bounded ideation rule
- `using-superpowers` -> `repo-owned-workflow-router` + root workflow phase order
- `writing-skills` -> repo-owned skill authoring surface、template、checklist 与 README entrypoint

## 4. 风险控制

- 被 retire 的 skill 若仍出现在角色卡或活跃文档，视为治理未收口
- watch/defer 不得长期停留；一旦真实引用面足够，应收口为 keep、maintain、preserve-non-default 或 retired-to-upstream-tracking
- repo-owned workflow router 只能保留本地 phase-order 价值，不能回退成外部 bootstrap
- repo-owned skill authoring surface 只能补强本地 authoring discipline，不能再造第二套 workflow 真值

## 5. 使用方式

- 看正式 inventory 判定、验收与决策：读 `skill-surface-replacement-governance-2026-05-19.prd.md`
- 看已完成任务与 follow-up 收口摘要：读 `skill-surface-replacement-governance-2026-05-19.project.md`
- 看当前默认/非默认 skill reachability：读 `doc/engineering/workflow/source-of-truth.md#12-specialist-skill-reachability` 与 `skills/README.md`
