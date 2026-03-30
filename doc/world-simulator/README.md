# world-simulator 文档索引

审计轮次: 9

## 从这里开始
- 想先理解模块边界、目标态与验收范围：`doc/world-simulator/prd.md`
- 想看当前执行任务、负责人、测试层级与完成态：`doc/world-simulator/project.md`
- 想直接启动 Viewer、走 Web 闭环或查操作步骤：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 想按专题文件名精确查找某个 Viewer / Launcher / Kernel / LLM 文档：`doc/world-simulator/prd.index.md`
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

## 模块职责
- 维护模拟世界主入口、viewer / launcher / kernel / scenario / llm / m4 六大主题口径。
- 汇总 Web 闭环、启动器可用性、场景初始化与规则执行相关专题。
- 承接 world-simulator 与 runtime / viewer / testing 的跨模块体验收口。

## 主题目录
- `viewer/`：Viewer 与 Web/交互/可视化相关设计。
- `llm/`：LLM 行为、Prompt、评估与稳定性相关设计。
- `launcher/`：启动器与链路编排相关设计。
- `scenario/`：场景定义、初始化与配置相关设计。
- `kernel/`：内核规则桥接与 WASM 规则执行相关设计。
- `m4/`：M4 专题文档。

## 近期专题
- `doc/world-simulator/viewer/viewer-web-closure-testing-policy.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`
- `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.prd.md`
- `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档已迁移到对应主题目录（`viewer/`、`llm/`、`launcher/`、`scenario/`、`kernel/`、`m4/`）。

## 专项手册
- Viewer 使用手册：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 公开静态镜像：`site/doc/cn/viewer-manual.html` / `site/doc/en/viewer-manual.html`

## 根目录 legacy
- `doc/world-simulator.prd.md`
- `doc/world-simulator.project.md`

## 维护约定
- 新文档按主题目录落位，不再默认平铺在模块根目录。
- 模块行为、Web 闭环或启动器体验口径变化时，需同步更新 `prd.md` 与 `project.md`。
- 新增专题后，需同步回写 `doc/world-simulator/prd.index.md` 与本目录索引。
- 若手册步骤改动，先更新仓库内 `viewer-manual.manual.md`，再评估是否需要同步静态镜像；公开镜像默认跟随仓库权威手册，而不是独立演化。
