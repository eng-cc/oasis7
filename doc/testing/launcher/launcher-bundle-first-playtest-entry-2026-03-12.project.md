# oasis7：启动器 bundle-first 试玩入口收敛（2026-03-12）（项目管理）

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LBFP-1 (PRD-TESTING-LAUNCHER-BUNDLE-001): 建立专题 PRD / design / project，并回写 testing 索引。
- [x] LBFP-2 (PRD-TESTING-LAUNCHER-BUNDLE-002): 为 `scripts/run-game-test.sh` 增加 `--bundle-dir` 并保持源码模式兼容。
- [x] LBFP-3 (PRD-TESTING-LAUNCHER-BUNDLE-001): 同步 `testing-manual.md`、启动器人工测试清单、README 与帮助文本，明确 bundle-first 口径。
- [x] LBFP-4 (PRD-TESTING-LAUNCHER-BUNDLE-002): 完成 bundle 构建、headed/headless 对照验证、SwiftShader 阻断证据归档与 devlog 回写。
- [x] LBFP-5 (PRD-TESTING-LAUNCHER-BUNDLE-002): 为 `run-game-test-ab.sh` 增加 `headless + SwiftShader` 环境快失败与 `browser_env.json` 证据落盘，避免误把环境阻断记成 fresh Web 回归。
- [x] LBFP-6 (PRD-TESTING-LAUNCHER-BUNDLE-001/002): 新增 `run-producer-playtest.sh`，把制作人 bundle-first 试玩收敛成单命令入口，并同步手册/帮助文本。
- [x] LBFP-7 (PRD-TESTING-LAUNCHER-BUNDLE-001/002): 为 `run-producer-playtest.sh` 增加 `--open-headed`，在 URL 就绪后自动打开 headed 浏览器并保留起栈日志。
- [x] LBFP-8 (PRD-TESTING-LAUNCHER-BUNDLE-001/002): 为 `run-producer-playtest.sh --open-headed` 增加退出自动关浏览器收尾，并补充手册/日志验证口径。
- [x] LBFP-9 (PRD-TESTING-LAUNCHER-BUNDLE-001/002): 固化 headed 浏览器的默认硬件 WebGL 启动参数，并把 headed 命中 software renderer 统一收口为环境阻断。
- [x] LBFP-10 (PRD-TESTING-LAUNCHER-BUNDLE-001/002): 为 bundle-first 入口增加 freshness manifest 守卫，自动识别并阻断/重建 stale bundle。

## 依赖
- `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md`
- `scripts/run-game-test.sh`
- `scripts/run-game-test-ab.sh`
- `scripts/run-producer-playtest.sh`
- `scripts/build-game-launcher-bundle.sh`
- `testing-manual.md`
- `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
- `doc/testing/project.md`
- `doc/testing/prd.index.md`
- `doc/devlog/README.md`

## 状态
- 更新日期：2026-03-12
- 当前阶段：已完成（入口、文档、headed 自动打开、硬件 WebGL 默认参数、software renderer guardrail 与 stale bundle freshness 守卫已收敛）
- 阻塞项：无新的代码阻塞；当前已确认此前阻断来自默认 headed Chrome 仍可能回退到 `SwiftShader`，现已在脚本层固定硬件参数并保留阻断兜底。
- 下一步：继续观察不同图形环境下 `--use-angle=gl` 是否仍有例外；若有，再单开专题追默认 ANGLE/Vulkan 回退原因。
