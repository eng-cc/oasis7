# 客户端启动器引导配置与可用性（当前 authority）

> 本文是 launcher 的语言/配置清晰度、事实性状态反馈和响应式自引导可用性的当前需求 authority。它吸收四组 2026-03 日期化专题中仍可由当前实现支持的表现层合同；活跃入口已迁移至本三件套，历史 source triplet 删除后仅通过 Git 与 GitHub task issue evidence 追溯。

- 对应设计: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.design.md`
- 对应项目: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md`

## 目标

- 让操作者在既有 launcher 表面理解当前语言、配置问题、可执行动作和受阻原因，而不是从灰色控件或原始日志猜测状态。
- 以当前事实呈现 blocked、未就绪或 observer 等状态；状态可见不等同于游戏、网络、公开加入或发布 readiness。
- 用响应式、可跳过的既有引导把配置修复和下一步提示放在当前操作面，但不创造新的玩家权能或世界规则。

## 范围

- 当前 launcher 的中英文界面选择、启动前配置检查、问题列表、可达配置入口和已有高频/高级配置分层。
- 当前 native/Web 表面中的状态、禁用原因、既有下一步提示、响应式信息可读性与自引导组件。
- 本文只记录 launcher 表现和控制边界；不收敛 runtime、chain execution-world、stale-world recovery、数据持久化或网络语义。

## 当前合同

| 范围 | 当前 launcher 表现 | 不得外推为 |
| --- | --- | --- |
| 语言与配置 | 中英文文案、必填/格式问题和预检阻断在既有表面可见；配置问题应有当前可达的修复入口 | 所有语言、字段或历史配置规则永久不变 |
| 状态与受阻反馈 | 启动、链、provider 或功能入口的当前 `ready`、未就绪、disabled、blocked/error 与 observer 语义必须按现有状态呈现 | 操作成功、runtime 健康、网络连通、公开玩家资格或发布 readiness |
| 引导与下一步 | 当前 onboarding、任务提示、错误卡或 CTA 只能编排现有动作；引导可跳过/重置，并必须保留专家配置入口 | 自动修复、自动启动、强制流程、远端运营承诺或新的控制能力 |
| 响应式可用性 | 状态、配置问题和关键操作在当前 native/Web 表面中保持可读、可访问；具体视觉验收以视觉角色规格为准 | 固定布局、所有设备适配或逐像素 native/Web 一致 |
| 编辑与请求状态 | 本地配置编辑态和当前请求/错误状态应保持可解释，不能以无关快照或空白覆盖用户可见问题 | 后台持续监控、任意并发保证、持久化恢复或数据完整性证明 |

## 接口 / 数据

- 行为事实来自 `oasis7_client_launcher` 的语言状态、配置问题收集、launcher 状态、self-guided 组件及其既有 native/Web 表现层；具体字段、CTA 和存储细节仍以当前实现为真值。
- 角色值中的 `observer` 仅表示当前配置/状态语义，不构成 node admission、验证者资格、公开链访问、结算或数据完整性证明。
- 本文不新增 API、GUI Agent 动作、CLI 参数、遥测、DOM、轮询、数据保留或 runtime 接口。

## 里程碑

- 已完成：四组历史专题分别记录了语言与必填配置、可用性修补、全量体验收口和自引导工作。
- 本轮：将可验证且仍适合默认入口的共同表现合同收敛为 stable authority，修复本 scope 的路由并退役十二个 source 文件。
- 后续：任何新的配置规则、引导动作、状态语义、可见交互或浏览器表面变更，均须在独立任务中重新定界和取证。

## 风险

- 状态标签或引导文案若脱离当前实现，可能把 blocked/observer 误读为可用或可加入；本页不以历史任务完成记录支撑当前结论。
- 过度引导可能妨碍专家操作，因此引导只组织既有动作，并保留当前高级配置入口。
- 不同窄屏、native 与 Web 表面可能有视觉差异；可见变更需由 game_visual_interaction_designer 定义验收并按 `testing-manual.md` S6 取证。

## 不作出的承诺

- 不主张日期化来源中的任务完成、测试、截图、默认值、脚本、端点、演示、自动补默认值、配置画像、计数或持久化仍是当前事实。
- 不主张玩家可玩性、hosted login、strong-auth、public join、node admission、chain/runtime readiness、mainnet、结算、最终性、公开服务或 release readiness。
- 本轮不修改代码、UI、DOM、协议、runtime 或测试；S6 截图不适用。未来触达可见表面时必须重新取得 desktop/mobile 浏览器和视觉验收证据。

## 验收与追溯

- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`python3 scripts/product-doc-governance-check.py`、`git diff --check`。
- 下列四组 source triplet 已退役删除：`i18n-required-config-2026-03-02`、`availability-ux-hardening-2026-03-08`、`full-usability-remediation-2026-03-08`、`self-guided-experience-2026-03-08`。历史需求、设计、测试和完成记录仅通过 Git 与 GitHub task issue evidence 追溯。
