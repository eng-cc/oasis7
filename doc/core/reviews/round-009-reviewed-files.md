# ROUND-009 文档消费入口与手册语义清单

审计轮次: 9

## 清单状态
- 当前 focused scope 数: 23
- 当前已完成首轮判定对象数: 23
- 当前状态: `completed`

## 字段说明
| 字段 | 说明 |
| --- | --- |
| 文档路径 | 被纳入 ROUND-009 的 focused scope 对象 |
| 当前角色 | 当前承担的消费/治理角色 |
| 关注点 | 本轮主要判断的问题 |
| 建议动作 | `keep` / `migrate` / `split` / `defer` |
| 优先级 | `P0` / `P1` / `P2` |
| owner role | 牵头角色 |
| 当前状态 | `scoped` / `issue_open` / `aligned` / `deferred` |
| 问题编号 | 对应 `I9-*` |
| 备注 | 当前已知事实或后续触发条件 |

## 汇总
| 范围 | 数量 | 状态 |
| --- | --- | --- |
| focused scope 文档/页面总数 | 23 | completed |

## 明细
| 文档路径 | 当前角色 | 关注点 | 建议动作 | 优先级 | owner role | 当前状态 | 问题编号 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `README.md` | repo 对外入口 | 是否需要补角色/任务型阅读入口 | split | P0 | `producer_system_designer` | aligned | I9-001 | 已新增“从这里开始”矩阵，按预览/验证/开发三类目标分流 |
| `doc/README.md` | 工程总导航 | 是否继续仅按模块组织，还是补消费层入口 | split | P0 | `producer_system_designer` | aligned | I9-001 | 已新增“按目标进入”矩阵，把开发/验证/追溯路径显式化 |
| `testing-manual.md` | 系统总手册 | 是否继续作为总手册权威源 | keep | P0 | `qa_engineer` | aligned | I9-002 | 已保留为总手册，并把 Web UI 执行步骤下沉到 `web-ui-agent-browser-closure-manual.manual.md` |
| `doc/core/README.md` | core 模块索引 | 是否补 ROUND-009 入口与治理说明 | keep | P2 | `producer_system_designer` | aligned | none | QA 复核认为当前 README 已承担 round 可达性，无需为本轮额外扩写 |
| `doc/engineering/README.md` | engineering 模块索引 | 是否需要记录 ROUND-009 规范挂靠 | keep | P2 | `producer_system_designer` | aligned | none | QA 复核认为 engineering 挂靠已由 `project.md` 与 ROUND 台账承担，README 无新增阻断 |
| `doc/game/README.md` | game 模块索引 | 入口是否对新读者足够友好 | defer | P2 | `producer_system_designer` | deferred | I9-001 | 抽样未见高频消费失焦，但低于本轮热点优先级，延期到后续入口轮 |
| `doc/headless-runtime/README.md` | headless-runtime 模块索引 | 入口可消费性抽样 | defer | P2 | `producer_system_designer` | deferred | I9-001 | 当前命名迁移说明仍可消费，本轮不额外扩容入口 |
| `doc/p2p/README.md` | p2p 模块索引 | 入口可消费性抽样 | defer | P2 | `producer_system_designer` | deferred | I9-001 | 当前主题目录与近期专题可达，入口增强延期到后续专轮 |
| `doc/playability_test_result/README.md` | 证据模块索引 | 是否需新增消费层说明 | defer | P2 | `qa_engineer` | deferred | I9-001 | 证据模块当前主要服务 QA/追溯使用者，本轮不扩写面向新读者的分流层 |
| `doc/readme/README.md` | readme 模块索引 | 模块职责是否混合规范/素材/执行包 | split | P0 | `liveops_community` | aligned | I9-003 | 已显式拆分 `canonical / runbook / material / execution_log`，模块入口边界已可消费 |
| `doc/scripts/README.md` | scripts 模块索引 | 入口可消费性抽样 | defer | P2 | `producer_system_designer` | deferred | I9-001 | 当前已清楚说明 worktree/harness/landing 入口，本轮不再扩写读者层 landing |
| `doc/site/README.md` | site 模块索引 | 是否与静态 docs hub 形成清晰映射 | keep | P1 | `producer_system_designer` | aligned | I9-005 | 已明确 `doc/site/README.md` 只负责模块治理入口映射，公开 docs hub 与静态手册镜像继续由 `site/doc/**` 承担 |
| `doc/testing/README.md` | testing 模块索引 | 手册总入口与分册关系是否足够清楚 | keep | P1 | `qa_engineer` | aligned | I9-002 | QA 复核确认当前总手册 + `manual/` 分册关系已清楚，无新增阻断 |
| `doc/world-runtime/README.md` | world-runtime 模块索引 | 高体量模块入口可消费性抽样 | defer | P2 | `producer_system_designer` | deferred | I9-005 | 当前根入口可达但非本轮高频热点；延期到后续高体量索引专轮 |
| `doc/world-simulator/README.md` | world-simulator 模块索引 | 高体量模块入口与专项手册关系是否清楚 | split | P1 | `viewer_engineer` | aligned | I9-005 | 已补“从这里开始”分流，并明确 README / `prd.index.md` / canonical 手册 / 公开静态镜像边界 |
| `doc/ui_review_result/README.md` | 活跃样本目录说明 | 是否继续作为标准模块外例外目录存在 | split | P1 | `viewer_engineer` | aligned | I9-004 | 已明确其为活跃评审样本池，补齐根级例外目录的进入/退出条件，不再伪装成正式模块 |
| `doc/readme/prd.index.md` | readme 文件级索引 | 规范文档与素材包并列导致索引语义混杂 | split | P0 | `liveops_community` | aligned | I9-003 | 已将索引拆成 `canonical` 与 `material/execution/SOP` 两层，并标注使用边界 |
| `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` | Web UI 分册手册 | 是否应迁移为 `*.manual.md` | migrate | P0 | `qa_engineer` | aligned | I9-002 | 已新增 `web-ui-agent-browser-closure-manual.manual.md` 承接操作步骤；PRD 保留需求/验收权威源 |
| `doc/world-simulator/viewer/viewer-manual.md` | Viewer 高频手册 | 是否保留 legacy 命名或迁移为 `*.manual.md` | migrate | P0 | `viewer_engineer` | aligned | I9-002 | 已新增 `viewer-manual.manual.md` 作为 canonical 手册，旧路径降级为兼容入口 |
| `site/doc/cn/index.html` | 中文公开 docs hub 入口 | 是否需要与 repo 入口共享消费层分流 | keep | P1 | `liveops_community` | aligned | I9-001/I9-005 | 已新增“按目标开始”区块，与 repo 入口共享预览/验证/开发三类分流 |
| `site/doc/en/index.html` | 英文公开 docs hub 入口 | 是否需要与中文入口保持同一消费分层 | keep | P1 | `liveops_community` | aligned | I9-001/I9-005 | 已与中文页同步新增 “Choose by Goal” 分流区块 |
| `site/doc/cn/viewer-manual.html` | 中文静态手册镜像 | 是否需随仓库手册迁移一起改名/改链 | defer | P1 | `viewer_engineer` | deferred | I9-005 | 仓库 README 已明确其为公开只读 mirror；本轮不改镜像文件名与正文结构 |
| `site/doc/en/viewer-manual.html` | 英文静态手册镜像 | 是否需随仓库手册迁移一起改名/改链 | defer | P1 | `viewer_engineer` | deferred | I9-005 | 仓库 README 已明确其为公开只读 mirror；本轮不改镜像文件名与正文结构 |
