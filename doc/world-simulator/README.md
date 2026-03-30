# world-simulator 文档索引

审计轮次: 6

## 入口
- PRD: `doc/world-simulator/prd.md`
- 设计总览: `doc/world-simulator/design.md`
- 标准执行入口: `doc/world-simulator/project.md`
- 文件级索引: `doc/world-simulator/prd.index.md`

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

## 根目录 legacy
- `doc/world-simulator.prd.md`
- `doc/world-simulator.project.md`

## 维护约定
- 新文档按主题目录落位，不再默认平铺在模块根目录。
- 模块行为、Web 闭环或启动器体验口径变化时，需同步更新 `prd.md` 与 `project.md`。
- 新增专题后，需同步回写 `doc/world-simulator/prd.index.md` 与本目录索引。
