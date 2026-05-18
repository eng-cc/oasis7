# 文档存量维护成本治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] doc-corpus-maintenance-governance (PRD-ENGINEERING-025) [test_tier_required]: 建立专题 `prd/design/project`、新增 `scripts/doc-inventory-report.sh`、回写 engineering 主入口与 `doc-surface-area-governance` handoff，冻结从“阅读面噪音”转向“存量维护成本”的阶段判断。 Trace: .pm/tasks/task_851d3d1452534a2c83355317ae385ade.yaml
- [x] devlog-history-compaction-followup (PRD-ENGINEERING-025/026) [test_tier_required]: 作为第一条 follow-up，建立 `devlog-history-compaction` 专题并新增 `doc/devlog/README.md`，把 `doc/devlog` 从“历史归档声明”收口到按月导航的 canonical archive 入口。 Trace: .pm/tasks/task_caaa7c575ec845dc9c0756c9e92d24f7.yaml
- [x] world-simulator-viewer-path-followup (PRD-ENGINEERING-025/027) [test_tier_required]: 作为第二条 follow-up，建立 `world-simulator-viewer-path-governance` 专题并新增 `doc/world-simulator/viewer/README.md`，把 `world-simulator/viewer` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_7d222c2f13454b23889baad383fbdf7e.yaml
- [x] p2p-node-path-followup (PRD-ENGINEERING-025/028) [test_tier_required]: 作为第三条 follow-up，建立 `p2p-node-path-governance` 专题并新增 `doc/p2p/node/README.md`，把 `p2p/node` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_533ac29c20a84ee8a5e6914839ad0761.yaml
- [x] testing-evidence-path-followup (PRD-ENGINEERING-025/029) [test_tier_required]: 作为第四条 follow-up，建立 `testing-evidence-path-governance` 专题并新增 `doc/testing/evidence/README.md`，把 `testing/evidence` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_38707b4060b54e5e8b8ebcdb8d18a602.yaml
- [x] readme-governance-path-followup (PRD-ENGINEERING-025/030) [test_tier_required]: 作为第五条 follow-up，建立 `readme-governance-path-governance` 专题并新增 `doc/readme/governance/README.md`，把 `readme/governance` 从“热点路径内无首读入口”收口到按治理控制 / release communication / Moltbook / limited preview 与 reward / 小红书 / 公开定位分流的 canonical 子域入口。 Trace: .pm/tasks/task_d37f636846fa44449988240af8630454.yaml
- [x] quarterly-doc-inventory-review-followup (PRD-ENGINEERING-025) [test_tier_required]: 执行 2026-04-24 季度复核，重新运行 `bash ./scripts/doc-inventory-report.sh` 固定当前 `doc/` 体量快照，并把下一条 follow-up 从“待季度复核”改判为“near-limit active project docs 拆分优先”，避免继续停留在泛化的 review placeholder。 Trace: .pm/tasks/task_1104ff9bb9114aaa85c445785950a939.yaml
- [x] world-simulator-doc-redundancy-reduction-followup (PRD-ENGINEERING-015/025) [test_tier_required]: 作为已完成 `world-simulator/viewer` 路径治理后的 aftercare，收口 `doc/world-simulator/viewer/viewer-manual.md` 的 legacy 正文残留，并把 `doc/world-simulator/prd.index.md` 压回“精确检索优先”的文件级索引角色，避免与模块 `README.md` 再做一套 landing。 Trace: .pm/tasks/task_aaebf3a722a847b9b2e8d23695ea71c0.yaml
- [x] viewer-manual-canonical-source-cleanup-followup (PRD-ENGINEERING-025) [test_tier_required]: 继续对当前真值/当前基线类正式文档执行 aftercare，把 `world-simulator/site github-pages` 相关正式文档中的 `viewer-manual.md` 基线路径统一改回 `viewer-manual.manual.md`，避免 legacy redirect 再次被误读成 canonical source。 Trace: .pm/tasks/task_010e133e25f5411daa05bbbf80ff3727.yaml
- [x] viewer-manual-sync-contract-refresh-followup (PRD-ENGINEERING-025) [test_tier_required]: 刷新 `scripts/site-manual-sync-check.sh` 的 source/manual 与 HTML mirror 校验契约，使其重新匹配当前 `render_mode=viewer&test_api=1` 基线，避免镜像同步门禁继续盯过时命令字符串。 Trace: .pm/tasks/task_03cd617323c840d29a36b5bfa91792ed.yaml

## 2026-04-24 季度复核快照
- `bash ./scripts/doc-inventory-report.sh` 当前快照：
  - `doc/` Markdown 总量 `1764`
  - `doc/devlog` 文件数 `57`
  - 最大 Markdown / devlog 文件仍为 `doc/devlog/2026-02-16.md`，`3288` 行
  - 模块体量前三：`world-simulator=553`、`p2p=270`、`testing=180`
  - 热点子目录前三：`doc/world-simulator/viewer=297`、`doc/readme/governance=98`、`doc/world-simulator/launcher=86`
  - near-limit active docs：`doc/world-simulator/project.md=1000`、`doc/readme/project.md=978`、`doc/core/reviews/round-006-reviewed-files.md=932`、`doc/core/reviews/round-007-reviewed-files.md=906`
- 复核结论：
  - 首批路径级治理入口已全部落地，但热点体量仍在增长；当前更急的风险已从“缺 landing page”转成“活跃 project/review 文档逼近或触达长度门禁”。
  - 因此本轮不再把下一步写成泛化的“季度复核后再看”，也不继续在当前 PR 横向扩 `ci/longrun/templates`、`gap` 或 `production` 新路径治理。
  - 下一条正式 follow-up 应优先拆分 near-limit active project docs，先处理 `doc/world-simulator/project.md` 与 `doc/readme/project.md`，再视结果判断是否需要把 `core/reviews` 或 `world-simulator/launcher` 升级为下一轮专题。

说明:
`doc/devlog` 历史压缩、`world-simulator/viewer`、`p2p/node`、`testing/evidence` 与 `readme/governance` 路径级治理都已完成首批入口收口。季度复核也已完成当前快照重算；后续若要继续扩 `core/reviews`、`world-simulator/launcher`、`ci/longrun/templates`、`gap` 或 `production` 等 follow-up，仍需至少各自独立创建 `.pm` task；默认仍建议独立 worktree，除非用户明确要求复用当前 PR/工作树。
2026-05-18 补充：已对 `world-simulator` 已治理路径执行一轮重复入口 aftercare，清掉 `viewer-manual.md` 的 legacy 正文残留，并将 `prd.index.md` 收回 index-first 角色；该动作不改变季度复核已冻结的“near-limit active project docs 拆分优先”下一步顺序。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md`
- `scripts/doc-governance-check.sh`
- `scripts/doc-inventory-report.sh`

## 状态
- 当前阶段: M2 已完成
- 阶段说明: formalize + report + five path follow-ups + quarterly review closed
- 阻塞项: 无
- 最近更新: 2026-05-18
- 后续动作: 入口减重专题 `PRD-ENGINEERING-024` 与首批五条路径级 follow-up 已完成；`world-simulator` 既有路径也已完成一轮重复入口 aftercare。下一条正式 follow-up 仍然不是新的路径扩张，而是 near-limit active project docs 拆分优先：先处理 `doc/world-simulator/project.md` 与 `doc/readme/project.md`，随后再评估 `core/reviews` 与 `world-simulator/launcher` 是否需要独立专题。
