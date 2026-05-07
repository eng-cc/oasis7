# Gameplay 物理尺度一致性矩阵（2026-05-07）

审计轮次: 1

## Meta
- 关联专题: `PRD-GAME-013`
- 关联任务: `task_8205baa6d2fb46388b11c1eed340fdf5`
- 责任角色: `qa_engineer`
- 协作角色: `producer_system_designer`、`runtime_engineer`、`viewer_engineer`、`agent_engineer`
- 当前结论: `pass`
- 目标: 复核“厘米真值 / coarse native resolution / 表现层夸张 / 动作边界”四层合同是否在 runtime、viewer、agent 文档三侧保持一致，并记录可复现 blocker 签名。

## 最终结论
- `PRD-GAME-013` 当前四层尺度合同已收口为一致口径：
  - 世界物理真值继续以 `1cm` 为 canonical unit。
  - coarse-grained runtime 子系统都已声明自己的 native resolution、cm mapping rule 与 rounding rule。
  - `software_safe` 正式 Web 主入口已把 world bounds、锚点半径与真实距离标签暴露给玩家，并明确 marker/zoom 只是可读性夸张。
  - agent current action surface 已收口为低频间接控制白名单，没有再把 embodied / block-editing 写成现行正式能力。
- 当前未发现新的 blocker；本轮 QA 结论是“尺度合同一致性 pass”，不是“`PRD-GAME-012` trust/capability gate 已恢复”。两者必须继续分开表述。

## 一致性矩阵

| 合同层 | 结论 | repo truth | 验证方式 | blocker 签名 |
| --- | --- | --- | --- | --- |
| canonical physical scale | `pass` | `crates/oasis7/src/simulator/native_resolution.rs` 将 `canonical-physical-space` 固定到 `SPACE_UNIT_CM = 1`，并把 `GeoPos` / `space_distance_cm` / `radius_cm` / `CuboidSizeCm` 绑定到整数厘米真值。 | `env -u RUSTC_WRAPPER cargo test -p oasis7 native_resolution_ -- --nocapture` | 任一核心空间字段脱离整数厘米、或出现 `<1cm` 几何边长未 clamp 的实现，即视为 blocker。 |
| subsystem native resolution | `pass` | 同一声明表已覆盖 `chunk-grid`、`asteroid-fragment-voxel`、`asteroid-fragment-spacing`、`movement-energy-cost`、`power-transfer-distance`、`location-site-actions` 与 `fragment-block-geometry`。 | `env -u RUSTC_WRAPPER cargo test -p oasis7 native_resolution_ -- --nocapture` + declaration grep 复核 | 出现未声明 `cm_mapping_rule` / `rounding_rule` 的 coarse 子系统，或动作/生成逻辑绕开声明表，即视为 blocker。 |
| presentation scale | `pass` | `crates/oasis7_viewer/software_safe_src/legacy_core.js::buildWorldScaleSurface` 已显式展示 canonical `1cm`、`snapshot.config.space` world bounds、选中锚点坐标/半径、最近地点真实距离，并写明 marker/zoom 不等于真实几何尺寸。 | `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs` | 如果正式表面只剩 marker/zoom、缺少真实距离/半径标签，或文案把屏幕直径误导成世界几何尺寸，即视为 blocker。 |
| current action boundary | `pass` | `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md` 与 `provider-agent-dual-mode-contract-2026-03-16.md` 已统一冻结为 `wait / wait_ticks / move_agent / speak_to_nearby / inspect_target / simple_interact`。 | `rg -n "wait / wait_ticks / move_agent / speak_to_nearby / inspect_target / simple_interact|jump / attack / use_item / block_editing" doc/world-simulator/llm/*.md` + `./scripts/doc-governance-check.sh` | 任一 active contract 再把 `jump`、`attack`、`use_item`、`block_editing` 写成当前正式动作面，即视为 blocker。 |

## 执行命令
- `env -u RUSTC_WRAPPER cargo test -p oasis7 native_resolution_ -- --nocapture`
- `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `./scripts/pm/lint.sh`

## 备注
- 本文档证明的是“尺度语义一致”，不证明 embodied / block-editing 已成为当前产品方向。
- 若未来要开启 embodied candidate，需要先新增独立 gate，再重新生成本矩阵，而不是复用本次 `pass` 结论。
