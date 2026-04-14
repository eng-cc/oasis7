# GitHub Pages 发布入口 + Release 安装包流水线（2026-03-01）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md`

审计轮次: 6

- 对应标准执行入口: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md`

## ROUND-002 主从口径
- 主入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`
- 本文仅维护本专题增量，不重复主文档口径。

## 目标
- 建立可直接上线的发布系统：GitHub Pages 作为发行入口页，GitHub Releases 作为安装包分发源。
- 支持用户在官网页面“一键下载”最新安装包（Windows/macOS/Linux）。
- 将“打包 + 发布 + 校验”纳入 GitHub Actions，减少手工发布步骤和失误率。

## 范围
- 范围内
  - 新增 Release 发布工作流（tag 触发 + 手动触发）。
  - Release gate 拆分为 runtime/sync、web strict、S9/S10 soak 三个并行子门，并通过 aggregate job 统一决定是否放行打包。
  - 自动构建桌面启动器安装包并上传到 GitHub Release。
  - 生成并上传校验文件（SHA256）。
  - 在 `site/index.html` 与 `site/en/index.html` 增加下载入口和直链（`releases/latest/download/...`）。
  - 站点脚本补充下载入口的存在性/基本格式校验。
- 范围外
  - 不改动 Rust 世界规则与游戏逻辑。
  - 不引入新的前端构建工具链。
  - 不实现应用内自动更新器（auto-updater）。

## 接口 / 数据
- Release 产物命名（固定名，保证 `latest/download` 可长期使用）：
  - `oasis7-windows-x64.exe`
  - `oasis7-macos-x64.dmg`
  - `oasis7-linux-x86_64.AppImage`
  - `oasis7-linux-x64.deb`（次级 / 高级入口）
  - `oasis7-checksums.txt`
- 下载直链：
  - `https://github.com/<owner>/<repo>/releases/latest/download/<asset>`
- 工作流触发：
  - `push tags: v*`
  - `workflow_dispatch`
- Release gate 拓扑：
  - `release-gate-runtime`：执行 `ci_full + sync_m1/m4/m5`
  - `release-gate-web`：在 `xvfb` 下以 `--web-headed` 执行 `web_strict`，显式预装 `node + trunk`，并由 QA loop 自行预热 `oasis7_viewer_live + oasis7_chain_runtime`
  - `release-gate-soak`：执行 `S9 + S10` soak
  - `release-gate`：聚合三个子门结果，作为 `build-web-dist` 的唯一前置依赖
- 打包 runner 与目标三元组（release workflow）：
  - `package-native` 在各平台 job 内显式 provision `trunk + wasm32-unknown-unknown`，同时 `scripts/build-game-launcher-bundle.sh` 会在执行 `trunk build` 前自检并补装缺失的 `wasm32-unknown-unknown`，确保 `web-launcher/` 构建不依赖 runner 初始状态
  - `package-native` 先生成标准 `oasis7-<platform>` bundle 目录，再通过 `scripts/package-native-installer.sh` 输出固定名 `AppImage` / `.dmg` / `.exe` 主公开资产；Linux `.deb` 仅作为次级高级入口保留。bundle 目录必须先通过 `scripts/validate-release-platform-entrypoints.sh` 校验，确保公开下载层不只是“换扩展名”，而是真正存在平台原生入口
  - linux：`ubuntu-24.04` + `native`
  - macOS：`macos-14` + `x86_64-apple-darwin`（避免仓库不支持的 `macos-13` 配置）
  - windows：`windows-2022` + `native`
- 打包内容（每个平台）：
  - `bin/oasis7_game_launcher`
  - `bin/oasis7_viewer_live`
  - `bin/oasis7_chain_runtime`
  - `bin/oasis7_client_launcher`
  - linux：`run-*.sh` wrapper + `oasis7-linux-x86_64.AppImage` 直接运行入口（另保留 `.deb` 安装后的 `/usr/bin/oasis7-*` 次级入口）
  - macOS：`oasis7 Client Launcher.app` + `.dmg` 内 `/Applications` 拖拽安装入口
  - windows：`run-*.cmd` + SFX `.exe` 默认运行 `run-client.cmd`
  - `web/`（viewer 静态资源）
  - `run-game.sh` / `run-client.sh`（Windows 额外提供 `.cmd`）
  - `README.txt`

## 里程碑
- M0：建档（设计 + 项目管理）。
- M1：发布流水线可产出三平台安装包并写入 Release。
- M2：Pages 首页接入下载入口并直连 latest release assets。
- M3：完成校验、文档回写、devlog 记录与结项。

## 风险
- 风险：跨平台构建在 GitHub Runner 上依赖差异较大，可能导致单平台失败。
  - 缓解：Web 资源单独构建后复用；native 构建采用矩阵分离，失败平台可独立定位。
- 风险：将 runtime、web、soak 关卡串成单个长 job 会放大基础设施抖动与缺依赖问题，导致定位慢且重复重跑成本高。
  - 缓解：拆分为并行子门，分别上传 gate summary artifact，并在 web 子门、package-native job 与 bundle 脚本内部显式 provision/自检必需前端工具链（`node + trunk`、`wasm32-unknown-unknown`），减少“未预装工具”导致的假红。
- 风险：固定资产名若被误改，页面直链会失效。
  - 缓解：新增下载入口校验脚本并接入 CI。
- 风险：`latest` 语义受 prerelease 影响，用户可能下载到非稳定版。
  - 缓解：工作流默认发布正式 release，必要时在文档中要求 prerelease 另行命名与渠道区分。
- 风险：当前 `AppImage` / `.dmg` / `.exe` 虽然已收口为每个平台一个可直接安装/运行的主包，但 Windows 签名、macOS notarization 与官网单主 CTA 支持信息仍未闭环，离普通用户级安装分发体验还有差距。
  - 缓解：普通用户向目标（Windows 标准安装器、macOS 签名+notarization、Linux AppImage、官网单主 CTA）单独由 `doc/world-simulator/launcher/game-client-launcher-broad-user-release-distribution-2026-04-14.prd.md` 跟踪，并在本专题后续阶段接入实现。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
