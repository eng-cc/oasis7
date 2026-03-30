# headless-runtime 文档索引（原 nonviewer）

审计轮次: 10

## 说明
- 模块目录已从旧名称 `nonviewer` 重命名为 `headless-runtime`。
- 历史专题文件名保留 `nonviewer-*` 前缀，仅路径发生变化。
- 不再保留 `doc/headless-runtime/archive/` 归档目录。

## 入口
- PRD: `doc/headless-runtime/prd.md`
- 设计总览: `doc/headless-runtime/design.md`
- 标准执行入口: `doc/headless-runtime/project.md`
- 文件级索引: `doc/headless-runtime/prd.index.md`

## 从这里开始
- 想先确认 headless-runtime 当前职责、生命周期边界与发布接口：先读 `doc/headless-runtime/prd.md`。
- 想看这个模块还有没有活跃执行项、最近一次收口了什么：先读 `doc/headless-runtime/project.md`。
- 想理解旧 `nonviewer` 命名为什么还保留在专题文件里：先读上面的“说明”，再进入 `doc/headless-runtime/nonviewer/`。
- 想查生命周期 / 鉴权一致性自检入口：先读 `doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`。
- 想查长稳归档、事故追溯或 release gate 对接模板：进入 `doc/headless-runtime/templates/`。

## 模块职责
- 维护无界面运行链路的生命周期、鉴权与长稳追溯口径。
- 汇总 `nonviewer/` 主题下的历史 hardening、设计对齐与评审专题。
- 承接与 testing / core 的 headless 证据链和发布门禁对接口径。

## 主题文档
- `nonviewer/`：历史无界面运行、鉴权、长稳归档与设计对齐专题。
- `checklists/`：生命周期 / 鉴权一致性检查清单。
- `templates/`：长稳归档、事故追溯与 release gate 对接模板。

## 近期专题
- `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md`
- `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
- `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md`
- `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `nonviewer/`、`checklists/`、`templates/`。

## 维护约定
- 无界面运行链路行为变更，优先回写 `prd.md` 与 `project.md`。
- 历史专题文件名可保留 `nonviewer-*`，但新文档优先使用 `headless-runtime-*` 前缀。
- 新增专题后，需同步回写 `doc/headless-runtime/prd.index.md` 与本目录索引。
- README 负责解释命名迁移与入口顺序，不替代 `nonviewer/`、`checklists/`、`templates/` 或 `prd.index.md` 的详细内容。
