# world-simulator PRD 分册：Web-first 与 LLM 测试证据模板

审计轮次: 6
## 目标
- 固化 Viewer Web-first 闭环（S6）与 LLM 链路（S8）的统一证据模板，减少“跑了测试但不可复核”的记录漂移。
- 为 `TASK-WORLD_SIMULATOR-003` 提供可直接复制的结果卡片格式。

## 适用范围
- Web-first 闭环：`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- LLM 链路压力/覆盖：`testing-manual.md` 的 S8（`scripts/llm-longrun-stress.sh`）

## 模板 A：Web-first 闭环证据卡（S6）
```md
### S6 Web-first 闭环证据卡
- 执行日期：
- 执行者：
- 环境：OS / Browser / Node / 是否 headed
- 启动命令：
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- ...`
- 闭环命令：
  - `agent-browser --headed open "$URL"`
  - `agent-browser wait --load networkidle`
  - `agent-browser snapshot -i`
  - `agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)"`
  - `agent-browser screenshot output/playwright/viewer/<name>.png`
  - `agent-browser console | tee output/playwright/viewer/console.log`
- 结果：
  - 页面加载：pass/fail
  - `window.__AW_TEST__` 可用：pass/fail
  - console error = 0：pass/fail
  - 截图产物存在：pass/fail
- 证据路径：
  - 截图：`output/playwright/viewer/<name>.png`
  - console：`output/playwright/viewer/console.log`（或等价重定向日志）
- 结论：
```

## 模板 B：LLM 链路证据卡（S8）
```md
### S8 LLM 链路证据卡
- 执行日期：
- 执行者：
- 场景：
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
- 结论：
```

summary/report 不提供 `run.status` 或 `metric_gate.status`。stress 可能写好 summary 后才在阈值检查失败，因此产物存在不等于 gate 通过：wrapper 最终退出 0 且要求的检查全部执行才填 passed；执行失败时 gate 为 not_evaluated 或已有明确失败；阈值失败填 failed 并保留 wrapper stderr，不能用 demo run.log 代替。缺最终退出/日志时填 unknown。

离线核对分别覆盖完整成功、demo 运行失败、生成 summary 后阈值失败、缺少退出结果四种卡片；这只验证取值与判定来源，不证明真实模型行为。

## 模板 C：合并发布口径（S6 + S8）
```md
### Web + LLM 合并结论
- Web-first（S6）：pass/fail（证据卡链接）
- LLM 链路（S8）：pass/fail（证据卡链接）
- 阻塞项：
- 发布结论：go / no-go
```

## 必填检查
- S6 至少包含 1 张截图和 1 份 console 日志。
- S8 包含本次实际生成的 `report.json + summary.txt + run.log` 及 wrapper stdout/stderr/最终退出码；提前失败缺产物时保留失败记录，不伪造正常文件。
- 若任一条失败，必须补“失败原因 + 复跑结论”。
