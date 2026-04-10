# oasis7: CI 与提交钩子测试分级设计

- 对应需求文档: `doc/testing/ci/ci-tiered-execution.prd.md`
- 对应项目管理文档: `doc/testing/ci/ci-tiered-execution.project.md`

## 1. 设计定位
定义 CI 与测试门禁专题设计，统一流水线分层、门禁策略、产物校验与失败保护。

## 2. 设计结构
- 流水线分层：按 `commit` / `required` / `full`、runner、target 或专题阶段划分执行链路。
- 门禁策略层：定义通过条件、阻断条件与 required check 保护。
- 校验执行层：收敛构建、测试、hash/determinism 等自动校验入口。
- 回归治理层：沉淀失败签名、发布影响与后续演进。

## 3. 关键接口 / 入口
- `pre-commit` 本地 commit baseline 入口
- CI workflow / check 入口
- 门禁/required check 配置
- runner/target/产物校验点
- CI 回归与失败签名

## 4. 约束与边界
- 门禁变更必须可审计、可回放。
- 本地默认提交路径与显式 required / full 门禁需边界清晰。
- 不在本专题重构整个平台 CI 基础设施。

## 5. 设计演进计划
- 先冻结门禁与执行分层。
- 再补专题校验与保护策略。
- 最后固化失败签名与回归。
