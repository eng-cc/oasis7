# headless-runtime 活跃 hardening 专题

本目录仅收纳仍需当前读取的两组历史命名 `nonviewer-*` 专题。文件名保留是为了维持 PRD、实现与审计追溯连续性；当前模块身份、生命周期边界与执行状态仍以父目录的 `doc/headless-runtime/prd.md` 和 `doc/headless-runtime/project.md` 为准。

## 从这里开始

| 问题 | 读取入口 | 权威边界 |
| --- | --- | --- |
| 鉴权 proof、重放防护或 live 控制协议如何收口？ | `nonviewer-onchain-auth-protocol-hardening.prd.md` | 该 triplet 定义鉴权协议硬化；不定义 viewer 视觉行为或共识经济规则。 |
| 长稳内存边界、CAS 冷归档或事故追溯如何收口？ | `nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md` | 该 triplet 定义长稳归档硬化；不替代模块生命周期总边界或发布门禁。 |
| 需要定位对应设计或完成状态？ | `doc/headless-runtime/prd.index.md` | 文件级索引是两组 triplet 的精确配对入口。 |
| 需要生命周期、鉴权自检、事故模板或 release-gate 对接？ | 父目录 `checklists/`、`templates/` | 操作步骤和模板仍在这些专用目录，不在本路由页复制。 |

## 维护边界

- 本页只负责活跃专题的首次分流，不重述技术规格、任务状态或历史审计结论。
- 新增或退役本目录中的当前专题时，同一改动需更新本页和 `doc/headless-runtime/prd.index.md`；历史证据继续按父目录 README 所述保留在 `doc/core/reviews/` 与 GitHub task evidence 中。
