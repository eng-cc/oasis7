# CI 分级执行

- 状态：active
- 专业 owner：`repository_health_engineer`；验证范围由 `qa_engineer` 协作审读
- 通用规则：[开发流程规范](../../engineering/workflow/source-of-truth.md)
- 配对系统设计：[CI 分级执行设计](ci-tiered-execution.design.md)
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

CI 结果对应实际 source HEAD。main 前进或分支显示 `BEHIND` 本身不使该结果失效；只有合并冲突或 main 的相关变化带来明确集成风险时，才按需更新分支并验证实际组合。source HEAD 变化后重新运行必要普通 CI，复审实际变化与受影响部分；具体合入规则遵循开发流程规范。产品层的签名、checkpoint 和状态一致性证明仍按专业合同验证。

CodeQL 技术扫描见 [CodeQL 设计](codeql-integration.design.md)，实际 required/advisory 按 GitHub CI 与生效保护执行。

## 依赖准备目标与当前边界

本节面向开发者、CI 维护者和 Release 维护者，目标是减少重复工具准备、准确识别外部依赖耗时，并使必要安装的失败可诊断、可结束。它属于 testing 专业域，不增加玩家层承诺，不改变已有必要测试和发布验证范围。

2026-10-12 审读基线为 `fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e`。下表固定描述该基线，区分当时的实现与本次目标；`active`、文档合入和旧 Issue 关闭都不代表优化已实现或已验收。

| 对象 | 审读基线事实 | 本节承接的目标与证据缺口 |
| --- | --- | --- |
| Trunk 安装 | [基线安装器](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/scripts/install-ci-trunk.sh)已为 Linux x86_64 固定预编译包、校验摘要与版本；[基线 Rust CI](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/.github/workflows/rust.yml)的相关路径已使用它。[基线 Release Web 构建](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/.github/workflows/release-packages.yml)仍在 binary 缓存未命中时执行源码安装。 | Release 复用现有经过验证的预编译安装路径；真实安装、构建与失败行为仍需实现后验收。 |
| 工具缓存 | Rust CI 与 Release 使用不同的 Trunk 缓存对象和 key。 | 明确 Release 的可信写入来源、实际 ref/scope 和消费者；当前审读未建立跨 tag 命中及节省时间的实测结论。 |
| apt 依赖 | 当前 workflow 多处直接执行 `apt-get update/install`。 | 先核实实际耗时与失败分布；是否需要统一安装预算或重试实现，由观察到的问题决定。 |

<a id="ci-install-trunk"></a>
### Release 复用经过验证的 Trunk

**目标**：在现有安装器支持的 Linux x86_64 环境，Release Web 构建复用经过固定摘要和版本验证的预编译 Trunk，不因冷缓存而重复源码编译同一工具。工具版本与支持平台由安装器及配对设计维护，升级时同步审读来源、摘要、兼容性和调用方。

**本次实现合同**：Release 只读恢复预编译 archive，再调用[共享验证安装器](../../../scripts/install-ci-trunk.sh)准备工具；冷未命中按同一安装器下载与校验，不增加 Release 缓存 writer。此处说明候选接线范围，main 合入、hosted 运行及实际耗时仍以各自记录为准。

**验收**：

- 冷路径取得正确工具后能继续执行原有 Web 构建与验证；热路径使用同一验证合同，不以缓存命中跳过摘要或版本检查。
- 不支持的平台、版本不符、摘要不符、下载或提取失败必须有明确的失败结果，不得继续执行未验证工具或静默降级到另一种来源。
- 工具准备成功、Web 构建成功和 Release 完成分别记录，不互相代替；真实 Release 验证由实际运行结果证明。

<a id="ci-install-cache-evidence"></a>
### 缓存复用与真实证据

**目标**：缓存是可以丢弃的加速层。未命中、被回收或首次使用时，冷安装仍应正确、安全且可观察。可信 writer 与普通 PR/候选消费者的权限边界遵循[包来源与缓存安全规则](../../engineering/workflow/package-source-security.md)，不为提高命中率扩大写权限。

**验收**：

- 区分未命中、命中后校验成功、缓存损坏和恢复错误；只有验证合格的工具可进入构建。损坏缓存不得作为成功的冷/热样本，也不得回退到未经验证的 binary。
- 描述实际生产者与消费者的 workflow、事件、ref/scope、平台、架构、工具版本和安装策略。若跨 tag 复用需要默认分支 seed，必须确认有效 main writer、实际保存结果及后续消费者可达性；不能仅凭 key 不含 tag 或名称相同宣称可跨 tag 复用。
- 跨 tag 命中须有真实 writer 和 reader 的运行记录；尚未 seed、没有可达缓存或未执行 reader 时如实记为未验证，允许通过正确的冷路径完成必要工作。
- 冷/热对比包含恢复、准备/验证和保存成本，不能只统计省去的安装时间。

<a id="ci-install-apt-budget"></a>
### apt 安装预算与故障处置

**目标**：先从实际 job 日志核实 apt 耗时、锁等待、网络失败和重试频率，再决定是否改造。若进入实现，安装必须有覆盖命令执行、等待、退避、重试及清理的有限总预算；具体参数由观测数据和运行环境决定，不建立仓库统一的固定全局 SLO。

**实施后的验收**：

- 仅对已识别的可重试临时错误进行有限重试，次数和退避计入总预算；永久错误、权限/配置错误和未知失败保留诊断并失败，不无条件重跑。
- 超时或取消应终止安装启动的受控进程组，必要的终止宽限也受预算约束；返回时不得遗留继续持锁、下载或安装的受控进程。不得为此终止其他任务或宿主的无关进程。
- 分别验证正常完成、短暂失败后恢复、重试耗尽、永久失败、子进程超时及取消；父 shell 退出不能单独证明子进程已结束。
- 必要依赖未就绪时不运行依赖它的测试，不将失败改写成跳过或成功；结果沿现有 job 与 required 汇总传播。

<a id="ci-install-measurement"></a>
### 量测与交付判定

**目标**：以实际数据判断优化是否减少关键路径耗时，同时保持原有构建、测试、权限与失败语义。记录固定源提交、workflow/job 与运行链接、平台和工具版本、缓存状态、样本窗口与样本数；安装、缓存恢复、校验、构建和保存分别计时，失败与取消样本单独保留。

**验收**：可比较的冷/热样本需具有一致的工具与平台范围，并说明 runner、网络和其他并发差异。没有足够样本时只报告观察值及限制，不设默认性能门槛或宣称稳定收益；Cargo cold/no-op 编译指标不能替代依赖准备和跨 tag 缓存量测。

本次交付包含需求与系统设计的整理、Release archive 恢复与共享安装器接线；候选实现、本地合同验证、main 合入、hosted 构建、缓存 writer/reader 运行和冷/热收益分别以实际记录证明，不互相代替。apt 本次仅承接观察与条件实施要求，不宣称已改造或完成故障验收。结构检查、本地 fixture、旧绿色 CI 和文档状态不能证明未执行的 hosted 或量测结果。本节不新增通用检查名称、审批链或强制性能门禁。
