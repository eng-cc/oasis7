# `readme/governance` 热点路径治理（2026-04-18）设计文档

- 对应需求文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.project.md`

审计轮次: 1

## 1. 背景与目标
- `doc/readme/README.md` 与 `prd.index.md` 已完成模块级入口减重，但 `doc/readme/governance/` 内部仍没有子域 landing page。
- 当前目标不是减少 governance 文件总量，而是先把“进入热点路径后的第一步”变得可预测。
- 本设计只处理 `governance/` 的入口收口，不重写现有 Moltbook / 小红书 / reward / release communication / root positioning 内容本身。

## 2. 设计原则
- 入口页只回答“先看哪里”，不复制完整文件清单。
- 优先保留现有 canonical 文档职责:
  - `doc/readme/README.md` 仍是 `readme` 模块级 landing page。
  - `doc/readme/prd.index.md` 仍是完整文件级索引。
  - `doc/readme/governance/README.md` 只承担热点子域 landing page。
- 先按读者问题分簇，再给每个簇推荐 1 到 3 个代表入口，避免 README 重新长表化。

## 3. 信息架构
- `doc/readme/governance/README.md` 首屏固定为:
  - 从这里开始
  - 入口分工
  - 密度快照
  - 首读主题簇
  - 定向检索边界
  - 维护约定
- 推荐主题簇:
  - `governance-controls`: consistency audit、link check、quarterly review、root status alignment
  - `release-communication`: release brief / template / announcement draft / template
  - `moltbook-liveops`: promotion plan、post drafts、liveops runbook 与 repair follow-up
  - `limited-preview-and-reward`: invite pack、reward pack、ledger、round closure 与 reward scan
  - `xiaohongshu-and-promoter`: 小红书 liveops、post pack、轮播包与博主/公众号激励
  - `public-positioning-and-world-rules`: resource model layering、world rules consolidation 与公开主定位相关专题

## 4. 上游回链策略
- `doc/readme/README.md`
  - 在“从这里开始”和“入口分工”中补 `governance/README.md`
  - 在热点子域导航里把 `governance/README.md` 设为 `governance/` 默认入口
- `doc/readme/prd.index.md`
  - 在首读分流里补“想先进入 governance 热点子域”
  - 在活跃补充文档区加入 `governance/README.md`
- `doc/readme/project.md`
  - 追加本轮已完成任务行，并在状态区把最新完成更新到 `readme-governance-path-governance`
- `doc/engineering/*`
  - 将本专题登记为 `PRD-ENGINEERING-030`
  - 明确 `governance/` 已完成第五条 follow-up，下一步转季度复核

## 5. 非目标与边界
- 不在本批更新 `doc/readme/governance/*.md` 的正文内容。
- 不在本批裁定任何 governance 专题“废弃/合并/删除”。
- 不在本批复制 `doc/readme/prd.index.md` 的完整长表。

## 6. 验证
- 人工检查:
  - 从 `doc/readme/README.md` 能在一跳内进入 `governance/README.md`
  - 从 `governance/README.md` 能在两跳内进入 `readme-moltbook-liveops-runbook-2026-03-21.prd.md`、`readme-xiaohongshu-liveops-runbook-2026-03-23.md`、`readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- 工具检查:
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`

## 7. 风险
- 若入口聚类失真，读者仍会回退到 `prd.index.md` 长表直查。
- 若后续 `governance/` 新增热点主题但 README 不更新，入口会失去可信度。
- 若把 README 写成“完整清单缩写版”，会重新制造第二份噪音索引。
