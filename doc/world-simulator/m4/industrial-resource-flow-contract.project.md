# M4 工业资源流转合同（项目管理文档）

- 对应需求文档: `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`
- 对应设计文档: `doc/world-simulator/m4/industrial-resource-flow-contract.design.md`

## 任务拆解

- [x] industrial-resource-flow-contract-source-consolidation (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 收敛工业 WASM、M4 电力系统、材料多账本/物流与 Location 电力池下线/辐射电厂四组已完成专题的现行专业语义。 Trace: https://github.com/eng-cc/oasis7/issues/2587 (task_2546eda47af242e991b9f4b47ba2ba63)
- [x] industrial-resource-flow-contract-current-power-boundary (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 将 Location electricity pool、`PowerStorage` 与已下线充放电路径明确为非当前能力。 Trace: https://github.com/eng-cc/oasis7/issues/2587 (task_2546eda47af242e991b9f4b47ba2ba63)
- [x] industrial-resource-flow-contract-quote-debt-preservation (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 将未实现的工业可读性 quote 保留为 GitHub #2166 债务，不把历史完成态转换为实现或放行结论。 Trace: https://github.com/eng-cc/oasis7/issues/2587 (task_2546eda47af242e991b9f4b47ba2ba63)
- [x] industrial-resource-flow-contract-inbound-repair (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 修复 module index、project debt route 与 Viewer industry graph 的文档入口，并删除已吸收的十二份来源文档。 Trace: https://github.com/eng-cc/oasis7/issues/2587 (task_2546eda47af242e991b9f4b47ba2ba63)

## 依赖

- `doc/world-simulator/prd.md`
- `doc/world-simulator/prd.index.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/project.md`
- GitHub issue `eng-cc/oasis7#2166`
- runtime/ABI canonical manifests、tests 与 GitHub task issue evidence comments

## 状态

- 当前阶段：文档 authority consolidation 完成。
- 下一阶段：由 `runtime_engineer`、`gameplay_designer` 与 `viewer_engineer` 分别实现并验证 #2166 的未收口可读性债务；该项目页不替代 task truth 或 QA verdict。
- 追溯：已删除的 2026-02/03 M4 和 kernel 三件套通过 Git history 与 GitHub task issue evidence comments 查询。
