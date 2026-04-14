# world-simulator 文档索引

审计轮次: 9

## 从这里开始
- 想先回答 world-simulator 在做什么、覆盖哪些边界：`doc/world-simulator/prd.md`
- 想看当前执行任务、负责人、测试层级与最新完成态：`doc/world-simulator/project.md`
- 想执行 Viewer、走 Web 闭环或查操作步骤：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 想按子域或文件名继续下钻，而不是从长表里逐行找：`doc/world-simulator/prd.index.md`
- 想给仓库外读者分享公开可读手册：`site/doc/cn/viewer-manual.html` / `site/doc/en/viewer-manual.html`

## 入口
- PRD: `doc/world-simulator/prd.md`
- 设计总览: `doc/world-simulator/design.md`
- 标准执行入口: `doc/world-simulator/project.md`
- 文件级索引: `doc/world-simulator/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：告诉读者先去哪个权威入口，不重复长表索引内容。
- `prd.md` 是模块权威规格入口，适合先理解 world-simulator 的范围、主线能力与跨模块接口。
- `project.md` 是执行台账，适合确认当前活跃任务、测试层级、阻断与最新完成项。
- `prd.index.md` 是定向检索索引，适合已经知道主题后按文件名查找，不是新读者的首读入口。
- `viewer/viewer-manual.manual.md` 是仓库内 canonical 操作手册；静态 `site/doc/**/viewer-manual.html` 仅作为公开只读镜像，不反向替代仓库权威源。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接罗列近期专题长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`viewer-manual.manual.md` 和 `prd.index.md`。
- 审计留痕、历史背景或只在特定问题下才需要的专题，改为从 `prd.index.md` 的定向检索入口进入。

## 模块职责
- 维护模拟世界主入口、viewer / launcher / kernel / scenario / llm / m4 六大主题口径。
- 汇总 Web 闭环、启动器可用性、场景初始化与规则执行相关专题。
- 承接 world-simulator 与 runtime / viewer / testing 的跨模块体验收口。

## 热点子域导航（2026-04-10 快照）
- `viewer/`（296）：Viewer、Web 闭环、`software_safe`、2D/3D 与操作手册；先看 `viewer-manual.manual.md`，再去 `prd.index.md` 定向找专题。
- `launcher/`（81）：启动器、控制面、转账、explorer 与自引导体验。
- `llm/`（54）：provider、loopback、本地桥接、体验等价和 direct-connect 相关口径。
- `kernel/`（36）：规则桥接、WASM 执行、资源与 runtime 约束。
- `m4/`（36）：M4 方案与配套设计。
- `scenario/`（30）：场景初始化、配置与模板。
- `prd/`（9）：验收模板、质量趋势与补充附件。

## 高密度提示
- `doc/world-simulator/` 当前共有 547 份文件；这一层入口不再尝试把热点专题直接摊平展示。
- 需要完整活跃专题清单时，进入 `doc/world-simulator/prd.index.md`；需要历史回溯时，再按具体任务或专题路径进入。

## 根目录 legacy
- `doc/world-simulator.prd.md`
- `doc/world-simulator.project.md`

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 模块行为、Web 闭环或启动器体验口径变化时，优先更新 `doc/world-simulator/prd.md` / `doc/world-simulator/project.md`；手册步骤改动时，先更新仓库内 `viewer-manual.manual.md`，再评估是否同步静态镜像。
