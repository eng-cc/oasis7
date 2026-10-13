# CI 分级执行

- 状态：active
- 专业 owner：`repository_health_engineer`；CI 控制、来源与缓存边界由独立评审者审读
- 通用规则：[开发流程规范](../../engineering/workflow/source-of-truth.md)
- 上游要求：[CI 分级执行 PRD](ci-tiered-execution.prd.md)
- 可执行测试命令和平台限制：[testing-manual.md](../../../testing-manual.md)

## 测试范围

普通 CI 是默认验证方式。轻量 baseline 始终执行；选择器使用完整 Git name-status diff、base/head 两侧 Cargo 关系及必要目录映射选择 Rust、Web、场景、平台、打包和运维合同测试。

PR 测试合并提交时，选择范围合并两组差异：实际 target/source 的唯一共同祖先到 source HEAD，以及实际 target 第一父提交到 tested merge。main 在事件 base 之后已存在的改动不重复计入；PR 自身变化和合并结果额外变化仍覆盖。校验 tested merge 恰有两个父提交、第二父提交等于 source HEAD、事件 base 是实际 target 的祖先；父提交身份不符、共同祖先不唯一或对象不可读时扩大为普通全量 required。依赖及嵌入输入分析保留事件 base、共同祖先、source、实际 target 与 tested merge 的图。push/source-only 检查继续使用事件 base 到 source 的完整差异。

Rust 变化覆盖自身及需要验证的反向消费者，删除和重命名覆盖两端；保留有效 features、WASM、生成输入和平台组合。workspace、features、锁文件、共享生成器及 CI 控制输入变化采用保守普通 required，未知路径或图解析失败扩大覆盖。

`./scripts/ci-tests.sh required` 是普通必要测试的本地入口；`full` 保留广覆盖回归，nightly 执行适用 full。真实产品测试、WASM 确定性、共识、持久化、安全、升级恢复和跨平台打包均继续保留，不因流程精简删除。

## 调度和结果

select 先产生明确的非空逻辑组列表，按资源组并行执行。普通组使用原生 job，fleet-health 使用固定 Linux/Windows/macOS 矩阵。最终唯一 `required-gate` 使用可靠的 always 汇总，并显式依赖 select 和所有组。选中组必须 success；未选中组可以 skipped 或 success；失败、取消、缺失、未知及意外 skipped 都必须阻断。fleet 矩阵不吞退出码，采用 fail-fast=false。

记录 source HEAD、base 和实际测试对象及运行链接。普通 PR 权限只读，从同一事件 BASE 提取可信选择器、scope 配置与结果汇总器，候选 checkout 的 driver 执行真实测试；候选控制变化不能缩小基线覆盖。CI 控制、安全或兼容边界变化接受对应能力的独立评审。

## 专项条件

缺测试先补普通 CI。只有普通 runner 难以提供的真实长期状态、专用硬件或受控环境才使用专项验证，说明具体不足、环境、版本、通过标准和实际结果。阻塞专项纳入同一 PR 最终汇总；调试 dispatch 不能替代 PR 检查。

CI 结果对应实际 source HEAD。main 前进或分支显示 `BEHIND` 本身不使该结果失效；只有合并冲突或 main 的相关变化带来明确集成风险时，才按需更新分支并验证实际组合。source HEAD 变化后重新运行必要普通 CI，复审实际变化与受影响部分；具体合入规则遵循开发流程规范。产品层的签名、checkpoint 和状态一致性证明仍按专业合同验证。

CodeQL 技术扫描见 [CodeQL 设计](codeql-integration.design.md)，实际 required/advisory 按 GitHub CI 与生效保护执行。

## 2. 上游与本设计

### 2.1 需求承接与分配表

| 上游要求 | 分配内容 | 本设计 | 负责范围 | 限制 |
| --- | --- | --- | --- | --- |
| [测试范围](ci-tiered-execution.prd.md#测试范围) | 保留产品测试并按真实影响选择 | [测试范围](#测试范围) | 选择器与执行器 | 未知影响扩大普通 required |
| [调度和结果](ci-tiered-execution.prd.md#调度和结果) | 最终汇总真实结果 | [调度和结果](#调度和结果) | workflow 与结果函数 | 本地回归不证明 hosted 运行成功 |
| [专项条件](ci-tiered-execution.prd.md#专项条件) | 普通 CI 优先，专项需具体环境不足 | [专项条件](#专项条件) | 对应产品运行场景 | 不新增通用 PM 集成链 |
| [Release Trunk](ci-tiered-execution.prd.md#ci-install-trunk) | Linux x86_64 冷热路径共用摘要与版本验证的预编译安装器 | [安装流程](#ci-install-trunk-design) | Release workflow 与共享安装器 | 工具成功不等于 Web 构建或发布成功 |
| [缓存复用](ci-tiered-execution.prd.md#ci-install-cache-evidence) | 对齐归档身份、可信 main writer 与只读消费者，验证真实 scope | [缓存边界](#ci-install-cache-design) | GitHub cache 与 main full-regression | 静态 key 相同不证明跨 tag 恢复 |
| [apt 预算](ci-tiered-execution.prd.md#ci-install-apt-budget) | 先量测；需要治理时约束总预算、有限重试及受控子进程清理 | [apt 评估](#ci-install-apt-design) | 临时 CI runner 的依赖准备 | 本次不实现未经数据确认的 apt 策略 |
| [量测与判定](ci-tiered-execution.prd.md#ci-install-measurement) | 分阶段、分冷暖记录可比较的准备成本与失败样本 | [耗时证据](#ci-install-measurement-design) | 实际 Actions 运行与维护者分析 | 不由 Cargo 指标或本地 fixture 推断发布提速 |

## 3. 依赖准备的基线与目标

2026-10-12 审读基线是 `fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e`。本次采用 Release 复用现有安装器的最小改造；真实 hosted 构建、main 缓存写入、跨 tag 恢复及节省时间分别以实际运行记录确认。apt 保留观察与条件实施要求。

| 对象 | 审读基线事实 | 本设计的变化与限制 |
| --- | --- | --- |
| Release Web 工具准备 | [原工作流](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/.github/workflows/release-packages.yml)恢复 `~/.cargo/bin/trunk`，未命中时 `cargo install trunk`。 | 恢复预编译归档并无条件调用共享验证安装器，去掉重复源码编译。 |
| 普通 CI 与 main writer | [Rust 工作流](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/.github/workflows/rust.yml)已使用归档；main schedule/manual-full 的 `full-regression` 可写入。 | 复用已有 writer、路径与 key，不新增预热工作流。 |
| 安装器 | [原安装器](https://github.com/eng-cc/oasis7/blob/fd5fa7c8ff1851a32cb3e71d488783ca59e05a5e/scripts/install-ci-trunk.sh)固定 Trunk 0.21.14、官方归档摘要及 Linux x86_64。 | 继续由该脚本集中拥有版本、来源、摘要、平台与失败规则。 |
| apt | 原工作流直接执行 `apt-get update/install`，本轮没有当前卡死或遗留进程的运行证据。 | 先核实阶段耗时与失败，再决定预算和重试参数。 |

<a id="ci-install-trunk-design"></a>
## 4. Release Trunk 安装流程

消费者是 [Release `build-web-dist`](../../../.github/workflows/release-packages.yml)，支持范围是现有 Linux x86_64 runner。源码身份继续由 `package-source-plan.py` 一次解析，job checkout 固定的计划提交并核对 HEAD；工具优化不改变原有源码、BUILDINFO、构建验证或发布时 tag 复核规则，完整边界见[包来源与缓存安全规则](../../engineering/workflow/package-source-security.md)。

1. 在计划源码校验后确认 `scripts/install-ci-trunk.sh` 是非空普通文件且不是符号链接；缺失、为空或为符号链接时明确失败，不能用空安装器 hash 恢复缓存，也不从其他 ref 临时借用脚本。
2. 使用固定提交的 `actions/cache/restore` 恢复同平台归档。该步骤只恢复，不为 Release 增加 Trunk 缓存保存；不把缓存命中作为跳过安装器的条件。
3. 调用 `bash scripts/install-ci-trunk.sh "$RUNNER_TEMP/ci-tools/trunk-x86_64-unknown-linux-gnu.tar.gz" "$RUNNER_TEMP/ci-tools/bin"`。归档不存在时，安装器从固定官方来源下载并校验后落盘；归档已存在时仍校验固定 SHA-256，校验通过后才解压及执行。
4. 安装器检查提取的 binary、精确版本和支持平台，成功后写入 `GITHUB_PATH`，后续原有 Web 构建使用该工具。任何必要安装失败沿 job 失败传播。

缓存损坏或下载摘要不符时在解压、执行前失败；版本不符、网络失败、提取失败和不支持平台均不得继续构建。保留现有安装器的有界下载重试，不引入任意版本、未审核镜像或 `cargo install` 回退。损坏缓存不静默改写成一次成功的冷启动。

<a id="ci-install-cache-design"></a>
## 5. 缓存身份、作用域与恢复

归档路径与普通 CI/main writer 保持一致：`${{ runner.temp }}/ci-tools/trunk-x86_64-unknown-linux-gnu.tar.gz`。key 复用现有表达式：

```yaml
trunk-archive-${{ runner.os }}-${{ runner.arch }}-${{ env.TRUNK_VERSION }}-${{ hashFiles('scripts/install-ci-trunk.sh') }}
```

安装器来自已核验的计划 source；key 中的版本、平台、架构和安装器内容指纹共同描述本次工具输入。升级脚本或 pin 会产生正常冷启动。缓存路径、归档格式及 cache action 的兼容实现也必须一致；仅对齐 key 字符串不充分。GitHub 的 cache version 包含路径与压缩方式，tag 间不能直接读取彼此缓存，可恢复默认分支范围内的缓存，见 [GitHub 缓存引用](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)。

现有 main `full-regression` 的 schedule/manual-full 是共享归档 writer；Release 使用只恢复 action，普通 PR 继续受只读 cache token 约束。其他 Release/Rust 缓存的 key、保存策略和权限保持其现有合同。不得增加宽泛 `restore-keys`、扩大缓存写权限，或把缓存内容当作可执行来源的认证。

缓存是可回收的加速数据，没有发布状态、事务或持久化权威。main 尚未保存该身份、缓存被回收或 reader 不可达时，正确的冷下载仍可完成工具准备。恢复服务异常须保留实际 action 结果；若 action 允许继续且归档不存在，安装器仍执行同一校验冷路径，不用忽略必要安装失败来维持 job 绿色。

跨 tag 复用的验证必须关联一个可确认的 main 保存记录及两个不同 tag 的实际 reader，记录 run/job、event/ref、请求和实际匹配 key、平台与 cache version 兼容条件、恢复及摘要验证结果。仅同 tag rerun 只能证明同 tag 恢复；未取得上述记录时保留“配置已接通、真实跨 tag 恢复待观察”。沿用自然发生且已授权的 main/full 与 Release 执行积累数据，不为本项新建发布事件或 required check。

<a id="ci-install-apt-design"></a>
## 6. apt 的有界安装评估

先从现有 job 日志区分索引更新、下载、安装与锁等待，记录 runner image、阶段耗时、失败类别及实际重试；日志粒度不足时由维护者在对应依赖步骤补量测。当前没有依据直接规定统一超时秒数、尝试次数或重试所有错误。

若观察到足以实施的长尾或取消清理问题，采用一个可单测的 helper 管理整个依赖准备预算：每次执行、终止宽限、退避与后续尝试都计入有限总预算。仅对可确认的瞬态失败重试；缺包、错误参数、权限/配置、签名或完整性失败不靠重复调用掩盖。

超时与取消针对本次启动的受控进程组，先请求终止，必要时在有界宽限后强制终止；确认该组及其受控子进程退出后才可结束或开始下一次允许的尝试。不得删除锁文件、终止无关进程，或把父 shell 退出当作清理完成。若 dpkg 已开始修改且被强制中断，不能盲目重复 install；无法证明状态可继续时让 job 失败，由新 runner 重试。

该后续设计仅面向临时 CI runner，不直接推广到开发机器或部署主机。实现时用假命令与进程 fixture 验证挂起、忽略终止信号、父进程先退出、瞬态/永久错误、预算耗尽及取消，不对真实 apt/dpkg 故障注入。未实施或未执行的场景不得写为已验证。

<a id="ci-install-measurement-design"></a>
## 7. 耗时与运行证据

维护者从实际 Actions 日志分别记录缓存恢复、官方下载、摘要校验/提取/安装/版本校验及总准备耗时；writer 的保存成本另列。当前日志可提供 action/step 级时间，缺少子阶段时间时标明粒度，不把合并步骤的时间伪拆成精确阶段。需要更细比较时再在同一安装路径补计时。

| 样本类型 | 判定依据 | 记录边界 |
| --- | --- | --- |
| 冷启动 | 没有恢复归档且本地归档不存在，实际执行下载 | 包括恢复尝试、下载与验证安装 |
| 暖缓存 | 从 GitHub 恢复对应归档并通过校验，未调用下载 | 包括恢复及验证安装，不跳过校验成本 |
| 本地已有 | 同 job 已存在归档且验证通过 | 单独列出，不充当跨 run 命中 |
| 恢复异常后下载 | action 异常后仍能按冷路径取得正确归档 | 单列异常与额外成本，不混入正常冷启动 |

每组记录 runner OS/image、架构、Trunk 版本及归档摘要、安装器指纹、source、event/ref、run/job 链接、key、样本窗口与数量。失败和取消保留原因及耗时，不能仅挑成功样本。总发布耗时、排队时间、Rust 编译与 Trunk 准备时间分别报告；Cargo cold/no-op 量测不覆盖本节的网络安装与 cache scope。

样本不足时报告单次观测和环境差异；至少三次可比观察后可以报告样本数、中位数及范围，仍不外推稳定性能承诺。当前不设仓库统一 SLO、预写提速比例或新增性能门禁。实现接线正确、本地合同通过、hosted 构建成功与真实缓存收益是分别验证的结论。

## 8. 兼容、启用与回退

实现随普通 PR 合入生效，实际 Release 使用哪个 workflow revision 和 source commit 由原有计划合同确定。计划源必须包含受支持的共享安装器；早于该安装器的历史 tag 在新 workflow 下会明确失败。需要重发历史版本时，先审读相应源码和工具兼容性，不能用移动 tag、任意 ref 脚本或源码安装回退绕过这一边界。

缓存内容无需迁移；旧 Release binary 缓存可以自然回收，首次使用新身份的冷成本单独记录。回退通过正常 PR 恢复已知工作流基线及配套合同测试，仍须独立评审和适用 CI；不在运行中自动选择另一种安装策略。apt 的条件设计不改变当前运行参数。此改造不涉及玩家接口、世界状态或游戏 Runtime，无需 Agent 游戏实玩验证。

## 11. 验证

### 11.1 验证映射表

| 上游要求 | 本设计 | 验收内容 | 验证来源 | 实际记录 | 限制 |
| --- | --- | --- | --- | --- | --- |
| [测试范围](ci-tiered-execution.prd.md#测试范围) | [测试范围](#测试范围) | 删除、重命名、依赖和未知输入覆盖 | [选择器回归](../../../scripts/plan-rust-required-scope.test.py) | 命令退出码及输出 | 不替代真实产品组运行 |
| [调度和结果](ci-tiered-execution.prd.md#调度和结果) | [调度和结果](#调度和结果) | 失败、取消、缺失和跳过均正确传播 | [结果函数回归](../../../scripts/ci-required-result.test.py) | 回归输出与 Actions 链接 | hosted 图仍需独立核对 |
| [专项条件](ci-tiered-execution.prd.md#专项条件) | [专项条件](#专项条件) | 适用环境和真实场景执行 | [测试手册](../../../testing-manual.md) | 所测版本和实际环境结果 | 未执行的环境保持未证明 |
| [Release Trunk](ci-tiered-execution.prd.md#ci-install-trunk) | [安装流程](#ci-install-trunk-design) | planned source、只恢复归档、冷热均调用验证安装器、必要失败传播 | [Release 接线回归](../../../scripts/release-packages-trunk-cache-contract.test.sh) | 候选版本、命令退出码及回归输出 | 不替代真实下载、Web 构建或发布 |
| [缓存复用](ci-tiered-execution.prd.md#ci-install-cache-evidence) | [缓存边界](#ci-install-cache-design) | main writer 与不同 tag readers 的真实身份、恢复及验证 | [依赖安装核验](../../../testing-manual.md#ci-dependency-installation-validation) | writer/reader run/job 链接与实际 key | 未 seed 或未执行 reader 时仍未证明跨 tag 命中 |
| [apt 预算](ci-tiered-execution.prd.md#ci-install-apt-budget) | [apt 评估](#ci-install-apt-design) | 量测后选定参数；实施时验证总预算、分类重试、超时/取消无受控遗留进程 | [依赖安装核验](../../../testing-manual.md#ci-dependency-installation-validation) | 实际 apt 观察及后续 helper 的 fixture 输出 | 当前未实现，不推断宿主或部署机适用性 |
| [量测与判定](ci-tiered-execution.prd.md#ci-install-measurement) | [耗时证据](#ci-install-measurement-design) | 相同工具/平台下分冷暖计入准备与缓存成本，保留失败样本 | [依赖安装核验](../../../testing-manual.md#ci-dependency-installation-validation) | 原始耗时、样本窗口/数量和条件差异 | 不以本地测试宣称 hosted 性能收益 |

## 12. 决策与残余风险

历史 [Issue #1808](https://github.com/eng-cc/oasis7/issues/1808) 中的旧 PM 工具问题随 [#4354](https://github.com/eng-cc/oasis7/pull/4354) 退出当前实现；本设计保留可适用于现行 CI 的依赖准备与缓存问题，不恢复旧任务或验收状态机。选择共享预编译安装器，是因为现有实现已集中拥有来源、摘要和版本检查，可以直接去掉 Release 的重复源码安装路径。

`repository_health_engineer` 维护安装器、缓存接线及 apt 观察；`qa_engineer` 协作核对真实运行证据。主要残余风险是外部下载服务可用性、main seed 尚未产生、缓存被回收以及旧 tag 缺少安装器；这些情况分别按既有失败语义或正确冷路径处理。版本、平台、来源策略或 cache action 更新时重新审读受影响合同；没有实际 hosted 数据时保留未知项，不从文档状态推导发布成功。
