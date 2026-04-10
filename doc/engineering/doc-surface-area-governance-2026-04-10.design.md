# 文档体量治理与活跃阅读面收敛设计（2026-04-10）

- 对应需求文档: `doc/engineering/doc-surface-area-governance-2026-04-10.prd.md`
- 对应项目管理文档: `doc/engineering/doc-surface-area-governance-2026-04-10.project.md`

## 1. 设计定位
本设计回答的不是“文件应该先搬到哪里”，而是“哪些文件应该出现在默认阅读面”。  
当前仓库里最突出的问题，是活跃规格、审计留痕、历史归档和兼容跳转共用同一阅读面，导致入口面比目录树本身更先失控。

因此，这个专题先定义消费层分层规则，再决定是否需要后续路径治理。

## 2. 四层消费模型

### 2.1 活跃真值
定义：
- 当前模块和专题的正式目标、设计、执行状态与必要操作入口。

典型载体：
- `doc/README.md`
- `doc/<module>/README.md`
- `doc/<module>/prd.md`
- `doc/<module>/design.md`
- `doc/<module>/project.md`
- `doc/<module>/prd.index.md`
- 仍在 active 的专题 `*.prd.md/*.design.md/*.project.md/*.manual.md/*.runbook.md`

入口策略：
- 默认保留在主阅读面。
- 必须优先回答 `what / where / next / risk`。

### 2.2 审计留痕
定义：
- 证明某项治理、评审、验证、采证或批次收口曾经发生过的材料。

典型载体：
- `doc/core/reviews/*`
- `doc/**/governance/*`
- `doc/**/evidence/*`
- checklist / reviewed-files / audit-progress 等记录

入口策略：
- 保持可检索和可定向引用。
- 不直接进入 root README 或模块 README 的主入口列表。
- 如需暴露，只能以“更多证据 / 审计记录”形式从活跃真值跳转。

### 2.3 历史归档
定义：
- 已退出运行态真值、仅供回溯“当时发生了什么”的历史材料。

典型载体：
- `doc/devlog/*`
- 已收口但仍需留存的历史专题

入口策略：
- 不进入默认阅读面。
- 只通过任务回溯、决策追证或历史背景说明进入。
- 不得重新承担 `.pm` 或正式 PRD/project 的运行态 source of truth。

### 2.4 兼容跳转
定义：
- 为历史路径、外部引用或旧入口保留的最小兼容页。

典型载体：
- root legacy redirect
- 已迁移专题的软 redirect 说明页

入口策略：
- 只保留主入口链接、必要相关入口与最小声明。
- 不扩展成第二份正文。

## 3. 默认阅读面设计

### 3.1 root 级
`doc/README.md` 只负责：
- 根入口导航
- 按目标分流
- 模块矩阵
- 阅读顺序
- 兼容跳转说明

它不再承担：
- round review 导航
- 审计/证据目录展示
- 历史归档入口展示

### 3.2 模块级
`doc/<module>/README.md` 只负责：
- 模块“从这里开始”
- 模块特有高频专题
- 模块特有例外和公开镜像

它不再承担：
- repo-wide 结构规则重复说明
- 审计目录罗列
- 历史归档总表

### 3.3 索引级
`doc/<module>/prd.index.md` 只负责：
- 活跃专题三件套可达性
- 当前仍需默认阅读的高频 manual/runbook

它不负责：
- round reviewed-files 总表
- history dump
- 大批量 evidence 清单

## 4. 密度触发与优先级

### 4.1 触发条件
出现以下任一情况时，模块进入 `action_required`：
- 模块文档数超过 200；
- 单个子目录文档数超过 80；
- 模块 README 或默认专题列表已经无法在一次屏幕阅读内完成基本分流；
- 项目经理视角无法在 15 分钟内回答 `what / where / next / risk`。

### 4.2 当前优先级
按 2026-04-10 仓库快照，建议优先级为：
1. `world-simulator`
2. `p2p`
3. `testing`
4. `readme/core`

原因：
- `world-simulator` 和其 `viewer/` 子目录已经形成明显热点。
- `p2p` 与 `testing` 同时承担高密度专题和大量治理/证据材料。
- `readme/core` 决定全局入口感知，虽非最大目录，但一旦入口面失控，项目经理视角最先受影响。

## 5. 执行顺序
后续模块减重一律按这个顺序：
1. 统计体量与热点目录。
2. 识别哪些属于四层模型中的非活跃层。
3. 收紧 README / `prd.index.md` / 主入口默认暴露面。
4. 必要时补“更多证据 / 历史背景 / 审计记录”定向跳转。
5. 若入口收紧后仍混乱，再评估是否做路径迁移或专题重分层。

这个顺序故意把“目录迁移”放后面，避免第一步就制造大量断链与 allowlist 变更。

## 6. 与现有规范的关系
- `doc-structure-standard` 解决“文档该放哪、后缀该叫什么”。
- 本专题解决“哪些文档该留在默认阅读面、哪些只能按需进入”。

两者关系是：
- `doc-structure-standard` 决定建模方式；
- `doc-surface-area-governance` 决定消费层曝光度。

## 7. 边界与例外
- 某些 audit/evidence 文档如果仍是唯一操作入口，应先补等价的活跃 manual/runbook，再把原文降回审计层。
- 某些 root redirect 若仍被外部频繁引用，可以保留，但正文不得继续膨胀。
- 不因为“文件很多”就默认迁目录；只有当入口收紧后仍无法表达对象边界时，才进入路径级治理。
