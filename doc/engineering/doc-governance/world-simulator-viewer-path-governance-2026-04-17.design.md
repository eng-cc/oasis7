# `world-simulator/viewer` 热点路径治理（2026-04-17）设计文档

- 对应需求文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. 背景与目标
- `doc/world-simulator/README.md` 与 `prd.index.md` 已完成模块级入口减重，但 `doc/world-simulator/viewer/` 内部仍没有子域 landing page。
- 当前目标不是减少 Viewer 文件总量，而是先把“进入热点路径后的第一步”变得可预测。
- 本设计只处理 `viewer/` 的入口收口，不重写 Viewer 内容本身。

## 2. 设计原则
- 入口页只回答“先看哪里”，不复制完整文件清单。
- 优先保留现有 canonical 文档职责:
  - `viewer-manual.manual.md` 仍是操作手册。
  - `world-simulator/prd.index.md` 仍是完整文件级索引。
  - `viewer/README.md` 只承担热点子域 landing page。
- 先按读者问题分簇，再给每个簇推荐 1 到 3 个代表入口，避免 README 重新长表化。

## 3. 信息架构
- `doc/world-simulator/viewer/README.md` 首屏固定为:
  - 从这里开始
  - 入口分工
  - 密度快照
  - 首读主题簇
  - 定向检索边界
  - 维护约定
- 推荐主题簇:
  - `manual-and-operator`: 手册、镜像、脚本化操作闭环
  - `software-safe-web`: `software_safe`、Web 正式入口、fatal surfacing、semantic test API
  - `runtime-live-control`: runtime live 接管、event-driven、step/control 相关主题
  - `chat-and-panel`: chat、prompt presets、right panel、输入桥接
  - `release-and-visual`: gameplay release、commercial release、visual gate、QA 闭环
  - `3d-hold-and-visual-mode`: 3D hold、2D/3D 表现、visual-only 入口

## 4. 上游回链策略
- `doc/world-simulator/README.md`
  - 在“从这里开始”和“入口分工”中补 `viewer/README.md`
  - 在热点子域导航里把 `viewer/README.md` 设为 `viewer/` 默认入口
- `doc/world-simulator/prd.index.md`
  - 在首读分流里补“想先进入 Viewer 热点子域”
  - 在活跃补充文档区加入 `viewer/README.md`
- `doc/engineering/*`
  - 将本专题登记为 `PRD-ENGINEERING-027`
  - 明确 `viewer/` 已完成第二条 follow-up，下一步转 `p2p`

## 5. 非目标与边界
- 不在本批更新 `viewer-manual.manual.md` 的正文步骤。
- 不在本批裁定任何 Viewer 专题“废弃/合并/删除”。
- 不在本批复制 `world-simulator/prd.index.md` 的完整长表。

## 6. 验证
- 人工检查:
  - 从 `doc/world-simulator/README.md` 能在一跳内进入 `viewer/README.md`
  - 从 `viewer/README.md` 能在两跳内进入 `viewer-manual.manual.md`、`viewer-web-software-safe-mode-2026-03-16.prd.md`、`viewer-live-full-event-driven-phase10-2026-02-27.prd.md`
- 工具检查:
  - `bash scripts/doc-inventory-report.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`

## 7. 风险
- 若入口聚类失真，读者仍会回退到 `prd.index.md` 长表直查。
- 若后续 `viewer/` 新增热点主题但 README 不更新，入口会失去可信度。
- 若把 README 写成“完整清单缩写版”，会重新制造第二份噪音索引。
