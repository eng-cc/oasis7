# oasis7 Runtime：Observer 同步源策略化设计

- 对应需求文档: `doc/p2p/observer/observer-sync-source-mode.prd.md`
- 对应项目管理文档: `doc/p2p/observer/observer-sync-source-mode.project.md`

## 1. 设计定位
定义 Observer 同步源策略化主设计，统一同步源选择、切换、回退和健康判断规则。

## 2. 设计结构
- 选源策略层：定义 observer 在不同环境下的同步源优先级。
- DHT 组合层：以 `HeadSyncSourceModeWithDht` 表达网络+DHT、路径索引及二者有界回退，不改写基础同步语义。
- 切换状态层：维护同步源切换、恢复和失败状态机。
- 健康判定层：依据 lag、可达性和一致性判断同步源健康。
- 治理观测层：把策略口径、指标和日志沉淀为运维入口。

## 3. 关键接口 / 入口
- 同步源策略配置
- DHT 组合策略与双错误上下文
- 切换状态机
- 健康判定信号
- observer 运维读数

## 4. 约束与边界
- 同步源切换必须保持数据一致性优先。
- 状态机要可回放、可解释。
- 回退只由前置网络/DHT 错误触发；二次失败必须保留两段诊断信息。
- 不在本专题扩展新的 observer 身份体系。

## 5. 设计演进计划
- 先冻结主策略和优先级。
- 再补切换与健康判定。
- 最后联动统一 metrics/observability 权威。
