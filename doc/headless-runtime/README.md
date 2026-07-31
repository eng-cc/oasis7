# headless-runtime 文档索引（原 nonviewer）

审计轮次: 10

## 说明
- 模块目录已从旧名称 `nonviewer` 重命名为 `headless-runtime`。
- 已完成的历史 `nonviewer-*` hardening 三件套不再保留为活跃文档；其稳定鉴权、长稳与归档边界已收敛到本模块 `prd.md` / `design.md` / `project.md`，历史实施从 Git history 与 GitHub task issue evidence comments 追溯。
- 不再保留 `doc/headless-runtime/archive/` 归档目录。

## 入口
- PRD: `doc/headless-runtime/prd.md`
- 设计总览: `doc/headless-runtime/design.md`
- 标准执行入口: `doc/headless-runtime/prd.md`
- 文件级索引: `doc/headless-runtime/prd.index.md`

## 从这里开始
- 想先确认 headless-runtime 当前职责、生命周期边界与发布接口：先读 `doc/headless-runtime/prd.md`。
- 想看这个模块还有没有活跃执行项、最近一次收口了什么：先读 `doc/headless-runtime/prd.md`。
- 想理解旧 `nonviewer` 命名及已退役专题的追溯边界：先读上面的“说明”，再读 `doc/headless-runtime/nonviewer/README.md`。
- 想查生命周期 / 鉴权一致性自检入口：先读 `doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`。
- 想查长稳归档、事故追溯或 release gate 对接模板：进入 `doc/headless-runtime/templates/`。

## 模块职责
- 维护无界面运行链路的生命周期、鉴权与长稳追溯口径。
- 在模块根 authority 中维护鉴权、防重放、长稳内存边界与冷归档合同；`nonviewer/README.md` 只说明旧命名与历史追溯。
- 承接与 testing / core 的 headless 证据链和发布门禁对接口径。

## 主题文档
- `nonviewer/`：旧命名与已退役 hardening 专题的追溯说明，不再承载当前 triplet authority。
- `checklists/`：生命周期 / 鉴权一致性检查清单。
- `templates/`：长稳归档、事故追溯与 release gate 对接模板。

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `nonviewer/`、`checklists/`、`templates/`。
- 2026-03-11 模块状态 closure / handoff root 文档已退役删除；当前状态与下一任务入口以 `doc/headless-runtime/prd.md` 为准。

## 维护约定
- 无界面运行链路行为变更，优先回写 `prd.md` 与 `project.md`。
- 新文档使用 `headless-runtime-*` 前缀；已吸收的历史 `nonviewer-*` slug 只从 Git history 与 GitHub task evidence 追溯。
- 新增专题后，需同步回写 `doc/headless-runtime/prd.index.md` 与本目录索引。
- README 负责解释命名迁移与模块级入口顺序；`nonviewer/README.md` 只保留历史追溯说明，二者都不替代 `checklists/`、`templates/` 或 `prd.index.md` 的详细内容。
