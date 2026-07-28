# oasis7 Runtime：奖励结算切换到网络共识主路径原生交易设计

- 对应需求文档: `doc/p2p/node/node-reward-settlement-native-transaction.prd.md`
- 对应项目管理文档: `doc/p2p/node/node-reward-settlement-native-transaction.project.md`

## 1. 设计定位
定义奖励结算切换到网络共识主路径原生交易的设计，确保奖励结果通过原生交易在共识链路上完成确认。

## 2. 设计结构
- 奖励交易化层：`oasis7_chain_runtime` collector 把 epoch report 与 mint records 转换为 `ApplyNodePointsSettlementSigned`，而非直接写资产状态。
- 共识提交层：沿网络共识主路径广播、确认与落账。
- 失败补偿层：处理交易失败、重试与重复提交守卫。
- 审计追踪层：记录奖励交易状态与结算结果。

## 3. 关键接口 / 入口
- 奖励原生交易模型
- 共识提交入口
- 失败补偿逻辑
- 奖励交易审计记录

## 4. 约束与边界
- 原生交易语义需与现有奖励口径一致。
- 重复提交不得造成重复结算。
- collector 只负责生成并提交本地原生交易；签名、守恒、预算和重复校验必须在 action/event 主路径执行。它不定义跨节点自动调度、观察轨迹 topic 或网络传输协议。
- 不在本专题扩展外部支付集成。

## 5. 设计演进计划
- 先定义奖励原生交易结构。
- 再接共识提交与确认。
- 最后补齐失败补偿和回归。
