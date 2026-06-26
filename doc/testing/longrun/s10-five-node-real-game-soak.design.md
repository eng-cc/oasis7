# oasis7：S10 五节点真实游戏数据在线长跑套件设计

- 对应需求文档: `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`
- 对应项目管理文档: `doc/testing/longrun/s10-five-node-real-game-soak.project.md`

## 1. 设计定位
定义 S10 五节点真实游戏数据在线长跑套件，统一五节点编排、真实游戏数据流、结算/资产指标、存储/共识健康采样与发布门禁证据。
S10 本身不引入 chaos 编排；恢复、故障注入和 state-sync/commit claim boundary 由 S9 长跑与 GWSC 方案关联覆盖。

## 2. 设计结构
- 五节点编排层：定义 sequencer、storage、observer 节点拓扑、运行时长和脚本入口。
- 游戏数据流层：覆盖 gameplay 数据、reward/settlement、mint 与资产不变量。
- 稳定性观测层：采集 committed progress、lag、DistFS、settlement apply 与关键告警。
- 验收归档层：沉淀 `summary.json`、`summary.md`、`timeline.csv`、节点日志、失败签名与门禁结论。

## 3. 关键接口 / 入口
- `scripts/s10-five-node-game-soak.sh`
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- S10 `summary.json` / `summary.md` / `timeline.csv` 证据产物
- GWSC claim boundary: `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`

## 4. 约束与边界
- 长跑场景需可重复执行、可比较。
- S10 不负责 chaos 注入编排；需要故障恢复覆盖时升级到 S9/GWSC 关联套件。
- S10 五进程或本地五节点结果不得冒充 real-env/public_testnet readiness。
- 不在本专题扩展新的线上编排平台。

## 5. 设计演进计划
- 固定五节点长跑场景、节点拓扑与探针。
- 固化 settlement/mint/DistFS/lag 门禁指标。
- 与 GWSC 方案对齐 state-sync/commit claim boundary。
