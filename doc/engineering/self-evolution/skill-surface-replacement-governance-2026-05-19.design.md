# oasis7：Skill Surface 替换治理（2026-05-19）设计

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`

审计轮次: 1

## 1. 设计定位
本专题负责把当前 `.agents/skills/` 拆成两类：一类是 repo-owned 或明确场景专属的保留资产，另一类是低耦合或 generic mirror 的可替换表面。目标不是“把 skill 全删掉”，而是先让角色卡和仓库真值不再推荐错误的 skill。

## 2. 分层策略
### 2.1 Keep
- `agent-browser`
- `prd`
- `xiaohongshu`
- `xiaohongshu-note-analyzer`
- `gpt-image-2`
- `humanizer-zh`

这些 skill 要么带明确的 repo workflow 依赖，要么对应第三方平台/图像链路等专属能力，不适合作为本轮精简目标。

### 2.2 Retire
- `documentation-writer`
- `frontend-ui-ux`
- `game-changing-features`

共同特征：
- 方法论通用，几乎没有 repo-specific 约束。
- 与当前系统级前端/文档/执行指令重复，或者默认输出路径/交互节奏与仓库真值冲突。
- 引用面小，适合本轮直接删除并同步角色卡。

### 2.3 Defer
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
- `tdd-test-writer`

这些 skill 仍有方法论价值，但要么承载多个角色卡推荐，要么与当前工程验证实践有交叉，批量删除成本高于本轮收益，因此先 defer。

## 3. 本轮实现边界
### 3.1 删除文件面
- 删除 `documentation-writer`
- 删除 `frontend-ui-ux`
- 删除 `game-changing-features`

### 3.2 同步回写
- 更新 `.agents/roles/*.md` 中的推荐 skill
- 清理活跃文档中对被退役 skill 的显式命名
- 更新 engineering 根入口，让 skill rationalization 成为正式治理专题

## 4. Replacement Surface
- `documentation-writer` -> repo-native 文档规则 + `prd` + `humanizer-zh`
- `frontend-ui-ux` -> 系统级前端指令 + `agent-browser` + `gpt-image-2`
- `game-changing-features` -> `prd` + `game-design-theory` + `content-creation`

## 5. 风险控制
- 任何被 retire 的 skill 若仍在角色卡中出现，视为治理未收口。
- `defer` 不等于“继续推荐为主技能”；后续如需再收缩，应优先先改角色卡，再删文件。
- 历史 `doc/devlog` 中出现的旧 skill 名称保留为归档痕迹，不作为活跃引用清理对象。
