# 本地启动会话连续性与恢复

## 文档身份

- 所属产品模块：玩家入口与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[`local-launch-session-continuity-and-recovery.design.md`](local-launch-session-continuity-and-recovery.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 公开状态权威：[`README.md`](../../../README.md)
- 专业域权威：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`Launcher 子域入口`](../../world-simulator/launcher/README.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文承载受支持本地 Launcher 路径的会话连续性与恢复产品承诺。它补充[访问模式与发行就绪](access-modes-and-release-readiness.prd.md)，不创建第三种玩家访问模式，也不定义启动器、链运行时、持久化或 Web/WASM 的实现合同。

配对产品设计的 canonical 路径为 `doc/product/player-entry-distribution/local-launch-session-continuity-and-recovery.design.md`。

## 1. 目标

玩家通过受支持的本地 Launcher 进入已声明的 `viewer` 或 `pure_api` 路径时，能够理解当前会话是在准备、可用、阻塞、停止还是恢复中；发生中断、陈旧状态或配置问题时，能够采取真实的恢复下一步，而不是把已启动进程、旧本地状态、浏览器页面或设置编辑误认为当前世界已经健康、已连接或可玩。

## 2. 产品承诺

### 2.1 可理解的本地会话生命周期

- 正式 Launcher 表面把本地路径组织为一条可理解的生命周期：选择当前入口与模式、准备或启动、确认当前可用或 blocked 状态、停止，以及恢复或重新进入。
- Launcher 是已声明 primary mode 的交付、控制和过渡表面；本地 session、运行 lane、provider、浏览器或执行环境不会形成新的玩家模式、世界或发行等级。
- 已启动的子过程、可见页面、旧输出或局部健康状态不能单独证明玩家已进入权威世界、已获得身份/权限或已达到可玩结论。

### 2.2 中断、停止与恢复

- 会话被停止、启动失败、发现陈旧本地状态或无法恢复时，玩家能够读到当前可信状态、主要原因和适用的下一步，例如重试、修复配置、重新进入、等待或安全停止。
- 重新启动、清理或恢复本地 session 必须与权威世界结果分开表达：它们可能改变本地进程或会话上下文，不自动撤销、确认、保存或重放玩家行动与世界后果。
- 没有安全恢复路径时，surface 明确返回受支持的入口或 blocked 决策面；不得静默复用旧 authority、旧 session 或旧配置结果。

### 2.3 设置与结果边界

- Launcher 中的设置、LLM/provider 配置或 Web 表面配置是待验证的输入；编辑、保存、重载或入口可达不等于配置已经被运行时应用，更不等于 Agent 行为、世界行动、结算、持久化或可玩性已经成功。
- 当前 authority 支持的设置结果必须真实地区分已暂存、已接受、已应用、被拒绝、blocked 或未知，并给出适用的恢复下一步。未支持的配置或凭据能力不得展示为可用。
- 诊断应帮助玩家判断本地 session 是否可恢复，但不得泄露凭据、敏感本地路径或把工程诊断提升为玩家世界事实。

### 2.4 Web 表面与跨入口连续性

- 当 Launcher 的 Web 表面不可初始化、不可轮询或发生致命错误时，玩家得到可理解的失败或恢复状态，而不是无限加载、假连接或假成功。
- native 与 Web 可以使用不同的表现、存储和恢复机制，但不能因入口不同而把配置编辑、本地 session 或诊断成功表达为不同的 primary-mode 或权威世界结果。

## 3. 组合验收

- LSC-1：受支持 Launcher 路径的代表性启动、blocked、停止和恢复样例能区分本地会话状态、当前 primary mode、权威世界状态与下一决策。
- LSC-2：陈旧状态、失败启动和安全停止样例不会静默恢复旧 authority 或把本地恢复表达为世界行动已经确认；没有安全路径时返回真实 blocked 或重新进入入口。
- LSC-3：设置样例能区分草稿/暂存、接受、实际应用、拒绝或未知；配置编辑、控制面可达和本地保存不代签 Agent/world 结果。
- LSC-4：Web 表面错误或不可用样例不会被表述为已连接、已启动或可玩；适用的恢复路径与当前可信 session 状态保持一致。
- LSC-5：Launcher、runtime、Viewer、QA 和公开 claim 的证据绑定同一候选与 primary mode；局部进程、浏览器或自动化 green 不能单独成立发行或可玩结论。

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| LSC-1 / LSC-2 | runtime_engineer / viewer_engineer / qa_engineer | Launcher 专业 authority；`doc/world-runtime/prd.md`; `doc/testing/prd.md` | 启动、blocked、停止、恢复与安全返回的当前表面和运行事实对账 | test_tier_required |
| LSC-3 | agent_engineer / viewer_engineer / qa_engineer | Launcher 设置专业 authority；`doc/world-simulator/prd.md`; `doc/testing/prd.md` | 配置阶段、秘密安全、结果和恢复负例 | test_tier_required |
| LSC-4 | wasm_platform_engineer / viewer_engineer / qa_engineer | Launcher Web/WASM 专业 authority；`doc/testing/prd.md` | 当前 Web 初始化、错误可见性与恢复闭环 | test_tier_required |
| LSC-5 | runtime_engineer / viewer_engineer / qa_engineer / liveops_community | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同候选 primary-mode、权威结果与公开 claim 审计 | test_tier_full |

## 4. 范围与非目标

覆盖本地 Launcher session 的玩家可理解状态、恢复边界、设置结果边界及 Web 表面失败的真实表达。

不定义 CLI、进程树、signals、端口、链/共识拓扑、execution-world 路径或文件、陈旧状态分类算法、存储/恢复协议、LLM/provider schema 与凭据、WASM 时钟、轮询/API、浏览器错误签名、打包、测试命令或历史 verdict。本分册不承诺无损恢复、自动迁移、跨目录/跨设备状态保留、自动更新、普适 Web 可用性、生产 custody、公开发行或新的玩家入口模式。

## 5. 权威与冲突处理

| 产品层拥有 | 专业域拥有 |
| --- | --- |
| 本地会话的可理解生命周期、恢复选择、设置与世界结果的语义边界、跨入口不误导 | `world-simulator` Launcher 文档拥有进程、控制面、设置、Web 表面与实现合同；runtime/P2P 文档拥有权威执行、存储、恢复和治理；testing/QA 拥有验证；根 README 拥有公开 claim |

发生冲突时，本分册不得以产品承诺改写启动器或运行时合同；由 `runtime_engineer`、`viewer_engineer`、`wasm_platform_engineer`、`agent_engineer`、QA 与产品 owner 形成显式裁决。

## 6. 接口 / 数据

产品层只定义 `primary mode → 本地 session 状态 → blocker/设置阶段 → 权威结果 → 恢复下一步` 的玩家阅读语义。进程、配置、凭据、存储、时钟、Web API 和诊断字段由对应专业 authority 定义。

## 7. 里程碑

1. 形成稳定 PRD 与 design，并由模块入口可达。
2. 六组历史 Launcher 专题的产品语义归位，专业实现合同回填 current authority。
3. 仅在专业后继和活跃引用修复后删除历史源文件。

## 8. 风险

- 把本地进程、浏览器页面或旧输出误写成进入权威世界或可玩成功。
- 把配置保存或请求受理误写成 Agent/runtime 已应用或世界行动已经成立。
- 删除仍承担启动器、运行时、存储、Web/WASM 或验证真值的专业源文件。
