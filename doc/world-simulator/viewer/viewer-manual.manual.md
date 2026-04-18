# oasis7 Viewer 使用说明书

审计轮次: 9

## 文档定位
- 本文件是 Viewer 使用说明的 canonical `*.manual.md` 入口。
- 历史兼容路径 `doc/world-simulator/viewer/viewer-manual.md` 仅保留跳转说明，后续内容更新统一回写到本文件。
- 系统级测试分层与 suite 选择仍以 `testing-manual.md` 为权威总入口；本文件只负责 Viewer 专项操作与观察闭环。

## 目标
- 提供一份可直接操作的 Viewer 使用手册，覆盖启动、交互、自动聚焦、自动步骤与 Web 闭环。
- 统一人工调试与脚本闭环的命令入口，减少重复沟通成本。
- 当前项目阶段默认优先非 3D / `software_safe` / Web 主链路；3D 观察只作为 hold-only 补充路径。

## 适用范围
- 可视化客户端：`crates/oasis7_viewer`
- 联调服务端：`crates/oasis7 --bin oasis7_viewer_live`
- Web 闭环入口：`scripts/run-viewer-web.sh` + agent-browser CLI
- native fallback 脚本：`scripts/capture-viewer-frame.sh`
- 角色边界：Web 端定位为 Viewer（观察/调试/间接控制），不承担完整分布式节点职责；共识与复制由后端节点进程负责。
- 边界说明：本手册仅适用于 Viewer 页面（`oasis7_viewer_live` / Web Viewer），不适用于 `oasis7_web_launcher` / launcher Web 控制面；后者产品动作默认应走 GUI Agent，再用页面校验状态与字段。

## 快速开始

### 1）启动 live server（推荐）
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```
`oasis7_viewer_live` 当前默认走 LLM 模式，且正式 gameplay 要求已配置且可连通的 LLM provider。

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --no-llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

`oasis7_viewer_live` 现已统一为 runtime/world 链路（协议兼容输出 `WorldSnapshot/WorldEvent`），不再提供 simulator fallback 启动分支。
传 `--llm` 可进入正式 gameplay、prompt/chat 鉴权与控制闭环；`--no-llm` 仅用于观战/调试，`gameplay_action/prompt/chat` 会直接返回 `llm_mode_required` 或 `llm_init_failed`，`step/play` 则会返回带 `Blocked + error_code/error_message` 的 `ControlCompletionAck`。

### 2）启动 viewer
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

### 3）离线模式（仅查看本地 UI，不连服务端）
```bash
OASIS7_VIEWER_OFFLINE=1 env -u RUSTC_WRAPPER cargo run -p oasis7_viewer
```

### 4）浏览器模式（Bevy + wasm）
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```
- 打开浏览器访问：`http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011`
- 显式中文标准 Viewer：`http://127.0.0.1:4173/?render_mode=standard&ws=ws://127.0.0.1:5011&locale=zh`
- 显式英文标准 Viewer：`http://127.0.0.1:4173/?render_mode=standard&ws=ws://127.0.0.1:5011&locale=en`
- Web 端通过 `oasis7_viewer_live --web-bind` 提供的 WebSocket bridge 在线连接 live server（Viewer + 网关路径）。
- Web 端不直接运行 `oasis7_node` 的完整分布式协议栈（不承担 gossip/replication/共识职责）。
- 首次运行前需安装：
  - `trunk`（`cargo install trunk`）
  - `wasm32-unknown-unknown`（`rustup target add wasm32-unknown-unknown`）

### 5）software_safe 主入口语言切换
- `software_safe` 主入口现在支持 `locale=zh|en`（兼容 `language=zh|en`）初始化。
- 本地 `run-game-test.sh` / `run-producer-playtest.sh` 默认会给出带 `locale=zh` 的主入口 URL。
- 进入 `software_safe` 页面后，可直接用页面内的 `中文 / English` 按钮切换语言。
- 页面内同时会给出“打开标准 Viewer”入口，分别指向中文和英文的显式 bilingual Viewer URL。
- `software_safe` 页面当前已收口为纯实时模式：不再提供 `play/pause/step`、tick jump、回放推进等控件；页面只保留实时观察、事件流、canonical gameplay 摘要，以及带 auth/bootstrap 时的 `prompt/chat/rollback` 等正式交互。
- `Prompt Overrides` 已改为默认收起的“高级 Prompt 设置”项；只有在选中 Agent 后显式展开，才会显示 `preview/apply/rollback` 编辑表单。该开关会按浏览器本地存储记住上次状态，不影响 `__AW_TEST__.sendPromptControl(...)` 等自动化接口。

## runtime 事件/快照覆盖
- Live server 输出的 `WorldEvent` 会附带原始 runtime 事件载荷：`runtime_event`（JSON 透传）。
- 未映射的 runtime DomainEvent 会封装为 `WorldEventKind::RuntimeEvent { kind, domain_kind }`，保证事件覆盖不丢失。
- `WorldSnapshot` 增加 `runtime_snapshot` 字段，透传完整 runtime world state，便于回归与诊断。
- Viewer 协议过滤器支持 `RuntimeEvent`，可用于订阅/筛选未映射事件。

## 发行模式（P2P 推荐）

`oasis7_viewer_live` 当前为纯 Viewer live 服务，不再承载 `--release-config`、`--runtime-world` 与 `--node-*` 控制面参数。
P2P 发行建议使用 `oasis7_chain_runtime`（可由 `oasis7_game_launcher` / `oasis7_web_launcher` / `oasis7_client_launcher` 托管）锁定链参数，Viewer 仅保留 `--bind`、`--web-bind`、`--llm/--no-llm`；其中 `--no-llm` 只用于 observer/debug。

## 常用交互
- 鼠标拖拽：旋转/平移观察视角。
- 滚轮：缩放。
- `W/A/S/D`：移动相机视角（平移 `focus`，2D/3D 均可用；仅在光标位于 3D 视口且未占用文本输入时生效）。
- `2D/3D` 切换：在顶部按钮切换视角模式。
- 控制区（观察模式）：默认仅显示 `播放/暂停` 单按钮；点击 `高级调试` 后展开 `单步` 与 `跳转 0`。
- `F`：对“当前选中对象”执行聚焦（适合人工巡检细节）。
- `F8`：循环切换材质变体预设（`default -> matte -> glossy -> default`），用于快速对比 roughness/metallic 观感。
- 右侧综合面板：查看控制、状态、事件、分块、诊断等模块信息。
- 右侧综合面板支持 `隐藏面板/显示面板` 总开关：隐藏后右侧面板与 Chat 面板都不渲染，3D 区域最大化。
- 最右侧 Chat 面板：独立承载 Agent Chat，不与综合面板混排；顶部区域展开时显示，可通过模块可见性中的 `Chat` 开关隐藏。面板内提供可展开的“预设 Prompt”小区域：支持聊天预设编辑并一键填充输入框，同时可编辑 `system prompt`、`短期目标`、`长期目标`，并直接应用到当前目标 Agent。三个字段会直接预填充当前生效值（未设置 override 时为系统默认值），可直接编辑；展开区内容过高时支持内部滚动。
- Chat 输入：输入框聚焦时，`Enter` 直接发送；`Shift+Enter` 换行。

## 自动聚焦（Auto Focus）

### 启动时自动聚焦（环境变量）
- `OASIS7_VIEWER_AUTO_FOCUS=1`
- `OASIS7_VIEWER_AUTO_FOCUS_TARGET=<target>`
- `OASIS7_VIEWER_AUTO_FOCUS_FORCE_3D=1|0`（Viewer 环境变量默认 `1`；`capture-viewer-frame.sh` 默认会显式写成 `0`，仅在 hold-only 3D 检查时改回 `1`）
- `OASIS7_VIEWER_AUTO_FOCUS_RADIUS=<number>`（可选）

支持目标：
- `first_fragment`
- `first_location`
- `first_agent`
- `location:<id>`
- `agent:<id>`

示例：
```bash
OASIS7_VIEWER_AUTO_FOCUS=1 \
OASIS7_VIEWER_AUTO_FOCUS_TARGET=first_fragment \
OASIS7_VIEWER_AUTO_FOCUS_RADIUS=18 \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

## 自动步骤（Auto Select / Automation Steps）
- `--auto-select-target`：启动后自动选中目标（例如 `first_agent`、`agent:agent-0`）。
- `--automation-steps`：执行一组自动步骤（例如 `mode=2d;focus=agent:agent-0;zoom=0.8;select=agent:agent-0`）。
- 常用于截图回归，减少手工定位误差。
- 常用步骤键：
  - `wait=<seconds>`
  - `mode=2d|3d`
  - `focus=<target>|selection`（或 `focus_selection=current`）
  - `select=<target>`
  - `pan=x,y,z`
  - `zoom=<factor>`
  - `orbit=<yaw_deg>,<pitch_deg>`
  - `panel=show|hide|toggle`
  - `top_panel=show|hide|toggle`
  - `module=<controls|overview|chat|overlay|diagnosis|event_link|timeline|details>:<show|hide|toggle>`
  - `locale=zh|en|toggle`（或 `language=zh|en|toggle`）
  - `layout=mission|command|intel`
  - `chat=<agent_id>|<message>`（`message` 支持 `%xx` 文本解码）
  - `prompt_system=<agent_id>|<text|clear>`
  - `prompt_short=<agent_id>|<text|clear>`
  - `prompt_long=<agent_id>|<text|clear>`
  - `timeline_seek=<tick>`
  - `timeline_filter=<err|llm|peak>:<show|hide|toggle>`
  - `timeline_jump=<err|llm|peak>`
  - `material_variant=next|cycle`

示例：
```bash
./scripts/capture-viewer-frame.sh \
  --scenario llm_bootstrap \
  --addr 127.0.0.1:5131 \
  --auto-select-target first_agent \
  --automation-steps "mode=2d;focus=first_agent;zoom=0.8"
```

示例（round-1 语义补齐）：
```bash
./scripts/capture-viewer-frame.sh \
  --scenario llm_bootstrap \
  --addr 127.0.0.1:5131 \
  --automation-steps "panel=show;module=chat:show;select=first_agent;focus=selection;material_variant=next;wait=0.2"
```

示例（round-2 语义补齐）：
```bash
./scripts/capture-viewer-frame.sh \
  --scenario llm_bootstrap \
  --addr 127.0.0.1:5131 \
  --automation-steps "top_panel=hide;locale=en;layout=command;panel=show;module=chat:show;wait=0.2"
```

示例（round-3 语义补齐）：
```bash
./scripts/capture-viewer-frame.sh \
  --scenario llm_bootstrap \
  --addr 127.0.0.1:5131 \
  --automation-steps "layout=command;chat=agent-0|hello%20from%20automation;prompt_short=agent-0|Prioritize%20power%20stability;wait=0.2"
```

示例（round-4 语义补齐）：
```bash
./scripts/capture-viewer-frame.sh \
  --scenario llm_bootstrap \
  --addr 127.0.0.1:5131 \
  --automation-steps "layout=intel;timeline_filter=err:hide;timeline_jump=llm;timeline_seek=120;wait=0.2"
```

## 3D 渲染档位与精调（商业化精致度）

### 档位入口
- `OASIS7_VIEWER_RENDER_PROFILE=debug|balanced|cinematic`
- 默认：`balanced`
- 建议：先选档位，再做单项覆盖（避免一次性改太多参数导致定位困难）。

### 档位差异（默认值）
- `debug`：低几何复杂度 + 关闭 location 壳层 + 偏可读性材质 + 无阴影 + 轻后处理（`Reinhard`、无 Bloom）。
- `balanced`：中等几何复杂度 + 壳层开启 + 可读性材质 + 三点光默认比率 + `TonyMcMapface` + Bloom 默认开启。
- `cinematic`：高几何复杂度 + 质感材质策略 + 阴影开启 + 三点光更强调轮廓 + `BlenderFilmic` + 更强 Bloom 与色彩后处理。

### 资产层（Geometry/Shell）
- `OASIS7_VIEWER_ASSET_GEOMETRY_TIER=debug|balanced|cinematic`
- `OASIS7_VIEWER_LOCATION_SHELL_ENABLED=1|0`
- 外部 mesh 覆盖（可选，未配置时回退到内置基础几何）：
  - `OASIS7_VIEWER_AGENT_MESH_ASSET=<path#label>`
  - `OASIS7_VIEWER_LOCATION_MESH_ASSET=<path#label>`
  - `OASIS7_VIEWER_ASSET_MESH_ASSET=<path#label>`
  - `OASIS7_VIEWER_POWER_PLANT_MESH_ASSET=<path#label>`
  - `OASIS7_VIEWER_POWER_STORAGE_MESH_ASSET=<path#label>`
- 示例：
```bash
OASIS7_VIEWER_LOCATION_MESH_ASSET=models/world/location.glb#Mesh0/Primitive0 \
OASIS7_VIEWER_AGENT_MESH_ASSET=models/agents/worker.glb#Mesh0/Primitive0 \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

### 材质层（PBR/Fragment）
- `OASIS7_VIEWER_FRAGMENT_MATERIAL_STRATEGY=readability|fidelity`
- `OASIS7_VIEWER_FRAGMENT_UNLIT=1|0`
- `OASIS7_VIEWER_FRAGMENT_ALPHA=<0.05..1.0>`
- `OASIS7_VIEWER_FRAGMENT_EMISSIVE_BOOST=<>=0`
- `OASIS7_VIEWER_MATERIAL_AGENT_ROUGHNESS=<0..1>`
- `OASIS7_VIEWER_MATERIAL_AGENT_METALLIC=<0..1>`
- `OASIS7_VIEWER_MATERIAL_AGENT_EMISSIVE_BOOST=<>=0`
- `OASIS7_VIEWER_MATERIAL_ASSET_ROUGHNESS=<0..1>`
- `OASIS7_VIEWER_MATERIAL_ASSET_METALLIC=<0..1>`
- `OASIS7_VIEWER_MATERIAL_ASSET_EMISSIVE_BOOST=<>=0`
- `OASIS7_VIEWER_MATERIAL_FACILITY_ROUGHNESS=<0..1>`
- `OASIS7_VIEWER_MATERIAL_FACILITY_METALLIC=<0..1>`
- `OASIS7_VIEWER_MATERIAL_FACILITY_EMISSIVE_BOOST=<>=0`
- `OASIS7_VIEWER_MATERIAL_VARIANT_PRESET=default|matte|glossy`（可选，启动时指定材质变体预设；运行中可按 `F8` 切换）
- 外部颜色覆盖（可选，值为严格 `#RRGGBB`，非法值自动回退默认）：
  - `OASIS7_VIEWER_AGENT_BASE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_AGENT_EMISSIVE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_LOCATION_BASE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_LOCATION_EMISSIVE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_ASSET_BASE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_ASSET_EMISSIVE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_POWER_PLANT_BASE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_POWER_PLANT_EMISSIVE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_POWER_STORAGE_BASE_COLOR=<#RRGGBB>`
  - `OASIS7_VIEWER_POWER_STORAGE_EMISSIVE_COLOR=<#RRGGBB>`
- 外部贴图覆盖（可选，值为 `<path#label>`；web/native 需按运行时支持选择贴图格式，如 `png/ktx2`）：
  - 基础色（Albedo/Base Color）：
    - `OASIS7_VIEWER_AGENT_BASE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_LOCATION_BASE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_ASSET_BASE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_PLANT_BASE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_STORAGE_BASE_TEXTURE_ASSET=<path#label>`
  - 法线（Normal）：
    - `OASIS7_VIEWER_AGENT_NORMAL_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_LOCATION_NORMAL_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_ASSET_NORMAL_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_PLANT_NORMAL_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_STORAGE_NORMAL_TEXTURE_ASSET=<path#label>`
  - 金属度/粗糙度（MetallicRoughness，ORM 贴图中的 MR 通道）：
    - `OASIS7_VIEWER_AGENT_METALLIC_ROUGHNESS_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_LOCATION_METALLIC_ROUGHNESS_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_ASSET_METALLIC_ROUGHNESS_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_PLANT_METALLIC_ROUGHNESS_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_STORAGE_METALLIC_ROUGHNESS_TEXTURE_ASSET=<path#label>`
  - 自发光（Emissive）：
    - `OASIS7_VIEWER_AGENT_EMISSIVE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_LOCATION_EMISSIVE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_ASSET_EMISSIVE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_PLANT_EMISSIVE_TEXTURE_ASSET=<path#label>`
    - `OASIS7_VIEWER_POWER_STORAGE_EMISSIVE_TEXTURE_ASSET=<path#label>`
  - 说明：任一通道配置即生效；location 在任一贴图通道覆盖时会启用专用 core/halo 材质，避免与 world/chunk 材质联动。
- 示例：
```bash
OASIS7_VIEWER_AGENT_BASE_COLOR=#FF6A38 \
OASIS7_VIEWER_AGENT_EMISSIVE_COLOR=#E66230 \
OASIS7_VIEWER_LOCATION_BASE_COLOR=#4B88D9 \
OASIS7_VIEWER_LOCATION_EMISSIVE_COLOR=#B8D8FF \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```
- 材质变体预设示例：
```bash
OASIS7_VIEWER_MATERIAL_VARIANT_PRESET=matte \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```
- 贴图示例：
```bash
OASIS7_VIEWER_AGENT_BASE_TEXTURE_ASSET=textures/agents/worker_albedo.png \
OASIS7_VIEWER_AGENT_NORMAL_TEXTURE_ASSET=textures/agents/worker_normal.png \
OASIS7_VIEWER_AGENT_METALLIC_ROUGHNESS_TEXTURE_ASSET=textures/agents/worker_mr.png \
OASIS7_VIEWER_AGENT_EMISSIVE_TEXTURE_ASSET=textures/agents/worker_emissive.png \
OASIS7_VIEWER_LOCATION_BASE_TEXTURE_ASSET=textures/world/location_albedo.png \
OASIS7_VIEWER_ASSET_BASE_TEXTURE_ASSET=textures/world/asset_albedo.png \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

### 光照层（三点光）
- `OASIS7_VIEWER_SHADOWS_ENABLED=1|0`
- `OASIS7_VIEWER_AMBIENT_BRIGHTNESS=<number>`
- `OASIS7_VIEWER_FILL_LIGHT_RATIO=<>=0`
- `OASIS7_VIEWER_RIM_LIGHT_RATIO=<>=0`

### 后处理层（Post Process）
- `OASIS7_VIEWER_TONEMAPPING=none|reinhard|reinhard_luminance|aces|agx|sbdt|tony_mc_mapface|blender_filmic`
- `OASIS7_VIEWER_DEBAND_DITHER_ENABLED=1|0`
- `OASIS7_VIEWER_BLOOM_ENABLED=1|0`
- `OASIS7_VIEWER_BLOOM_INTENSITY=<0..2>`
- `OASIS7_VIEWER_COLOR_GRADING_EXPOSURE=<-8..8>`
- `OASIS7_VIEWER_COLOR_GRADING_POST_SATURATION=<0..2>`

### 推荐启动模板
```bash
OASIS7_VIEWER_RENDER_PROFILE=cinematic \
OASIS7_VIEWER_FRAGMENT_MATERIAL_STRATEGY=fidelity \
OASIS7_VIEWER_BLOOM_INTENSITY=0.24 \
OASIS7_VIEWER_COLOR_GRADING_EXPOSURE=0.35 \
OASIS7_VIEWER_COLOR_GRADING_POST_SATURATION=1.08 \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

## 工业风主题包（industrial_v3，推荐）

### 资产内容
- 路径：`crates/oasis7_viewer/assets/themes/industrial_v3/`
- 包含：
  - 5 类实体 mesh（`*_industrial_v3.gltf + *.bin`）
  - 5 类实体 PBR 贴图（`base/normal/metallic_roughness/emissive`，默认 768x768）
  - 预设文件：`industrial_v3_default.env`、`industrial_v3_matte.env`、`industrial_v3_glossy.env`

### 一键应用（启动前）
```bash
source crates/oasis7_viewer/assets/themes/industrial_v3/presets/industrial_v3_default.env
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

切换变体（仅替换 preset）：
```bash
source crates/oasis7_viewer/assets/themes/industrial_v3/presets/industrial_v3_matte.env
```
```bash
source crates/oasis7_viewer/assets/themes/industrial_v3/presets/industrial_v3_glossy.env
```

### 运行中主题切换（右侧 Theme Runtime）
- 在右侧面板 `控制` 区域可看到 `Theme Runtime`：
  - `Preset`：`Off / industrial_v3 default|matte|glossy / industrial_v2 default|matte|glossy(兼容) / Custom file`
  - `Apply Theme`：立即应用当前 preset
  - `Auto Hot Reload`：打开后自动检测 preset 文件变更并重载
- 适用场景：美术调参与脚本生成资产后实时复验，无需重启 viewer。

### 运行中主题切换（环境变量）
- `OASIS7_VIEWER_THEME_PRESET=none|industrial_v3_default|industrial_v3_matte|industrial_v3_glossy|industrial_v2_default|industrial_v2_matte|industrial_v2_glossy|custom`
- `OASIS7_VIEWER_THEME_PRESET_FILE=<path/to/preset.env>`（设置后优先按文件路径加载）
- `OASIS7_VIEWER_THEME_HOT_RELOAD=1|0`

示例：
```bash
OASIS7_VIEWER_THEME_PRESET=industrial_v3_default \
OASIS7_VIEWER_THEME_HOT_RELOAD=1 \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

自定义 preset 示例：
```bash
OASIS7_VIEWER_THEME_PRESET_FILE=.tmp/custom_theme.env \
OASIS7_VIEWER_THEME_HOT_RELOAD=1 \
env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- 127.0.0.1:5023
```

### 主题包校验（提交前）
```bash
python3 scripts/validate-viewer-theme-pack.py \
  --theme-dir crates/oasis7_viewer/assets/themes/industrial_v3 \
  --profile v3
```

## 工业风主题包（industrial_v2 / industrial_v1，兼容）
- `industrial_v2` 路径：`crates/oasis7_viewer/assets/themes/industrial_v2/`
- `industrial_v1` 路径：`crates/oasis7_viewer/assets/themes/industrial_v1/`
- `industrial_v2` 预设：`industrial_v2_default.env`、`industrial_v2_matte.env`、`industrial_v2_glossy.env`
- `industrial_v1` 预设：`industrial_default.env`、`industrial_matte.env`、`industrial_glossy.env`
- 校验：
```bash
python3 scripts/validate-viewer-theme-pack.py \
  --theme-dir crates/oasis7_viewer/assets/themes/industrial_v2 \
  --profile v2
```
```bash
python3 scripts/validate-viewer-theme-pack.py \
  --theme-dir crates/oasis7_viewer/assets/themes/industrial_v1 \
  --profile v1
```

### 批量预览（default/matte/glossy）
```bash
./scripts/viewer-theme-pack-preview.sh \
  --scenario llm_bootstrap \
  --theme-pack industrial_v3 \
  --variants all \
  --base-port 5423
```

输出目录：
- `output/theme_preview/<timestamp>/default/viewer.png`
- `output/theme_preview/<timestamp>/matte/viewer.png`
- `output/theme_preview/<timestamp>/glossy/viewer.png`
- 每个变体目录附带 `live_server.log`、`viewer.log`、`capture_status.txt`、`meta.txt`
- `meta.txt` 记录 `theme_pack` 与实际 `preset_file`
- `capture_status.txt` 必须为 `connection_status=connected` 且 `snapshot_ready=1`（否则脚本失败）

常用参数：
- `--theme-pack <industrial_v3|industrial_v2|industrial_v1>`：选择主题包（默认 `industrial_v3`，推荐）。

### 资产重生成
```bash
python3 scripts/generate-viewer-industrial-theme-assets.py --quality v3 --out-dir crates/oasis7_viewer/assets/themes/industrial_v3
```
```bash
python3 scripts/generate-viewer-industrial-theme-assets.py --quality v2 --out-dir crates/oasis7_viewer/assets/themes/industrial_v2
```
```bash
python3 scripts/generate-viewer-industrial-theme-assets.py --quality v1 --out-dir crates/oasis7_viewer/assets/themes/industrial_v1
```

## 贴图查看器（可截图）

用途：
- 在统一构图下快速检查贴图观感（base/normal/metallic_roughness/emissive）。
- 支持批量实体来源与材质变体，输出可留痕截图目录。

### 基础调用
```bash
./scripts/viewer-texture-inspector.sh \
  --inspect all \
  --variants all \
  --scenario llm_bootstrap
```

### 常用参数
- `--preset-file <path>`：指定主题预设 env 文件（默认跟随 `scripts/viewer-theme-defaults.env`，即 `industrial_v3_default.env`）。
- `--inspect <list>`：贴图来源实体（`agent,location,asset,power_plant,all`）。
- `--variants <list>`：`default,matte,glossy,all`。
- `--base-texture/--normal-texture/--mr-texture/--emissive-texture`：临时覆盖贴图路径。
- `--use-source-mesh`：把“来源实体 mesh”作为预览载体（默认关闭，默认使用 location 载体保证构图稳定）。
- `--out-dir <dir>`：输出目录（默认 `output/texture_inspector/<timestamp>`）。

### 输出目录
- `output/texture_inspector/<timestamp>/<entity>/<variant>/viewer.png`
- 同目录附带：`live_server.log`、`viewer.log`、`capture_status.txt`、`meta.txt`
- `capture_status.txt` 必须为 `connection_status=connected` 且 `snapshot_ready=1`（否则脚本失败）

## Web 闭环（默认，推荐调试/回归）

说明：本节仅适用于 Viewer 页面，不适用于 `oasis7_web_launcher` / launcher 控制台。对 launcher 产品动作，应优先调用 GUI Agent 接口（`/api/gui-agent/*`），`agent-browser` 只用于页面加载、状态与字段校验。

说明：该闭环用于可视化观察与交互取证，不等价于“浏览器作为完整分布式节点运行”。

### 前置要求
- 已安装 `agent-browser`（默认直接使用 `agent-browser` 命令）
- `trunk`（`cargo install trunk`）
- `wasm32-unknown-unknown`（`rustup target add wasm32-unknown-unknown`）

### 标准流程
1) 启动 live server（终端 A）
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

2) 启动 Web Viewer（终端 B）
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

3) 执行 agent-browser 闭环采样（终端 C）
```bash
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
mkdir -p output/playwright/viewer
agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1"
agent-browser wait --load networkidle
agent-browser snapshot -i
agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)" | tee output/playwright/viewer/state.json
agent-browser console | tee output/playwright/viewer/console.log
agent-browser screenshot output/playwright/viewer/viewer-web.png
agent-browser close
```
注：证据目录当前沿用历史路径 `output/playwright/viewer/`，以兼容现有脚本与归档结构。

### 输出目录
- `output/playwright/viewer/*.png`
- `output/playwright/viewer/state.json`（`window.__AW_TEST__.getState()` 采样）
- `output/playwright/viewer/console.log`（或等价的 agent-browser 控制台重定向日志）

### 最小验收口径
- 页面加载成功（`snapshot -i` 可见交互树，且主视区正常渲染）。
- `window.__AW_TEST__.getState()` 返回 `connectionStatus=connected` 且 `lastError=null`。
- `console.log` 中不得出现 `SwiftShader` / `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` 等图形 fatal 签名。
- 至少产出 1 张截图。

### Web 图形门禁提示
- Viewer Web 闭环必须在 headed、硬件加速可用的浏览器环境下执行。
- 若 `state.json` 中长期停留 `connectionStatus="connecting"`、`logicalTime=0`，先检查 `lastError`。
- 若 `lastError` 或控制台包含 `copy_deferred_lighting_id_pipeline`、`CONTEXT_LOST_WEBGL`、`Shader compilation failed`，应判定为浏览器图形链路失败，而不是玩法/协议失败。
- 若控制台明确出现 `SwiftShader` / software renderer 信号，应停止用该浏览器环境继续给出玩法结论，改用硬件加速浏览器或走下面的 native fallback。

### native fallback（仅在 Web 无法复现或排查图形链路）
基础调用：
```bash
./scripts/capture-viewer-frame.sh --scenario asteroid_fragment_detail_bootstrap --addr 127.0.0.1:5131 --viewer-wait 12 --auto-focus-target first_fragment --auto-focus-radius 18
```

常用增强参数：
- `--capture-max-wait <sec>`：覆盖内置截图最大等待时间。
- `--no-prewarm`：跳过预热编译。
- `--keep-tmp`：保留 `.tmp/` 产物便于排查。
- 默认会保持 2D，不再把切 3D 作为 native fallback 的默认动作。
- `--auto-focus-force-3d`：仅在 hold-only 3D 检查时强制切到 3D。

## 右侧综合面板与 Chat 面板显隐
- 综合右侧面板支持按模块单独显示/隐藏：控制、总览、覆盖层、诊断、事件联动、时间轴、状态明细。
- 综合右侧面板顶部提供总开关：`隐藏面板/显示面板`；隐藏时主面板与 Chat 面板均不占右侧宽度，3D 视口扩展到全宽。
- Chat 功能已拆分为独立最右侧面板，不再出现在综合右侧面板内容区。
- `Chat` 可见性开关关闭时，不渲染独立 Chat 面板且不占用右侧宽度。
- 开关状态会落盘并在重启后恢复。
- 默认缓存路径：`$HOME/.oasis7_viewer/right_panel_modules.json`
- 可通过环境变量覆盖：`OASIS7_VIEWER_MODULE_VISIBILITY_PATH`

## Web 全屏自适应（wasm）
- Web 端 canvas 跟随浏览器父容器尺寸，默认占满可用视口（非固定 `1200x800` 逻辑窗口体验）。
- 右侧面板宽度采用“最小宽度 + 动态上限（随可用宽度变化）”，在大屏上不再受固定像素上限限制。
- 当需要更大观察区域时，优先使用右侧面板总开关收起面板。

## 选中详情面板
- 点击对象后会在详情区显示信息，支持：
  - Agent
  - Location
  - Asset
  - PowerPlant
  - Chunk
- LLM 场景下，Agent 详情可显示最近决策 I/O（输入、输出、错误与 token/时延摘要）。
- 离线或无 LLM trace 时会显示降级提示，不影响基础详情查看。

## 快速定位 Agent
- 入口：右侧 `Event Link / 事件联动` 区域的 `定位 Agent` 按钮。
- 行为：
  - 优先定位当前已选中的 Agent；
  - 否则定位当前场景字典序第一个 Agent。
- 适合对象密集场景下快速回到 Agent 观察位。

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
- 看不到细节：先用 `F` 或 `--auto-focus-target` 聚焦，并优先保持 2D / Web 主链路；只有在 hold-only 3D 排查时才显式切到 3D。
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
