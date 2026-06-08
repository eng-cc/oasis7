# oasis7：启动器 bundle-first 试玩入口收敛（2026-03-12）（设计）

审计轮次: 1

## 1. Overview
- 目标：在不打断现有自动化和开发回归的前提下，把制作人试玩、发布前人工验收和自动化哨兵统一切到 bundle-first 口径。
- 范围：`scripts/run-launcher-stack.sh`、`scripts/run-game-test-ab.sh`、`scripts/run-producer-playtest.sh`、`testing-manual.md`、启动器人工测试清单与 testing 索引。

## 2. Design
- 启动模式分层：
  - `producer wrapper`：`run-producer-playtest.sh` 负责自动准备或复用本地 bundle，然后转入 `run-launcher-stack.sh --bundle-dir <bundle>`；这是制作人日常试玩的最短入口。
  - `bundle mode`：由 `--bundle-dir <bundle>` 触发，执行 `<bundle>/run-game.sh`，默认使用 bundle 自带 `web/` 静态资源。
  - `source mode`：未传 `--bundle-dir` 时沿用 `cargo run -p oasis7 --bin oasis7_game_launcher`，并保留 fresh `web` 构建兜底。
- 参数策略：
  - 端口、链参数、`--no-llm`、`--with-llm` 等继续由 `run-launcher-stack.sh` 统一透传。
  - `--viewer-static-dir` 在 `source mode` 沿用现有逻辑；在 `bundle mode` 仅作为高级覆盖，不再默认触发源码静态目录 fresh build。
- 文档策略：
  - `testing-manual.md` 明确 bundle-first 是“制作人试玩 / 发布前人工验收默认入口”。
  - `scripts/run-game-test-ab.sh --help` 明确其可透传 `--bundle-dir`，但仍仅是自动化哨兵。
  - `--headless` 命中 `SwiftShader` 时，脚本立即按环境阻断失败，并将 headed 标为 Viewer Web 的默认建议模式。
  - 启动器人工清单把 bundle-first 写进执行说明，避免人工验收仍走开发态脚本默认值。

## 3. Failure Model
- bundle 目录不存在或不完整：脚本直接失败并给出可操作错误。
- 端口冲突：继续沿用现有 fail-fast 探针与占用打印。
- 运行时失败：保留原有日志尾部输出与 URL 就绪检查。
- 误用源码模式做发布结论：通过帮助文本和手册口径降低误用，而不是硬删源码模式。

## 4. Validation
- `bash -n scripts/run-launcher-stack.sh scripts/run-game-test-ab.sh`
- `./scripts/run-producer-playtest.sh --help`
- `./scripts/run-launcher-stack.sh --help`
- `./scripts/run-game-test-ab.sh --help`
- `timeout 30s ./scripts/run-producer-playtest.sh --profile dev --no-llm`
- `./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-bundle-first-20260312`
- `./scripts/run-game-test-ab.sh --bundle-dir output/release/game-launcher-bundle-first-20260312 --no-llm --headless`（若命中 SwiftShader，应快失败并输出 `browser_env.json`）
- `./scripts/run-game-test-ab.sh --bundle-dir output/release/game-launcher-bundle-first-20260312 --no-llm`
