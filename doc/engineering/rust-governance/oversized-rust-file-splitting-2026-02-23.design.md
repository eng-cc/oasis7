# 超长 Rust 文件拆分设计（2026-02-23）

- 对应需求文档: `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.prd.md`
- 对应项目管理文档: `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md`

## 1. 设计定位
定义 Rust 超长文件拆分的判定规则、拆分边界、模块化策略与回归要求，避免只按行数机械切割。

## 2. 设计结构
- 判定层：由治理规则识别超过阈值的 Rust 文件。
- 设计层：先识别职责聚类，再确定模块边界和公共抽象。
- 执行层：按子模块拆分实现并补齐测试。
- 验证层：通过 `cargo check`、定向测试与文档回写确认不退化。

## 3. 关键接口 / 入口
- 工程约束：`AGENTS.md` 中单个 Rust 文件长度限制
- 任务追踪：`doc/engineering/project.md`
- 校验方式：`env -u RUSTC_WRAPPER cargo check` 与对应测试套件

## 4. 约束与边界
- 先按职责拆分，避免为过门禁而过度碎片化。
- 拆分后模块命名、导出边界与测试归属必须清晰。
- 文档与代码都要回写到同一 PRD-ID 追踪链。

## 5. 设计演进计划
- 先收口最超限文件，再形成持续治理清单。
- 后续可把拆分建议纳入工程门禁与例行巡检。
