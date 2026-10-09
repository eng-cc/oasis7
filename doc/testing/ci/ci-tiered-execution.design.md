# CI 分级执行

- 状态：active
- 通用规则：[开发流程规范](../../engineering/workflow/source-of-truth.md)
- 可执行测试命令和平台限制：[testing-manual.md](../../../testing-manual.md)

## 测试范围

普通 CI 是默认验证方式。轻量 baseline 始终执行；选择器使用完整 Git name-status diff、base/head 两侧 Cargo 关系及必要目录映射选择 Rust、Web、场景、平台、打包和运维合同测试。

Rust 变化覆盖自身及需要验证的反向消费者，删除和重命名覆盖两端；保留有效 features、WASM、生成输入和平台组合。workspace、features、锁文件、共享生成器及 CI 控制输入变化采用保守普通 required，未知路径或图解析失败扩大覆盖。

`./scripts/ci-tests.sh required` 是普通必要测试的本地入口；`full` 保留广覆盖回归，nightly 执行适用 full。真实产品测试、WASM 确定性、共识、持久化、安全、升级恢复和跨平台打包均继续保留，不因流程精简删除。

## 调度和结果

select 先产生明确的非空矩阵，按资源组并行执行。最终唯一 `required-gate` 使用可靠的 always 汇总，并显式依赖 select 和所有组。选中组必须 success；未选中组可以 skipped 或 success；失败、取消、缺失、未知及意外 skipped 都必须阻断。矩阵不吞退出码，采用 fail-fast=false。

记录 source HEAD、base 和实际测试对象及运行链接。普通 PR 权限只读，从事件 base 提取可信选择器及执行清单；候选控制变化不能缩小基线覆盖。CI 控制、安全或兼容边界变化接受对应能力的独立评审。

## 专项条件

缺测试先补普通 CI。只有普通 runner 难以提供的真实长期状态、专用硬件或受控环境才使用专项验证，说明具体不足、环境、版本、通过标准和实际结果。阻塞专项纳入同一 PR 最终汇总；调试 dispatch 不能替代 PR 检查。

main 前进按原生 up-to-date 保护更新分支并重新执行必要普通 CI，不创建 Task、epoch、receipt 或第二条通用严格集成流程。产品层的签名、checkpoint 和状态一致性证明仍按专业合同验证。

CodeQL 技术扫描见 [CodeQL 设计](codeql-integration.design.md)，实际 required/advisory 按 GitHub CI 与生效保护执行。

## 2. 上游与本设计

### 2.1 需求承接与分配表

| 上游要求 | 分配内容 | 本设计 | 负责范围 | 限制 |
| --- | --- | --- | --- | --- |
| [测试范围](ci-tiered-execution.prd.md#测试范围) | 保留产品测试并按真实影响选择 | [测试范围](#测试范围) | 选择器与执行器 | 未知影响扩大普通 required |
| [调度和结果](ci-tiered-execution.prd.md#调度和结果) | 最终汇总真实结果 | [调度和结果](#调度和结果) | workflow 与结果函数 | 本地回归不证明 hosted 运行成功 |
| [专项条件](ci-tiered-execution.prd.md#专项条件) | 普通 CI 优先，专项需具体环境不足 | [专项条件](#专项条件) | 对应产品运行场景 | 不新增通用 PM 集成链 |

## 11. 验证

### 11.1 验证映射表

| 上游要求 | 本设计 | 验收内容 | 验证来源 | 实际记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| [测试范围](ci-tiered-execution.prd.md#测试范围) | [测试范围](#测试范围) | 删除、重命名、依赖和未知输入覆盖 | [选择器回归](../../../scripts/plan-rust-required-scope.test.py) | 命令退出码及输出 | 不替代真实产品组运行 |
| [调度和结果](ci-tiered-execution.prd.md#调度和结果) | [调度和结果](#调度和结果) | 失败、取消、缺失和跳过均正确传播 | [结果函数回归](../../../scripts/ci-required-result.test.py) | 回归输出与 Actions 链接 | hosted 图仍需独立核对 |
| [专项条件](ci-tiered-execution.prd.md#专项条件) | [专项条件](#专项条件) | 适用环境和真实场景执行 | [测试手册](../../../testing-manual.md) | 所测版本和实际环境结果 | 未执行的环境保持未证明 |
