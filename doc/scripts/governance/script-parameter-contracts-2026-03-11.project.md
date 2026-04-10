# oasis7: 高频脚本参数契约与失败语义（2026-03-11）（项目管理）

- 对应设计文档: `doc/scripts/governance/script-parameter-contracts-2026-03-11.design.md`
- 对应需求文档: `doc/scripts/governance/script-parameter-contracts-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] SPC-1 (PRD-SCRIPTS-CONTRACT-001/002) [test_tier_required]: 抽取高频脚本最小命令、关键参数、默认值与典型失败语义。
- [x] SPC-2 (PRD-SCRIPTS-CONTRACT-002/003) [test_tier_required]: 显式标注 `dry-run` / `skip-*` 等覆盖范围变化参数。
- [x] SPC-3 (PRD-SCRIPTS-CONTRACT-001/003) [test_tier_required]: 完成 `runtime_engineer -> qa_engineer` handoff，并回写模块主项目。

## 契约表
| 脚本 | 最小调用 | 关键参数 / 默认值 | 典型失败语义 | 备注 |
| --- | --- | --- | --- | --- |
| `scripts/ci-tests.sh` | `./scripts/ci-tests.sh commit` | 位置参数：`commit|required|full|full-core|full-support`；默认 `full` | 非零通常表示对应 tier 套件存在失败，而非参数解析成功后静默降级 | `commit` 为默认本地提交基线；`required` 为显式本地重门禁与 PR/CI required gate；`full` 为更广覆盖。 |
| `scripts/release-gate.sh` | `./scripts/release-gate.sh --out-dir .tmp/release_gate` | `--out-dir` 默认 `.tmp/release_gate`；`--quick` 缩短长跑；`--dry-run` 只记录不执行；`--skip-ci-full/--skip-sync/--skip-web-strict/--skip-s9/--skip-s10` 会缩减覆盖范围 | 非零可能来自某个 gate step 失败；使用 `skip-*` 不等同完整放行；`--dry-run` 不产生真实通过结论 | Step 语义：`ci_full | sync_m1 | sync_m4 | sync_m5 | web_strict | s9 | s10`。 |
| `scripts/build-game-launcher-bundle.sh` | `./scripts/build-game-launcher-bundle.sh --profile release` | `--out-dir` 默认 `output/release/game-launcher-<timestamp>`；`--profile` 默认 `release`；`--target-triple` 默认 `native`；`--web-dist` 可复用预构建产物；`--dry-run` 只打印命令 | 非零通常来自 cargo build / trunk / 产物复制失败；`--web-dist` 路径错误会导致 bundle 缺失静态资源 | 打包主入口，不替代 `release-prepare-bundle.sh` 的总装配语义。 |
| `scripts/run-viewer-web.sh` | `./scripts/run-viewer-web.sh` | 入口实际代理到 `trunk serve`；常用参数来自 trunk，如 `--port` 默认 `8080`、`--address` 默认 loopback、`--open` 默认 false | 非零多来自 trunk 未安装、WASM 构建失败、端口占用或前端依赖问题，而不是脚本自定义业务校验 | Web-first 默认入口；native 问题才升级到 fallback。 |
| `scripts/site-link-check.sh` | `./scripts/site-link-check.sh` | 无高频必填参数；按脚本内默认站点路径巡检 | 非零通常表示检测到断链、目标不存在或站点构建产物不完整 | 与 `site-download-check.sh` 组合使用，但其本身是站点巡检主入口。 |

## 失败语义分类
| 分类 | 含义 | 典型脚本 | 推荐动作 |
| --- | --- | --- | --- |
| `usage_error` | 参数不合法、位置参数不在允许范围 | `ci-tests.sh`、`release-gate.sh` | 先查看 `--help`，修正参数后重跑 |
| `environment_error` | 依赖缺失、端口占用、工具未安装、路径不存在 | `run-viewer-web.sh`、`build-game-launcher-bundle.sh` | 修环境或产物路径，再重跑 |
| `gate_failure` | 套件、长跑或链接检查真实失败 | `ci-tests.sh`、`release-gate.sh`、`site-link-check.sh` | 保留日志并按测试/站点结论处理 |

## 依赖
- `scripts/ci-tests.sh`
- `scripts/release-gate.sh`
- `scripts/build-game-launcher-bundle.sh`
- `scripts/run-viewer-web.sh`
- `scripts/site-link-check.sh`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-03-11
- 当前阶段：已完成
- 阻塞项：无
- 下一步：转入 `TASK-SCRIPTS-004`，建立脚本稳定性趋势跟踪指标。
