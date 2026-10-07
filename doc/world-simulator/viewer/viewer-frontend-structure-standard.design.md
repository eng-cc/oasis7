# Viewer Frontend Structure Standard Design

- 对应需求文档: `doc/world-simulator/viewer/viewer-frontend-structure-standard.prd.md`
- 结构债务 task 与验证记录：GitHub task issue evidence。

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Viewer Frontend Structure Standard PRD §范围](viewer-frontend-structure-standard.prd.md#范围) | Keep canonical Viewer sources and generated dist-only compatibility artifacts distinct so checkout cannot form a second source of truth. | [Generated artifact strategy](viewer-frontend-structure-standard.design.md#canonical-and-generated-artifacts) | Viewer build/finalize and dist copy scripts own generated outputs. | Does not alter UI direction, runtime protocol, WASM ABI, or hosted auth semantics. |
| [Viewer Frontend Structure Standard PRD §范围](viewer-frontend-structure-standard.prd.md#范围) | Delivery helpers consume generated outputs from dist. | [Dist copy contract](viewer-frontend-structure-standard.design.md#dist-copy-contract) | Viewer dist copy and delivery scripts. | Does not alter UI direction, runtime protocol, WASM ABI, or hosted auth semantics. |

审计轮次: 1

## 设计概览
- 采用轻量 Feature-Sliced-inspired 分层作为 review 语言，不强制一次性目录迁移。
- 采用 Solid component model 作为 JSX 组件拆分基线：组件接收 display inputs 和 callbacks，避免在组件体内混入 transport、storage 和 generated artifact logic。
- 采用 Google HTML/CSS style-guide 风格作为 raw HTML/CSS 卫生底线：shell 清晰、语义稳定、格式一致。
- 采用 ESLint/Prettier 作为后续可自动化 gate 候选，但本轮只落文档标准，不新增 tooling。
- 继续以 `viewer-web-single-source-build-truth` 为 generated artifact 真值：canonical Viewer HTML/JS/claim fixture -> dist compat `software_safe.*`。

## 分层模型

```text
app
  pages
    widgets
      features
        entities
          shared
```

- `app` 负责 mount、query/bootstrap、top-level orchestration。
- `pages` 负责 Viewer entry composition。
- `widgets` 负责世界、目标、命令、诊断、聊天等屏幕区域。
- `features` 负责用户动作和可独立测试的交互能力。
- `entities` 负责 agent/location/resource/event/session/runtime-health display models。
- `shared` 负责纯 helper、constants、fixtures、storage adapters、browser capability helpers。

## 目录演进
当前 `software_safe_src/` 已经包含多个职责模块。新任务不需要先创建完整目录树；当同类文件超过 3 个或 import 边界开始泄漏时，再引入目录：

```text
software_safe_src/
  app/
  pages/
  widgets/
  features/
  entities/
  shared/
```

迁移时每次只抽一个 coherent boundary，并保留已有 import path 或提供明确 public API。

## Facade 策略
- `legacy_core.js` 可以继续作为稳定 facade，但不得继续吸收新职责。
- 可接受的 facade 内容：export assembly、compat wrapper、factory wiring、thin initialization delegation。
- 不可接受的 facade 内容：新 DOM rendering tree、新 transport loop、新 gameplay display model、新 generated artifact writer。

## 组件策略
- 组件名使用业务名词，不使用 `CommonThing`、`Panel2`、`WidgetNew` 这类临时名。
- 组件 props 应偏 display model，而不是 raw runtime snapshot 全量透传。
- 组件内可以做轻量 derived view state；复杂 derivation 下沉到 pure helper 或 entity display model。
- 大组件先按用户可见区域拆，再按数据/服务边界拆。

## State / Service 策略
- WebSocket、hosted auth、storage、runtime loader、metrics、crypto、locale preference 属于 service/state module。
- service module 应优先暴露 factory 或小 public API，便于测试注入。
- Browser global 和 localStorage key 只能集中在 owning module 或 shared constants 中。

## Test 策略
- 抽出的 pure helper 需要 narrow unit test。
- 抽出的 component 需要 Solid UI test 或被现有 entry UI test 覆盖到 stable DOM anchor。
- 从大测试文件抽 fixture/helper 时，行为断言必须留在原测试或迁到同名 test 文件。
- 测试选择器是合同；更名要同步测试、manual 或 automation helper。

<a id="canonical-and-generated-artifacts"></a>
## Generated Artifact 策略
- 手写 source truth 只在 `software_safe_src/**`、canonical HTML shell、build/finalize scripts 和 docs 中维护。
- `viewer.js` 由 Vite/finalize 生成后作为 canonical checked-in bundle。
- `dist/software_safe.js` 是 compat alias，不承载独立实现；compat claim evidence page 由 canonical claim fixture 生成。
- `dist/pixel-world-bridge/**` 由 finalize flow 生成或同步，不作为普通 JS module 手改。

<a id="dist-copy-contract"></a>
- Delivery copy helpers consume generated dist aliases; checkout-root compatibility copies are not inputs.

## 外部规范适配
- Feature-Sliced Design: 采纳 layer/slice/public API 思想，不照搬完整目录和术语。
- Solid docs: 采纳组件、props、JSX 的框架语义。
- Google HTML/CSS style guide: 采纳 shell/HTML/CSS 卫生原则。
- ESLint/Prettier: 作为后续 gate 候选，不在本次 docs-only 任务新增依赖。
- Atomic Design: 仅作为视觉系统词汇参考，不作为代码架构主轴。
- Airbnb JS/React: 可参考 JS/JSX 命名和 lint 思路，不作为 Solid 主规范。

## 风险控制
- 不用阈值制造大爆改；阈值只在 touched files 上触发收敛或豁免记录。
- 不把 checked-in generated bundle 当作普通 source file 审查。
- 不用机械切片替代真实职责边界。
- 不让视觉 IA 文档承担代码结构治理职责；本标准只管 source/module/test/artifact 边界。

## 11. 验证与证据

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source or ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Viewer Frontend Structure Standard PRD §范围](viewer-frontend-structure-standard.prd.md#范围) | [Generated artifact strategy](viewer-frontend-structure-standard.design.md#canonical-and-generated-artifacts) | Verify generated aliases derive from canonical inputs and root compat copies are unnecessary. | Run [`viewer-compat-aliases.test.mjs`](../../../crates/oasis7_viewer/scripts/viewer-compat-aliases.test.mjs); the isolated fixture checks deterministic transformations and root-copy absence. | Current task evidence and generated alias fixture output. | Does not prove delivery packaging or production hosting. |
| [Viewer Frontend Structure Standard PRD §范围](viewer-frontend-structure-standard.prd.md#范围) | [Dist copy contract](viewer-frontend-structure-standard.design.md#dist-copy-contract) | Verify delivery packaging consumes the dist aliases and preserves public compatibility paths. | Run [`copy-viewer-web-dist.test.sh`](../../../scripts/copy-viewer-web-dist.test.sh); the isolated fixture checks copied aliases. | Current task evidence and copied dist fixture output. | Does not prove production hosting/CDN delivery or headed browser rendering. |
