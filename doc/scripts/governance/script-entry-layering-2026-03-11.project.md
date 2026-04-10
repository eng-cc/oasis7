# oasis7: 脚本分层与主入口 / fallback 入口梳理（2026-03-11）（项目管理）

- 对应设计文档: `doc/scripts/governance/script-entry-layering-2026-03-11.design.md`
- 对应需求文档: `doc/scripts/governance/script-entry-layering-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] SL-1 (PRD-SCRIPTS-LAYER-001/002) [test_tier_required]: 盘点根 `scripts/` 高频脚本并按主入口 / 辅助 / fallback 分层。
- [x] SL-2 (PRD-SCRIPTS-LAYER-001/003) [test_tier_required]: 建立需求类型 -> 推荐主入口脚本映射表，并回写模块 project / index。
- [x] SL-3 (PRD-SCRIPTS-LAYER-002/003) [test_tier_required]: 完成 `runtime_engineer -> qa_engineer` handoff，锁定 Web-first 与 fallback 边界。

## 关键清单

### 1. 开发 / 提交 / required 主入口
| 需求类型 | 推荐主入口 | 辅助入口 | fallback | 说明 |
| --- | --- | --- | --- | --- |
| commit baseline / required 套件 | `scripts/ci-tests.sh` | `scripts/pre-commit.sh` | 无 | 日常本地提交默认走 `./scripts/ci-tests.sh commit`（由 `scripts/pre-commit.sh` 调用）；需要补跑较重 runtime/simulator shard 或进入 PR/CI required gate 时，再显式执行 `./scripts/ci-tests.sh required`。 |
| 站点文档治理 | `scripts/doc-governance-check.sh` | `scripts/site-manual-sync-check.sh` | 无 | 先检查文档治理，再做站点专项。 |
| 本地游戏验证 | `scripts/worktree-harness.sh` | `scripts/run-game-test.sh` | 无 | worktree harness 负责隔离端口、bundle、日志与浏览器 session；`run-game-test.sh` 降为底层 bootstrap。 |
| Viewer Web 验证 | `scripts/run-viewer-web.sh` | `scripts/viewer-release-qa-loop.sh` | `scripts/capture-viewer-frame.sh` | Web-first 为默认链路；原生抓帧只在 Web 无法复现或 native 图形问题时使用。 |

### 2. 发布 / 打包主入口
| 需求类型 | 推荐主入口 | 辅助入口 | fallback | 说明 |
| --- | --- | --- | --- | --- |
| 发布门禁总入口 | `scripts/release-gate.sh` | `scripts/release-gate-smoke.sh` | 无 | smoke 只做轻量预检。 |
| Bundle 预构建 | `scripts/release-prepare-bundle.sh` | `scripts/build-game-launcher-bundle.sh` | 无 | 先准备发布目录，再调用具体 bundle。 |
| Launcher bundle | `scripts/build-game-launcher-bundle.sh` | 无 | 无 | 独立 launcher 打包主入口。 |

### 3. 长跑 / 在线稳定性
| 需求类型 | 推荐主入口 | 辅助入口 | fallback | 说明 |
| --- | --- | --- | --- | --- |
| P2P soak | `scripts/p2p-longrun-soak.sh` | `scripts/s10-five-node-game-soak.sh` | 无 | S10 更偏五节点真实游戏场景。 |
| Runtime 存储治理 | `scripts/oasis7-runtime-storage-gate.sh` | `scripts/oasis7-runtime-finality-baseline.sh` | 无 | 一个看 storage gate，一个看 finality baseline。 |
| LLM 压测 | `scripts/llm-longrun-stress.sh` | `scripts/llm-baseline-fixture-smoke.sh` | 无 | smoke 只做基线。 |

### 4. 站点 / 发布资产巡检
| 需求类型 | 推荐主入口 | 辅助入口 | fallback | 说明 |
| --- | --- | --- | --- | --- |
| 站点断链检查 | `scripts/site-link-check.sh` | `scripts/site-download-check.sh` | 无 | 下载校验作为断链后的专项补充。 |
| 手册镜像同步 | `scripts/site-manual-sync-check.sh` | `scripts/doc-governance-check.sh` | 无 | 先检查镜像语义，再回总门禁。 |

### 5. 受控 fallback / 专项诊断
| 脚本 | 触发条件 | 不可替代的主入口 | 产物要求 |
| --- | --- | --- | --- |
| `scripts/capture-viewer-frame.sh` | Web-first 链路无法复现，或 native 图形链路故障 | `scripts/run-viewer-web.sh` | 必须回写截图/帧文件与故障上下文 |
| `scripts/viewer-texture-inspector.sh` | 仅在材质/纹理专项排障时使用 | `scripts/run-viewer-web.sh` / `scripts/viewer-release-qa-loop.sh` | 需附材质包与对比说明 |
| `scripts/fix-precommit.sh` | precommit 失败后的修复辅助 | `scripts/pre-commit.sh` | 需先记录原失败签名 |

## 依赖
- `scripts/`
- `AGENTS.md`
- `testing-manual.md`
- `doc/scripts/project.md`
- `doc/scripts/prd.index.md`

## 状态
- 更新日期：2026-03-11
- 当前阶段：已完成
- 阻塞项：无
- 下一步：转入 `TASK-SCRIPTS-003`，为高频主入口补参数契约与失败语义。
