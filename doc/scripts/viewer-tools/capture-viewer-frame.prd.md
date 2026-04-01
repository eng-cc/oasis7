# Capture Viewer Frame（Agent UI截图闭环调试脚本）

- 对应设计文档: `doc/scripts/viewer-tools/capture-viewer-frame.design.md`
- 对应项目管理文档: `doc/scripts/viewer-tools/capture-viewer-frame.project.md`

审计轮次: 4


> 状态说明（2026-02-15）：该脚本已降级为 **native fallback**。默认闭环路径为 Web 端：`scripts/run-viewer-web.sh + agent-browser`。

## 目标
- 提供一个面向 agent 的 native 图形链路应急入口：`启动服务 -> 启动虚拟显示与 viewer -> 抓图 -> 留存日志`。
- 在 Web 闭环无法复现问题时，提供可回放的本地截图证据。
- 默认在每次新调试开始前清空 `.tmp/`，避免历史残留影响判断。
- 增加平台识别与分支实现，使脚本在 Linux 与 macOS 都能完成最小截图闭环。
- 在 macOS 录屏权限受限时，使用 Bevy 内置截图能力完成窗口截图，避免依赖系统录屏授权。
- 增加预热编译与可调超时，降低首次运行或重场景下的截图超时概率。

## 范围
- **范围内**：
  - 新增脚本 `scripts/capture-viewer-frame.sh`。
  - 自动启动 `oasis7_viewer_live`、`oasis7_viewer` 并抓取 `root.png`/`window.png`。
  - Linux 分支：使用 `Xvfb + xwininfo + ffmpeg` 完成无头抓图。
  - macOS 分支：优先使用 viewer 进程内截图（Bevy `Screenshot::primary_window`），不依赖 `screencapture` 权限。
  - viewer 新增可选“自动截图并退出”能力，通过环境变量控制输出路径与触发时机。
  - 输出统一日志与窗口几何信息（`live_server.log`/`viewer.log`/`window_geom.txt`）。
  - 默认清空 `.tmp/`，可通过 `--keep-tmp` 保留。
- **范围外**：
  - 不作为默认闭环路径（默认闭环改为 `scripts/run-viewer-web.sh + agent-browser`）。
  - 不提供自动鼠标键盘交互回放。
  - 不替代完整 UI 自动化测试（仍以现有单测/联测为准）。

## 接口 / 数据
- 默认 Web 闭环入口（本脚本外）：
  - `./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173`
  - agent-browser CLI：`open/snapshot/console/screenshot`
- 脚本路径：`scripts/capture-viewer-frame.sh`
- 典型调用：
  - `./scripts/capture-viewer-frame.sh`
  - `./scripts/capture-viewer-frame.sh --scenario llm_bootstrap --addr 127.0.0.1:5023 --viewer-wait 8`
- 可选参数：
  - `--scenario` / `--addr` / `--display` / `--width` / `--height` / `--viewer-wait` / `--llm` / `--keep-tmp`
  - `--auto-focus-target`：启动 viewer 后自动聚焦目标（如 `first_fragment`、`location:frag-1`、`agent:agent-0`）
  - `--auto-focus-radius`：自动聚焦半径覆盖值
  - `--auto-focus-keep-2d`：自动聚焦时保持 2D（当前默认行为）
  - `--auto-focus-force-3d`：仅在 hold-only 3D 排查时强制切换 3D
  - `--auto-select-target`：启动后自动选中目标（如 `first_agent`、`agent:agent-0`）
  - `--automation-steps`：启动后自动执行步骤（如 `mode=2d;focus=agent:agent-0;zoom=0.8;select=agent:agent-0`）
  - `--capture-max-wait`：覆盖 macOS 内置截图最大等待秒数（默认自动推导）
  - `--no-prewarm`：跳过预热编译（默认会预热 `oasis7_viewer_live` 与 `oasis7_viewer`）
- viewer 内置截图环境变量：
  - `OASIS7_VIEWER_CAPTURE_PATH`：截图输出文件路径（PNG）。
  - `OASIS7_VIEWER_CAPTURE_DELAY_SECS`：最短等待秒数（默认 2 秒）。
  - `OASIS7_VIEWER_CAPTURE_MAX_WAIT_SECS`：无快照时的最大等待秒数（由脚本按 `viewer_wait` 自动推导，可被 `--capture-max-wait` 覆盖）。
  - （可选）`OASIS7_VIEWER_AUTO_FOCUS*`：脚本在传入 `--auto-focus-*` 时自动注入。
  - （可选）`OASIS7_VIEWER_AUTO_SELECT*`：脚本在传入 `--auto-select-target` 时自动注入。
  - （可选）`OASIS7_VIEWER_AUTOMATION_STEPS`：脚本在传入 `--automation-steps` 时自动注入。
- 输出目录：`.tmp/screens/`
  - `root.png`：整屏截图（macOS 内置截图模式下与 `window.png` 相同）
  - `window.png`：viewer 窗口截图
  - `live_server.log` / `viewer.log` / `xvfb.log`
  - `window_line.txt` / `window_geom.txt`

## 里程碑
- **M1**：输出脚本设计文档与项目管理文档。
- **M2**：实现脚本与参数解析，接入清空 `.tmp/` 机制。
- **M3**：更新 AGENTS/README/任务日志并完成运行验证。
- **M4**：补充 Linux/macOS 平台分支逻辑与依赖检查。
- **M5**：接入 viewer 内置自动截图并在 macOS 默认启用，绕过录屏权限约束。
- **M6**：接入自动聚焦参数，保证目标区域更易复现。
- **M7**：增加预热编译、可调超时与失败日志，降低截图超时风险。
- **M8**：策略切换后定位为 fallback（Web 闭环默认启用，native 保留应急能力）。

## 风险
- **路径分叉风险**：团队误把 fallback 当默认流程。
  - 缓解：在 AGENTS/手册中统一声明“Web 默认、native fallback”。
- **暂停策略漂移**：operator 脚本若继续默认切 3D，会与 `PRD-WORLD_SIMULATOR-041` 冲突。
  - 缓解：native fallback 默认保持 2D，只有显式传参时才进入 hold-only 3D 检查。
- **依赖差异**：Linux 与 macOS 命令能力不同，需要分别校验命令可用性。
- **截图时机**：若触发过早可能抓到“连接中”界面，需配合 `--viewer-wait` 与内置延迟参数。
- **渲染链路**：viewer 内置截图依赖 Bevy 渲染完成回调，若渲染异常可能导致截图未落盘。
- **资源占用**：脚本会启动 viewer/server，调试完成后必须清理后台进程（脚本已 trap 清理）。
- **启动时延**：默认预热编译会增加启动前等待；可通过 `--no-prewarm` 跳过。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
