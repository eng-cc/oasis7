# 启动器可用性与体验硬化设计（2026-03-08）

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.project.md`

## 1. 设计定位
定义 launcher 在 native/web 双端的可用性加固方案：修复源码直跑路径、禁用态可解释性、参数编码、stop 语义、移动端布局与首次配置引导，并将主界面收敛到高频操作。

## 2. 设计结构
- 路径回退层：`oasis7_web_launcher` 为静态目录提供候选路径回退与明确报错。
- 状态表达层：链未就绪/已禁用/stop no-op 等状态通过稳定语义与文案反馈给 UI。
- 请求编码层：explorer/transfer/search 统一使用编码函数，避免特殊字符破坏请求。
- 界面收敛层：高频操作保留在主面板，低频配置收口到高级配置弹窗。
- 引导修复层：遇到阻断配置时弹出可编辑引导窗，首次进入自动触发一次轻量引导。

## 3. 关键接口 / 入口
- `oasis7_web_launcher/runtime_paths.rs`
- `oasis7_web_launcher/control_plane.rs`
- `oasis7_client_launcher/src/config_ui.rs`
- `oasis7_client_launcher/src/launcher_core.rs`
- `oasis7_client_launcher/src/app_process.rs`
- `oasis7_client_launcher/src/app_process_web.rs`

## 4. 约束与边界
- native/web 必须复用同一套状态语义和参数编码规则。
- stop 空操作不得覆盖最后一次失败态或未启动态语义。
- 主界面首屏优先保证状态、启停、日志入口可见，不追求展示全部配置字段。
- 本专题不新增敏感数据采集，也不改变权限边界。

## 5. 设计演进计划
- 先补路径回退、禁用提示与参数编码等底层可用性问题。
- 再优化移动端布局与 favicon 噪声，完成跨端回归。
- 最后将低频配置和阻断修复引导收口为渐进披露体验。
