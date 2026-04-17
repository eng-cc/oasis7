# `doc/devlog` 历史压缩与入口收口设计（2026-04-17）

- 对应需求文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.project.md`

## 1. 设计定位
本设计不解决“怎么删除历史”，而是解决“怎么让历史不再只能按天盲扫”。

`doc/devlog` 已经退出运行态真值，但目前仍只有 57 个日文件散放在目录里，没有 canonical archive entrypoint。第一步应先把入口从“文件系统目录”收口到一份可读的 `README.md`。

## 2. 当前快照
- 日文件总数: 57
- 月份分布:
  - `2026-02`: 26
  - `2026-03`: 30
  - `2026-04`: 1
- 超过 1000 行的重文件: 13
- 最大文件:
  - `2026-02-16.md` 3288 行
  - `2026-02-17.md` 2812 行
  - `2026-02-23.md` 2426 行

这些数值说明当前问题首先是导航与聚合缺失，其次才是后续月报/阶段摘要是否需要补。

## 3. README 设计

### 3.1 结构
`doc/devlog/README.md` 固定包含：
1. 职责边界
2. 月份分布
3. 高体量热点表
4. 按月导航
5. 使用约定

### 3.2 职责边界
首屏必须明确：
- `doc/devlog` 是历史归档。
- 不再承担 `.pm`、模块 `project.md` 或正式专题的运行态真值。
- 若想看当前状态，应回到 `doc/<module>/project.md` 与 `.pm/tasks/*.execution.md`。

### 3.3 月份分组
按月份组织：
- `2026-02`
- `2026-03`
- `2026-04`

每个月下按日期升序列出原始日文件链接，避免继续用目录浏览器当入口。

### 3.4 高体量热点表
首批只列行数最高的一组日文件，字段为：
- 文件名
- 行数
- 说明

说明采用固定词表：
- `priority-summary-candidate`
- `heavy-read-archive`

其中 `priority-summary-candidate` 表示下一轮更适合补月度/阶段摘要。

## 4. 与上游治理的关系
- `doc-corpus-maintenance-governance` 负责指出 `doc/devlog` 是第一优先级 follow-up。
- 本专题负责把这个 follow-up 真正落到 `doc/devlog` 入口层。
- 后续若要做月度摘要、阶段摘要或清理 lingering “回写 devlog”口径，再开独立专题。

## 5. 后续动作排序
1. 先建立 `doc/devlog/README.md`。
2. 再把 engineering 根入口和上游 `PRD-ENGINEERING-025` 项目页指向这里。
3. 之后再决定是否按月生成摘要。

## 6. 边界
- 不修改任何既有日文件正文。
- 不将 `doc/devlog` 重新纳入运行态 source of truth。
- 不在本批尝试用脚本重写历史内容。
