# 客户端启动器可用性与体验硬化（2026-03-08）项目管理文档

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-027) [test_tier_required]: 完成专题 PRD 建模、验收标准冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-027) [test_tier_required]: 落地启动器可用性与 UX 硬化修复（静态目录回退、wasm 禁用原因提示、查询参数编码、stop no-op 状态语义、移动端布局、favicon 噪声）并完成跨端回归。
- [x] T2 (PRD-WORLD_SIMULATOR-027) [test_tier_required]: 将主界面低频配置收口到“高级配置”弹窗，保留高频操作面板与错误摘要入口，并完成 native/web 回归。
- [x] T3 (PRD-WORLD_SIMULATOR-027) [test_tier_required]: 实现“启动阻断配置引导”弹窗（点击启动即弹出可编辑输入框，首次进入执行一次轻量引导），并完成 native/web 回归。
- [x] T4 (PRD-WORLD_SIMULATOR-027) [test_tier_required]: 对齐 `oasis7_web_launcher` / `oasis7_client_launcher` 的 `viewer_static_dir=web` 预校验语义，使其按目标 `launcher_bin` bundle 相对路径解析，并补回归测试。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_web_launcher/runtime_paths.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs`
- `crates/oasis7_client_launcher/src/platform_ops.rs`
- `crates/oasis7_client_launcher/src/main.rs`
- `crates/oasis7_client_launcher/src/config_ui.rs`
- `crates/oasis7_client_launcher/src/launcher_core.rs`
- `crates/oasis7_client_launcher/src/app_process.rs`
- `crates/oasis7_client_launcher/src/app_process_web.rs`
- `crates/oasis7_client_launcher/index.html`
- `testing-manual.md`

## 状态
- 最近更新：2026-03-11
- 当前阶段: completed
- 当前任务: 无
- 备注: T0/T1/T2/T3/T4 已完成；`PRD-WORLD_SIMULATOR-027` 可用性、引导体验与 bundle 相对静态目录预校验闭环交付。
