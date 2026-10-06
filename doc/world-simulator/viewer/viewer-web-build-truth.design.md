# Viewer Web Build Truth and Legacy Core Split 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-build-truth.prd.md`
- 未完成的 facade 拆分债务见 `viewer-frontend-structure-standard.prd.md#当前已登记的结构债务`；具体 task 状态由 GitHub task issue evidence 维护。

审计轮次: 1

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | Canonical `viewer.html` and claim fixture generate the served HTML, JS, and evidence-page compatibility outputs in dist; checkout has no tracked root compat copies. | [Canonical to alias generation](viewer-web-build-truth.design.md#canonical-to-alias-generation) | Viewer build/finalize scripts generate canonical-derived aliases. | Does not change public URL behavior, viewer UI behavior, bundle contents, or Pixel World runtime behavior. |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | Delivery copies consume only finalized dist outputs. | [Dist copy contract](viewer-web-build-truth.design.md#dist-copy-contract) | Viewer dist copy and delivery scripts. | Does not alter the generated files or public URL behavior. |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | Stale-dist browser fallback preserves the same compatibility routes. | [Browser rebuild fallback](viewer-web-build-truth.design.md#browser-rebuild-fallback) | Agent-browser rebuild helper and generated dist. | Does not prove production hosting/CDN delivery or headed browser rendering. |

## 设计概览
- 保留 `legacy_core.js` 作为稳定 facade import path，但将主要实现拆到 `software_safe_src/` 新子模块。
- `vite` / finalize 流改为产出 canonical `viewer.js`，并由 finalize 从 canonical inputs 生成 dist compat `software_safe.js` 与 evidence page。
- `dist/pixel-world-bridge/` 继续由 finalize flow 统一生成；dist / bundle / freshness helper 全部基于 canonical 产物复制 compat alias，而不是反向复制。浏览器运行时仍以 served-dist 相对路径 `./pixel-world-bridge/` 加载。

## 模块拆分
- `legacy_core.js`
  - 只负责组装导出面、调用子模块 factory、保留兼容入口。
- `software_safe_src/*_module.js`
  - 按 state/auth/gameplay/rendering 等职责拆分实现。
  - 允许使用 factory + dependency injection，避免 ESM 循环引用。

## 产物关系
- Canonical:
  - `viewer.html` 源码页面文件
  - `viewer.js` 生成 bundle
  - `dist/pixel-world-bridge/*` 生成 runtime
- Compat alias:
<a id="canonical-to-alias-generation"></a>
<a id="viewer-html-dist-compat"></a>
  - dist `software_safe.js`
  - 从 canonical 页面生成到 dist / bundle 的 `software_safe.html`
  - 从 canonical claim fixture 生成到 dist 的 `software_safe_first_agent_claim_evidence.html`

<a id="dist-copy-contract"></a>
## 脚本改造
- `vite.software-safe.config.mjs`
  - 产出 canonical `viewer.js`
- `finalize-software-safe-build.mjs`
  - 将 canonical `viewer.html` 复制到 dist，并在 dist 中生成 compat `software_safe.html`
  - 复制 canonical bundle 到 `viewer.js`
  - 在 dist 中生成 compat `software_safe.js` 与 claim evidence page；不在源码根目录保留 aliases
  - 继续生成 `dist/pixel-world-bridge/*`
- `copy-viewer-web-dist.sh` / `package-viewer-web-delivery.sh`
  - 将 finalize 生成在 dist 的 compat aliases 纳入 copy 与发布产物
- `run-viewer-web.sh` / `build-game-launcher-bundle.sh`
  - 继续从 fresh dist 提供 canonical 页面和 compat URL
<a id="browser-rebuild-fallback"></a>
- `agent-browser-lib.sh`
  - stale-dist fallback 在 finalize 后从 dist 复制 compat aliases
- `bundle-freshness-lib.sh`
  - 把 canonical `viewer.js` 纳入 freshness scope

## 风险控制
- 先保留 facade import path，避免一次性改动 `main.jsx` / `pixel_world_host.jsx` / tests 的所有 import。
- compat alias 改成显式 wrapper，避免因为 alias 文件也被重新生成而产生第二份 bundle 真值。
- 不在本轮修改 `viewer` / `software_safe` taxonomy，只收口生成产物和源码边界。

## 11. 验证与证据

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source or ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | [Canonical to alias generation](viewer-web-build-truth.design.md#canonical-to-alias-generation) | Prove the HTML, JS, and evidence aliases derive from canonical inputs and no root compat copies are required. | Run [`viewer-compat-aliases.test.mjs`](../../../crates/oasis7_viewer/scripts/viewer-compat-aliases.test.mjs); its isolated fixture checks alias transformations, current claim fields, and root-copy absence. | Current task evidence and generated alias fixture output. | Does not prove that copy helpers or production hosting deliver the files. |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | [Dist copy contract](viewer-web-build-truth.design.md#dist-copy-contract) | Prove the delivery copy helper copies generated dist aliases while canonical viewer files remain its source inputs. | Run [`copy-viewer-web-dist.test.sh`](../../../scripts/copy-viewer-web-dist.test.sh); its fixture generates aliases under dist and checks copied HTML, JS, and evidence outputs. | Current task evidence and copied dist fixture output. | Does not prove production hosting/CDN delivery or headed browser rendering. |
| [Viewer Web Build Truth PRD §4.2](viewer-web-build-truth.prd.md#42-generated-artifact-single-source-of-truth) | [Browser rebuild fallback](viewer-web-build-truth.design.md#browser-rebuild-fallback) | Prove the stale-dist rebuild fallback serves the compatibility route and preserves current claim fields. | Run [`agent-browser-viewer-dist-freshness-test.sh`](../../../scripts/agent-browser-viewer-dist-freshness-test.sh); its isolated rebuild fixture checks compatibility HTML/JS/evidence paths and canonical claim fields. | Current task evidence and rebuilt dist fixture output. | Does not prove production hosting/CDN delivery or headed browser rendering. |
