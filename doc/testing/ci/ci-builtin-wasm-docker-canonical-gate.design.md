# oasis7: Builtin Wasm Docker Canonical Gate 设计

- 对应需求文档: `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md`
- 可变任务状态与历史: PR、实际 CI 与评审记录（Issue 按需）

## 1. 设计定位
定义 CI 与测试门禁专题设计，统一流水线分层、门禁策略、产物校验与失败保护。

## 2. 设计结构
- 流水线分层：按 required/full、runner、target 或专题阶段划分执行链路。
- 门禁策略层：定义通过条件、阻断条件与 required check 保护。
- 校验执行层：收敛构建、测试、hash/determinism 等自动校验入口。
- 回归治理层：沉淀失败签名、发布影响与后续演进。

## 3. 关键接口 / 入口
- CI workflow / check 入口
- 门禁/required check 配置
- runner/target/产物校验点
- CI 回归与失败签名

## 4. 约束与边界
- 门禁变更必须可审计、可回放。
- 基础门禁与增量专题门禁需边界清晰。
- 不在本专题重构整个平台 CI 基础设施。

## 5. 设计演进计划
- 先冻结门禁与执行分层。
- 再补专题校验与保护策略。
- 最后固化失败签名与回归。

## 2. 需求与验证关系

### 2.1 需求承接与分配表

| 上游要求 | 分配内容 | 本设计 | 负责范围 | 限制 |
| --- | --- | --- | --- | --- |
| [canonical gate](ci-builtin-wasm-docker-canonical-gate.prd.md#1-executive-summary) | Docker/hash/receipt 与测试层级分工 | [设计结构](#2-设计结构) | Wasm workflow 与汇总器 | 原生保护由实际配置确认 |
| [只读与发布边界](ci-builtin-wasm-docker-canonical-gate.prd.md#4-technical-specifications) | 默认校验和受控写入 | [约束与边界](#4-约束与边界) | 构建与发布脚本 | 本地通过不证明生产发布 |

## 11. 验证

### 11.1 验证映射表

| 上游要求 | 本设计 | 验收内容 | 验证来源 | 实际记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| [canonical gate](ci-builtin-wasm-docker-canonical-gate.prd.md#1-executive-summary) | [设计结构](#2-设计结构) | module scope、hash 与 receipt 对账 | [测试手册](../../../testing-manual.md) | 当前 HEAD 的 CI 与 summary | 本地文档检查不能证明 Docker 产物 |
| [只读与发布边界](ci-builtin-wasm-docker-canonical-gate.prd.md#4-technical-specifications) | [约束与边界](#4-约束与边界) | 校验/发布写入分离 | [测试手册](../../../testing-manual.md) | 节点验收与发布记录 | CI 不执行生产激活 |
