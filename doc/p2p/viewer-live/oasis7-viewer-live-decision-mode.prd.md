# `oasis7_viewer_live` 决策模式

> 本文是 `oasis7_viewer_live` LLM 默认值与 `--no-llm` observer/debug 边界的当前专业 authority。它收敛两组 2026-02 源三件套；历史变更仅从 Git 与 GitHub task evidence 追溯。

- 对应设计: `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.design.md`
- 对应项目: GitHub Issue / GitHub Project

## 目标

`oasis7_viewer_live` 默认启用 LLM 决策；`--llm` 是等价的显式开启，`--no-llm` 明确切换到 Script 决策。该 binary 是纯 viewer/observer 服务，链节点、release 和 runtime 控制面由 `oasis7_chain_runtime`（通常经 `oasis7_game_launcher`）拥有。

## 范围

本文只定义当前 CLI 决策模式及其证据边界，不新增决策模式、provider 编排、场景默认值、链控制面或 launcher 行为。

## 接口 / 数据

- CLI：`--llm`、`--no-llm`，以及现有观察服务 bind/Web bridge 参数。
- 配置：`CliOptions.llm_mode` 映射为 `ViewerLiveDecisionMode::{Llm, Script}`。
- 启动边界：launcher stack 的 LLM 可用性门控与 direct viewer 的 observer/debug 诊断边界。

<a id="vl-decision-contract"></a>
## 当前合同

| 调用 | 决策模式 | 可作为的证据 | 不可推断的结论 |
| --- | --- | --- | --- |
| 未传 flag 或 `--llm` | LLM | 已配置且可连通 provider 的 direct viewer 观察/调试；在适用的完整 launcher/Web 流程中再按对应证据判定 gameplay | 不单凭 binary 启动宣称 launcher、release 或 gameplay 通过 |
| `--no-llm` | Script | direct `oasis7_viewer_live` observer/debug 诊断 | formal gameplay、launcher 成功、release readiness 或 provider 可用 |

<a id="vl-last-flag"></a>
- `--llm` 与 `--no-llm` 同时出现时，线性解析以最后出现的 flag 为准。
<a id="vl-launcher-no-fallback"></a>
- launcher stack 和 `oasis7_game_launcher` 要求 active LLM；其 `--no-llm` 是负向路径，会 fail fast，不能把 Script 当成 launcher fallback。
<a id="vl-legacy-control-rejection"></a>
- 已移除的 `--release-config`、`--runtime-world`、`--node-*` 等 legacy 控制面必须拒绝，并引导到 `oasis7_chain_runtime` 或纯 viewer 参数。

## 风险与验证边界

- 无 LLM 的 Script 模式保留为 observer/debug 诊断能力，不得因它可启动而扩大为正式玩法承诺。
- CLI 参数、帮助文案、手册与实际解析必须同向；代码真值优先于早期专题中的过时 Script fallback 叙述。
- 受影响行为改动至少覆盖 `oasis7_viewer_live` 参数解析测试，并按 `testing-manual.md` 的实际变更路径补 Viewer/launcher/Web 验证；本轮仅做文档迁移，不产生 UI、DOM 或 browser 证据。

## 里程碑

- M1（已完成）：默认 LLM、显式 `--no-llm` 与参数解析测试落地。
- M2（本轮）：将当前语义收敛至稳定 authority 并修复入口路由。
- M3（已完成）：已完成引用审计并退役已吸收的 2026-02 源三件套。

## 追溯

- 当前实现与参数测试：`crates/oasis7_viewer_runtime/src/bin/oasis7_viewer_live.rs`。
- 操作与证据边界：`doc/world-simulator/viewer/viewer-manual.manual.md`、`scripts/worktree-harness.sh`。
- P2P observer/chain 控制面边界：`doc/p2p/prd.md`。

## 专业承接定位与产品关系

本节仅定位上述既有合同，不新增模式或改写原里程碑。CLI 的 Llm/Script 决策配置不等于产品的 viewer/pure_api primary mode。设计及验收关系见[需求分配](oasis7-viewer-live-decision-mode.design.md#21-需求承接与分配表)与[验证映射](oasis7-viewer-live-decision-mode.design.md#111-验证映射表)。

<a id="vl-observer-evidence"></a>
### observer/debug 证据边界

Trace refs: O7-REQ-f3f198ce-2afd-4487-a99e-9f371bdd8264；承接 [REQ-ENTRY-MODE-001](../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-mode-001) 中入口能力与证据不得互相代签的适用部分。上述[当前合同](#vl-decision-contract)及[风险与验证边界](#风险与验证边界)只允许 Script 作为 direct viewer observer/debug；LLM flag 或服务启动也不单独证明正式玩法、launcher 或 release readiness。primary mode、完整 parity 和页面可辨识性仍由产品、Viewer、Launcher 与 QA 的各自合同负责。

<a id="vl-launcher-product-boundary"></a>
### launcher 失败分类承接

Trace refs: O7-REQ-94ed85c5-c05d-4385-a919-687bfd66b270；承接 [REQ-ENTRY-MODE-002](../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-mode-002) 中前置失败不得用 fallback 伪装成功的适用部分。本条沿用[launcher 原合同](#vl-launcher-no-fallback)，只承接无 LLM 的 fail-fast 和 direct observer/debug 提示边界；不覆盖所有 provider、身份/会话、后端故障及 UI 恢复流程。

默认值、flag 顺序与 legacy 拒绝继续由本专业合同拥有；legacy 义务与 P2P 专业 [PRD-P2P-010/011](../prd.md#2-user-experience--functionality) 同向。本轮只增追踪定位；原 M1/M3 是历史里程碑，不等于当前候选测试记录。当前验证的覆盖与缺口必须按设计映射逐项记录。
