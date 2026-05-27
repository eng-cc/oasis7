# oasis7：启动器 bundle-first 试玩入口收敛（2026-03-12）

审计轮次: 1

## 目标
- 建立启动器 bundle-first 试玩入口专题规格，统一制作人试玩、发布前人工验收与开发源码回归之间的使用边界。
- 确保启动器 bundle 入口、手册、自动化与人工清单的执行口径可追溯到同一组 PRD-ID / task / 证据链。

## 范围
- 覆盖 `scripts/run-game-test.sh --bundle-dir`、`scripts/run-producer-playtest.sh`、`scripts/run-game-test-ab.sh` 在启动器 bundle-first 场景中的职责划分。
- 覆盖 `testing-manual.md`、启动器人工测试清单、`doc/testing/project.md`、`doc/testing/prd.index.md` 与本专题项目文档的回写要求。
- 不覆盖 `oasis7_game_launcher` / `oasis7_web_launcher` 的业务功能扩展，也不新增独立第四条自动化链路。

## 接口 / 数据
- PRD 主入口: `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md`
- 项目管理入口: `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.project.md`
- 模块主 PRD: `doc/testing/prd.md`
- 模块主项目: `doc/testing/project.md`
- 文件级索引: `doc/testing/prd.index.md`
- 操作手册: `testing-manual.md`
- 关联清单: `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`

## 里程碑
- M1 (2026-03-12): 建立启动器 bundle-first 专题三件套并回写 testing 索引。
- M2 (2026-03-12): 为 bundle-first 试玩入口补齐 headed / renderer / freshness guardrail 口径与验证。
- M3: 继续观察不同图形环境下的 bundle-first 默认稳定性，必要时再拆专项治理。

## 风险
- 若专题 PRD 缺少 paired project 追溯或 legacy 头部章节，将直接触发文档门禁失败，导致测试结论无法放行。
- 若 bundle-first 与源码模式口径再次混淆，制作人试玩仍可能退回开发态链路，削弱证据可信度。
- 若 freshness / renderer 阻断规则未同步到手册与自动化，容易重复出现环境阻断与真实玩法回归混淆。

## 1. Executive Summary
- Problem Statement: 当前 `testing-manual.md` 与 `scripts/run-game-test.sh` 都把 `oasis7_game_launcher` 说成默认 Web 闭环入口，但没有明确区分“源码直接运行”与“打包后 bundle 产物运行”。这会让制作人试玩、发布前人工验收和开发回归混用同一口径，导致操作者更容易直接走 `cargo run`，偏离真实交付物体验，也放大静态资源、执行目录与本地状态混杂带来的误判。
- Proposed Solution: 将启动器试玩入口明确收敛为“双模”策略：`bundle-first` 作为制作人试玩、发布前人工验收和对外交付样张的默认入口；`scripts/run-game-test.sh` 保留，但降级为开发回归 bootstrap，并新增 `--bundle-dir` 以直接消费打包产物。进一步提供 `scripts/run-producer-playtest.sh` 作为制作人一键入口，自动准备 bundle 后再进入 bundle 模式启动。
- Success Criteria:
  - SC-1: 手册、启动器人工测试清单和脚本帮助文本都明确区分 `bundle` 验收入口与源码回归入口。
  - SC-2: `scripts/run-game-test.sh` 支持 `--bundle-dir <bundle>`，可直接通过 `<bundle>/run-game.sh` 启动游戏。
  - SC-3: `scripts/run-game-test-ab.sh` 无需额外适配即可透传 `--bundle-dir`，并在帮助口径中明确其作为 bundle 自动化哨兵的用法。
  - SC-4: 至少完成一轮 bundle 构建 + `run-game-test-ab.sh --bundle-dir <bundle>` 闭环验证，并输出明确的通过或阻断证据，避免口径停留在纸面。
  - SC-5: 提供单命令制作人试玩入口，默认可复用或自动构建本地 bundle，再进入 bundle 模式启动。
  - SC-6: 单命令入口支持可选的 `--open-headed`，在 URL 就绪后自动打开 headed 浏览器，避免制作人再手动复制 URL。
  - SC-7: 通过 `--open-headed` 拉起的浏览器会话在脚本退出时自动关闭，避免遗留残窗或脏会话。
  - SC-8: `run-producer-playtest.sh --open-headed` 与 `run-game-test-ab.sh --headed` 默认固定硬件 WebGL 启动参数；若 headed 仍命中 software renderer，必须按环境阻断而不是给出玩法结论。
  - SC-9: 已存在 bundle 若缺少 freshness manifest 或落后于当前工作区源码，制作人入口必须自动重建，底层 `run-game-test.sh --bundle-dir` 必须默认 fail-fast，避免继续误用旧 Viewer Web 产物。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要以真实交付物口径亲自试玩，而不是落回源码态调试链路。
  - `qa_engineer`：需要一条稳定、可审计的 bundle 验收入口，避免把开发态结果误当发布结论。
  - `viewer_engineer`：需要保留源码态快速回归入口，便于调试协议、静态资源与 Viewer Web 问题。
- User Scenarios & Frequency:
  - 每次制作人试玩、发布前人工验收、对外样张抽检时执行 bundle-first 入口。
  - 每次脚本开发调试、端口/协议问题排查时，允许继续使用源码态 bootstrap。
- User Stories:
  - PRD-TESTING-LAUNCHER-BUNDLE-001: As a `producer_system_designer`, I want manual playtests to start from packaged launcher artifacts, so that my verdict reflects the real product handoff.
  - PRD-TESTING-LAUNCHER-BUNDLE-002: As a `qa_engineer`, I want the existing `run-game-test.sh` bootstrap to consume a bundle via one flag, so that automation and manual validation share one script surface.
- Critical User Flows:
  1. Flow-LBFP-001: `run-producer-playtest.sh --open-headed -> 自动准备/复用 bundle -> run-game-test.sh --bundle-dir -> 自动打开 headed agent-browser（默认 `--use-angle=gl,--ignore-gpu-blocklist`） -> 人工游玩 -> 脚本退出时自动关闭该浏览器会话 -> 记录人工结论`
  2. Flow-LBFP-002: `run-game-test.sh --bundle-dir <bundle> -> freshness manifest 校验 -> 输出 URL/日志 -> run-game-test-ab.sh 采样并校验 renderer 不是 software path`
  3. Flow-LBFP-003: `run-game-test.sh (源码模式) -> 开发者快速复现 -> 不作为发布结论`

## 3. Scope & Acceptance
- In Scope:
  - `scripts/run-game-test.sh` 增加 bundle 入口并保留源码 fallback。
  - `scripts/run-producer-playtest.sh` 提供制作人一键入口。
  - `scripts/run-game-test-ab.sh`、`testing-manual.md`、启动器人工测试清单同步 bundle-first 口径。
  - `doc/testing/project.md`、`doc/testing/prd.index.md`、`doc/testing/README.md`、`doc/devlog/README.md` 回写追溯。
- Out of Scope:
  - 不移除 `run-game-test.sh` 源码模式。
  - 不改造 `oasis7_game_launcher` / `oasis7_web_launcher` 参数协议。
  - 不新建独立的第三套 bundle 专用自动化脚本。
- Acceptance Criteria:
  - AC-1: `./scripts/run-game-test.sh --help` 明确 `--bundle-dir` 的定位与 bundle-first 推荐口径。
  - AC-2: 传入 `--bundle-dir` 时，脚本使用 `<bundle>/run-game.sh` 而不是 `cargo run` 拉起游戏。
  - AC-3: `testing-manual.md` 明确“制作人试玩 / 发布前人工验收默认走 bundle-first；源码模式仅用于开发回归”。
  - AC-4: `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md` 将 bundle-first 写为执行说明的一部分。
  - AC-5: headed 默认浏览器参数与 software renderer 阻断规则在脚本帮助、主手册和人工清单中保持一致。
  - AC-6: bundle 构建脚本会写 freshness manifest；复用旧 bundle 时，`run-game-test.sh` 默认阻断 stale bundle，`run-producer-playtest.sh` 默认自动重建 stale bundle。

## 4. Technical Specifications
- Architecture Overview: 维持现有 `run-game-test.sh -> URL/日志/端口就绪 -> agent-browser` 的外部契约不变，仅把启动执行器从单一 `cargo run oasis7_game_launcher` 扩展为 `bundle mode` 与 `source mode` 两条分支，其中 `bundle mode` 优先供人工验收和自动化哨兵使用。
- Integration Points:
  - `scripts/run-game-test.sh`
  - `scripts/run-game-test-ab.sh`
  - `scripts/run-producer-playtest.sh`
  - `scripts/build-game-launcher-bundle.sh`
  - `testing-manual.md`
  - `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
- Edge Cases & Error Handling:
  - `--bundle-dir` 指向不存在目录：脚本必须 fail-fast 并提示目录路径。
  - `--bundle-dir` 缺少 `run-game.sh`：脚本必须明确提示 bundle 不完整。
  - bundle 模式下未显式传 `--viewer-static-dir`：默认使用 bundle 自带 `web/`，不再偷偷 fresh build 源码目录。
  - `--headless` 若命中 `SwiftShader` / software renderer：必须按浏览器环境阻断快失败，并输出可操作提示，不得误判成 fresh Web 构建或玩法回归。
  - `--open-headed` / `--headed` 若在默认硬件 WebGL 参数下仍命中 `SwiftShader` / software renderer：同样必须阻断，并把 `browser_env.json` 作为环境证据落盘。
  - bundle 缺少 freshness manifest、或 manifest 与当前工作区 fingerprint 不一致：必须视为 stale bundle；`run-game-test.sh` 默认 fail-fast，`run-producer-playtest.sh` 默认重建。
  - 开发者仍需源码回归：保留现有 `cargo run` 分支，但帮助文本与手册必须标注其仅供开发排障。
- Non-Functional Requirements:
  - NFR-LBFP-1: 新增 bundle 模式不能破坏现有 `run-game-test-ab.sh` 透传契约。
  - NFR-LBFP-2: 脚本帮助、手册和人工清单的口径必须 0 冲突。
  - NFR-LBFP-3: bundle-first 入口的失败必须在一次执行中能定位到“目录错误 / 产物不完整 / 端口冲突 / 运行时失败”中的至少一类。
  - NFR-LBFP-4: `--headless` 下若浏览器退化到 `SwiftShader`，自动化必须给出环境级阻断，而不是返回模糊的 `connecting` 超时。
  - NFR-LBFP-5: `--open-headed` / `--headed` 默认参数、renderer 证据和阻断语义必须一致，避免“有头但仍是 SwiftShader”被误判为兼容。
  - NFR-LBFP-6: bundle-first 入口必须能识别本地 bundle 与当前工作区源码的漂移，避免旧 bundle 静态产物与新 runtime 二进制混跑。
- Security & Privacy: 不新增敏感数据采集，仅调整启动入口与文档口径。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (LBFP-1): 建立专题 PRD / design / project，并更新 testing 主索引。
  - v1.1 (LBFP-2): 为 `run-game-test.sh` 增加 `--bundle-dir` 并同步帮助文本。
  - v1.2 (LBFP-3): 更新主手册与人工清单，将 bundle-first 明确为人工验收默认入口。
  - v1.3 (LBFP-4): 完成 bundle 实机闭环验证与 devlog 收口。
- Technical Risks:
  - 风险-1: 若只改文档不改脚本，使用者仍会被现成 `cargo run` 入口带回源码模式。
  - 风险-2: 若直接删除源码模式，会影响开发期问题复现效率。
  - 风险-3: 若 bundle 模式继续沿用源码静态目录 fresh build 逻辑，会再次混淆“真实产物”与“开发态修补”的边界。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-LAUNCHER-BUNDLE-001 | LBFP-1/3/6/7/8/9/10 | `test_tier_required` | 手册、人工清单、README、索引互链审阅 | 人工验收口径、一键试玩入口说明 |
| PRD-TESTING-LAUNCHER-BUNDLE-002 | LBFP-2/4/5/6/7/8/9/10 | `test_tier_required` | `bash -n` + `--help` + bundle 构建 + stale bundle fail-fast / auto-rebuild 抽样 + `run-producer-playtest.sh --open-headed` + 退出后确认不残留对应 headed 浏览器进程/窗口 + `run-game-test-ab.sh --bundle-dir` + renderer 证据核验，记录通过或阻断证据 | 启动脚本 bootstrap、bundle 试玩闭环 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LBFP-001 | 保留 `run-game-test.sh`，但切换到 bundle-first 解释与用法 | 直接删除脚本 | 现有自动化依赖较多，立即删除会破坏回归面。 |
| DEC-LBFP-002 | 用 `--bundle-dir` 把 bundle 入口并入现有脚本 | 新建独立 bundle 脚本 | 共享端口、日志、URL 输出与 agent-browser 接口，更易维护。 |
| DEC-LBFP-003 | 制作人试玩/发布前验收默认走 bundle | 继续默认源码态 `cargo run` | 前者更贴近真实交付物，也更能暴露 bundle 级问题。 |
| DEC-LBFP-004 | 提供 `run-producer-playtest.sh` 作为 bundle-first 一键入口 | 要求制作人手动执行 build + bootstrap 两条命令 | 单命令更符合制作人实际使用路径，也更不容易退回源码模式。 |
| DEC-LBFP-005 | `--open-headed` 做成可选模式 | 默认总是自动打开浏览器 | 让脚本既能服务纯起栈，也能服务制作人直接入场，避免强绑浏览器副作用。 |
| DEC-LBFP-006 | `run-producer-playtest.sh --open-headed` 退出时自动关闭自己拉起的浏览器会话 | 保留 `agent-browser` 会话常驻，要求人工手动关窗 | 制作人单命令试玩默认应无残窗副作用，脚本应负责自己创建资源的收尾。 |
| DEC-LBFP-007 | headed 浏览器默认固定 `--use-angle=gl,--ignore-gpu-blocklist`，并把 headed 命中 software renderer 视为环境阻断 | 仅把 `--headed` 当作充分条件 | 当前环境已验证默认 headed 仍可能回退 SwiftShader，必须把硬件路径策略写进入口。 |
| DEC-LBFP-008 | bundle 复用必须带 freshness manifest 守卫；producer 入口自动重建，底层 bootstrap 默认阻断 | 继续无条件复用本地 bundle | 已实际复现旧 Viewer Web 产物与新 runtime 二进制协议漂移，必须把 bundle freshness 做成默认机制。 |
