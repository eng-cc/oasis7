# 玩家访问模式产品契约（2026-03-19）

- 父模块 PRD: [`玩家入口与发行 PRD`](prd.md)
- 产品模块总入口: [`doc/product/README.md`](../README.md)
- 配对产品设计: [`玩家访问模式产品契约设计`](player-access-mode-contract-2026-03-19.design.md)
- 产品追踪边界: [`玩家访问模式产品契约追踪`](player-access-mode-contract-2026-03-19.project.md)
- 对应项目管理文档: `doc/product/player-entry-distribution/player-access-mode-contract-2026-03-19.project.md`

## 目标

- 为受支持的玩家访问路径提供唯一、可读的产品 taxonomy。
- 防止兼容名称和执行方式被误作新的玩家入口或公开 claim。

## 产品承诺

玩家能够辨认自己进入的是哪一种受支持的产品入口，并据此理解可主张的体验边界：`viewer` 是正式 Web/UI 入口，`pure_api` 是正式无 UI 入口。两者服务同一产品叙事，不能把执行方式、渲染兼容名或 provider 名称误作新的玩家入口。

## 范围与 taxonomy

| 分类 | 产品语义 | 不应误读为 |
| --- | --- | --- |
| `viewer` | 玩家通过 Web/UI 进入和观察受支持体验 | 一个独立的渲染后端或调试模式 |
| `pure_api` | 玩家或自动化消费者通过无 UI 入口使用受支持体验 | Web 体验的替代验收或隐藏的第三入口 |
| execution lane | 说明同一产品入口的执行、观测或验证方式 | 玩家访问模式 |
| compatibility alias | 兼容旧名称或实现迁移的说明 | 当前产品入口或公开 claim |

`software_safe`、`player_parity`、`headless_agent` 和历史 provider 名称只能按上述附加维度或兼容说明理解；它们不增加玩家访问模式。

## 接口 / 数据

产品入口使用 `viewer`、`pure_api`、execution lane 和 compatibility alias 四类术语；专业域以各自权威文档提供实现、证据和状态。

## 里程碑

本专题的产品收口是：当前入口、公开说明和专业域证据采用一致的访问模式语言；后续实现演进不改变此处的产品分层边界。

## 风险

若公开说明、专业域证据或兼容说明将附加维度升格为玩家入口，玩家可用性和发布 claim 会重新出现双真值。

## Claim 边界

- 任何玩家、QA、发布或对外结论必须先声明 `viewer` 或 `pure_api`，再补充其执行环境或兼容信息。
- 一种入口的证据不能替代另一种入口的体验或可用性结论。
- 根 [`README.md`](../../../README.md) 仍拥有当前公开状态和 claim envelope；本专题不宣布新版本、可用性或发布就绪。

## 专业域权威

| 主题 | 权威文档 | 产品层使用方式 |
| --- | --- | --- |
| Viewer、Launcher、Agent/provider 与访问实现 | [`doc/world-simulator/prd.md`](../../world-simulator/prd.md) | 提供入口实现合同和可用性证据 |
| 玩法与 pure API 体验规则 | [`doc/game/prd.md`](../../game/prd.md) | 提供玩家规则与体验验收 |
| 测试、发布证据与门禁 | [`doc/testing/prd.md`](../../testing/prd.md) | 提供验证机制和证据判断 |

## 跨域产品验收

- 玩家和评审者可从当前入口辨认 `viewer` 与 `pure_api` 的差异、适用边界和禁止误读。
- 专业域证据可明确绑定到其中一个访问模式，不把执行 lane 或兼容 alias 包装为独立产品。
- 公开说明、产品入口与专业域术语不产生相互矛盾的模式 claim。

## 决策背景与非目标

本专题保留“双模式、claim-first、兼容名不升格”为产品决策背景。它不定义 Viewer、provider、runtime、Launcher、命令行、失败处理、测试步骤或发布任务；这些事项必须回到对应专业域和 GitHub task evidence。
