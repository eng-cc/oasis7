# 客户端启动器区块链浏览器（当前 authority）

> 本文是启动器区块链浏览器的当前需求 authority。它收敛五组 2026-03/04 专题中已经实现或记录的只读查询、七个业务视图与状态呈现合同；本轮已完成语义回填和活跃引用修复，日期化源三件套随之退役删除，追溯使用 Git 与 GitHub task issue evidence。

- 对应设计: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.design.md`
- 历史迁移、验收与 task 状态：GitHub task issue evidence。

## 目标

- launcher 的 native/Web 同源 explorer 让玩家、测试和运维人员在同一只读表面查看当前可得的链概览和查询结果，而不必依赖命令行日志。

## 范围

- 既有概览覆盖高度、节点/world 标识、最近区块或执行哈希及已有状态计数；七个业务视图为 `Blocks`、`Txs`、`Search`、`Address`、`Contracts`、`Assets`、`Mempool`。
- 本文只收敛当前 launcher 表现层、既有控制面代理与可观察结果；不定义世界规则、余额/交易计算、索引策略或 runtime 状态演化。

## 当前合同

| 表面 | 当前可观察行为 | 交互与状态 | 边界 |
| --- | --- | --- | --- |
| 概览 | 展示当前已有高度、身份、哈希与交易状态聚合 | 刷新期间、空结果、未就绪或结构化失败可见 | 只读；字段与含义以 runtime/控制面当前实现为准 |
| `Blocks` / `Txs` | 分页浏览已有区块或交易结果，并可查看选中详情 | 保留现有排序、过滤、翻页与选择语义 | 不保证完整历史或跨重启全量数据 |
| `Search` | 按已有 `height`、block/transaction hash、`action_id` 或账户标识检索并跳转 | 空查询、无命中和失败保持可解释 | 不创造新的查询类别或索引 |
| `Address` / `Contracts` / `Assets` | 查看已有账户、系统合约目录/详情、主 token 供应和持仓等查询结果 | 输入校验、分页、空态和错误态沿用既有结构化语义 | NFT 或其他能力以返回的显式 capability 状态为准 |
| `Mempool` | 查看已有 `accepted` / `pending` 交易并按现有过滤条件浏览 | 筛选、清空、分页和选中详情保持当前请求参数语义 | 不改变交易状态或提交路径 |

native 与 Web 复用同一 launcher UI 与控制面语义；Web 经既有 `/api/chain/explorer/*` 代理访问 runtime 的只读查询。任何请求中的 `loading`、无数据的 `empty`、链未就绪的 `not_ready` 及 `error_code + error` 结果必须在当前 explorer 表面可见，不能以外层日志替代。

## 接口 / 数据

- runtime 已有只读 explorer 查询位于 `/v1/chain/explorer/*`；launcher Web 代理位于 `/api/chain/explorer/*`。
- 现有查询范围包括 overview、blocks/block、txs/tx、search、address、contracts/contract、assets 与 mempool；本文不将路径可达性解释为任何对外服务可用性。
- 查询结果、排序、分页 cursor、capability 标志与错误字段仍以当前 runtime/控制面返回为真值；launcher 不在本地补造、重算或持久化这些结果。

## 不作出的承诺

- 历史专题里的 “public chain” 或 “mainnet-grade” 是当时查询范围或信息架构的名称，不构成 live mainnet、网络 readiness、公开发布或对外服务承诺。
- explorer 的 receipt-like 字段、交易状态或展示结果不是结算、最终性、资产可转移性、validator 参与或任何链规则正确性的证明。
- 不承诺无重置、跨重启永久可用、全历史 archive、Merkle 证明、钱包管理、跨链、EVM 解码或新的写操作。
- 本次 authority 迁移不修改 UI、DOM、协议、轮询、查询频率、runtime、控制面或数据保留行为；将可见结果外推为玩家支持、运维服务或发布承诺均不成立。

## 里程碑

- 已完成：早期 panel、P0、P1、UX 与信息架构专题分别记录了既有只读查询、七视图与状态呈现。
- 本轮：将已记录的共同合同收敛为 stable authority，修复默认入口，并删除五组已吸收的源三件套。
- 后续：任何 API、数据保留、可用性、公开发布或玩家控制面变化，均须在独立任务中重新定界并取证。

## 风险

- 数据不可用、链未就绪、查询失败或最近窗口不足时，explorer 的结果可能为空或失败；界面必须忠实呈现状态，不能把空白或历史缓存表示为成功。
- 同一只读 UI 不能证明 runtime、共识、结算或网络性质；若文档未跟随实现更新，容易再次产生 authority 漂移。
- 历史命名只能通过 Git 与 GitHub task evidence 追溯；任何将 “public chain” / “mainnet-grade” 外推为当前承诺的解释都无效。

## 验收与追溯

- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`git diff --check`。
- 行为或可见界面变更必须重新按 `testing-manual.md` 的命中测试层执行；触达可见表面时由 game_visual_interaction_designer 给出验收，并按 S6 获取 desktop/mobile 浏览器证据。
- 五组已吸收的源三件套已删除；删除后的追溯使用 Git 与 GitHub task issue evidence。
