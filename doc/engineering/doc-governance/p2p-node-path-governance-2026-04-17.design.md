# `p2p/node` 热点路径治理（2026-04-17）设计文档

- 对应需求文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. 背景与目标
- `doc/p2p/README.md` 与 `prd.index.md` 已完成模块级入口减重，但 `doc/p2p/node/` 内部仍没有子域 landing page。
- 当前目标不是减少 `node/` 文件总量，而是先把“进入热点路径后的第一步”变得可预测。
- 本设计只处理 `node/` 的入口收口，不重写节点专题内容本身。

## 2. 设计原则
- 入口页只回答“先看哪里”，不复制完整文件清单。
- 优先保留现有 canonical 文档职责:
  - `p2p/prd.index.md` 仍是完整文件级索引。
  - `p2p/README.md` 仍是模块级 landing page。
  - `node/README.md` 只承担热点子域 landing page。
- 先按读者问题分簇，再给每个簇推荐 1 到 3 个代表入口，避免 README 重新长表化。

## 3. 信息架构
- `doc/p2p/node/README.md` 首屏固定为:
  - 从这里开始
  - 入口分工
  - 密度快照
  - 首读主题簇
  - 定向检索边界
  - 维护约定
- 推荐主题簇:
  - `reward-and-asset`: 奖励、贡献分、资产、结算、执行验证
  - `replication-and-network`: libp2p 复制、net stack、signer binding、DistFS 复制闭环
  - `pos-time-and-control`: slot clock、subslot tick、control-plane alignment
  - `identity-bootstrap`: keypair/bootstrap 与节点身份初始化
  - `wasm-build`: wasm32 guard 与 builtin wasm fetch/compile fallback

## 4. 上游回链策略
- `doc/p2p/README.md`
  - 在“从这里开始”和“入口分工”中补 `node/README.md`
  - 在热点子域导航里把 `node/README.md` 设为 `node/` 默认入口
- `doc/p2p/prd.index.md`
  - 在首读分流里补“想先进入 node 热点子域”
  - 在活跃补充文档区加入 `node/README.md`
- `doc/engineering/*`
  - 将本专题登记为 `PRD-ENGINEERING-028`
  - 明确 `node/` 已完成第三条 follow-up，下一步转 `testing`

## 5. 非目标与边界
- 不在本批更新 `node/` 既有专题正文。
- 不在本批裁定任何 `node/` 专题“废弃/合并/删除”。
- 不在本批复制 `p2p/prd.index.md` 的完整长表。

## 6. 验证
- 人工检查:
  - 从 `doc/p2p/README.md` 能在一跳内进入 `node/README.md`
  - 从 `node/README.md` 能在两跳内进入 `node-contribution-points.prd.md`、`node-replication-libp2p-migration.prd.md`、`node-pos-slot-clock-real-time-2026-03-07.prd.md`
- 工具检查:
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`

## 7. 风险
- 若入口聚类失真，读者仍会回退到 `prd.index.md` 长表直查。
- 若后续 `node/` 新增热点主题但 README 不更新，入口会失去可信度。
- 若把 README 写成“完整清单缩写版”，会重新制造第二份噪音索引。
