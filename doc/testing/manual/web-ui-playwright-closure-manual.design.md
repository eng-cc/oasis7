# oasis7：Web UI Playwright 闭环测试手册（2026-02-28）设计

- 历史/共享设计 companion: `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- 历史任务追溯: GitHub task issue evidence comments 与 git history
- 当前 Playwright 实跑系列入口: `doc/testing/manual/web-ui-playwright-closure-manual.manual.md`

## 1. 设计定位
定义测试手册专题设计，统一系统性测试、Web UI Playwright 闭环与工程化维护方式。

## 2. 设计结构
- 手册结构层：明确入口、章节组织与适用范围。
- 执行方法层：沉淀测试步骤、命令模板与证据要求。
- 工具链对齐层：把 Playwright、系统测试与门禁口径统一到手册。
- 维护治理层：建立版本更新、引用互链与长期维护约定。

## 3. 关键接口 / 入口
- 测试手册入口
- 步骤/命令模板
- Playwright / 系统测试链接点
- 手册维护约定

## 4. 约束与边界
- 手册必须服务真实测试流程而非重复 PRD。
- 命令与步骤需可直接执行或映射。
- 不在本专题扩展新的测试框架。

## 5. 设计演进计划
- 先固化手册目录与范围。
- 再补步骤模板和工具链对齐。
- 最后沉淀维护约定。
