# world-simulator PRD 分册：Web-first 与 LLM 测试证据模板

审计轮次: 6
## 目标
- 固化 Viewer Web-first 闭环（S6）与 LLM 链路（S8）的统一证据模板，减少“跑了测试但不可复核”的记录漂移。
- 提供可直接复制的结果卡片格式；`TASK-WORLD_SIMULATOR-003` 保留为原模板来源，不要求新建同类任务。

## 适用范围
- Web-first 闭环：`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- LLM 链路压力/覆盖：`testing-manual.md` 的 S8（`scripts/llm-longrun-stress.sh`）

## 候选、范围与结论边界

本页是记录模板，不是测试选择器或发布授权。适用层级与 gate 取自[总手册 S6](../../../../testing-manual.md#testing-s6-qa-receiver)及对应 S8 合同；来源到场景的记录边界见 [QA source-to-scene](../../../testing/prd.md#qa-infrastructure-source-scene-method)。只填写本次选中范围，不要求每次 smoke 执行所有 full 场景，也不增加审批层。

每张卡绑定 source/base/实际 tested tree、构建产物、独立 attempt、环境窗口、入口/会话与适用合同（精确路径/锚点和版本）。构建记录说明产物确由该源码生成；main 前进或 PR 合入不能把旧运行改名为新树的实跑。未执行、未证明和确实不适用分别填写；不适用须有合同依据，不能用 N/A 隐藏必需证据缺失。需要世界效果、receipt/root 或真实 Provider 的结论，必须另有对应权威 oracle 与真实环境证据。

## 模板 A：Web-first 闭环证据卡（S6）
```md
### S6 Web-first 闭环证据卡
- 候选：source OID / base OID / tested tree / build ID 或产物 digest：
- 构建来源 / freshness 记录：
- attempt ID / 执行时间窗口：
- 适用合同路径、锚点、版本 / scope / tier / 所需 gate：
- 实际场景与断言 / 未执行或未证明范围：
- 执行日期：
- 执行者：
- 环境：OS / Browser / Node / 是否 headed / desktop 与 narrow 实际 viewport
- 玩家访问模式 / execution lane / provider 身份与 mock/real 分类：
- 本次 harness/producer 的入口与 session（脱敏，不写 token/密钥）：
- 启动命令：
  - `env -u RUSTC_WRAPPER cargo run -p oasis7_launcher_backend --bin oasis7_game_launcher -- ...`
- 闭环命令：
  - `agent-browser --headed open "$URL"`
  - `agent-browser wait --load networkidle`
  - `agent-browser snapshot -i`
  - `agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)"`
  - `agent-browser screenshot output/playwright/viewer/<name>.png`
  - `agent-browser console | tee output/playwright/viewer/console.log`
- Surface observation（不是完整 S6 或 release verdict）：
  - 页面加载：pass/fail/unknown
  - `window.__AW_TEST__` 可用（仅适用的显式测试构建）：pass/fail/unknown/not_applicable（依据）
  - console error = 0：pass/fail
  - 截图产物存在：pass/fail
- 证据路径：
  - 截图：`output/playwright/viewer/<name>.png`
  - console：`output/playwright/viewer/console.log`（或等价重定向日志）
- 实际交互、前后状态与结果断言：
- 适用的权威 source/journal/API oracle、原始产物与判定（未执行则明确标注）：
- 该 scope/tier 的结论：pass/fail/blocked/unknown（适用 gate 的证据卡链接）
- 未证明范围与 blocker：
```

以上浏览器命令是采集示例；`__AW_TEST__` 仅用于手册规定的显式测试产物，不能据此给 release 包注入测试 API。截图、console 和页面/API 可用只证明 surface observation；完整 S6 要满足所选层级的交互与环境条件，playability 或发布仍需相应更高层证据。

## 模板 B：LLM 链路证据卡（S8）
```md
### S8 LLM 链路证据卡
- 候选：source OID / base OID / tested tree / build ID 或产物 digest：
- 构建来源 / freshness 记录：
- attempt ID / 执行时间窗口：
- 适用合同路径、锚点、版本 / scope / tier / 所需 gate：
- 实际场景与断言 / 未执行或未证明范围：
- 执行日期：
- 执行者：
- 场景：
- 玩家访问模式 / execution lane / provider 身份与 mock/real 分类（不记录凭据）：
- 关键参数：
  - ticks:
  - release-gate/profile:
  - runtime gameplay bridge: on/off
- 执行命令：
  - `./scripts/llm-longrun-stress.sh --scenario <scenario> --ticks <n> --out-dir .tmp/llm_stress/<run>`
- 人工执行判定（不是 summary/report 字段）：
  - run verdict: succeeded / failed / unknown
  - metric gate verdict: passed / failed / not_evaluated / unknown
  - wrapper 最终退出码：
  - 判定依据：完整 wrapper stdout/stderr 的相关输出
- 指标摘要（真实来源）：
  - summary.txt: parse_errors / llm_errors / active_ticks
  - summary.txt: action_kinds_total / action_kind_counts
  - report.json: trace_counts.parse_errors / trace_counts.llm_errors / active_ticks
- 证据路径：
  - ` .tmp/llm_stress/<run>/report.json`
  - ` .tmp/llm_stress/<run>/summary.txt`
  - ` .tmp/llm_stress/<run>/run.log`（demo 输出）
  - wrapper stdout/stderr：放在独立 attempt 目录
- 结论（仅本次场景、参数、Provider 与时间窗口）：
- 未证明范围与 blocker：
```

summary/report 不提供 `run.status` 或 `metric_gate.status`。stress 可能写好 summary 后才在阈值检查失败，因此产物存在不等于 gate 通过：wrapper 最终退出 0 且要求的检查全部执行才填 passed；执行失败时 gate 为 not_evaluated 或已有明确失败；阈值失败填 failed 并保留 wrapper stderr，不能用 demo run.log 代替。缺最终退出/日志时填 unknown。

离线核对分别覆盖完整成功、demo 运行失败、生成 summary 后阈值失败、缺少退出结果四种卡片；这只验证取值与判定来源，不证明真实模型行为。

<a id="模板-c合并发布口径s6--s8"></a>
## 模板 C：限定范围的合并结论（S6 + S8）
```md
### Web + LLM 合并结论
- 候选 source/base/tested tree、build 与证据卡身份是否匹配（不匹配不得合并为 pass）：
- 合并 scope / tier / 场景 / 时间窗口及合同版本：
- Web-first（S6）：pass/fail/blocked/unknown/not_applicable（依据与证据卡链接）
- LLM 链路（S8）：run verdict + metric gate verdict（原始证据卡链接）
- 未执行 / 未覆盖 / 不适用项及其依据：
- 阻塞项：
- 限定样本结论：pass / fail / blocked / unknown
- 是否评估发布：not_evaluated / evaluated
- 仅 evaluated 时：适用发布判据、其余必需 gate 与证据、实际 go/no-go 记录链接：
```

两张卡通过不能独立推出 release go。发布判定须引用同候选/构建、适用范围及所有必需 gate 的实际证据；可在既有 [release evidence bundle](../../../testing/templates/release-evidence-bundle-template.md) 中汇总，不新建并行审批。未评估发布保持 `not_evaluated`；必需证据缺失不能填 go。S6 surface pass、S8 产物存在、旧候选 pass 均不替代上述匹配。

## 必填检查
- 已执行的 S6 browser 卡至少包含 1 张截图和 1 份 console 日志，并满足实际选中 scope/tier 的全部断言；提前失败保留失败记录，不伪造产物。
- S8 包含本次实际生成的 `report.json + summary.txt + run.log` 及 wrapper stdout/stderr/最终退出码；提前失败缺产物时保留失败记录，不伪造正常文件。
- 失败时记录失败原因；复跑另开 attempt 并链接旧失败，没有复跑就写未执行，不能要求补造复跑结论。
- 候选/构建、实际范围、适用判据及未证明项是否齐全；不把必需证据的 unknown/blocked 改成 pass。
