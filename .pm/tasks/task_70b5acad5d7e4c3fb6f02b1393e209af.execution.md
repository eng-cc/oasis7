# task_70b5acad5d7e4c3fb6f02b1393e209af Execution Log

- task_uid: task_70b5acad5d7e4c3fb6f02b1393e209af
- title: diagnose local viewer lag on 127.0.0.1:49850
- owner_role: tpm
- worktree_hint: /Users/scc/ccwork/worktrees/oasis7-viewer-local-url-lag-diagnosis-20260608

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
  - Action: ...
  - Validation Command: ...
  - Expected Result: ...
  - Actual Result: ...
  - Blocker / Next Action: ...
-->

## 2026-06-08 20:55:30 CST / tpm
- 完成内容: 完成 local viewer 卡顿的第二轮根因缩小，确认当前机器虽然有 GPU，但浏览器内嵌的 pixel-world Bevy/WASM runtime 实际跑在 `WebGL2/Gl` 后端，因此命中 Bevy 0.18 的 GPU preprocessing 禁用分支并回退到 CPU preprocessing。
- 遗留事项: 若要真正消除当前 URL 的卡顿，需要继续决定是改本地 playtest 默认 URL 让其 deferred/fallback，还是为本机 playtest 提供可稳定命中 WebGPU/非-Gl backend 的浏览器/启动路径。
- Repository State Impact: 当前回合只追加诊断证据，不修改产品代码。
- Routed Next Phase: `systematic-debugging` read-only narrowing；已从“主观卡顿”收缩到“embedded renderer backend capability mismatch”。
- Subagent Intent / Limitation: 按 oasis7 workflow，此类 viewer 性能判断本应由 `viewer_engineer` / `qa_engineer` bounded slice 产出；但本线程当前工具策略禁止未获用户显式授权的 subagent 派发，因此本回合只能由 TPM 记录 fallback evidence path 与 attribution boundary，不把结论包装为角色 slice 定论。
- Fallback Evidence Path: in-app browser console logs for `http://127.0.0.1:49850/?ws=ws://127.0.0.1:49851&test_api=1&locale=zh`; local process sampling for PIDs `13756` and `13885`; repo inspection of `crates/pixel_world_bridge/Cargo.toml`, `crates/pixel_world_bridge/src/lib.rs`, `crates/oasis7_viewer/software_safe_src/pixel_world_runtime_module_wasm.js`, and Bevy registry source `bevy_render-0.18.1/src/batching/gpu_preprocessing.rs`.
- Attribution Boundary: TPM 只整合客观证据与源码条件；专业性能判定若需正式 role 归因，后续应在允许的环境下补派 `viewer_engineer` / `qa_engineer` slice。
- Action: 复现原始 URL 与 `pixel_world_renderer=defer` 对照 URL；读取浏览器日志中的 `AdapterInfo` 和 GPU preprocessing 提示；检查 `pixel_world_bridge` 的 Bevy web 构建特征与 Bevy 0.18 对 `adapter_info.backend == Gl` 的 CPU fallback 条件。
- Validation Command: in-app browser open `http://127.0.0.1:49850/?ws=ws://127.0.0.1:49851&test_api=1&locale=zh`; sample console logs; `ps -p 13756,13885 -o pid,ppid,%cpu,%mem,etime,command`; `sed -n '1,80p' crates/pixel_world_bridge/Cargo.toml`; `sed -n '559,640p' crates/pixel_world_bridge/src/lib.rs`; `sed -n '1,220p' crates/oasis7_viewer/software_safe_src/pixel_world_runtime_module_wasm.js`; `sed -n '1110,1165p' ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy_render-0.18.1/src/batching/gpu_preprocessing.rs`.
- Expected Result: 如果卡顿主因来自 renderer backend 能力而不是 ws 或服务端负载，则应同时观察到 `pixel-world-bridge` 运行日志、`backend: Gl` / `WebGL 2.0` 适配器信息、Bevy 的 CPU preprocessing fallback 条件、以及本地 live 进程未被打满。
- Actual Result: 证据一致。浏览器日志报告 `AdapterInfo ... driver_info: "WebGL 2.0 (OpenGL ES 3.0 Chromium)", backend: Gl`，随后紧跟 `GPU preprocessing is not supported on this device. Falling back to CPU preprocessing.`；`pixel_world_bridge` 当前以 `bevy = { default-features = false, features = ["2d", "web"] }` 构建，且 Bevy 0.18 的 `GpuPreprocessingSupport` 在 `adapter_info.backend == wgpu::Backend::Gl` 时直接回退 `GpuPreprocessingMode::None`。进程采样里 `oasis7_viewer_live` 仅约 `4.9%` CPU，未见 ws/服务端瓶颈。
- Blocker / Next Action: 无代码层 blocker。向用户汇报根因链条，并建议优先改默认本地 playtest 路径或切换到可稳定提供 WebGPU backend 的浏览器环境，再决定是否需要产品代码或启动脚本修补。

## 2026-06-08 21:08:40 CST / tpm
- 完成内容: 确认当前 pixel-world wasm 构建本身就没有显式选择 Bevy `webgpu` feature，因此现有本地页面不是“浏览器支持 WebGPU 但没被用上”，而是编译产物和运行日志都指向 `webgl` 路线。
- 遗留事项: 若要验证 Codex in-app browser / Chromium 会话本身是否也禁用了 `navigator.gpu`，仍可在独立最小页面里继续验证；但这已经不是当前 embedded renderer 落到 `Gl` 的唯一解释。
- Action: 对照 Bevy 0.18 feature 映射与本地 wasm 构建日志，检查 `bevy/web`、`bevy/webgpu`、`bevy/webgl2` 关系，并核对 `pixel_world_bridge` 当前依赖声明。
- Validation Command: `system_profiler SPDisplaysDataType`; `sed -n '1,80p' crates/pixel_world_bridge/Cargo.toml`; `sed -n '2588,2602p' ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy-0.18.1/Cargo.toml`; `sed -n '441,466p' ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy_internal-0.18.1/Cargo.toml`; local wasm build process sample showing `bevy_sprite_render ... --cfg feature="webgl"` and no `feature="webgpu"`.
- Expected Result: 若当前 crate 只开了通用 `web` 而未开 `webgpu`，则 Bevy feature 图应显示 `web = bevy_internal/web`，而 `webgpu` / `webgl2` 为独立 feature；本地编译日志应出现 `webgl` 相关 cfg。
- Actual Result: 证据一致。硬件侧 `system_profiler` 报告 `Apple M3` / `Metal 4`；但 `pixel_world_bridge` 仅声明 `bevy` features `["2d", "web"]`。Bevy 0.18 顶层 feature 映射显示 `web = ["bevy_internal/web"]`，而 `webgl2 = ["bevy_internal/webgl"]`、`webgpu = ["bevy_internal/webgpu"]` 为独立开关；`bevy_internal/web` 只包含 `bevy_app/web`、`bevy_platform/web`、`bevy_reflect/web`，并不等于 `webgpu`。同时本地 wasm 构建日志中 `bevy_sprite_render-0.18.1` 以 `--cfg feature="webgl"` 编译，未见 `feature="webgpu"`。
- Blocker / Next Action: 当前“不支持 WebGPU”的首要原因可归结为本仓库这条 pixel-world wasm 构建没有显式启用 Bevy `webgpu` feature；后续若要继续排查浏览器会话是否也屏蔽 `navigator.gpu`，需要再做最小化浏览器 capability 验证。

## 2026-06-08 21:18:10 CST / tpm
- 完成内容: 尝试对 Codex in-app browser 做最小化 `navigator.gpu` / WebGPU capability 验证，但受 Browser Use URL policy 限制，无法访问 `chrome://gpu`、`data:`、`file://` 等隔离探针页面，因此没法在本回合直接给出“该浏览器会话一定暴露/一定屏蔽 `navigator.gpu`”的硬证。
- 遗留事项: 若要拿到最终浏览器 capability 结论，需要在可控浏览器里打开最小 HTML 探针页，或换到允许 `chrome://gpu` / `file://` 检查的浏览器验证面。
- Action: 依次尝试 `chrome://gpu`、`about:blank` 只读探针、`data:` URL 探针、`file://` 本地探针，并退回到宿主/子进程命令行旁证。
- Validation Command: in-app browser open `chrome://gpu`; `tab.playwright.evaluate` on `about:blank`; in-app browser open `data:text/html,...`; in-app browser open `file:///Users/scc/ccwork/worktrees/oasis7-viewer-local-url-lag-diagnosis-20260608/.tmp/webgpu-probe.html`; `ps -Ao pid,ppid,command | rg 'Codex|Chrome for Testing|--use-angle=gl|--disable-webgpu|--enable-unsafe-webgpu'`.
- Expected Result: 若浏览器策略允许，则可直接读取 `navigator.gpu`/`requestAdapter`; 否则至少从宿主进程参数确认是否存在显式禁用 WebGPU 的开关。
- Actual Result: Browser Use 明确拦截了 `chrome://gpu`、`data:`、`file://` 导航；`playwright.evaluate` 的只读沙箱拿不到真实页面 `navigator`，因此不适合作为 capability 真值。宿主侧旁证里，Codex 自身 renderer/gpu 进程未见显式 `--disable-webgpu`；但自动化 Chrome for Testing 多次以 `--use-angle=gl` 启动，说明我们当前自动化浏览器面确实偏向 GL 路线，足以解释为何现有自动化观察长期落在 `backend: Gl`。
- Blocker / Next Action: 浏览器 capability 的“最终真值”本回合仍受工具策略阻挡；但这不改变主结论：当前 pixel-world wasm 构建未启用 Bevy `webgpu`，已经足以解释当前页面未走 WebGPU。

## 2026-06-08 21:27:35 CST / tpm
- 完成内容: 已把 `pixel_world_bridge` 的 Bevy 依赖改为显式启用 `webgpu`，并完成 Rust/WASM 侧最小验证，确认 feature 图里已经同时出现 `webgpu` 依赖链，`cargo check` 也通过。
- 遗留事项: 若要看到最终页面运行效果，还需要在当前 checkout 补齐 `crates/oasis7_viewer` 的 npm 依赖后再重建 software-safe bundle；本机当前 `vite` 缺失，导致前端 bundle 验证尚未跑通。
- Repository State Impact: 修改 `crates/pixel_world_bridge/Cargo.toml`，让当前 wasm renderer 显式尝试 Bevy `webgpu` 路线。
- Action: 将 `bevy` features 从 `["2d", "web"]` 调整为 `["2d", "web", "webgpu"]`；随后运行 feature tree 验证与 `cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`。
- Validation Command: `cargo tree --manifest-path /Users/scc/ccwork/oasis7/crates/pixel_world_bridge/Cargo.toml -e features --target wasm32-unknown-unknown | rg 'bevy_internal feature "webgpu"|bevy_render feature "webgpu"|wgpu feature "webgpu"|bevy_internal feature "webgl"|wgpu feature "webgl"'`; `env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`; attempted `./scripts/build-viewer-software-safe.sh`.
- Expected Result: feature 图中应出现 `bevy_internal -> bevy_render -> wgpu` 的 `webgpu` 分支；Rust 侧 `pixel_world_bridge` wasm 目标检查通过；若前端依赖齐全，则 software-safe bundle 也可继续重建。
- Actual Result: feature 图已同时出现 `bevy_internal feature "webgpu" -> bevy_render feature "webgpu" -> wgpu feature "webgpu"`，且保留 `webgl` 分支；`cargo check -p pixel_world_bridge --target wasm32-unknown-unknown` 在 2m46s 后成功完成。`./scripts/build-viewer-software-safe.sh` 未能继续，因为当前 checkout 缺少 `crates/oasis7_viewer/node_modules`，`npm run build:software-safe` 调起 `vite` 时命中 `sh: vite: command not found`。
- Blocker / Next Action: 代码侧改动已成立。下一步若要验证页面是否真的从 `Gl` 切到 `WebGPU`，先执行 `npm --prefix crates/oasis7_viewer ci`，再重建 software-safe bundle 并重新打开本地 viewer URL 看适配器日志。

## 2026-06-08 22:48:30 CST / tpm
- 完成内容: 已完成“装上并验证”的完整闭环。前端依赖已安装，software-safe viewer 产物已用显式 `bevy/webgpu` 配置重建，本地 launcher 栈也已在原始端口组合 `49850/49851` 成功启动；随后使用 in-app browser 对原始 URL 做了 fresh runtime 验证。
- 遗留事项: 当前问题已从“仓库未启用 Bevy webgpu”收缩为“当前浏览器运行时仍选择 `backend: Gl`”。后续若要真正消除 CPU preprocessing fallback，需要继续追浏览器会话是否不暴露 WebGPU、或 Bevy/wgpu 在该 Chromium 会话下为何未选中 WebGPU adapter。
- Repository State Impact: 保留 `crates/pixel_world_bridge/Cargo.toml` 的 `webgpu` feature 变更，并因依赖图扩展产生对应 `Cargo.lock` 更新。
- Action: 执行 `npm --prefix crates/oasis7_viewer ci`；执行 `./scripts/build-viewer-software-safe.sh`；执行 `./scripts/run-launcher-stack.sh --viewer-port 49850 --web-bind 127.0.0.1:49851 --live-bind 127.0.0.1:49852 --skip-llm-provider-preflight --output-dir output/local-webgpu-verify --json-ready`；在 in-app browser 打开 `http://127.0.0.1:49850/?ws=ws://127.0.0.1:49851&test_api=1&locale=zh` 并抓取 console logs。
- Validation Command: `npm --prefix crates/oasis7_viewer ci`; `./scripts/build-viewer-software-safe.sh`; `find output/local-webgpu-verify/web-dist/pixel-world-bridge -maxdepth 1 -type f | sort`; `rg -n "requestAdapter" output/local-webgpu-verify/web-dist/pixel-world-bridge/pixel_world_bridge_bindgen.js`; in-app browser logs for the original local URL.
- Expected Result: 若“装上”成功，则新的 `pixel-world-bridge` 产物应生成且含有 WebGPU API 绑定；若运行时也成功切换，则浏览器日志不应再显示 `backend: Gl` / `Falling back to CPU preprocessing`。
- Actual Result: 构建侧成功。`viewer.js`、`software_safe.js`、`output/local-webgpu-verify/web-dist/pixel-world-bridge/*` 全部刷新；`pixel_world_bridge_bindgen.js` 中已存在 `requestAdapter` 绑定，证明 WebGPU API 路径已进入前端产物。运行时侧仍未切换：in-app browser 打开原始 URL 后，日志继续报告 `AdapterInfo { ... driver_info: "WebGL 2.0 (OpenGL ES 3.0 Chromium)", backend: Gl }`，随后再次出现 `GPU preprocessing is not supported on this device. Falling back to CPU preprocessing.`。
- Blocker / Next Action: 本回合“安装变更 + 重新构建 + 原始 URL 运行时验证”已完成，没有构建 blocker；但功能目标“摆脱 Gl/CPU fallback”仍未达成。下一步应优先验证当前 Codex in-app browser/Chromium 会话是否实际暴露 WebGPU，并据此决定是改浏览器启动参数/运行环境，还是继续排 Bevy/wgpu 的 web backend 选择条件。

## 2026-06-08 22:57:30 CST / tpm
- 完成内容: 已按用户要求改用外部 Chrome 复验，同一条原始 URL 在外部浏览器里成功切到 WebGPU 后端，说明仓库变更已生效，且“回退到 CPU”的现象并非代码侧必然结果，而是与特定浏览器会话有关。
- 遗留事项: 需要继续解释为什么 Codex in-app browser 仍落在 `backend: Gl`，而外部 Chrome 能跑 `BrowserWebGpu`；当前最可能方向是 in-app browser/自动化 Chromium 的 WebGPU 能力或启动策略差异。
- Action: 通过 Chrome extension 通道接管外部 Chrome 中已打开的 `http://127.0.0.1:49850/?ws=ws://127.0.0.1:49851&test_api=1&locale=zh` tab，reload 后抓取 console logs。
- Validation Command: `./scripts/run-launcher-stack.sh --viewer-port 49850 --web-bind 127.0.0.1:49851 --live-bind 127.0.0.1:49852 --skip-llm-provider-preflight --output-dir output/local-webgpu-verify-ext --json-ready`; external Chrome tab logs for the original local URL.
- Expected Result: 若外部 Chrome 确实暴露 WebGPU，则同一条本地 URL 的日志应不再是 `backend: Gl`，而会出现 WebGPU 相关 backend，且不再打印 `Falling back to CPU preprocessing`。
- Actual Result: 复验成功。外部 Chrome 日志显示 `AdapterInfo { name: "", vendor: 0, device: 0, device_type: Other, driver: "", driver_info: "", backend: BrowserWebGpu }`，随后日志变为 `Some GPU preprocessing are limited on this device.`，不再出现 `GPU preprocessing is not supported on this device. Falling back to CPU preprocessing.`。这表明 `bevy/webgpu` feature + 重建产物在外部 Chrome 下已真正生效。
- Blocker / Next Action: “安装 + 外部浏览器验证”目标已完成。后续如需改善 Codex 内嵌调试体验，应单独排查 in-app browser 的 WebGPU 暴露/启动参数，而不是继续修改 `pixel_world_bridge` 代码。

## 2026-06-08 23:12:30 CST / tpm
- 完成内容: 开始按仓库 PR 流程推进并完成 preflight 缩面。确认当前分支的代码/任务证据尚未满足 `prepare-task-pr` 的 workflow gate，主要差异是 source worktree 仍为 dirty 状态，且 execution log 中还没有 `Pre-PR Local Role Review: passed` evidence packet。
- 遗留事项: 若要继续走 `prepare-task-pr --create`，需要先完成 closeout/commit；随后还需要 fresh local role review evidence。按本仓库规范这一步应由相关专业角色 slice 产出，但当前会话的 subagent 工具策略要求用户显式授权后才可实际派发。
- Action: 执行 `./scripts/prepare-task-pr.sh` 和 `./scripts/prepare-task-pr.sh --json` 预检 PR gate；检查当前 diff、task execution log、以及 `prepare-task-pr.sh` 的 local role review gate 条件。
- Validation Command: `git status --short`; `git diff --stat`; `./scripts/prepare-task-pr.sh`; `./scripts/prepare-task-pr.sh --json`; `rg -n "Pre-PR Local Role Review|review" scripts/prepare-task-pr.sh .pm/tasks/task_70b5acad5d7e4c3fb6f02b1393e209af.execution.md`.
- Expected Result: 若分支已可直接进 PR preflight，则 `prepare-task-pr` 应至少越过 clean worktree / local role review gate；否则应给出明确的缺失 gate。
- Actual Result: `prepare-task-pr` 当前立即失败于 `error: source worktree is dirty`。同时脚本源码明确要求存在 `- Pre-PR Local Role Review: passed` 及配套字段，否则在 PR create path 上会继续失败。
- Blocker / Next Action: 先推进 `task-closeout -> commit` 以清理 source worktree；然后评估/记录 local role review gate 的实际可执行性。若当前工具策略仍不允许实际派发 required role review slices，则需把 intended dispatch / tool limitation / attribution boundary 正式留痕，并向用户请求显式授权后再继续派发。

## 2026-06-09 00:07:30 CST / tpm
- 完成内容: 已完成当前 task 的 closeout 与提交，当前 source head 为 `6a58398e0b78ae55fcc46c33e6f4c7591cfb0426`。重新执行 `prepare-task-pr` 后，dirty worktree 阻塞已消失，preflight 现仅剩 `Pre-PR Local Role Review: passed` 缺失这一条明确 workflow gate。
- 遗留事项: 仍需 fresh local role review evidence；从 changed paths 看，最相关的专业角色至少包括 `wasm_platform_engineer`（web wasm / bevy webgpu 依赖路径）与 `qa_engineer`（外部浏览器验证与运行时差异复验）。当前会话的 subagent 工具策略要求用户显式授权后才可实际派发，因此尚未满足该 gate。
- Repository State Impact: 已提交本 task slice，commit 为 `Enable Bevy WebGPU for pixel world bridge` (`6a58398e0`)。
- Action: 执行 `./scripts/pm/task-closeout.sh --role tpm --task-uid task_70b5acad5d7e4c3fb6f02b1393e209af --verify-command './scripts/build-viewer-software-safe.sh' --no-lint --json`；执行 `git commit -m 'Enable Bevy WebGPU for pixel world bridge'`；重新执行 `./scripts/prepare-task-pr.sh` 与 `./scripts/prepare-task-pr.sh --json`。
- Validation Command: `./scripts/pm/task-closeout.sh --role tpm --task-uid task_70b5acad5d7e4c3fb6f02b1393e209af --verify-command './scripts/build-viewer-software-safe.sh' --no-lint --json`; `git show --stat --oneline --summary HEAD`; `./scripts/prepare-task-pr.sh`.
- Expected Result: closeout 与 commit 成功后，preflight 若仍有阻塞，应只剩 role review evidence 或 full required validation 一类明确 gate，而不再有脏工作区问题。
- Actual Result: closeout 成功，`last_verification_status=verified`；commit 成功，HEAD 为 `6a58398e0`。`prepare-task-pr` 现报告：`Pre-PR Local Role Review` 状态为 `missing`，原因为 `no pre-PR local role review packet found`；同时给出 full required validation 推荐命令，但未再报 dirty worktree。
- Blocker / Next Action: 继续运行 recommended required validation 作为技术 readiness 证据；与此同时，若用户允许实际派发本地 role-review subagent slices，则下一步应派发 `wasm_platform_engineer` 与 `qa_engineer` review 并写回 passed evidence packet，然后才能继续 `prepare-task-pr --create`。

## 2026-06-09 00:14:30 CST / tpm
- Review Trigger: pre-PR local role review
- Review Scope: `crates/pixel_world_bridge/Cargo.toml`; `Cargo.lock`; task evidence in `.pm/tasks/task_70b5acad5d7e4c3fb6f02b1393e209af.execution.md`; validation claim around local/external browser WebGPU behavior
- Review Roles: wasm_platform_engineer, qa_engineer
- Review Question: 1) 这次 `bevy/webgpu` 依赖改动是否是最小且正确的 wasm/web 平台修复；2) 当前验证证据是否足以支撑“外部浏览器已切到 WebGPU、内嵌浏览器仍是环境差异”的 PR 口径；3) 是否存在阻断 PR 的剩余风险或缺失验证
- Evidence Available: `cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`; `./scripts/build-viewer-software-safe.sh`; in-app browser runtime logs showing `backend: Gl`; external Chrome runtime logs showing `backend: BrowserWebGpu`; `prepare-task-pr --json` output; current source head `6a58398e0b78ae55fcc46c33e6f4c7591cfb0426`
- Expected Return Contract: findings | no_findings | residual_risk
- Formal Sink: `.pm/tasks/task_70b5acad5d7e4c3fb6f02b1393e209af.execution.md`

## 2026-06-09 00:20:30 CST / wasm_platform_engineer
- Review Outcome: no_findings
- Review Scope: `crates/pixel_world_bridge/Cargo.toml`; `Cargo.lock`; execution evidence tied to source head `6a58398e0b78ae55fcc46c33e6f4c7591cfb0426`
- Findings: none
- Residual Risk: 本次 `bevy/webgpu` 改动是最小且正确的 wasm/web 平台修复；剩余风险主要来自浏览器运行环境差异，而不是仓库内 wasm/platform 配置错误。PR 口径应限定为“启用 WebGPU feature 并验证外部 Chrome 生效”，不要把 in-app browser 的 `Gl` 行为写成代码回归。

## 2026-06-09 00:21:30 CST / qa_engineer
- Review Outcome: findings
- Review Scope: current verification evidence, closeout/commit state, `prepare-task-pr` preflight result, required validation coverage
- Findings:
  - 当前还不能把 `Pre-PR Local Role Review` 直接判定为 passed，因为 execution log 里尚未有最终 passed evidence packet。
  - `./scripts/build-viewer-software-safe.sh` 的 fresh verification 只证明构建闭环成立，不等同于 full required validation 已完成；PR 口径不能夸大为“所有 required 面已完成”。
- No Findings:
  - 对“代码改动已生效，外部 Chrome 已切到 BrowserWebGpu，而 in-app browser 仍是环境差异”这一结论，现有证据充分。
- Residual Risk:
  - in-app browser 根因尚未查清，不应把该环境的体验改善作为本 PR 已交付结果。
  - 推荐的 `./scripts/ci-tests.sh required` 仍需完成，作为更完整的技术 readiness 证据。

## 2026-06-09 00:23:00 CST / tpm
- 完成内容: 已合流两条本地 role review 结果。`wasm_platform_engineer` 给出 `no_findings`；`qa_engineer` 的有效 findings 主要是流程/验证口径约束，而非代码缺陷。与此同时，recommended required validation 已经从最早的 `doc-governance-check.sh` 卡点继续向后推进，目前已观察到 `doc-governance-check: OK`、`lint-skills: OK`、`check-windows-paths: OK`、`check-script-executable-bits: OK`、`cargo-dev-lib.test: OK`、`plan-rust-required-scope.test: OK` 等连续通过信号。
- 遗留事项: 还需要等 `./scripts/ci-tests.sh required` 整体收口后，才能决定是否把 QA 对“required verification 未完成”的流程性 finding 记为已解决；在此之前，不应写入 `Pre-PR Local Role Review: passed` 最终 packet。
- Action: 读取 `wasm_platform_engineer` / `qa_engineer` review 返回；继续轮询 required validation 输出。
- Validation Command: local role review slices for `wasm_platform_engineer` and `qa_engineer`; running `OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=true OASIS7_CI_RUN_CONSENSUS_TESTS=true OASIS7_CI_RUN_DISTFS_TESTS=true OASIS7_CI_RUN_OASIS7_NODE_TESTS=true OASIS7_CI_RUN_OASIS7_NET_TESTS=true OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=true OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=true OASIS7_CI_RUN_VIEWER_WASM_CHECK=true OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=true ./scripts/ci-tests.sh required`.
- Expected Result: 若 review 无代码阻断，且 required validation 继续通过，则剩余阻塞应只收敛为“等待验证链完成并据此写入 final passed packet”。
- Actual Result: review 已确认无新的代码级阻断；required validation 目前已越过 `doc-governance` 并开始连续通过后续脚本，但尚未整体结束。
- Blocker / Next Action: 继续等待/读取 required validation 完整结果；若整条链通过，则合流 review findings disposition 并补写 `Pre-PR Local Role Review: passed` packet，再重新执行 `prepare-task-pr`。

## 2026-06-09 00:42:30 CST / tpm
- 完成内容: 为了去掉伪阻塞，已清理当前 worktree 中残留的旧 `ci-tests.sh required` 进程和旧 `run-launcher-stack` 进程，随后在干净环境下重启 recommended required validation。重启后的链路再次从 `+ ci test tier: required` 开始，并进入 `+ ./scripts/doc-governance-check.sh`；当前确认该步骤仍在真实执行而非 build lock 卡死。
- 遗留事项: 在干净环境下，`doc-governance-check.sh` 仍是长时间运行项，暂时没有成功或失败结论，因此 recommended required validation 依旧未收口。
- Action: 杀掉旧的 `bash ./scripts/ci-tests.sh required`、`cargo test -p oasis7 --bin oasis7_newapi_bridge_service`、以及历史 `run-launcher-stack.sh --viewer-port 49850 ...` 进程；重新执行 full required validation；检查 `doc-governance-check.sh` 的子进程与运行状态。
- Validation Command: `kill <old ci-tests / cargo test / launcher pids>`; `OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=true OASIS7_CI_RUN_CONSENSUS_TESTS=true OASIS7_CI_RUN_DISTFS_TESTS=true OASIS7_CI_RUN_OASIS7_NODE_TESTS=true OASIS7_CI_RUN_OASIS7_NET_TESTS=true OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=true OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=true OASIS7_CI_RUN_VIEWER_WASM_CHECK=true OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=true ./scripts/ci-tests.sh required`; `ps -Ao pid,ppid,etime,%cpu,%mem,command | rg 'ci-tests.sh required|doc-governance-check.sh'`.
- Expected Result: 若旧进程竞争是主因，则重启后的 required validation 应避免 build lock 并继续推进；若仍慢，则应至少确认是 `doc-governance-check.sh` 本身长跑，而不是伪阻塞。
- Actual Result: 旧竞争进程已清理；新的 required validation 不再报 build lock，目前实际运行在 `bash ./scripts/doc-governance-check.sh`（PID `23105`）及其子进程上，说明阻塞面已缩小到该长跑检查本身。
- Blocker / Next Action: 当前 PR 流程剩余未收口项为 recommended required validation。local role review 的代码级意见已收口，但由于 QA 明确把“required validation 未完成”列为有效流程 finding，暂不写入 `Pre-PR Local Role Review: passed` 最终 packet；继续等待或单独拆分 `doc-governance-check.sh` 的完成结论。

## 2026-06-09 10:01:01 CST / runtime_engineer
- Review Outcome: no_findings
- Review Scope: `crates/oasis7/src/bin/oasis7_newapi_bridge_service/tests.rs`; `crates/oasis7/Cargo.toml`; `scripts/newapi-bridge-service/pricing-rules.example.env`; fresh rerun of `cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture`
- Findings: none
- Residual Risk: `pricing_rules_example_env_path()` 在当前仓库下可正确解析到 `scripts/newapi-bridge-service/pricing-rules.example.env`；若后续再次出现同类“文件不存在”噪音，应优先检查 shared target 缓存是否复用了其他 worktree 产物，而不是先修改测试路径。

## 2026-06-09 10:01:01 CST / tpm
- 完成内容: 已在 fresh required validation 中确认整条 `./scripts/ci-tests.sh required` 完整通过。过程中一度复现 `oasis7_newapi_bridge_service` 测试读取 `pricing-rules.example.env` 失败，但进一步取证表明那不是代码回归：共享 `target` 缓存曾复用旧 worktree 编出的测试二进制，导致其内嵌的 `CARGO_MANIFEST_DIR` 指向其他 worktree；在执行 `cargo clean -p oasis7` 后，单测与 required 链均在当前 canonical worktree 上 fresh 通过。
- 遗留事项: none for PR readiness. in-app browser 仍使用 `backend: Gl` 的环境差异问题不在本 PR 修复范围内。
- Action: 读取 required validation 全量输出；对照运行最小通过/失败单测 `parse_cli_options_rejects_zero_route_ttl` 与 `bind_user_persists_binding_and_project_binding`；检查测试二进制里的嵌入路径；执行 `cargo clean -p oasis7` 后重跑单测与 full required validation；合流 `runtime_engineer` 只读核对结果。
- Validation Command: `OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=true OASIS7_CI_RUN_CONSENSUS_TESTS=true OASIS7_CI_RUN_DISTFS_TESTS=true OASIS7_CI_RUN_OASIS7_NODE_TESTS=true OASIS7_CI_RUN_OASIS7_NET_TESTS=true OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=true OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=true OASIS7_CI_RUN_VIEWER_WASM_CHECK=true OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=true ./scripts/ci-tests.sh required`; `cargo clean -p oasis7`; `cargo test -p oasis7 --bin oasis7_newapi_bridge_service bind_user_persists_binding_and_project_binding -- --nocapture`; `cargo test -p oasis7 --bin oasis7_newapi_bridge_service parse_cli_options_rejects_zero_route_ttl -- --nocapture`; `strings target/debug/deps/oasis7_newapi_bridge_service-* | rg 'pricing-rules\\.example\\.env|crates/oasis7'`
- Expected Result: full required validation green；若 shared target 噪音是根因，则包级 clean 后 `oasis7_newapi_bridge_service` 的 fixture 读取测试应恢复通过，且 fresh 编译产物会绑定当前 worktree 路径。
- Actual Result: `./scripts/ci-tests.sh required` fresh 通过，覆盖 `doc-governance-check`、Rust required tests、`oasis7_newapi_bridge_service` 28 个单测、`oasis7_net` libp2p 测试、viewer UI tests、`./scripts/build-viewer-software-safe.sh`、以及 `trunk build --release --dist ../../output/release/web-launcher-dist`。清理前的失败来自旧测试二进制内嵌了另一条 worktree 路径；`cargo clean -p oasis7` 后，`bind_user_persists_binding_and_project_binding` 与 `parse_cli_options_rejects_zero_route_ttl` 都在当前 worktree fresh 通过，证明这是 shared target 缓存噪音而非本次改动引入的代码问题。
- Blocker / Next Action: fresh required validation 与 local role review findings 已全部收口；补写最终 `Pre-PR Local Role Review: passed` packet 并重新执行 `prepare-task-pr`.

- Pre-PR Local Role Review: passed
- Task UID: task_70b5acad5d7e4c3fb6f02b1393e209af
- Source Worktree: /Users/scc/ccwork/worktrees/oasis7-viewer-local-url-lag-diagnosis-20260608
- Source Branch: task/viewer-local-url-lag-diagnosis-20260608
- Source Head: 6a58398e0b78ae55fcc46c33e6f4c7591cfb0426
- Comparison Ref: origin/main
- Reviewed Changed Paths: crates/pixel_world_bridge/Cargo.toml; Cargo.lock; .pm/tasks/task_70b5acad5d7e4c3fb6f02b1393e209af.execution.md
- Role Selection Basis: changed wasm/web dependency surface plus verification-dependent runtime claim about external Chrome switching to `BrowserWebGpu` while in-app browser remains `Gl`; included `wasm_platform_engineer` for Bevy/WASM feature correctness and `qa_engineer` for validation/release-readiness boundary. Added read-only `runtime_engineer` confirmation only to explain the fresh required-validation false negative in unrelated `oasis7_newapi_bridge_service` tests.
- Review Roles: wasm_platform_engineer, qa_engineer
- Review Evidence: execution-log sections `2026-06-09 00:14:30 CST / tpm`, `2026-06-09 00:20:30 CST / wasm_platform_engineer`, `2026-06-09 00:21:30 CST / qa_engineer`, `2026-06-09 10:01:01 CST / runtime_engineer`, and `2026-06-09 10:01:01 CST / tpm`
- Review Findings Disposition: addressed
- Finding Disposition Evidence: `qa_engineer` 的流程性 findings 已被 fresh required validation 通过所消解；`runtime_engineer` 只读核对进一步确认 `oasis7_newapi_bridge_service` 的早先失败来自 shared target 缓存复用旧 worktree 产物，而非当前分支代码回归。代码改动本身无新增 findings，且 `wasm_platform_engineer` 维持 `no_findings`。
- Residual Risk: 代码层剩余风险低。当前未收口项是环境差异而非仓库改动本身: Codex in-app browser 仍落在 `backend: Gl`，所以本 PR 只能声明“为 pixel world bridge 启用 Bevy WebGPU，并在外部 Chrome 验证生效”，不能宣称内嵌浏览器 CPU fallback 已被修复。
