# `testing/evidence` 热点路径治理（2026-04-17）设计文档

- 对应需求文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. 背景与目标
- `doc/testing/README.md` 与 `prd.index.md` 已完成模块级入口减重，但 `doc/testing/evidence/` 内部仍没有子域 landing page。
- 当前目标不是减少 `evidence/` 文件总量，而是先把“进入热点路径后的第一步”变得可预测。
- 本设计只处理 `evidence/` 的入口收口，不重写 evidence 正文内容本身。

## 2. 设计原则
- 入口页只回答“先看哪里”，不复制完整文件清单。
- 优先保留现有 canonical 文档职责:
  - `testing/prd.index.md` 仍是完整文件级索引。
  - `testing/README.md` 仍是模块级 landing page。
  - `evidence/README.md` 只承担热点子域 landing page。
- 先按读者问题分簇，再给每个簇推荐 1 到 3 个代表入口，避免 README 重新长表化。

## 3. 信息架构
- `doc/testing/evidence/README.md` 首屏固定为:
  - 从这里开始
  - 入口分工
  - 密度快照
  - 首读主题簇
  - 定向检索边界
  - 维护约定
- 推荐主题簇:
  - `release-gate`: release evidence bundle、candidate gate、trust gate
  - `hosted-world-and-web`: hosted world browser/auth/abuse/web surface 与主链 web validation
  - `p2p-and-shared-network`: triad snapshot、mixed topology、shared devnet、upgrade 与 rollout follow-up
  - `governance-drill`: clone/live world drill、finality、foundation ops 等治理演练留痕
  - `claim-and-audit`: claim abuse/restricted grant matrix、token allocation audit、quality baseline
  - `targeted-validation`: provider recertification、pure-api parity、software-safe web entry、headless smoke、launcher UX

## 4. 上游回链策略
- `doc/testing/README.md`
  - 在“从这里开始”和“入口分工”中补 `evidence/README.md`
  - 在热点子域导航里把 `evidence/README.md` 设为 `evidence/` 默认入口
- `doc/testing/prd.index.md`
  - 在首读分流里补“想先进入 evidence 热点子域”
  - 在活跃补充文档区加入 `evidence/README.md`
- `doc/engineering/*`
  - 将本专题登记为 `PRD-ENGINEERING-029`
  - 明确 `testing/evidence` 已完成第四条 follow-up，下一步转季度复核

## 5. 非目标与边界
- 不在本批更新 evidence 文件正文。
- 不在本批裁定任何 evidence 文件“废弃/合并/删除”。
- 不在本批复制 `testing/prd.index.md` 的完整长表。

## 6. 验证
- 人工检查:
  - 从 `doc/testing/README.md` 能在一跳内进入 `evidence/README.md`
  - 从 `evidence/README.md` 能在两跳内进入 `release-evidence-bundle-task-game-018-2026-03-10.md`、`hosted-world-browser-auth-surface-2026-03-26.md`、`p2p-real-env-triad-snapshot-2026-04-07.md`
- 工具检查:
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`

## 7. 风险
- 若入口聚类失真，读者仍会回退到目录平铺或 `prd.index.md` 直查。
- 若后续 `evidence/` 新增热点主题但 README 不更新，入口会失去可信度。
- 若把 README 写成“完整清单缩写版”，会重新制造第二份噪音索引。
