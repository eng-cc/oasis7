# Rust 1200 行根治治理（2026-03-29）

- 对应设计文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- 对应项目管理文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.project.md`

审计轮次: 6

- 对应标准执行入口: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.project.md`

## 1. Executive Summary
- Problem Statement: 仓库已重新出现大规模 Rust 超限文件，当前实况为 32 个生产文件和 15 个测试文件超过 1200 行，说明“拆一次大文件”没有真正解决文件继续膨胀的问题。现有 round3 治理偏向 `include!`/`split_part` 结构切片，未把职责边界、目录模型和 CI 阻断做成长期机制。
- Proposed Solution: 将“1200 行限制”从一次性清债任务升级为长期工程治理专题：冻结当前超限基线、在 required gate 中引入 Rust 文件体量检查、禁止新增 `split_part` 作为完成态、对被触碰的超限文件执行“touch-and-shrink”规则，并按 runtime/viewer/launcher 三个高风险域分批做职责拆分。
- Success Criteria:
  - SC-1: 基线冻结后，新增生产 Rust 超限文件数为 0，新增测试 Rust 超限文件数为 0。
  - SC-2: `scripts/ci-tests.sh required` 默认包含 Rust 文件体量检查，且该检查单次执行时间 <= 15 秒。
  - SC-3: 新增或改动文件中，`split_part` / `part1` / `part2` / `include!` 不再被用作超限治理完成态；新增违规数为 0。
  - SC-4: 被本次任务触碰的超限文件，行数必须单调下降，且对应职责边界说明与回归证据齐全率 100%。
  - SC-5: Phase-2 收口时，生产 Rust 超限文件从 32 降到 0，测试 Rust 超限文件从 15 降到 <= 3，并为剩余测试债务建立冻结清单与下一批治理计划。

## 2. User Experience & Functionality
- User Personas:
  - 工程维护者：需要把“1200 行限制”变成可持续执行的门禁，而不是一次性整治。
  - 贡献开发者：需要知道当自己触碰超限文件时，必须如何拆、能拆到哪里、哪些做法被禁止。
  - 评审者：需要快速判断一次拆分究竟是在清债，还是在继续制造新的结构债。
  - QA / 发布维护者：需要确保文件治理不会破坏 required/full 的测试链路与发布入口。
- User Scenarios & Frequency:
  - 开发者修改超限文件：每次碰到 `runtime_live`、`chain_runtime`、`viewer` 大文件时触发。
  - 评审超限治理 PR：每个相关任务至少 1 次。
  - required gate 扫描：每次本地提交前和 CI required gate 触发时执行。
  - 季度工程复盘：每季度至少 1 次，核对超限趋势、误报和余量。
- User Stories:
  - PRD-ENGINEERING-R1200-001: As an 工程维护者, I want a frozen oversized-file baseline plus a blocking gate, so that the repository can stop growing new 1200-line debt immediately.
  - PRD-ENGINEERING-R1200-002: As a 开发者, I want explicit touch-and-shrink rules and target module boundaries, so that I can refactor oversized files without guessing the acceptable end state.
  - PRD-ENGINEERING-R1200-003: As a 评审者, I want split-part and include-only refactors rejected as final state, so that mechanical file slicing does not masquerade as real debt reduction.
  - PRD-ENGINEERING-R1200-004: As a QA / 发布维护者, I want each oversized-file治理批次 tied to concrete regression suites, so that structure changes do not silently weaken validation.
  - PRD-ENGINEERING-R1200-005: As a 技术负责人, I want a phased burn-down plan by subsystem, so that the largest god modules are reduced in a controlled order instead of via one risky mega-refactor.
- Critical User Flows:
  1. Flow-R1200-001: `开发者修改超限文件 -> 文件体量检查识别当前文件已在冻结基线内 -> 校验本次改动是否让文件继续变大 -> 若变大则阻断，若缩小或完成模块迁移则允许继续`
  2. Flow-R1200-002: `开发者准备通过 split_part/include! 做机械切片 -> 门禁识别新增 split-part 完成态 -> 失败并提示改为目录模块/职责模块拆分`
  3. Flow-R1200-003: `owner 为超限文件创建治理任务 -> 在 project 中声明目标边界、目标目录和回归集 -> 实施拆分 -> 执行对应 required/full 回归 -> 回写 devlog 和基线`
  4. Flow-R1200-004: `评审者审查治理提交 -> 比较文件行数、职责边界和测试证据 -> 决定 pass / blocked / needs further split`
  5. Flow-R1200-005: `季度治理复盘 -> 重新扫描超限清单 -> 对照基线和趋势 -> 调整下一批 subsystem burn-down 顺序`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Rust 超限扫描 | `path`、`line_count`、`is_test`、`baseline_status`、`delta_vs_baseline` | 执行 `scripts/check-rust-file-size.sh` 输出超限列表与 delta | `pass/fail` | 先按 `is_new_violation`，再按 `line_count desc` | 所有人可执行；engineering owner 可更新冻结基线 |
| 冻结基线 | `frozen_at`、`path`、`baseline_line_count`、`owner_module`、`priority_batch` | 生成/更新基线清单；非治理任务不得扩大同路径基线 | `draft -> frozen -> retired` | 生产代码优先于测试代码；行数越大优先级越高 | 仅治理 owner 可改，评审者需复核 |
| touch-and-shrink | `touched_path`、`before_lines`、`after_lines`、`target_module_dir`、`shrink_reason` | 若本次提交触碰超限文件则强制检查是否变小或完成迁移 | `pending -> blocked -> satisfied` | `after_lines` 必须 < `before_lines`，或旧文件被职责模块替代后从基线移除 | 触碰者必须满足；评审者不可豁免 |
| split-part 禁止规则 | `new_file_path`、`naming_pattern`、`parent_module`、`migration_ticket` | 检查新增 `split_part/part1/part2/include!` 完成态并阻断 | `pass/fail` | 新增命名违规优先报错；存量文件允许在治理批次中逐步消化 | 所有人受限；仅主题治理任务可在迁移中短暂保留 |
| 子系统 burn-down | `subsystem`、`owner_role`、`target_files`、`target_boundaries`、`required_tests`、`full_tests` | 在 project 中分批登记 runtime/viewer/launcher 治理任务 | `planned -> in_progress -> verified -> closed` | 优先级按风险和行数叠加排序；`chain_runtime`、`runtime_live`、`viewer guide` 为首批 | owner role 牵头，QA 负责验证结论 |
| 评审结论 | `finding`、`boundary_ok`、`test_evidence`、`baseline_update` | 评审时输出 `pass / blocked / risky` | `draft -> reviewed -> merged/rejected` | 优先关注边界真实性，其次关注行数结果 | reviewer / qa_engineer 主判，owner 回写 |
- Acceptance Criteria:
  - AC-1: 新增专题将当前“32 个生产文件 + 15 个测试文件超限”的事实基线写入正式文档，并在 project 中拆成可执行批次。
  - AC-2: required gate 的目标态明确要求默认执行 Rust 文件体量检查，并区分生产代码、测试代码、`third_party/` 与生成目录。
  - AC-3: PRD 明确禁止把 `split_part` / `include!` 机械切片作为治理完成态，并定义可接受的目录模块化完成态。
  - AC-4: 每个治理批次都必须声明目标目录边界、触碰即缩小规则、回归命令和证据落点。
  - AC-5: 主题 project 至少覆盖：基线冻结、门禁脚本、迁移规则、runtime 首批治理、viewer 首批治理、测试债务收口六类任务。
  - AC-6: 文档入口、索引、module project 与 devlog 均可追溯到该专题和对应任务。
- Non-Goals:
  - 不在本专题一次性完成全部超限文件代码重构。
  - 不把“所有文件都压到 1200 以下”作为单提交大爆改目标。
  - 不修改 `third_party/` 代码。
  - 不把 lint 风格、命名风格或通用性能优化混入本专题。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。本专题面向工程治理和代码结构约束，不新增 AI 推理链路。
- Evaluation Strategy: 不适用；验证以脚本扫描、代码评审和 required/full 回归为准。

## 4. Technical Specifications
- Architecture Overview:
  - 治理入口层：`doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.{prd,design,project}.md` 定义规则、批次和追踪。
  - 扫描与门禁层：新增 `scripts/check-rust-file-size.sh` 作为单一事实源，负责扫描 `crates/**/src/*.rs` 与测试文件并输出基线差异；`scripts/ci-tests.sh required` 调用该脚本。
  - 基线层：在工程治理目录中维护冻结清单，记录当前允许存在但必须逐步收缩的超限文件；新文件不允许进入基线。
  - 迁移层：以目录模块（`mod.rs + 子模块文件`）或按职责拆分的独立模块替代 `include!` 分段；每次迁移必须在 project 中定义目标边界与回归集。
  - 验证层：每批治理至少执行 `test_tier_required` 定向回归，涉及 viewer live、链运行时或 launcher 等关键入口时追加 `test_tier_full`、脚本 smoke 或 Web 闭环验证。
- Integration Points:
  - `AGENTS.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/README.md`
  - `doc/engineering/oversized-rust-file-splitting-2026-02-23.prd.md`
  - `doc/engineering/oversized-rust-file-splitting-2026-02-23.project.md`
  - `scripts/ci-tests.sh`
  - `scripts/doc-governance-check.sh`
  - `testing-manual.md`
  - `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
  - `crates/oasis7_viewer/src/web_test_api.rs`
- Edge Cases & Error Handling:
  - 存量超限文件但本次未触碰：允许保留在冻结基线中，但不能新增同类文件。
  - 触碰超限文件仅增加注释/测试 helper：仍视为扩大体量，必须同步做 shrink 或抽离。
  - 测试文件暂时难以拆到 <= 1200：允许通过冻结清单保留少量尾债，但必须标明 owner、阻塞原因和下一批计划。
  - 平台条件编译导致单文件被双端共用：必须优先抽共用逻辑到子模块，不能用条件编译继续堆高单文件。
  - 紧急 hotfix 需要修改超限文件：允许短期修复，但 merge 前必须补最小 shrink 或创建同日治理任务补偿，不允许静默放行。
  - 生成文件或外部模板误入扫描范围：脚本必须排除 `third_party/`、`target/`、worktree 产物和非仓库源码目录。
  - `include!` 存量文件迁移期：允许在对应 burn-down 任务存续期间保留，但 project 必须给出退场目标目录和移除条件。
  - 多人并行治理同一 god module：必须在 project 中切成互斥责任面；若目标目录重叠则阻断并重新拆任务。
- Non-Functional Requirements:
  - NFR-R1200-1: Rust 文件体量检查脚本在 Linux/macOS 下可执行，单次 required-tier 额外耗时 <= 15 秒。
  - NFR-R1200-2: 新增生产/测试 Rust 超限文件数为 0。
  - NFR-R1200-3: 新增 `split_part` / `part1` / `part2` / `include!` 机械切片完成态违规数为 0。
  - NFR-R1200-4: 被触碰的超限文件“行数单调下降 + 目标边界说明 + 回归证据”覆盖率 100%。
  - NFR-R1200-5: 首批高风险文件治理任务必须全部绑定 `test_tier_required`，涉及入口/联机/Viewer Web 的任务再追加 `test_tier_full` 或 Web 闭环。
  - NFR-R1200-6: 冻结基线更新必须与对应治理任务和 devlog 同提交回写，追溯完整率 100%。
  - NFR-R1200-7: Phase-1 结束时，`chain_runtime`、`runtime_live`、`viewer player guide` 三个首批文件均完成目录边界设计并至少清掉一个实际超限文件。
  - NFR-R1200-8: 任何治理任务不得通过复制类型定义、复制测试 helper 或新建平行 god file 来“转移”超限债务。
- Security & Privacy:
  - 本专题不引入新的用户数据、权限模型或外部服务凭据。
  - 工程脚本日志不得输出开发者本机敏感路径之外的秘密信息；若扫描结果包含 worktree 路径，仅作为本地治理证据使用。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-29): 建立 root-cause 治理专题、冻结超限基线、明确门禁与拆分完成态。
  - v1.1: 落地 Rust 文件体量检查脚本、接入 required gate，并建立 `touch-and-shrink` / `split-part` 阻断规则。
  - v2.0: 分三批治理 `chain_runtime`、`runtime_live`、`viewer` 侧超限文件，持续下降生产代码超限数。
  - v2.1: 收口测试债务，压缩超限测试文件数量，并把残余债务转成冻结清单与季度治理节奏。
- Technical Risks:
  - 风险-1: 规则过硬但没有基线冻结，会导致现有仓库无法渐进演进。
  - 风险-2: 仅以行数为目标，开发者可能继续通过 `include!` 或复制粘贴制造伪治理。
  - 风险-3: 高风险入口文件拆分时可能破坏 viewer live、链运行时或 launcher 测试稳定性。
  - 风险-4: 若 project 不提前定义职责边界，多人并行治理会相互覆盖并造成冲突。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-R1200-001 | TASK-ENGINEERING-051/052 | `test_tier_required` | 专题文档检查、基线扫描脚本、required gate 接入验证 | 工程治理入口、提交门禁 |
| PRD-ENGINEERING-R1200-002 | TASK-ENGINEERING-053/054/055 | `test_tier_required` + `test_tier_full` | touch-and-shrink 规则校验、目标目录边界抽样、定向回归 | runtime/viewer/launcher 拆分策略 |
| PRD-ENGINEERING-R1200-003 | TASK-ENGINEERING-052/053 | `test_tier_required` | split-part 命名阻断、include-only 完成态审查、基线 delta 校验 | 结构治理真实性 |
| PRD-ENGINEERING-R1200-004 | TASK-ENGINEERING-054/055/056 | `test_tier_required` + `test_tier_full` | 首批超限文件治理定向回归、脚本 smoke、必要时 Web 闭环 | 入口文件、联机链路、Viewer 可用性 |
| PRD-ENGINEERING-R1200-005 | TASK-ENGINEERING-051/054/055/056/057 | `test_tier_required` | project 批次追踪、devlog 与索引回写、趋势扫描 | 长期 burn-down 可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-R1200-001 | 采用“冻结基线 + 禁止新增 + 触碰即缩小”的渐进治理 | 再做一轮一次性全仓大拆分 | 当前超限范围已回弹，一次性大拆风险高且难持续执行。 |
| DEC-R1200-002 | 目录模块/职责模块作为最终完成态 | `include!` / `split_part` 机械切片作为完成态 | 机械切片没有减少认知复杂度，只是把大文件换了文件名。 |
| DEC-R1200-003 | 将 required gate 接入 Rust 文件体量检查 | 继续仅靠 PRD/人工评审提醒 | 当前规则已写在文档里但未形成持续阻断，治理已失效。 |
| DEC-R1200-004 | 先治 `chain_runtime`、`runtime_live`、`viewer` 三个高风险域 | 按字母序或随机顺序清债 | 这些文件最大、耦合最重，也是新功能最容易继续堆积的入口。 |
| DEC-R1200-005 | 测试文件允许少量尾债基线，但必须冻结并计划退场 | 强行要求单批把全部测试文件也降到 1200 以下 | 测试体量大且覆盖关键入口，允许少量尾债能降低重构风险，但不能放任不管。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 是否明确说明本期解决什么问题：第 1 章给出“规则回弹、机械切片、长期机制缺失”的问题定义。
  - ✔ 是否定义成功指标（可量化）：SC-1~SC-5、NFR-R1200-1~8 明确了数量、时长、覆盖率和完成条件。
  - ✔ 是否与公司/项目阶段目标一致：与 engineering 模块“控制技术债、建立稳定门禁”的目标一致。
  - ✔ 是否说明优先级来源：基于当前 32 个生产文件与 15 个测试文件超限的现状，以及高风险入口文件分布。
- 用户与场景（Who / When）:
  - ✔ 是否明确目标用户是谁：工程维护者、开发者、评审者、QA/发布维护者。
  - ✔ 是否区分主用户与边缘用户：主用户为维护者/开发者/评审者，QA/发布为验证链路相关角色。
  - ✔ 是否定义使用场景：提交前、评审、治理批次、季度复盘四类场景已定义。
  - ✔ 是否说明频率与关键路径：在 User Scenarios & Frequency 与 Critical User Flows 中明确。
- 范围定义（Scope Control）:
  - ✔ 是否列出本期功能清单：基线、门禁、touch-and-shrink、split-part 禁止、burn-down、评审结论均已列出。
  - ✔ 是否明确 Out of Scope：Non-Goals 已列出不做的一次性全仓重构、`third_party` 修改等。
  - ✔ 是否避免隐性功能：功能矩阵对字段、行为、状态和权限均已明确。
  - ✔ 是否有版本拆分说明：第 5 章给出 MVP / v1.1 / v2.0 / v2.1。
- 功能规格（What）:
  - ✔ 每个功能是否描述完整：功能矩阵逐项说明。
  - ✔ 是否有交互流程说明：Critical User Flows 已覆盖核心路径。
  - ✔ 是否明确字段定义：功能矩阵列出扫描、基线、touch-and-shrink、批次任务的关键字段。
  - ✔ 是否描述所有按钮行为：本专题无 UI 按钮，改为脚本/门禁动作行为说明。
  - ✔ 是否定义状态变化逻辑：`pass/fail`、`planned -> in_progress -> verified -> closed` 等已定义。
  - ✔ 是否描述排序规则 / 计算规则：按新增违规、行数、生产优先等规则排序。
  - ✔ 是否明确权限控制逻辑：谁可执行、谁可更新基线、谁可做最终评审均已说明。
- 异常与边界（Edge Cases）:
  - ✔ 网络异常如何处理：本专题不涉及网络调用，改为脚本误扫和热修例外等边界处理。
  - ✔ 空数据如何展示：若扫描无新增违规则门禁通过；基线和 delta 规则仍保持。
  - ✔ 权限不足如何反馈：工程 owner / reviewer 权限边界已写明。
  - ✔ 接口超时如何处理：脚本执行时长目标与 required gate 时延预算已定义。
  - ✔ 并发冲突如何处理：同一 god module 并发治理需阻断并重拆责任面。
  - ✔ 数据异常如何兜底：扫描范围、排除目录、迁移中间态等兜底已说明。
- 非功能需求（NFR）:
  - ✔ 是否定义性能要求：NFR-R1200-1。
  - ✔ 是否定义兼容性要求：Linux/macOS 可执行。
  - ✔ 是否定义安全要求：Security & Privacy 已覆盖。
  - ✔ 是否定义数据规模预期：以当前 32 + 15 超限文件为治理规模基线。
  - ✔ 是否定义可扩展性约束：禁止把债务转移到新 god file 或复制定义中。
- 可测试性（Testability）:
  - ✔ 是否定义验收标准：AC-1~AC-6。
  - ✔ 是否定义完成标准：SC、AC 和 project 批次共同构成 done。
  - ✔ 是否定义数据验证方式：脚本扫描、required/full 回归、索引和 devlog 回写。
  - ✔ 是否定义回归影响范围：Traceability 表已列出。
- 逻辑一致性（Consistency）:
  - ✔ 是否存在逻辑冲突：未发现明显冲突；“测试尾债允许冻结”与“生产代码必须归零”边界已区分。
  - ✔ 是否存在目标与设计不匹配：目标直接对应基线、门禁和 burn-down 设计。
  - ✔ 是否存在自相矛盾：未发现。
  - ✔ 是否与历史版本冲突：与旧 round3 形成补充纠偏，而非重复宣称其仍有效。
- 依赖与影响分析（Impact）:
  - ✔ 是否明确依赖系统：脚本、CI、工程文档、关键大文件入口均已列出。
  - ✔ 是否明确接口依赖：`scripts/ci-tests.sh`、`scripts/doc-governance-check.sh` 等明确。
  - ✔ 是否评估影响模块：runtime、viewer、launcher、engineering 均已覆盖。
  - ✔ 是否评估数据迁移：冻结基线与存量 `split_part` 迁移策略已说明。
  - ✔ 是否识别上线风险：第 5 章风险已列出。
- 决策透明度（Decision Record）:
  - ✔ 是否说明方案选择原因：DEC-R1200-001~005。
  - ✔ 是否记录被否决方案：一次性大拆、机械切片等已列为否决方案。
  - ✔ 是否有数据支持：以 32 个生产文件 + 15 个测试文件超限现状为证据。
- 文档树一致性与结构约束（Documentation Architecture）:
  - ✔ 本 PRD 是否明确归属于某个模块目录：`doc/engineering/` 根下专题。
  - ✔ 是否符合文档树层级规范：按 `*.prd.md / *.design.md / *.project.md` 三件套落位。
  - ✔ 是否重复定义已有模型：未重复定义 runtime/viewer 内部模型，只描述治理边界和调用点。
  - ✔ 是否清晰标注跨模块依赖：Integration Points 已给出。
  - ✔ 是否遵守抽象层级：本文聚焦治理目标和规则，实施细节下沉到 design/project。
  - ✔ 是否保证依赖可追溯性：Traceability、module project、索引与 devlog 路径均已定义。
- 总体 Gate 结果: 🟢 Ready
