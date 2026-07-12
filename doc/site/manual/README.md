# Site 手册镜像治理入口

`doc/site/manual/` 只治理公开静态手册镜像与仓库 canonical 手册之间的映射；它不替代公开 HTML 页面，也不成为 Viewer 产品行为的权威来源。

## 从这里开始

- 想确认静态 docs hub、CN/EN 镜像和发布同步边界：读 `site-manual-static-docs.prd.md`。
- 想核对该专题的已完成状态、内容基线与后续同步责任：读 `site-manual-static-docs.project.md`。
- 需要追溯 2026-02 Viewer 手册内容搬迁的具体增量：按需读 `viewer-manual-content-migration-2026-02-15.*`。
- 需要当前中文 canonical 手册：读 `doc/world-simulator/viewer/viewer-manual.manual.md`；公开只读镜像位于 `site/doc/cn|en/viewer-manual.html`。

## 收敛规则

- `site-manual-static-docs.*` 是本子树的主专题，承担当前静态手册架构、镜像边界与维护约定。
- `viewer-manual-content-migration-2026-02-15.*` 是已完成的增量迁移记录，只在需要理解已搬迁范围或历史决策时进入；不应再作为默认首读入口。
- 当前内容真值由 canonical Viewer 手册及实际 `site/doc/**` 镜像共同决定。专题三件套保留任务、范围和审计可追溯性，不能据此覆盖当前产品行为。

## 删除审计

本轮未删除 `viewer-manual-content-migration-2026-02-15.*`：它仍被 `doc/site/prd.index.md`、`doc/core/reviews/` 的主从/回填审计记录，以及主专题的显式增量关系引用。虽已完成，尚不满足“无活跃调用且当前入口足以保留所需追溯”的删除条件。
