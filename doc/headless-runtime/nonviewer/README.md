# headless-runtime 旧 `nonviewer` 专题追溯

两组历史命名 `nonviewer-*` hardening 三件套已在完成语义回填后退役。当前鉴权、防重放、长稳内存边界和冷归档合同以父目录的 `doc/headless-runtime/prd.md`、`design.md` 与 GitHub task issue evidence comments 为准；实现细节继续由对应代码、测试及 `doc/world-runtime/runtime/runtime-storage-footprint-governance.prd.md` 承载。

## 从这里开始

| 问题 | 读取入口 | 权威边界 |
| --- | --- | --- |
| 鉴权 proof、重放防护或 live 控制协议如何收口？ | `doc/headless-runtime/prd.md`、`doc/headless-runtime/design.md` | 当前模块 authority 定义协议硬化；不定义 viewer 视觉行为或共识经济规则。 |
| 长稳内存边界、CAS 冷归档或事故追溯如何收口？ | `doc/headless-runtime/prd.md`、`doc/world-runtime/runtime/runtime-storage-footprint-governance.prd.md` | headless 根 authority 定义运行约束，world-runtime 定义 storage/checkpoint/replay/GC 合同。 |
| 需要定位完成状态？ | `doc/headless-runtime/prd.md` | 当前状态与迁移收口记录只在模块项目页维护。 |
| 需要生命周期、鉴权自检、事故模板或 release-gate 对接？ | 父目录 `checklists/`、`templates/` | 操作步骤和模板仍在这些专用目录，不在本路由页复制。 |

## 维护边界

- 本页只解释旧命名与退役路由，不重述技术规格、任务状态或历史审计结论。
- 历史专题内容继续从 Git history 与 GitHub task issue evidence comments 追溯；不得把旧 split-crate 路径恢复为当前实现 authority。
