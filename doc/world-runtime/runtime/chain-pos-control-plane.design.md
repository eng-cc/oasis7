# oasis7 Runtime：链 PoS 时间与控制面合同设计

审计轮次: 1

- 上游规格：`doc/world-runtime/runtime/chain-pos-control-plane.prd.md`
- 上游项目：`doc/world-runtime/runtime/chain-pos-control-plane.project.md`

## 1. 设计目标

把 PoS 时间语义收敛为一个可由 runtime、launcher、status 与恢复链路共同引用的专业合同：共享时间锚决定 logical tick/slot/phase，runtime 只在确定的相位和完整 guard 下推进，恢复只恢复已持久化后果。

## 2. 结构与数据流

```text
validated defaults / CLI
          |
          v
slot-clock genesis + timing config
          |
          v
logical_tick -> slot + tick_phase -> guard evaluation -> proposal or idle
          |                                  |
          v                                  v
missed counters / snapshot ------------> status (read-only)
          |
          v
restart / checkpoint / replay
```

- `chain_pos_defaults` 提供受校验的仓库默认参数；CLI 可以显式覆盖，但不能接受零值 duration/ticks 或越界 phase。
- timing calculation 只依赖共享输入与整数公式，避免把本地 scheduler jitter 转化为共识语义。
- strict-lag alignment 只生成当前 tick 的临时恢复边沿；它不是持久状态，也不是 replay 中的“补块”指令。
- snapshot/reconcile 保存结果性 counters 和已选配置；状态读取从 immutable snapshot 构造，不可借由读取写入或延长任何观测 episode。

## 3. 边界与错误处理

- future/stale proposal 或 attestation：在进入后续共识处理前拒绝；不得以本地 wall-clock 差异放宽窗口。
- launcher 输入错误：在启动/构造参数阶段给出字段级错误；不以隐式默认值掩盖显式非法值。
- restart 数据不一致：保留持久状态为诊断依据并 fail closed；不得重置 counters 来伪造健康进度。
- 无新 committed action 的 status polling：仅返回当前状态，不得被解释为 logical world progression。

## 4. 演进约束

任何涉及 slot 算法、准入窗口、validator 规则、参数 ABI 或跨节点配置源的修改都需要 TPM 触发相应 runtime/系统/链角色联审。此设计不授权改动协议经济或节点运维流程。
