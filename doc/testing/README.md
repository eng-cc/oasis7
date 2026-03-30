# testing 文档索引

审计轮次: 7

## 入口
- PRD: `doc/testing/prd.md`
- 设计总览: `doc/testing/design.md`
- 标准执行入口: `doc/testing/project.md`
- 文件级索引: `doc/testing/prd.index.md`

## 模块职责
- 维护系统测试手册、required/full 分层门禁与发布证据包口径。
- 汇总 CI、启动器、长稳、性能、人工手册与治理专题。
- 承接跨模块测试范围定义、证据归档与趋势基线建设。

## 关键文档
- 系统测试手册：`testing-manual.md`
- 模块化测试细则：`doc/testing/manual/`
- CI 与门禁专题：`doc/testing/ci/`
- 启动器链路测试：`doc/testing/launcher/`
- 长稳与压力测试：`doc/testing/longrun/`、`doc/testing/performance/`
- 门禁策略与治理：`doc/testing/governance/`、`doc/testing/chaos-plans/`

## 近期专题
- `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md`
- `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
- `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md`
- `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
- `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md`
- `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `ci/`、`launcher/`、`longrun/`、`performance/`、`manual/`、`governance/`。

## 维护约定
- 测试门禁变更需同步 required/full 分层口径、对应脚本与证据模板。
- 新增专题后，需同步回写 `doc/testing/prd.index.md` 与本目录索引。
