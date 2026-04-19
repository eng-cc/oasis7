# Rust 1200 行根治治理设计（2026-03-29）

- 对应需求文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`
- 对应项目管理文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.project.md`

审计轮次: 6

## 目标
- 把 Rust 1200 行限制从一次性结构治理升级为持续执行的工程门禁。
- 为后续 runtime / viewer / launcher 超限文件治理冻结统一完成态，禁止继续用 `split_part` 伪装治理完成，并把结构切片门禁收口为实时扫描归零。

## 当前现状
- 当前扫描结果：31 个生产 Rust 文件和 14 个测试 Rust 文件超过 1200 行。
- 主要集中区：
  - `crates/oasis7`：21 个生产超限文件，集中在 `chain_runtime`、`viewer runtime_live`、`runtime state/events/world`。
  - `crates/oasis7_viewer`：8 个生产超限文件，集中在 `player guide`、`automation`、`web test api`、`timeline` 等 UI/automation 混合区域。
  - 测试侧仍有 14 个超限文件，最大热点为 `runtime_live/tests.rs`、`module_action_loop_split_part3.rs`、`economy.rs`、`main_tests.rs`。
- 失败根因：
  - round3 的完成态过度依赖 `include!`/`split_part`，未真正建立目录模块边界。
  - required gate 未默认执行 Rust 文件体量检查。
  - 缺少“触碰超限文件必须缩小”的提交规则。
  - 测试和生产代码都存在 god module，新增功能会继续往原入口堆。

## 目标完成态
- 脚本层：
  - 新增 `scripts/check-rust-file-size.sh`。
  - 默认扫描 `crates/**/src/*.rs` 及测试文件，排除 `third_party/`、`target/` 与产物目录。
  - 输出三类结果：新增超限、存量基线、触碰后未缩小。
- 基线层：
  - 冻结当前超限清单，记录 `path / line_count / is_test / owner / priority_batch`。
  - 基线文件固定为 `doc/.governance/rust-oversized-file-baseline.tsv`。
  - 结构切片不再保留 frozen baseline；`scripts/check-rust-file-size.sh` 直接要求当前 `slice_file` / `include_target` 扫描结果为 0。
  - 新文件一律不得进入基线。
- 规则层：
  - 新增或重命名出的 `split_part*` / `part1` / `part2` / `include!` 完成态一律阻断。
  - 触碰基线内超限文件时，必须满足 `after_lines < before_lines`，比较基准取当前工作树相对 `HEAD` / `HEAD^` 的上一版本行数；或将旧文件职责迁出并从基线中退休。
- 结构层：
  - 最终完成态统一采用“目录模块 + 职责模块”。
  - 示例：
    - `viewer/runtime_live/` 下拆 `server.rs`、`session_policy.rs`、`recovery.rs`、`mapping.rs`、`control_plane.rs`
    - `bin/oasis7_chain_runtime/` 下拆 `cli.rs`、`status_api.rs`、`reward_runtime.rs`、`execution_bridge.rs`、`governance_registry.rs`
    - `viewer/panel/player_guide/` 下拆 `copy.rs`、`progress.rs`、`layout_preset.rs`、`render.rs`

## 分批策略
### Batch A: 门禁与基线
- 产出扫描脚本、冻结超限基线、接入 `scripts/ci-tests.sh required`，并把结构切片门禁改成实时清零。
- 新增命名与 `include!` 完成态阻断，并对触碰到的超限文件执行 `touch-and-shrink`。

### Batch B: 高风险入口治理
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
- `crates/oasis7/src/viewer/runtime_live.rs`
- 目标：先把入口文件变成目录编排层，不再承载细节实现。

### Batch C: Viewer 侧治理
- `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
- `crates/oasis7_viewer/src/web_test_api.rs`
- `crates/oasis7_viewer/src/viewer_automation.rs`
- 目标：把文案、状态机、自动化协议、render 分成独立职责面。

### Batch D: 测试尾债治理
- `runtime_live/tests.rs`
- `module_action_loop_*`
- `economy.rs`
- `main_tests.rs`
- 目标：按行为域拆测试文件，而不是继续增加 `split_part`。

## 验证策略
- Batch A:
  - `./scripts/doc-governance-check.sh`
  - `./scripts/check-rust-file-size.sh`
  - `git diff --check`
  - 如需验证 required 链路，执行 `./scripts/ci-tests.sh required`；当前仓库若仍被无关编译红灯阻断，需在 devlog 中明确标注失败点与归因。
- Batch B / C:
  - 对应 crate 的 `test_tier_required`
  - 入口文件涉及联机/Viewer 时追加 `test_tier_full`
  - Viewer Web 相关改动按 `testing-manual.md` 评估是否需要 Web strict/agent-browser 闭环
- Batch D:
  - 定向测试分组回归
  - 确保拆分后失败定位粒度提升，而不是只换文件名

## 风险与缓解
- 风险：扫描脚本阻断过严，导致现有仓库不可提交。
  - 缓解：先冻结基线，只阻断新增违规与触碰未缩小。
- 风险：开发者通过复制 helper 或复制类型绕过规则。
  - 缓解：评审标准明确“不可转移债务”，必要时追加重复定义检查。
- 风险：入口文件拆分破坏稳定路径。
  - 缓解：Batch B/C 必须绑定定向回归和 QA/Viewer 验证。

## 交付物
- `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`
- `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/project.md`
- `doc/.governance/rust-oversized-file-baseline.tsv`
- `doc/devlog/2026-03-29.md`
