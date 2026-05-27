# oasis7 Simulator：默认电力设施语义下沉（项目管理文档）

- 对应设计文档: `doc/world-simulator/scenario/scenario-power-facility-baseline.design.md`
- 对应需求文档: `doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）

### S1 文档
- [x] 输出设计文档（`doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md`）
- [x] 输出项目管理文档（本文件）

### S2 场景配置调整
- [x] 清理非专项场景中的默认 `power_plants`（`power_storages` 已于 2026-03-06 全量下线）
- [x] 保留 `power_bootstrap` 作为显式设施场景

### S3 测试与回归
- [x] 更新 `simulator/tests/init.rs` 的场景断言
- [x] 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 simulator::tests::init -- --nocapture`
- [x] 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 --test oasis7_init_demo -- --nocapture`

### S4 文档回写
- [x] 更新 `doc/world-simulator/scenario/scenario-files.prd.md` 场景矩阵描述
- [x] 追加当日 `doc/devlog/README.md`
- [x] 提交 git commit

## 依赖
- `WorldScenario` 场景文件（`crates/oasis7/scenarios/*.json`）
- `WorldInitConfig::from_scenario` + `build_world_model`
- 场景稳定性测试（`crates/oasis7/src/simulator/tests/init.rs`）

## 状态
- 当前阶段：S4 完成
- 最近更新：完成默认设施下沉改造与场景回归测试（2026-02-07）
