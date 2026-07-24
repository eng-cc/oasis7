# Viewer 控制面：回放与 Live 分离（项目与历史追溯）

> 对应需求: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`
> 对应设计: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：将 control profile 拆分和 live 禁用 seek 收敛到稳定三件套，并修复 Viewer landing/index 默认路由。
- 本轮未做：不修改协议、server、Viewer UI、automation、DOM 或测试；两个 2026-02 源三件套仍保留，等待迁移治理删除切片。

## 历史范围与当前归属

| 历史专题 | 已完成范围 | 当前归属 |
| --- | --- | --- |
| `viewer-control-plane-split-live-playback-2026-02-27` | profile、握手、server 路由、legacy bridge 与结构化 dispatch | 控制 profile 合同与兼容边界。 |
| `viewer-live-disable-seek-p2p-2026-02-27` | live `seek` 禁用、入口与 test API 收敛 | live 单调推进与无 seek 表现边界。 |

## 任务拆解

- [x] viewer-control-profile-stable-authority (PRD-WORLD_SIMULATOR-001) [test_tier_required]: 建立 current PRD/design/project，回填 profile、dispatch 与 live 无 seek 语义。 Trace: https://github.com/eng-cc/oasis7/issues/2569 (task_478961979bbf43fe81997816547f8258)
- [x] viewer-control-profile-routing (PRD-WORLD_SIMULATOR-001) [test_tier_required]: 修复 Viewer landing 与 world-simulator 文件索引的默认路由。 Trace: https://github.com/eng-cc/oasis7/issues/2569 (task_478961979bbf43fe81997816547f8258)
- [ ] viewer-control-profile-source-retirement (PRD-WORLD_SIMULATOR-001) [test_tier_required]: 在后续删除切片完成活跃引用审计后删除两组 2026-02 源三件套；本轮不删除。

## 依赖

- `crates/oasis7_proto/src/viewer.rs`
- `crates/oasis7/src/viewer/`
- `crates/oasis7_viewer/src/`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 当前验证责任

- 实现变更：协议 round-trip、handler/dispatch、Viewer 控制与 Web test API 的受影响测试；可见交互改动追加 `testing-manual.md` S6。
- 文档迁移：`./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh && git diff --check`。

本轮没有 UI/DOM/行为变更，因此 S6 截图与浏览器回归不适用；任何未来的 profile、seek 或玩家控制面改动都需重新取得 viewer、runtime、视觉交互和 QA 的对应结论。
