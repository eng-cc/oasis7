# Viewer Web 单一真值构建与 Legacy Core 拆分（2026-05-19）设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.project.md`

审计轮次: 1

## 设计概览
- 保留 `legacy_core.js` 作为稳定 facade import path，但将主要实现拆到 `software_safe_src/` 新子模块。
- `vite` / finalize 流改为产出 canonical `viewer.js`，并由 finalize 同步生成 compat `software_safe.js`。
- `pixel-world-bridge/` 继续由 finalize flow 统一生成；dist / bundle / freshness helper 全部基于 canonical 产物复制 compat alias，而不是反向复制。

## 模块拆分
- `legacy_core.js`
  - 只负责组装导出面、调用子模块 factory、保留兼容入口。
- `software_safe_src/*_module.js`
  - 按 state/auth/gameplay/rendering 等职责拆分实现。
  - 允许使用 factory + dependency injection，避免 ESM 循环引用。

## 产物关系
- Canonical:
  - `software_safe.html` 源码页面文件
  - `viewer.js` 生成 bundle
  - `pixel-world-bridge/*` 生成 runtime
- Compat alias:
  - `software_safe.js`
  - dist / bundle 中复制出的 `software_safe.html`

## 脚本改造
- `vite.software-safe.config.mjs`
  - 产出 canonical `viewer.js`
- `finalize-software-safe-build.mjs`
  - 复制 canonical bundle 到 `viewer.js`
  - 生成 compat `software_safe.js`
  - 继续生成 `pixel-world-bridge/*`
- `run-viewer-web.sh` / `agent-browser-lib.sh` / `build-game-launcher-bundle.sh`
  - 改成先复制 canonical `viewer.js`，再复制 compat alias
- `bundle-freshness-lib.sh`
  - 把 canonical `viewer.js` 纳入 freshness scope

## 风险控制
- 先保留 facade import path，避免一次性改动 `main.jsx` / `pixel_world_host.jsx` / tests 的所有 import。
- compat alias 改成显式 wrapper，避免因为 alias 文件也被重新生成而产生第二份 bundle 真值。
- 不在本轮修改 `viewer` / `software_safe` taxonomy，只收口生成产物和源码边界。
