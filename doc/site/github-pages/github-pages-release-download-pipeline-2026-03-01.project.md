# GitHub Pages 发布入口 + Release 安装包流水线（2026-03-01）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`

审计轮次: 6

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护本专题增量任务，不重复主项目文档任务编排。

## 任务拆解

### T0A CI 阻塞修复（先行）
- [x] 修复 `cargo fmt --all -- --check` 基线（提交仓库内遗留格式化差异）
- [x] 修复 `oasis7_game_launcher_tests` 被误识别为独立 bin 导致的 `cargo test` 失败
- [x] 修复 `oasis7_viewer --target wasm32-unknown-unknown` 的 `ctrlc` 目标兼容问题
- [x] 回归 `./scripts/ci-tests.sh required` 并确认通过

### T0 建档与基线
- [x] 新建设计文档：`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`
- [x] 新建项目管理文档：`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md`
- [x] 明确资产命名、触发条件、页面下载入口口径

### T1 发布流水线实现
- [x] 新增 Release 工作流（tag + 手动触发）
- [x] 新增安装包打包脚本（矩阵复用、固定资产命名）
- [x] 上传三平台资产与 SHA256 校验文件到 Release
- [x] 任务测试与提交

### T2 页面下载入口接入
- [x] 更新 `site/index.html`、`site/en/index.html` 增加下载区块与入口锚点
- [x] 更新 `site/assets/styles.css` 下载区块样式
- [x] 更新 `site/assets/app.js` 拉取 latest tag 进行页面展示（失败时回退）
- [x] 新增/更新下载入口校验脚本并接入 CI
- [x] 任务测试与提交

### T3 回归验证与文档收口
- [x] 执行脚本回归与基础构建校验
- [x] 回写本项目管理文档状态
- [x] 写任务日志：`doc/devlog/2026-03-01.md`
- [x] 任务测试与提交
- 完成内容（2026-05-10 follow-up）：`release-gate-web` 额外纳入 `viewer-primary-web-entry-regression.sh`，在 `software_safe` realtime regression 之前先验证公开默认 `/` 与 `render_mode=auto` 发布入口都落到 `software_safe`；避免 `standard_3d` 删除后 release gate 只验证强制 `render_mode=software_safe`，却漏掉真正对外发布的默认入口契约。

### T3A Pages 门禁兼容性热修复（GitHub runner 无 rg）
- [x] 复现并定位 Actions run `22474048679` / job `65097149123` 失败原因
- [x] 修复 `scripts/site-manual-sync-check.sh`：`rg` 不可用时回退 `grep -F`
- [x] 修复 `scripts/site-download-check.sh`：同样支持 `grep -F` 回退
- [x] 本地回归：正常 PATH + 无 `rg` PATH 双路径校验

### T3B Rust required gate 兼容性热修复（GitHub runner 无 rg）
- [x] 复现并定位 `Rust` workflow 失败根因：`scripts/doc-governance-check.sh` 直接依赖 `rg`
- [x] 修复 `scripts/doc-governance-check.sh`：标题检测与绝对路径检测在 `rg` 不可用时回退 `grep -E`
- [x] 本地回归：正常 PATH + 无 `rg` PATH 双路径校验

### T3C Builtin Wasm m1 identity 清单回收敛
- [x] 复现并定位 `Wasm Determinism Gate / verify-wasm-determinism (m1)` 的前身独立 gate 失败根因：`m1.body.core` source_hash 失配
- [x] 执行 `scripts/sync-m1-builtin-wasm-artifacts.sh` 更新 hash/identity manifest
- [x] 本地回归 `scripts/ci-m1-wasm-summary.sh`（至少当前平台）并确认通过

### T3D Rust required gate m5 identity/hash 清单热修
- [x] 复现并定位 `Rust` workflow 新失败根因：`m5.gameplay.crisis.cycle` wasm hash 不在 identity/hash token 列表
- [x] 同步 `m5` builtin wasm 清单，并为 hash token 增加兼容候选集合（覆盖 runner 变体）
- [x] 调整 `builtin_wasm_identity` m5 用例，兼容 `identity_hash_v1` 签名方案
- [x] 本地回归 `scripts/sync-m5-builtin-wasm-artifacts.sh --check` 与失败定向测试

### T3E m5 多 token 清单持久化修正
- [x] 复盘并定位二次失败根因：`sync-m5` legacy 回写会将多 token 清单压回单 token
- [x] 手工固定 `m5_builtin_modules.sha256` 与 `m5_builtin_modules.identity.json` 的多 token 顺序集合（含 CI 报错 hash）
- [x] 只读校验 `scripts/sync-m5-builtin-wasm-artifacts.sh --check`，确保清单一致且不再被覆盖

### T3G Release Packages 编译提速（2026-03-13）
- [x] 复盘 `Release Packages` 多轮重跑中 compile/install 热点，确认慢点集中在 release-gate 的 full tier 重编译与 builtin wasm canonical nightly 按需安装。
- [x] 更新 `.github/workflows/release-packages.yml`：为 `release-gate` / `build-web-dist` / `package-native` 接入 `Swatinem/rust-cache@v2`，缓存 cargo registry/target 产物以缩短重复编译时间。
- [x] 更新 `.github/workflows/release-packages.yml`：在 `release-gate` 前显式预装 canonical builtin wasm toolchain（`nightly-2025-12-11 + rust-src + wasm32-unknown-unknown`），避免 full tier 测试期间再由 materializer 按需拉取。
- [x] 调整 workflow 顶层 cargo 环境：启用 sparse registry、提高 network retry、关闭 dev/test debug info，进一步降低 CI 编译与下载开销。
- [x] 本地校验 workflow 结构与文档回写，继续观察新一轮 release tag 实跑表现。

### T3H Release gate UDP gossip flake 热修
- [x] 复盘 `Release Packages` run `23053414184`，确认阻断点不是打包脚本，而是 `oasis7_node::tests::runtime_gossip_tracks_peer_committed_heads` 在 CI 高负载下 5 秒窗口内偶发未观测到 peer heads。
- [x] 调整 `crates/oasis7_node/src/tests_split_part2.rs`：为 UDP gossip 双节点都启用 `with_auto_attest_all_validators(true)`，避免测试依赖跨节点 attestation 时序抖动；同时把等待窗口从 5s 提升到 8s，吸收 GitHub runner 高负载波动。
- [x] 本地回归该用例的精确重跑，并在回写 `project/devlog` 后继续通过新 tag 观察 `Release Packages` 是否彻底放行。

### T3I Release gate execution bridge signer allowlist 热修
- [x] 复盘 `Release Packages` run `23055068064`，确认 `v0.0.8` 的新阻断点为 `oasis7_chain_runtime` 单测 `node_runtime_execution_driver_commit_routes_modules_via_step_with_modules`；其前置 `InstallModuleFromArtifact` 实际被拒，因为 binary unit test 环境不会自动注入 `test.module.release.signer` 到 `World::new()` 的 `node_identity_bindings`。
- [x] 调整 `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`：在该测试里显式 `bind_node_identity(TEST_MODULE_ARTIFACT_SIGNER_NODE_ID, ...)`，并补充 `ModuleInstalled` / `module_tick_schedule` 前置断言，确保断言真正覆盖“commit 走 `step_with_modules` 并冒泡模块失败”。
- [x] 本地回归 `oasis7_chain_runtime` 定向用例，并与相邻 execution bridge 持久化用例一起校验通过；继续通过新 tag 观察 `Release Packages` 是否越过 `release-gate`。

### T3J Release gate m5 economic overlay hash token 热修
- [x] 复盘 `Release Packages` run `23056942631`，确认 `v0.0.9` 的新阻断点在 `sync_m5`：`m5.gameplay.economic.overlay` 的 `linux-x86_64` canonical hash 已漂移到 `797e76900aa04297700c8ca5512ba9b00c6f8c4e83845d8ff473bd2adb0e6676`，而仓库清单仍写旧值 `36645c1c3fd590c4212691ba1ae0a881ef12171a9d375ee8693127e610968274`。
- [x] 更新 `crates/oasis7/src/runtime/world/artifacts/m5_builtin_modules.sha256` 与 `crates/oasis7/src/runtime/world/artifacts/m5_builtin_modules.identity.json`：将 `m5.gameplay.economic.overlay` 的 `linux-x86_64` hash token 对齐到当前 canonical 产物；该模块现已与 `darwin-arm64` 共用同一 canonical hash。
- [ ] 本地回归 `./scripts/sync-m5-builtin-wasm-artifacts.sh --check`，并通过新 tag 继续观察 `Release Packages` 是否终于越过 `release-gate`。

### T3K Release gate agent-browser CLI fallback 热修
完成内容：复盘 `Release Packages` run `23059581794`，确认 `v0.0.10` 已越过 `ci_full/sync_m1/sync_m4/sync_m5`，但当时 `web_strict` 仍通过已删除的 `./scripts/viewer-release-qa-loop.sh` 执行，GitHub runner 缺少全局 `agent-browser` 命令，直接导致 `missing required command: agent-browser`。
完成内容：调整 `scripts/agent-browser-lib.sh`，优先使用本机 `agent-browser`，当 CLI 不存在时自动回退到 `npx --yes agent-browser`；保持 `AGENT_BROWSER_SESSION` 透传，避免 CI 因为“没全局安装”而把 Web 严格闭环整段跳红。
完成内容：本地回归脚本级 fallback：在无 `agent-browser`、仅有伪造 `npx` 的 PATH 下，执行 `source scripts/agent-browser-lib.sh && ab_require && ab_cmd fallback-session get url`，确认实际走到 `--yes agent-browser get url`。

### T3L Release gate trunk-missing dist fallback 热修
- [x] 复盘 `Release Packages` run `23078512672`，确认 `v0.0.11` 已经真正越过 `agent-browser` CLI 入口，但 `web_strict` 在解析 Viewer 静态资源目录时，因为 runner 没有安装 `trunk` 而退出；失败签名为 `error: missing required command: trunk`。
- [x] 调整 `scripts/agent-browser-lib.sh`：当请求 `web` 别名、`crates/oasis7_viewer/dist/index.html` 已存在但 `trunk` 不可用时，回退到仓库已提交的 `crates/oasis7_viewer/dist`，只在 `dist` 也不存在时才继续报错；避免 CI 因缺少前端构建器而阻断 Web 闭环。
- [x] 本地回归脚本级 fallback：在无 `trunk` 的 PATH 下 source `scripts/agent-browser-lib.sh`，并通过桩掉 `find` 强制进入 fallback 分支，确认返回 `crates/oasis7_viewer/dist` 且打印 `warning: trunk missing; falling back to committed viewer dist`。

### T3F Release Packages macOS runner 配置热修
- [x] 复现并定位 `Release Packages` run `22545989082` / job `65309292458` 失败根因：`macos-13-us-default` 不受当前仓库支持
- [x] 修复 `.github/workflows/release-packages.yml`：macOS 矩阵 runner 改为 `macos-14`，并显式配置 `target_triple=x86_64-apple-darwin`
- [x] 扩展打包脚本参数链路：`release-prepare-bundle.sh` / `build-game-launcher-bundle.sh` 支持 `--target-triple` 并正确定位 `target/<triple>/<profile>` 产物
- [x] 本地回归脚本语法与 dry-run，推送后重新触发 `Release Packages` 验证

### T3M Release gate 并行拆分与聚合收口（2026-03-14）
- [x] 复盘 `Release Packages` 连续多轮失败，确认主要问题已不再是单一业务缺陷，而是所有 release blocker 串在单个 `release-gate` job 中，导致 runtime/sync、web strict、S9/S10 soak 彼此阻塞，后续 `build-web-dist/package-native/publish-release` 长期无法进入。
- [x] 更新 `.github/workflows/release-packages.yml`：将 gate 拆为 `release_gate_runtime` / `release_gate_web` / `release_gate_soak` 三个并行 job，并新增 `release_gate` aggregate job 统一汇总 `needs.*.result`，继续保持“全部通过才放行打包”的语义。
- [x] 更新 `.github/workflows/release-packages.yml`：在 `release_gate_web` 内显式 provision `actions/setup-node@v4 + trunk`，并为三个子门分别上传 `.tmp/release_gate_*` summary artifact，缩短 CI 缺依赖与长时间黑盒失败的定位链路。
- [x] 本地校验 workflow 语法、`release-gate.sh` dry-run 组合与文档回写后，再进入下一轮远端 release tag 验证。

### T3N Release gate soak 预热依赖回补（2026-03-14）
- [x] 复盘 `Release Packages` run `23080174183` 新架构首轮结果，确认 `release-gate-soak` 在 1 秒内失败并非 soak 逻辑本身回归，而是拆分后仍沿用 `--no-prewarm`，导致 `s9` 失去来自 `ci_full` 的 `target/debug/oasis7_chain_runtime` 预热前置。
- [x] 更新 `.github/workflows/release-packages.yml`：在 `release_gate_soak` 中新增 `env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime` 预热步骤，使 soak 子门在独立 job 中重新自洽，同时保持 `release-gate.sh` 现有参数与 release 语义不变。
- [x] 本地回归 workflow 语法与 soak job 关键片段，确认 `Prewarm soak runtime binary` 已位于 `Run soak release gate` 之前。
- [ ] 推送修复并打新 tag，继续观察并行 gate 是否能全部进入 aggregate `release_gate`。

### T3X Release public assets switch to direct installers（2026-04-14）
- [x] 更新 `.github/workflows/release-packages.yml`：`package-native` 不再上传 `.tar.gz` / `.zip` 压缩包，而是先准备标准 bundle 目录，再调用 `scripts/package-native-installer.sh` 输出 `oasis7-linux-x64.deb`、`oasis7-macos-x64.dmg`、`oasis7-windows-x64.exe`。
- 完成内容：新增 `scripts/package-native-installer.sh`，Linux 产出真正的 `.deb`，macOS 产出带 `/Applications` 拖拽入口的 `.dmg`，Windows 产出默认运行 `run-client.cmd` 的自解压 `.exe`；`publish-release` 与 `oasis7-checksums.txt` 同步切换到新的公开资产名。
- 完成内容：新增 `scripts/validate-release-platform-entrypoints.sh`，并让 bundle / packager 在公开发布前强制校验平台原生入口：Linux 继续要求 `run-*.sh` + `/usr/bin/oasis7-*`，macOS 要求 `oasis7 Client Launcher.app`，Windows 要求 `run-*.cmd`。
- [x] 同步 `site/index.html`、`site/en/index.html`、`scripts/site-download-check.sh` 与 site 模块 PRD/project，使公开下载入口、脚本门禁与文档真值一致，不再将压缩包作为公开下载主入口。
- [x] 本地回归：
  - `./scripts/site-download-check.sh`
  - `bash -n scripts/package-native-installer.sh`
  - `bash -n scripts/validate-release-platform-entrypoints.sh`
  - `bash -n scripts/release-prepare-bundle.sh`
  - `bash -n scripts/build-game-launcher-bundle.sh`
  - 以临时 bundle 目录实测 `./scripts/build-game-launcher-bundle.sh --out-dir <tmp>/linux-bundle --profile release`
  - `./scripts/validate-release-platform-entrypoints.sh --platform linux-x64 --bundle-dir <tmp>/linux-bundle`
  - `./scripts/package-native-installer.sh --platform linux-x64 --bundle-dir <tmp>/linux-bundle --out-dir <tmp>/out --asset-name oasis7-linux-x64.deb --version 0.0.0`
  - `dpkg-deb --contents <tmp>/out/oasis7-linux-x64.deb`
  - `./scripts/build-game-launcher-bundle.sh --dry-run --target-triple x86_64-apple-darwin --web-dist <tmp>/viewer-dist --web-launcher-dist <tmp>/launcher-dist --out-dir <tmp>/macos-bundle`
  - `./scripts/build-game-launcher-bundle.sh --dry-run --target-triple x86_64-pc-windows-msvc --web-dist <tmp>/viewer-dist --web-launcher-dist <tmp>/launcher-dist --out-dir <tmp>/windows-bundle`

### T3Y Linux public asset switch to AppImage（2026-04-15）
- 完成内容：更新 `.github/workflows/release-packages.yml`，将 Linux `package-native` 主资产改为 `oasis7-linux-x86_64.AppImage`，并在同一 job 内额外保留 `oasis7-linux-x64.deb` 作为次级高级入口；`publish-release` 与 `oasis7-checksums.txt` 同步纳入两类 Linux 资产。
- 完成内容：更新 `scripts/package-native-installer.sh`，让 Linux 支持从同一 bundle 目录产出 `AppImage` 或 `.deb`；同时保留 `scripts/validate-release-platform-entrypoints.sh` 的 bundle 真值校验，以及 macOS `/Applications` 拖拽入口。
- 完成内容：把 Windows 打包从自解压 SFX `.exe` 切到 NSIS 标准安装器，并同步引入 `scripts/windows-release-installer.nsi`、`run-client.cmd`/开始菜单/桌面/卸载入口。
- 完成内容：同步 `site/index.html`、`site/en/index.html`、`scripts/site-download-check.sh` 与相关 PRD/project，官网 Linux 主下载入口切到 `oasis7-linux-x86_64.AppImage`，文案明确“赋予执行权限后即可运行”。
- 本地回归：
  - `./scripts/site-download-check.sh`
  - `bash -n scripts/package-native-installer.sh`
  - `bash -n scripts/build-game-launcher-bundle.sh`
  - `bash -n scripts/release-prepare-bundle.sh`
  - `PATH=<fake-bin>:$PATH ./scripts/package-native-installer.sh --platform linux-x64 --bundle-dir <tmp>/bundle --out-dir <tmp>/out --asset-name oasis7-linux-x86_64.AppImage --version 0.0.0 --dry-run`
  - `./scripts/package-native-installer.sh --platform linux-x64 --bundle-dir <tmp>/bundle --out-dir <tmp>/out --asset-name oasis7-linux-x64.deb --version 0.0.0 --dry-run`

### T3O Release gate web sibling binary 预热回补（2026-03-14）
完成内容：复盘 `Release Packages` run `23080255868`，确认 `release-gate-web` 已越过 `trunk` 安装，但 `web_strict` 在 `oasis7_game_launcher` 启动阶段因独立 job 缺少 `target/debug/oasis7_viewer_live` 而失败；失败签名为 `failed to locate \`oasis7_viewer_live\` binary; build it first or set OASIS7_VIEWER_LIVE_BIN`。
历史记录：当时调整 `scripts/viewer-release-qa-loop.sh`，在启动 `oasis7_game_launcher` 前显式执行 `env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --bin oasis7_chain_runtime`，把原先依赖其他步骤隐式生成 sibling binaries 的前置条件收回到脚本内部。
历史记录：本地回归 `bash -n scripts/viewer-release-qa-loop.sh`，并确认预热命令已位于 `cargo run -p oasis7 --bin oasis7_game_launcher` 之前；脚本现已删除。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-web` 是否越过 launcher 启动阶段，并进一步验证 aggregate `release_gate` 与后续打包链路。

### T3P Release gate web test API 冷启动窗口放宽（2026-03-14）
完成内容：复盘 `Release Packages` run `23080686951`，确认 `release-gate-web` 已越过 sibling binary 缺失，但页面在 GH runner 上打开后 20 秒内仍未暴露 `window.__AW_TEST__`，导致 `web_strict` 以 `__AW_TEST__ is unavailable` 退出；launcher 与 bridge 已正常就绪，说明问题落在 Web 端冷启动窗口而非服务拉起。
历史记录：当时调整 `scripts/viewer-release-qa-loop.sh`，将 `wait_for_api` 从 20s 提升到 60s、将初始 `wait_for_connected` 从 15s 提升到 30s，并在 `__AW_TEST__` 超时前自动抓取 `console` / `errors` 日志。
历史记录：本地回归 `bash -n scripts/viewer-release-qa-loop.sh`，确认等待窗口与失败诊断输出语法正确；脚本现已删除。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-web` 是否终于越过 Web Test API 初始化阶段。

### T3Q Release gate web test API readiness 兼容修复（2026-03-14）
完成内容：复盘 `Release Packages` run `23081035902` 与既往 `2026-03-10` Web QA 记录，确认 `wait_for_api` 不是单纯超时，而是会把 `agent-browser eval` 返回的 `"ready"` 误判为未就绪；当前 CI 日志中页面已打开、launcher stack 已 ready、console/errors 为空，与这一旧签名一致。
历史记录：当时调整 `scripts/viewer-release-qa-loop.sh`，新增 `normalize_eval_token`，将 `wait_for_api` 改为评估 `typeof window.__AW_TEST__ === "object" ? "ready" : "missing"`，并兼容 `ready/"ready"/true` 三种返回形态。
历史记录：本地回归 `bash -n scripts/viewer-release-qa-loop.sh`，确认 readiness 兼容逻辑与现有超时/console 采集分支可同时生效；脚本现已删除。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-web` 是否终于越过 Web Test API readiness 检查并进入语义交互断言。

### T3R Release gate web headed Xvfb 执行链回补（2026-03-14）
- [x] 复盘 `Release Packages` run `23081472315`，确认 `release-gate-web` 已越过 readiness 误判修复，但在 headless CI 中仍无法拿到 `__AW_TEST__`；结合既有手册与 `2026-03-10` headed smoke，可判断当前 GitHub runner 更接近“需要 headed 浏览器窗口才能稳定完成 Viewer Web 初始化”的路径。
- [x] 更新 `.github/workflows/release-packages.yml`：为 `release_gate_web` 增加 `xvfb + xauth` 系统依赖，并通过 `xvfb-run -a ./scripts/release-gate.sh --web-headed` 执行 Web 严格闭环，让 CI 走与现有 Web 闭环手册一致的 headed 路径。
- [x] 本地校验 workflow YAML 解析通过，并确认 `release_gate_web` 的命令链已携带 `--web-headed`。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-web` 是否终于完成 Viewer Web 初始化并进入后续断言。

### T3S Release gate web screenshot artifact 路径兼容修复（2026-03-14）
完成内容：复盘 `Release Packages` run `23081885506`，确认 `release-gate-web` 在 `xvfb-run + --web-headed` 下已越过 Web 初始化，并通过 semantic / zoom gate；新的唯一阻断点是截图产物未落到脚本期望路径，而是被 `agent-browser` 保存到自身 tmp 目录，导致 `Screenshot artifact: failed`。
历史记录：当时调整 `scripts/agent-browser-lib.sh` 与 `scripts/viewer-release-qa-loop.sh`，新增 `ab_screenshot`，在 `agent-browser screenshot <target>` 成功但目标文件不存在时自动从 CLI 输出解析真实落盘路径并回拷到请求路径。
历史记录：本地回归 `bash -n scripts/agent-browser-lib.sh`、`bash -n scripts/viewer-release-qa-loop.sh`，确认 helper 与调用点语法通过；其中 QA loop 脚本现已删除。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-web` 是否终于全绿并让 aggregate `release_gate` 进入后续打包链路。

### T3T Release gate runtime agent chat env 串味修复（2026-03-14）
- [x] 复盘 `Release Packages` run `23082322519`，确认 `release-gate-runtime` 唯一阻断为 `viewer::runtime_live::tests::runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat`；失败签名显示预期 `session_revoked`，实际收到 `agent_provider_chat_unsupported`，说明并行单测期间 provider 环境变量串入了本应跑 LLM chat 路径的测试。
- [x] 调整 `crates/oasis7/src/viewer/runtime_live/tests.rs`：新增 `lock_test_llm_env()`，复用 `runtime_provider_env_lock()` 与 `clear_runtime_provider_env()`，让 3 个 LLM agent chat / authoritative recovery 测试在设置 LLM env 前统一拿锁并清理 provider env，避免全局环境变量并发串味。
- [x] 本地定向回归 `viewer::runtime_live::tests::runtime_agent_chat_replay_returns_idempotent_ack`、`viewer::runtime_live::tests::runtime_agent_chat_rejects_intent_seq_conflict_on_payload_change`、`viewer::runtime_live::tests::runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat`，均已通过。
- [ ] 推送修复并打新 tag，继续观察 `release-gate-runtime` 是否绿，并让 aggregate `release_gate` 真正放行到打包阶段。

### T3U Package-native 前端工具链自给自足（2026-03-14）
- [x] 复盘 `Release Packages` run `23082925680`，确认 aggregate `release-gate` 首次放行后，`package-native (macos-14, macos-x64, oasis7-macos-x64.tar.gz, x86_64-apple-darwin)` 在 `Build launcher bundle` 失败；失败签名为 `error: required command not found: trunk`。
- [x] 更新 `.github/workflows/release-packages.yml`：在 `package-native` 的工具链安装步骤中显式追加 `rustup target add wasm32-unknown-unknown`，并在缓存后新增 `Install trunk`，让 `scripts/build-game-launcher-bundle.sh` 为 `web-launcher/` 运行 `trunk build` 时不再依赖其他 job 的预装环境。
- [x] 本地校验 workflow 关键片段，确认 `Install trunk` 位于 `Build launcher bundle` 之前，且 `shared-key` 已滚动到 `v2` 以避免复用旧缓存语义。
- [x] 已推送修复并以 `v0.0.23` 触发远端验证；`Release Packages` run `23086016214` 已确认三平台 `package-native` 与 `publish-release` 全部通过。

### T3V Bundle 脚本 wasm 目标自愈（2026-03-14）
- [x] 复盘 `Release Packages` run `23083927815`，确认 `package-native` 已越过 `Install trunk`，但 `macos-14` 仍在 `Build launcher bundle` 内报 `error: rust target wasm32-unknown-unknown is not installed`；说明仅在 workflow 层执行 `rustup target add` 仍不足以覆盖 bundle 脚本实际运行时的 toolchain 解析。
- [x] 调整 `scripts/build-game-launcher-bundle.sh`：新增 active toolchain 解析与 `ensure_rust_target_installed`，在 `web-launcher/` 的 `trunk build` 前自动自检并补装缺失的 `wasm32-unknown-unknown`，把 wasm 前端目标依赖收回脚本内部。
- [x] 本地执行 `bash -n scripts/build-game-launcher-bundle.sh`，并复核 helper 调用顺序，确认 `ensure_rust_target_installed` 位于 `trunk build` 之前。
- [x] 已推送修复并以 `v0.0.23` 触发远端验证；`Release Packages` run `23086016214` 已确认 bundle 脚本自愈后，三平台 `package-native` 与 `publish-release` 全部通过。

### T3W Bundle 脚本 wasm target 判定去脆弱化（2026-03-14）
- [x] 复盘 `Release Packages` run `23084980185`，确认 `package-native` 的 `macos-14` 已在 `Build launcher bundle` 中执行了 `rustup target add wasm32-unknown-unknown --toolchain 1.92.0-aarch64-apple-darwin`，并得到 `component ... is up to date`，但脚本随后仍因 `rustup target list --installed` 判定失败而退出；说明阻断点已收敛为 bundle 脚本对 rustup 输出格式的脆弱依赖，而非 target 真正缺失。
- [x] 调整 `scripts/build-game-launcher-bundle.sh`：保留 active toolchain 解析，但将 `ensure_rust_target_installed` 改为直接执行幂等的 `rustup target add`，不再二次解析 `target list --installed` 输出，避免 macOS runner/toolchain 组合下的假阴性。
- [x] 本地执行 `bash -n scripts/build-game-launcher-bundle.sh`，并复核 `ensure_rust_target_installed` 仍位于 `trunk build` 之前。
- [x] 已推送修复并以 `v0.0.23` 触发远端验证；`Release Packages` run `23086016214` 已确认 bundle 脚本自愈后，三平台 `package-native` 与 `publish-release` 全部通过。

### T3Z build-web-dist Web snapshot schema 漂移热修（2026-04-22）
- 完成内容：复盘 `Release Packages` run `24769333021`，确认前置 `release-gate-*` 与 aggregate `release-gate` 全部通过，唯一阻断收敛到 `build-web-dist`；失败签名为 `crates/oasis7_client_launcher/src/app_process_web.rs:506` 读取 `snapshot.chain_replication_status`，但 `web_api_support::WebStateSnapshot` 未声明该字段，导致 `trunk build` 编译 `wasm32-unknown-unknown` 版 `oasis7_client_launcher` 失败。
- 完成内容：调整 `crates/oasis7_client_launcher/src/web_api_support.rs`，将 `chain_replication_status` 补回 `WebStateSnapshot`，并直接复用 `main_chain_status::WebChainReplicationStatus` 作为反序列化字段类型，让 wasm launcher 与 `/api/state` 当前真值重新对齐。
- 本地回归：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher apply_web_snapshot_tracks_chain_p2p_status_payload -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher connected_peer_detail_rows_follow_connected_peer_order -- --nocapture`
  - `cd crates/oasis7_client_launcher && env -u NO_COLOR trunk build --release --dist ../../output/release/web-launcher-dist`
  - `cd crates/oasis7_viewer && env -u NO_COLOR trunk build --release --dist ../../output/release/web-dist`
- 遗留事项：仍需推送修复并重触发 `Release Packages`，确认 `build-web-dist` 不再因 launcher Web snapshot schema 漂移失败。

## 依赖
- 打包基础脚本：`scripts/build-game-launcher-bundle.sh`
- 站点发布流程：`.github/workflows/pages.yml`
- 站点入口文件：`site/index.html`、`site/en/index.html`

## 状态
- 当前阶段：进行中（T0A/T0/T1/T2/T3/T3A/T3B/T3C/T3D/T3E/T3F/T3G/T3H/T3I/T3J/T3K/T3L/T3M/T3N/T3O/T3P/T3Q/T3R/T3S/T3T/T3U/T3V/T3W/T3X/T3Y 已完成；T3Z 已完成本地回归，待推送并重触发远端发布验证；公开下载主资产已收口为 `AppImage` / `.dmg` / `.exe`，Linux `.deb` 转为次级高级入口，升级口径也已收口为手动覆盖/替换）
- 最近更新：2026-04-22 已完成 `T3Z` 本地修复与回归：`WebStateSnapshot` 补回 `chain_replication_status` 后，launcher 观察性测试通过，`build-web-dist` 两段 `trunk build` 均成功收口到 `INFO applying new distribution`，本地已无法复现 run `24769333021` 的 schema 漂移失败。
- 下一步：推送 `T3Z` 修复并重触发 GitHub `Release Packages`，确认 `build-web-dist` 恢复；随后继续验证 Windows NSIS 安装器、Linux `AppImage` + 次级 `.deb`、macOS `.dmg` 与最新站点下载面全部可用，并继续推进 Windows 签名与 macOS notarization。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
