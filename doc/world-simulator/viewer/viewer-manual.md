# oasis7 Viewer 使用说明书（兼容入口）

审计轮次: 9

## 说明
- canonical 手册已迁移到 `doc/world-simulator/viewer/viewer-manual.manual.md`。
- 本文件保留为 legacy 兼容入口，供历史任务、旧引用和静态镜像基线继续跳转。
- 后续需要新增或修改 Viewer 操作说明时，统一编辑 `viewer-manual.manual.md`。

## 当前入口
- Viewer 使用手册：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 系统级测试总手册：`testing-manual.md`
- Web UI 闭环操作手册：`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`

## 全览图缩放切换（2D）
- 2D 视角支持“细节态 / 全览图态”自动切换。
- 默认进入细节态，便于看清 Agent 与局部关系。
- 缩放到阈值后自动进入全览图态，显示简化标记并隐藏部分细节几何。
- 切回近景后自动恢复细节显示。

## 文本可选中/复制面板
- 支持打开可选中文本面板，用于复制状态、事件、诊断与详情文本。
- 面板使用系统快捷键复制（macOS `Cmd+C` / Windows/Linux `Ctrl+C`）。
- 若遮挡视图可在顶部控制区切换显示/隐藏。

## UI 语言切换
- Viewer 支持中文/英文 UI。
- 通过顶部语言控件切换后即时生效。
- 若启用本地配置持久化，重启后会保持最近一次选择。
- 语言切换不改变协议字段，仅改变显示文案。

## 推荐调试场景
- 细粒度 location 渲染观察：`asteroid_fragment_detail_bootstrap`
- 常规联调：`llm_bootstrap`
- 双区域对比：`twin_region_bootstrap`

## 开采损耗可视化
- 当 location 含有 `fragment_budget` 时，Viewer 会按剩余质量比例缩放体量（体积比例映射到半径立方根）。
- 剩余越少，location 视觉半径越小；为避免完全不可见，存在最小可视半径保护。
- 详情面板会显示：`Fragment Depletion: mined=<x>% remaining=<a>/<b>`。

## 常见问题排查
- Web 页面空白：等待 `trunk` 首轮编译完成，确认访问端口与 `run-viewer-web.sh` 参数一致。
- `agent-browser` 启动失败：先检查 `agent-browser --version` 与本地浏览器依赖是否可用。
- Console 有 wasm 报错：先看 `output/playwright/viewer/state.json` 的 `lastError`；若命中 `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` / `SwiftShader`，按图形链路失败处理。
- 看不到细节：先用 `F` 或自动聚焦查看局部，并优先保持 2D / Web 主链路；只有在 hold-only 3D 排查时才显式切到 3D。
- 自动聚焦无效：确认 target 存在，或先使用 `first_fragment` 排除 ID 输入问题。
- 连接失败：检查 `oasis7_viewer_live` 是否运行、端口与 viewer 地址是否一致。

## 参考文档
- `doc/world-simulator/viewer/viewer-location-fine-grained-rendering.prd.md`
- `doc/world-simulator/viewer/viewer-auto-focus-capture.prd.md`
- `doc/world-simulator/viewer/viewer-web-closure-testing-policy.prd.md`
- `doc/world-simulator/viewer/viewer-selection-details.prd.md`
- `doc/world-simulator/viewer/viewer-right-panel-module-visibility.prd.md`
- `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md`
- `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md`
- `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md`
- `doc/world-simulator/viewer/viewer-copyable-text.prd.md`
- `doc/scripts/viewer-tools/capture-viewer-frame.prd.md`（native fallback）

## Fragment 元素分块渲染（默认开启）
- 目标：把 location 的 fragment 分块默认显示出来，并按主导元素显示不同颜色。
- 当前行为：不再渲染 location 外层几何与标签，仅保留逻辑锚点；frag 分块始终渲染。
- 选择交互：点击 frag 后，详情面板会显示所属 `location`（ID 与名称）。
- 配置说明：已移除 frag 渲染开关与对应环境变量，不再支持按开关隐藏 frag。
