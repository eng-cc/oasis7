# world-runtime PRD Project

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-WORLD_RUNTIME-001 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 完成 world-runtime PRD 改写，建立运行时设计主入口。
- [x] TASK-WORLD_RUNTIME-002 (PRD-WORLD_RUNTIME-001/002) [test_tier_required]: 补齐 runtime 核心边界（确定性、WASM、治理）验收清单。
  - 产物文件:
    - `doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md`
    - `rg -n "确定性边界|WASM 边界|治理边界|阻断条件|结论记录模板" doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md`
- [x] TASK-WORLD_RUNTIME-003 (PRD-WORLD_RUNTIME-002/003) [test_tier_required]: 建立运行时安全与数值语义回归跟踪模板。
  - 产物文件:
    - `doc/world-runtime/templates/runtime-security-numeric-regression-template.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/world-runtime/templates/runtime-security-numeric-regression-template.md`
    - `rg -n "安全回归|数值语义回归|失败签名|问题与处置|结论摘要" doc/world-runtime/templates/runtime-security-numeric-regression-template.md`
- [x] TASK-WORLD_RUNTIME-004 (PRD-WORLD_RUNTIME-003) [test_tier_required]: 对接跨模块发布门禁中的 runtime 质量指标。
  - 产物文件:
    - `doc/world-runtime/templates/runtime-release-gate-metrics-template.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/world-runtime/templates/runtime-release-gate-metrics-template.md`
    - `rg -n "关键指标|runtime 结论|conditional-go|对接规则|state root" doc/world-runtime/templates/runtime-release-gate-metrics-template.md`
- [x] TASK-WORLD_RUNTIME-005 (PRD-WORLD_RUNTIME-001/002/003) [test_tier_required]: 对齐 strict PRD schema，补齐关键流程/规格矩阵/边界异常/NFR/验证与决策记录。
- [x] TASK-WORLD_RUNTIME-006 (PRD-WORLD_RUNTIME-002) [test_tier_required]: 同步 m1/m5 builtin wasm 工件 `sha256` 与 identity manifest，修复 CI hash token 不一致导致的运行时加载失败；回归 `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required`。
- [x] TASK-WORLD_RUNTIME-016 (PRD-WORLD_RUNTIME-016/017/018) [test_tier_required]: 新增“线上模块发布合法性闭环补齐”专题 PRD/项目管理文档并纳入主索引。
- [x] TASK-WORLD_RUNTIME-017 (PRD-WORLD_RUNTIME-016) [test_tier_required]: 引入线上 builtin 发布清单入口与生产禁 fallback 策略（`ReleaseSecurityPolicy` + online manifest API）。
- [x] TASK-WORLD_RUNTIME-018 (PRD-WORLD_RUNTIME-016) [test_tier_required]: `m1/m4/m5` bootstrap 加载迁移到治理清单解析路径，保留受控 fallback。
- [x] TASK-WORLD_RUNTIME-019 (PRD-WORLD_RUNTIME-016) [test_tier_full]: 补齐线上 manifest 不可达/回滚/版本漂移场景回归与故障签名。
- [x] TASK-WORLD_RUNTIME-020 (PRD-WORLD_RUNTIME-017) [test_tier_required]: 生产策略下禁用 `identity_hash_v1` 回退并补齐回归。
- [x] TASK-WORLD_RUNTIME-021 (PRD-WORLD_RUNTIME-017) [test_tier_required + test_tier_full]: `apply_proposal` 去本地自签路径，改为外部 finality 证书必需并补齐 epoch 快照验证者签名集阈值与轮换回归。
- [x] TASK-WORLD_RUNTIME-022 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 新增去中心化发布提案与复构建证明收集流程（`proposal -> attestation`）并形成可审计证据结构。
- [x] TASK-WORLD_RUNTIME-023 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 落地“epoch 快照验证者签名集”阈值签名聚合与 release manifest 激活路径（不依赖 CI 服务）并补齐拒绝路径测试。
- [x] TASK-WORLD_RUNTIME-024 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 更新发布运行手册与告警策略（证明冲突、阈值不足、manifest 不可达），并明确 CI 仅用于开发回归且不承担生产发布写入。
- [x] TASK-WORLD_RUNTIME-025 (PRD-WORLD_RUNTIME-017) [test_tier_required + test_tier_full]: 扩展 finality 证书/信任根数据模型，落地 `epoch_id + validator_set_hash + stake_root + threshold_bps + min_unique_signers` 校验与回归。
- [x] TASK-WORLD_RUNTIME-026 (PRD-WORLD_RUNTIME-017) [test_tier_required]: 梳理安装/升级/回滚/发布应用调用点，生产路径禁止本地自签 `apply_proposal()`，统一切换外部证书 apply。
- [x] TASK-WORLD_RUNTIME-027 (PRD-WORLD_RUNTIME-016) [test_tier_required]: `ModuleRelease* -> release manifest` 映射状态落盘并补齐回放断言。
- [x] TASK-WORLD_RUNTIME-028 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 从主 CI 移除生产发布写入/激活职责，仅保留 `--check` 类回归；补齐节点侧发布运行手册与验收脚本。
- [x] TASK-WORLD_RUNTIME-029 (PRD-WORLD_RUNTIME-018) [test_tier_required + test_tier_full]: 增加 `stake/epoch` 验签耗时与“2 epoch 收敛”固定基准入口，产出可归档性能与收敛报告。
- [x] TASK-WORLD_RUNTIME-030 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 建立运行态存储体积治理专题 PRD / project，并回写模块主 PRD、项目文档与索引。
- [x] TASK-WORLD_RUNTIME-031 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 落地 execution bridge / execution world retention policy（head window、稀疏 checkpoint、manifest-aware GC）并验证 latest-state 恢复不回退。
- [x] TASK-WORLD_RUNTIME-032 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 实现 `tick_consensus_records` 热冷分层与 storage metrics/status 输出，建立 snapshot size regression 与 archive read 回归。
- [x] TASK-WORLD_RUNTIME-033 (PRD-WORLD_RUNTIME-014/015) [test_tier_required + test_tier_full]: 建立 launcher / chain runtime / soak profile 的 footprint gate、GC fail-safe 与重启恢复联合验证。
- [x] TASK-WORLD_RUNTIME-034 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 输出详细技术设计文档，明确 canonical replay log / checkpoint / GC / metrics / migration 方案。
- [x] TASK-WORLD_RUNTIME-035 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 将专题项目进一步拆解为 T1.1 ~ T7.5 子任务，明确执行顺序、依赖边界与测试闭环。
- [x] TASK-WORLD_RUNTIME-036 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 同步 `doc/world-runtime/README.md` 与 `doc/world-runtime/prd.index.md` 的模块入口索引，补齐近期专题、模块职责与根目录收口口径。
- [x] TASK-WORLD_RUNTIME-037 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 清理 `doc/world-runtime/prd.md` 顶部重复的根级兼容执行入口，保持接口区导航唯一。
- [x] TASK-WORLD_RUNTIME-038 (PRD-WORLD_RUNTIME-019) [test_tier_required]: 为工厂补齐生产阻塞/恢复状态与审计事件，确保前期工业引导能由 runtime 状态与事件历史解释。
- [x] TASK-WORLD_RUNTIME-039 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 为 viewer live runtime 增加 env-gated `agent_chat -> AgentSpoke` 测试态回声，支撑 software_safe 消息流回归。
- [x] TASK-WORLD_RUNTIME-040 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 为 OpenClaw 双轨模式补齐 mode/schema/environment/fixture/replay 元数据透传，并统一 replay/summary traceability。
- [x] TASK-WORLD_RUNTIME-041 (PRD-WORLD_RUNTIME-020/021/022) [test_tier_required]: 将 `WASM 确定性构建与工件治理管线` 专题修正为 Docker-first canonical builder 目标态，并回写 world-runtime 根 PRD、项目索引、README 与当日 devlog。
- [x] TASK-WORLD_RUNTIME-042 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 新增 pinned WASM builder image 与 host wrapper，固定 `linux-x86_64` container platform 为唯一 publish build 平台；构建入口改为 Docker-only，不再保留 host-native fallback。
- [ ] TASK-WORLD_RUNTIME-043 (PRD-WORLD_RUNTIME-021/022) [test_tier_required + test_tier_full]: 将 manifest / identity / CI summary / release evidence 切换为 Docker canonical hash，对不同宿主只比较容器输出，不再比较 host-native 发布 hash。
  - 2026-03-29 drift repair: GitHub-hosted `Wasm Determinism Gate` 已暴露 `m1` tracked canonical hash 继续漂移；本轮按 runner log 回写 `m1` builtin `sha256`/identity hash token，并同步修正 `builtin_wasm_identity.rs` 中残留的旧 `m1/m5` hardcoded canonical hash，避免 required gate 继续因 stale manifest / stale test constant 双重阻断。
- [x] TASK-WORLD_RUNTIME-044 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 将 `compile_module_artifact_from_source` 的生产路径外移到 external Docker builder 或 production 默认禁用，runtime 仅消费 binary + receipt。
- [x] TASK-WORLD_RUNTIME-045 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 收口 `doc/world-runtime/**` 仍可读专题标题的 `oasis7 Runtime` 品牌，不改动内部实现兼容名与历史证据正文。
- [x] TASK-WORLD_RUNTIME-046 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 将 WASM 构建、同步、CI summary 与 builder image 的 operator env key 默认优先切到 `OASIS7_WASM_*`，并移除旧品牌 fallback，收口 Docker-first canonical build 链路的低风险内部标识迁移。
  - 产物文件:
    - `doc/world-runtime/prd.md`
    - `doc/world-runtime/project.md`
    - `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
    - `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`
    - `doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md`
    - `scripts/build-wasm-module.sh`
    - `scripts/sync-m1-builtin-wasm-artifacts.sh`
    - `scripts/ci-m1-wasm-summary.sh`
    - `tools/wasm_build_suite/src/lib.rs`
    - `docker/wasm-builder/Dockerfile`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/build-wasm-module.sh`
    - `bash -n scripts/sync-m1-builtin-wasm-artifacts.sh`
    - `bash -n scripts/ci-m1-wasm-summary.sh`
    - `env -u RUSTC_WRAPPER cargo test --manifest-path tools/wasm_build_suite/Cargo.toml -- --nocapture`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-047 (PRD-WORLD_RUNTIME-020/021/022) [test_tier_required]: 将 builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 默认优先切到 `OASIS7_BUILTIN_WASM_*`，并移除旧品牌 fallback，收口运行时取件/抓取/编译回退链路的低风险内部标识迁移。
  - 产物文件:
    - `doc/world-runtime/prd.md`
    - `doc/world-runtime/project.md`
    - `crates/oasis7/src/runtime/builtin_wasm_materializer.rs`
    - `crates/oasis7/src/runtime/m1_builtin_wasm_artifact.rs`
    - `crates/oasis7/src/runtime/m4_builtin_wasm_artifact.rs`
    - `crates/oasis7/src/runtime/m5_builtin_wasm_artifact.rs`
    - `crates/oasis7/src/runtime/world/release_manifest.rs`
    - `crates/oasis7/src/runtime/tests/builtin_wasm_materializer.rs`
    - `crates/oasis7/src/runtime/tests/power_bootstrap_release_manifest_full.rs`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full builtin_wasm_materializer -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full power_bootstrap_release_manifest_full -- --nocapture`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-048 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 将 `compile_module_artifact_from_source` 的 compiler/limits/timeout env key 默认优先切到 `OASIS7_MODULE_SOURCE_*`，并移除旧品牌 fallback，收口 runtime source compile 链路的低风险内部标识迁移。
  - 产物文件:
    - `doc/world-runtime/prd.md`
    - `doc/world-runtime/project.md`
    - `crates/oasis7/src/runtime/module_source_compiler.rs`
    - `crates/oasis7/src/runtime/tests/module_action_loop_split_part1.rs`
    - `crates/oasis7/src/simulator/tests/module_lifecycle.rs`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full compile_module_artifact_from_source -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full module_lifecycle_compile_from_source_deploys_artifact -- --nocapture`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-049 (PRD-WORLD_RUNTIME-001) [test_tier_required]: 收口 `doc/world-runtime/project.md` 模块主入口中的当前 cargo 回归命令与 crate 路径，统一使用 `oasis7` / `crates/oasis7*` 口径。
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7|crates/oasis7" doc/world-runtime/project.md`
    - `pattern="$(printf 'cargo test -p agent_%sworld|crates/agent_%sworld' '' '')"; ! rg -n "$pattern" doc/world-runtime/project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-050 (PRD-WORLD_RUNTIME-001/013/016/019) [test_tier_required]: 收口 `world-runtime runtime/module/governance` 活跃专题中仍把旧 `oasis7*` crate/path 写成当前实现载体的口径，统一到 `oasis7*` 与当前源码布局。
  - 产物文件:
    - `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.project.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md`
    - `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
    - `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md`
    - `doc/world-runtime/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7(_consensus|_node|_net|_distfs|_proto|_wasm_executor)?|crates/oasis7|crates/oasis7_consensus|crates/oasis7_node|crates/oasis7_net|crates/oasis7_distfs|crates/oasis7_proto|crates/oasis7_wasm_executor" doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase1.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase2.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase8.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase8.project.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-051 (PRD-WORLD_RUNTIME-001/010/011/012) [test_tier_required]: 收口 `world-runtime` 其余活跃 module/testing/governance 与 numeric-correctness 专题中仍把旧 `oasis7*` crate/path/command 写成当前实现载体的口径，统一到 `oasis7*`。
  - 产物文件:
    - `doc/world-runtime/module/module-storage.project.md`
    - `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md`
    - `doc/world-runtime/module/module-subscription-filters.project.md`
    - `doc/world-runtime/governance/governance-events.md`
    - `doc/world-runtime/testing/testing.md`
    - `doc/world-runtime/runtime/bootstrap-power-modules.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md`
    - `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md`
    - `doc/world-runtime/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7(_consensus|_node|_wasm_abi)?|crates/oasis7|crates/oasis7_consensus|crates/oasis7_node|crates/oasis7_wasm_abi|cargo test -p oasis7" doc/world-runtime/module/module-storage.project.md doc/world-runtime/module/player-published-entities-2026-03-05.prd.md doc/world-runtime/module/module-subscription-filters.project.md doc/world-runtime/governance/governance-events.md doc/world-runtime/testing/testing.md doc/world-runtime/runtime/bootstrap-power-modules.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-WORLD_RUNTIME-052 (PRD-WORLD_RUNTIME-001/015/020/021/022) [test_tier_required]: 收口 `world-runtime` 仍活跃专题中把旧 builtin 模块命名、旧品牌 storage profile key 与旧品牌 wasm fallback 写成当前入口的口径，统一到 `oasis7_builtin_wasm`、`OASIS7_CHAIN_STORAGE_PROFILE`、`OASIS7_WASM_*` 与 `OASIS7_BUILTIN_WASM_*` 当前入口。
  - 产物文件:
    - `doc/world-runtime/runtime/bootstrap-power-modules.design.md`
    - `doc/world-runtime/runtime/bootstrap-power-modules.project.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
    - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md`
    - `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
    - `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`
    - `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md`
    - `doc/world-runtime/module/agent-default-modules.project.md`
    - `doc/world-runtime/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7_builtin_wasm|OASIS7_CHAIN_STORAGE_PROFILE|OASIS7_WASM_|OASIS7_BUILTIN_WASM_" doc/world-runtime/runtime/bootstrap-power-modules.design.md doc/world-runtime/runtime/bootstrap-power-modules.project.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md doc/world-runtime/module/agent-default-modules.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`

## 依赖
- 模块设计总览：`doc/world-runtime/design.md`
- doc/world-runtime/prd.index.md
- `doc/world-runtime/runtime/runtime-integration.md`
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- `doc/world-runtime/wasm/wasm-interface.md`
- `scripts/build-wasm-module.sh`
- `tools/wasm_build_suite/src/lib.rs`
- `doc/world-runtime/governance/governance-events.md`
- `testing-manual.md`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-03-20
- 当前状态: in_progress（OpenClaw/runtime live traceability 子切片已完成；WASM Docker builder image 与 wrapper 已落地，`TASK-WORLD_RUNTIME-043` 已完成 build receipt / canonical token / identity / CI summary / receipt-aware release gate / node-side proof flow 子切片，并先将 GitHub-hosted gate 收敛为 Linux-only；`TASK-WORLD_RUNTIME-044` 已完成 production source compile gate）
- 下一任务: `TASK-WORLD_RUNTIME-043`
- 最新完成: `TASK-WORLD_RUNTIME-052`（已完成 `world-runtime` 活跃专题中 builtin 模块命名、storage profile 覆盖 key 与 wasm operator env 当前入口口径的 `oasis7_builtin_wasm` / `OASIS7_CHAIN_STORAGE_PROFILE` / `OASIS7_WASM_*` / `OASIS7_BUILTIN_WASM_*` 收口。）
- 最新完成: `TASK-WORLD_RUNTIME-051`（已完成 `world-runtime` 其余活跃 module/testing/governance 与 numeric-correctness 专题中旧 `oasis7*` crate/path/command 当前真值口径的 `oasis7*` 收口。）
- 最新完成: `TASK-WORLD_RUNTIME-050`（已完成 `world-runtime runtime/module/governance` 活跃专题中旧 `oasis7*` crate/path 当前真值口径与已迁移源码路径的 `oasis7*` 收口。）
- 最新完成: `TASK-WORLD_RUNTIME-049`（world-runtime 模块主 `project.md` 中当前 cargo 回归命令与 crate 路径已统一切到 `oasis7` / `crates/oasis7*` 当前口径。）
- 最新完成: `TASK-WORLD_RUNTIME-045`（world-runtime 模块仍可读专题标题统一切到 `oasis7 Runtime` 品牌，保留内部实现兼容名与历史证据正文不变）。
- 最新完成: `TASK-WORLD_RUNTIME-046`（已将 WASM 构建、同步、CI summary 与 builder image 的 operator env key 收口到 `OASIS7_WASM_*` 当前入口；旧品牌 alias 已从 repo-owned 运行入口移除）。
- 最新完成: `TASK-WORLD_RUNTIME-047`（已将 builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 收口到 `OASIS7_BUILTIN_WASM_*` 当前入口；旧品牌 alias 已从运行时入口移除）。
- 最新完成: `TASK-WORLD_RUNTIME-048`（已将 `compile_module_artifact_from_source` 的 compiler/limits/timeout env key 收口到 `OASIS7_MODULE_SOURCE_*` 当前入口；旧品牌 alias 已从 runtime/simulator 入口移除）。
- 最新完成: `TASK-WORLD_RUNTIME-044`（production `ReleaseSecurityPolicy` 默认禁用 runtime source compile，`CompileModuleArtifactFromSource` 改为仅 dev/test 可用并要求 external Docker builder + deploy binary + receipt）；上一轮为 `TASK-WORLD_RUNTIME-042`（新增 Docker-only WASM builder image 与 host wrapper，固定 `linux-x86_64` canonical build 平台），`TASK-WORLD_RUNTIME-043` 当前仍待 Docker-capable macOS runner 的真实跨宿主 full-tier release evidence 归档。
- 阶段收口优先级: `P0`
- 阶段 owner: `wasm_platform_engineer`（联审：`producer_system_designer`、`runtime_engineer`；验证：`qa_engineer`）
- 阻断条件: 在 `TASK-WORLD_RUNTIME-002/003/004` 完成前，`TASK-WORLD_RUNTIME-033` 不再作为当前版本的首要发布驱动项。
- 承接约束: `TASK-WORLD_RUNTIME-002` 完成后方可进入 `TASK-WORLD_RUNTIME-003` 与 `TASK-WORLD_RUNTIME-004`；`TASK-WORLD_RUNTIME-033` 保留为后续联合验证切片。
- 实施备注:
  - `TASK-WORLD_RUNTIME-042` 已完成：新增 `docker/wasm-builder/Dockerfile`、`docker/wasm-builder/README.md` 与 Docker-only `scripts/build-wasm-module.sh` wrapper；当前 canonical build 平台固定为 `linux-x86_64`（Docker `linux/amd64`），脚本不再保留 host-native fallback。
  - `TASK-WORLD_RUNTIME-043` 进行中：`tools/wasm_build_suite` 已输出 `build receipt` 与 `source_hash/build_manifest_hash`；`sync_builtin_wasm_identity` 已切换为 receipt 驱动；builtin `m1/m4/m5` manifest/identity 已收敛为单 canonical token `linux-x86_64=<sha256>`；`scripts/ci-m1-wasm-summary.sh` / `scripts/ci-verify-m1-wasm-summaries.py` 已纳入 `receipt_evidence + identity_build_recipe` 对账；runtime module release attestation/apply gate 已显式校验 `builder_image_digest + container_platform + canonicalizer_version` 与 manifest identity 一致性；`ModuleReleaseManifestMappingState`、`scripts/module-release-node-attestation-flow.sh` 与 `scripts/module-release-node-acceptance.sh` 已补齐 release evidence 摘要、canonical proof input、submit API 与 receipt mismatch 阻断证据；`2026-03-29` 已追加一次 GitHub-hosted drift repair，补齐 `m1` canonical hash token 漂移与 `builtin_wasm_identity.rs` 的 stale `m1/m5` hash 常量；当前下一步只剩真实 Docker-capable `darwin-arm64` 节点对真实 `request_id` 产出并提交正式 attestation proof。
  - `TASK-WORLD_RUNTIME-044` 已完成：`ReleaseSecurityPolicy` 新增 `allow_runtime_source_compile`，production policy 默认关闭 runtime 内源码编译；`CompileModuleArtifactFromSource` 在 production 下会直接拒绝并提示改走 external Docker builder + `DeployModuleArtifact`，从而把 Docker daemon 依赖移出 runtime 热路径。
  - `TASK-WORLD_RUNTIME-039` 已完成：为 `oasis7_viewer_live` / runtime live 增加 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` 测试态回声开关，在 `agent_chat` 被接受后可注入一条标准 `WorldEventKind::AgentSpoke` 事件，供 Viewer / QA 在不依赖自然 LLM 回话的情况下稳定采样消息流。
  - `TASK-WORLD_RUNTIME-040` 已完成：在 `DecisionRequest` / `ObservationEnvelope` 中补齐 `mode`、`observation_schema_version`、`action_schema_version`、`environment_class`、`fallback_reason`、`fixture_id`、`replay_id`，并将其接入 `oasis7_openclaw_parity_bench`、`oasis7_openclaw_local_bridge`、`runtime_live llm_sidecar` 与聚合脚本，确保 headless parity 与 runtime live 产物可追溯到统一 replay/summary 元数据。
  - `TASK-WORLD_RUNTIME-041` 已完成：根据最新需求将专题从“host deterministic guard + keyed 平台 hash 对账”修正为“Docker-first canonical builder + single canonical publish hash”，并明确 `compile_module_artifact_from_source` 生产路径需要外移或 gated。
  - `TASK-WORLD_RUNTIME-028` 已完成：新增节点侧固定验收入口 `scripts/module-release-node-acceptance.sh` 并将 S11 运行手册切换为“脚本入口 + 等价拆分命令 + 证据目录”；同时收敛 `sync-m1/m4/m5` 非 `--check` 写入授权为“CI 禁止、仅本地显式授权（`OASIS7_WASM_SYNC_WRITE_ALLOW=local-dev`）”，主 CI 不再具备生产发布写入/激活路径。
  - `TASK-WORLD_RUNTIME-029` 已完成：新增 `scripts/oasis7-runtime-finality-baseline.sh` 固定基准入口，输出 `stake/epoch` 验签耗时聚合指标与 `2 epoch` 收敛状态（`summary.md`/`summary.json` 可归档）；S11 运行手册已补齐命令与产物路径。
  - `TASK-WORLD_RUNTIME-034` 已完成：补齐 `runtime-storage-footprint-governance-2026-03-08.design.md`，明确 replay contract、checkpoint、GC、metrics 与迁移边界。
  - `TASK-WORLD_RUNTIME-035` 已完成：将专题执行拆解到 T1.1 ~ T7.5，明确实现顺序、依赖边界与测试闭环。
  - `TASK-WORLD_RUNTIME-031` 已启动并完成 T1.1：execution bridge record 已升级为 V2 schema，并具备 legacy 兼容读取。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T1.2：checkpoint manifest 的目录布局、latest 指针与 hash/height 校验已落地。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T1.3：replay planner 已支持“最近 checkpoint + 本地 execution records”与无 checkpoint 全日志回放降级。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T1.4：external effect materialization 已通过 `external_effect_ref` 落 CAS，并在 replay plan 构建时执行 fail-closed 校验。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T1.5：retained-height replay / no-checkpoint fallback / replay mismatch / checkpoint corruption 定向测试已补齐。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T2.1：execution bridge 已按 latest head + hot window 重算 CAS pin set，历史 snapshot/journal 不再默认全量固定。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T2.2：sparse checkpoint cadence、latest pointer 与旧 checkpoint record 回写已接入 execution bridge 写路径。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T2.3：archive-only / checkpoint-only heights 的 snapshot/journal refs 会被压缩回写，随后按 pin set sweep orphan blobs。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T2.4：legacy V1 record 现支持按需升 V2 写回，legacy 样本会自动进入 safe-mode 禁 aggressive sweep。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T2.5：head-window retention / sparse checkpoint / restart recovery / dangling-ref 拒绝回归已补齐。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T3.1：sidecar generation index 与 generation pin 集已落到 `.distfs-state/sidecar-generations/` 元数据。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T3.2：`save_to_dir` 已接入 staging -> latest/rollback-safe 的 sidecar generation 两阶段切换，并限制 generation metadata 至少保留 2 代。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T3.3：sidecar sweep 已改为 manifest-aware blob 清扫；成功路径会把 `.distfs-state/blobs` 收敛到 latest/rollback-safe 引用集合，GC 失败则仅记录 `last_gc_result=failed` 并保留恢复数据。
  - `TASK-WORLD_RUNTIME-031` 已继续完成 T3.4：sidecar save 现会在 staging 成功提交后再刷新 root latest manifest/journal，且重试前会清理未提交的 `generation.tmp`；故障注入测试已覆盖中断回滚、部分 staging 写入与 orphan cleanup。
  - `TASK-WORLD_RUNTIME-032` 已启动并完成 T4.1：默认保存链路会把 `tick_consensus_records` 拆成热快照 + `tick-consensus.archive.json` 冷归档，并通过热区摘要字段保证恢复时能校验归档是否齐全。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T4.2：冷归档已升级为 `tick-consensus.archive.index.json` + `tick-consensus.archive.segments/`，每段记录 `from/to tick`、`content_hash`、`record_count`、`hash_chain_anchor` 与相对路径。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T4.3：新增显式 archive range read / verify 路径，并能在 index 缺失时回退读取 T4.1 legacy 单文件 archive，保证旧样本迁移可用。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T4.4：snapshot size regression / archive range read / legacy migration / tampered segment hash verify 回归已补齐，T4 系列任务已闭环。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T5.1：`replication_commit_messages` 热窗口现按 latest height 回推的连续高度范围裁剪，读路径统一为“热镜像优先 + cold index 归档读回”。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T5.2：shared cold index 协议已下沉到 `oasis7_proto`，统一 `<namespace>.cold-index/index.json`、`hot_range` 与 `cold_range_anchor` 元数据字段，并先接到 replication 冷索引写路径。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T5.3：replication 冷索引已支持 canonical/legacy 双写与读时回填迁移，旧样本只保留 alias 时仍可读回并自动补出 canonical 目录。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T5.4：新增 replication cold-index scan 边界回归与 tick archive range seek 回归，验证 shared protocol 在跨模块读回上的边界口径一致。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T6.1：共享 `StorageProfileConfig` 协议、runtime / launcher / web launcher / launcher UI 的统一 profile 入口已落地，并先让 replication 热窗口预算跟随 profile 默认值。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T6.2：`oasis7_chain_runtime` 新增共享 `StorageMetricsSnapshot`、`reward-runtime-storage-metrics.json` 状态文件与 `/v1/chain/status.storage` 输出，已先覆盖 bytes、ref_count、pin_count、checkpoint_count、orphan_blob_count 与 GC 最近结果。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T6.3：storage snapshot/status 现补齐 `effective_budget` 与 `replay_summary`，launcher / 脚本可直接读取 profile 预算、checkpoint 边界与回放模式，无需再扫内部目录。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T6.4：bundle 入口新增 `run-chain-runtime.sh`，且 `run-game.sh` / `run-web-launcher.sh` 与 direct chain wrapper 已统一走 `OASIS7_CHAIN_STORAGE_PROFILE` 覆盖通道，同时显式绑定 bundle 内 `oasis7_chain_runtime`。
  - `TASK-WORLD_RUNTIME-032` 已继续完成 T6.5：定向测试现覆盖 runtime status 的 error fields / replay summary，以及 game/web launcher 的 storage profile 参数校验与透传，`TASK-WORLD_RUNTIME-032` 至此闭环。
  - `TASK-WORLD_RUNTIME-033` 已启动并完成 T7.1：新增 `runtime::tests::storage_footprint_fixture` 作为 `2500` 记录级基线样本，后续默认 profile 体积预算、restart recovery 与 replay gate 将直接复用该输入。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- 说明: 本文档仅维护 world-runtime 模块设计执行状态；过程记录在 `doc/devlog/2026-03-03.md`、`doc/devlog/2026-03-06.md` 与 `doc/devlog/2026-03-08.md`。

## 阶段收口角色交接
### Meta
- Handoff ID: `HO-CORE-20260310-WR-001`
- Date: `2026-03-10`
- From Role: `producer_system_designer`
- To Role: `runtime_engineer`
- Related Module: `world-runtime`
- Related PRD-ID: `PRD-WORLD_RUNTIME-001/002/003`
- Related Task ID: `TASK-WORLD_RUNTIME-002/003/004`
- Priority: `P0`
- Expected ETA: `待接收方确认`

### Objective
- 目标描述：先补齐 runtime 核心边界验收、回归模板与发布门禁指标，再恢复更大范围的联合验证主路径。
- 成功标准：确定性 / WASM / 治理边界形成验收清单，安全与数值语义有模板，runtime 质量指标可进入发布评审。
- 非目标：本轮不要求完成所有 footprint / GC / 重启恢复联合验证切片。

### Current State
- 当前实现 / 文档状态：`TASK-WORLD_RUNTIME-033` 已有 T7.1 基线，但核心边界验收与发布门禁项仍未收口。
- 已确认事实：core 已将 runtime 验收列为 `P0`，优先级高于后续 soak / footprint 扩展。
- 待确认假设：`TASK-WORLD_RUNTIME-002` 的验收项是否需要拆到更细专题文档。
- 当前失败信号 / 用户反馈：如果 runtime 规则只能描述不能验证，发布评审会退化为口头判断。

### Scope
- In Scope: `TASK-WORLD_RUNTIME-002`、`TASK-WORLD_RUNTIME-003`、`TASK-WORLD_RUNTIME-004` 的文档与执行承接。
- Out of Scope: 非本轮必需的性能拓展、额外 P2 功能扩张。

### Inputs
- 关键文件：`doc/world-runtime/project.md`、`doc/world-runtime/prd.md`、相关 runtime / wasm / governance 专题文档。
- 关键命令：沿用 runtime 定向回归与 required/full 套件命令。
- 上游依赖：`producer_system_designer` 提供规则边界裁剪，`qa_engineer` 负责后续验证模板与门禁复核。
- 现有测试 / 证据：`TASK-WORLD_RUNTIME-033` 的 T7.1 基线输入、现有 runtime 定向回归结果。

### Requested Work
- 工作项 1：完成 `TASK-WORLD_RUNTIME-002` 的核心边界验收清单。
- 工作项 2：建立 `TASK-WORLD_RUNTIME-003` 的安全 / 数值语义回归模板。
- 工作项 3：完成 `TASK-WORLD_RUNTIME-004` 的发布门禁指标接入方案。

### Expected Outputs
- 代码改动：如需，仅限支撑 runtime 验收与指标暴露的必要实现。
- 文档回写：`doc/world-runtime/project.md` 与相关专题文档。
- 测试记录：补齐 runtime `test_tier_required`，必要时标注后续 `test_tier_full`。
- devlog 记录：记录验收项、风险与下一切片。

### Done Definition
- [ ] 输出满足目标与成功标准
- [ ] 影响面已核对 `producer_system_designer` / `qa_engineer`
- [ ] 对应 `prd.md` / `project.md` 已回写
- [ ] 对应 `doc/devlog/YYYY-MM-DD.md` 已记录
- [ ] required/full 测试证据已补齐或明确挂起原因

### Risks / Decisions
- 已知风险：若继续先推 `TASK-WORLD_RUNTIME-033`，会把更关键的边界验收继续后置。
- 待拍板事项：哪些 runtime 指标必须成为本轮 go/no-go 阻断项。
- 建议决策：先完成 `002/003/004`，再恢复 `033` 作为更大范围联合验证任务。

### Validation Plan
- 测试层级：`test_tier_required`（必要时补 `test_tier_full`）
- 验证命令：沿用 runtime 定向回归 / required / soak 相关命令并回写证据路径。
- 预期结果：runtime 规则边界、回归模板、门禁指标可直接用于发布评审。
- 回归影响范围：world-runtime / testing / launcher-chain-runtime 接口。

- 模块进展补充（2026-03-10）: 已新增 `doc/world-runtime/runtime-p0-candidate-evidence-handoff-2026-03-10.md`，明确当前 core `blocked` 的剩余缺口是 runtime 候选级实测证据绑定，而非模板缺失。

- 模块进展补充（2026-03-10 / candidate）: 已新增 `doc/world-runtime/evidence/runtime-release-gate-metrics-task-game-018-2026-03-10.md`，将 `TASK-GAME-018` 所需 runtime P0 候选级实测证据实例化，并绑定到 core go/no-go 记录。

- 模块进展补充（2026-03-10 / T7.2）: 已新增 `scripts/oasis7-runtime-storage-gate.sh` 作为 storage/GC/replay gate 固定入口，当前已用 `release_default` 样本生成 `.tmp/world_runtime_storage_gate/20260310-234359/summary.md`，下一步接真实 runtime 状态样本。

- 模块进展补充（2026-03-10 / T7.2 实测）: 已用真实 `oasis7_chain_runtime --storage-profile release_default` 样本跑通 `scripts/oasis7-runtime-storage-gate.sh`，且在扩展 probe 中确认 `checkpoint_count` 会在 `height=32` 左右出现，而不是 status budget 声明的 `64`。详见 `doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md`。
- 模块进展补充（2026-03-10 / T7.2 根因）: 已定位 `oasis7_chain_runtime` 的 execution bridge 仍使用硬编码 `32/4` retention 默认值，尚未绑定 `StorageProfileConfig`；当前真实 gate 的 `qa_engineer` 复跑已完成：真实 `release_default` 样本在 `47` 仍为 `full_log_only`，在 `65` 切到 `checkpoint_plus_log`，T7.2 已完成闭环。
- 模块进展补充（2026-03-11 / T7.2 QA 复验）: `qa_engineer` 已基于真实 `oasis7_chain_runtime` 样本完成修复后复验，`doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md` 已确认 `<64` 无 checkpoint、`>=64` 生成首个 checkpoint，下一步转入 T7.3 的 GC fail-safe / orphan 证据补齐。
- 模块进展补充（2026-03-11 / T7.3 handoff）: `qa_engineer` 已新增 `doc/world-runtime/qa-to-runtime-task-world_runtime-033-t7.3-orphan-gc-failsafe-2026-03-11.md`，将 pre-checkpoint 窗口瞬时 `orphan_blob_count=1` 交接给 `runtime_engineer` 作为下一步闭环目标。
- 模块进展补充（2026-03-11 / T7.3 收口）: `runtime_engineer` 已新增 `doc/world-runtime/evidence/runtime-sidecar-orphan-gc-failsafe-2026-03-11.md` 与定向回归 `collect_storage_metrics_sidecar_orphan_recovers_after_successful_save`，将该 orphan 信号收敛为“可被下一次成功 save/GC 清零的窗口态”。

### Handoff Acknowledgement
- 接收方确认范围：`已接收 TASK-WORLD_RUNTIME-002/003/004；当前提交完成边界清单、回归模板与发布门禁指标模板`
- 接收方确认 ETA：`TASK-WORLD_RUNTIME-002/003/004 已完成；本轮已补齐 task 级 runtime P0 证据，下一步继续推进 TASK-WORLD_RUNTIME-033 的 T7.2~T7.5`
- 接收方新增风险：`当前模板统一了字段与门禁规则，但部分指标仍依赖后续真实样本与 soak 结果填值`

- 模块进展补充（2026-03-11 / T7.4 启动）: 已为 `oasis7_game_launcher` 与 `oasis7_web_launcher` 新增三档 storage profile 参数透传回归，并发起 `runtime_engineer -> viewer_engineer` handoff `doc/world-runtime/runtime-to-viewer-task-world_runtime-033-t7.4-profile-consistency-2026-03-11.md`，下一步补 bundle/launcher 实测证据。

- 模块进展补充（2026-03-11 / T7.4 收口）: `viewer_engineer` 已通过 bundle 产物与 `bash -x` trace 确认 `OASIS7_CHAIN_STORAGE_PROFILE` 在 `run-game.sh` / `run-web-launcher.sh` / `run-chain-runtime.sh` 中分别映射到 `--chain-storage-profile` / `--storage-profile`，T7.4 已完成。

- 模块进展补充（2026-03-11 / T7.5 收口）: `TASK-WORLD_RUNTIME-033` 已随 T7.2~T7.4 的 runtime / qa / viewer 证据链闭环完成；当前专题后续仅保留更大范围的 soak/版本迭代，不再作为未完成任务阻断。
