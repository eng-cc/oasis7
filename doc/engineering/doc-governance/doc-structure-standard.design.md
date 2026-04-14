# 文档分工与组织规范正文（2026-03-09）

- 对应需求文档: `doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 对应项目管理文档（当前仓库文件）: `doc/engineering/doc-governance/doc-structure-standard.project.md`
- 规范目标命名: `<topic>.project.md`

## 1. 规范定位
本规范是仓库 `doc/` 文档树的顶层组织约定，用于回答两个问题：
1. 一个新文档应该放在哪个目录；
2. 一个新文档应该承担什么职责。

本规范只规定组织形式与职责边界，不处理历史迁移节奏。

## 2. 核心原则

### 2.1 目录按对象
目录用于表达“正在描述什么对象”，而不是“这是一类什么文档”。

对象层级从大到小分三层：
- 模块：如 `doc/world-runtime/`、`doc/world-simulator/`。
- 专题：如 `doc/world-simulator/launcher/`、`doc/game/gameplay/`。
- 分册：专题内部因复杂度拆出的子目录或分册集合。

### 2.2 文件按职责
文件后缀用于表达“这份文档回答什么问题”。

推荐职责后缀如下：
- `*.prd.md`：Why / What / Done
- `*.design.md`：How / Structure / Contract
- `*.project.md`：How / When / Who
- `*.manual.md`：How to use / verify
- `*.runbook.md`：How to operate / release / recover
- `README.md`：目录导航
- `prd.index.md`：专题 PRD 索引

### 2.3 同对象优先同目录
同一个对象的 PRD、Design、Project、Runbook、Manual 优先放在同一目录中，避免读者跨仓库跳转。

### 2.4 同专题优先同名
同一专题的核心文档优先采用同一个 basename：
- `foo.prd.md`
- `foo.design.md`
- `foo.project.md`

这样可以保证“一眼知道这些文档是同一个专题的不同视角”。

## 3. 一级模块组织规范
每个一级模块目录固定保留以下入口：

```text
doc/<module>/
  README.md
  prd.md
  design.md
  project.md
  prd.index.md
```

职责如下：
- `README.md`：模块目录导航。
- `prd.md`：模块目标、范围、验收与边界。
- `design.md`：模块总体技术设计与阅读入口。
- `project.md`：模块级任务拆解、依赖、状态。
- `prd.index.md`：模块内专题 PRD / project 可达索引。

### 3.1 模块级阅读顺序
固定阅读顺序为：
1. `prd.md`
2. `design.md`
3. `project.md`
4. `prd.index.md`
5. 下钻专题目录

### 3.2 模块级 design.md 的职责
模块级 `design.md` 不应重复 `prd.md` 的用户故事，而应承担：
- 模块架构总览
- 核心组件职责划分
- 模块内/跨模块主链路
- 关键数据流与状态流
- 主题分册导航

### 3.3 模块级 README.md 的职责边界
模块级 `README.md` 是 landing page，不是第二份规范正文。

应保留的内容：
- 按读者目标分流到 `prd.md`、`project.md`、`prd.index.md` 或高频专题；
- 模块特有的入口分工、公开镜像、命名迁移或例外说明；
- 当前最值得优先阅读的少量高频专题。

不应重复维护的内容：
- repo-wide 的根入口集合定义；
- 通用的目录落位规则；
- 各模块都相同的维护约定模板。

这些共享规则统一回链到本规范正文，而不是在每个模块 README 中散写一遍。

### 3.4 Legacy Redirect 的职责边界
root-level legacy redirect 只承担兼容跳转职责。

推荐最小结构：
- 一个明确的标题；
- 当前主入口；
- 必要时补一个相关入口（如配套的 `project.md` / `design.md`）；
- “仅保留兼容跳转，不承载正文”的声明。

不再推荐的做法：
- 为 redirect 再写一整套 `目标 / 范围 / 里程碑 / 风险`；
- 为 redirect 补完整的任务状态壳；
- 把 redirect 继续当作活跃业务入口维护。

## 4. 专题级组织规范
对于单一专题，优先使用下列最小三件套：

```text
doc/<module>/<topic>/
  <topic>.prd.md
  <topic>.design.md
  <topic>.project.md
```

当专题需要操作说明时，再追加：

```text
  <topic>.manual.md
  <topic>.runbook.md
```

### 4.1 专题文档角色边界
- `<topic>.prd.md`
  - 说明为什么做、做成什么、如何验收。
  - 不写任务拆解，不写每日进度。
- `<topic>.design.md`
  - 说明系统怎么设计、如何分层、接口契约、状态机、错误处理。
  - 不写任务排期，不写单日开发日志。
- `<topic>.project.md`
  - 说明怎么拆任务、先后顺序、依赖、owner、状态。
  - 不重写目标态需求。
- `<topic>.manual.md`
  - 说明如何使用、如何操作、如何验证。
- `<topic>.runbook.md`
  - 说明如何发布、值班、排障、回滚、恢复。

### 4.1.1 测试相关文档分工
测试专题或功能专题中的测试内容，按“要求 / 设计 / 步骤 / 值班剧本”四层拆分：

- `*.prd.md`
  - 写测试目标、覆盖范围、门禁等级、验收口径。
  - 适合描述 `test_tier_required/full`、必须覆盖的用户路径、通过标准与证据要求。
  - 不写逐步点击、逐条命令、逐屏检查细节。
- `*.design.md`
  - 写测试系统或测试方案如何设计。
  - 适合描述分层测试策略、自动化架构、fixture/mock、测试 API、观测点、采证模型、失败注入与覆盖矩阵。
  - 当测试本身涉及状态机、时序、并发、桥接协议或数据采集模型时，应放在 `*.design.md`。
- `*.manual.md`
  - 写详细执行步骤。
  - 适合承载手动测试步骤、自动测试执行步骤、环境准备、启动命令、操作顺序、预期结果、截图点、结果记录方式。
  - 这是“详细测试步骤”的默认落点。
- `*.runbook.md`
  - 写发布前验证、线上巡检、故障排查、回滚验证、值班处置剧本。
  - 只要测试步骤明显偏向运行值守或发布处置，就不应继续放在 `*.manual.md`。

判断规则：
- 测试“要测什么、算什么通过” -> `*.prd.md`
- 测试“为什么这样测、测试系统怎么搭” -> `*.design.md`
- 测试“具体怎么一步一步执行” -> `*.manual.md`
- 测试“线上/发布/故障场景怎么验证和处置” -> `*.runbook.md`

### 4.2 什么时候必须补 `*.design.md`
满足以下任一条件时，专题必须补充 `*.design.md`：
- 存在明确的组件边界；
- 存在 API / 协议 / 接口契约；
- 存在状态机、并发语义或时序约束；
- 存在异常处理、回滚、兼容性设计；
- PRD 已经开始承载“怎么实现”的细节。

### 4.3 什么时候可以暂不写 `*.design.md`
只有当专题同时满足以下条件时，可以短暂只保留 PRD + Project：
- 范围很小；
- 没有新增结构设计；
- 没有独立接口或状态机；
- 预期不会演化为长期维护主题。

一旦条件失效，应立即补齐 `*.design.md`。

## 5. 分册级组织规范
当 `*.design.md` 或专题内容过长时，可在专题目录下继续拆分分册。

推荐形式：

```text
doc/<module>/<topic>/
  <topic>.design.md
  design/
    <topic>-architecture.design.md
    <topic>-interfaces.design.md
    <topic>-state-machine.design.md
```

规则：
- 保留一个总入口 `<topic>.design.md`。
- 分册命名继续显式携带 `.design.md`，不要退回自由命名。
- 总入口负责索引分册，不要求读者直接从分册开始。

## 6. 命名规则

### 6.1 推荐命名
- 模块根入口：固定名 `prd.md` / `design.md` / `project.md`。
- 专题文档：`<topic>.prd.md` / `<topic>.design.md` / `<topic>.project.md`。
- 分册文档：`<topic>-<aspect>.design.md` / `manual.md` / `runbook.md`。

### 6.2 不推荐命名
以下命名不应再作为新专题的主入口类型：
- `*architecture*.md`
- `*interface*.md`
- `*integration*.md`
- `*overview*.md`

这些名字可以作为 `*.design.md` 的分册名存在，但不再承担“统一入口类型”的角色。

## 7. 引用关系规范
文档间应形成固定的引用链：

```text
doc/README.md
  -> doc/<module>/prd.md
  -> doc/<module>/design.md
  -> doc/<module>/project.md

 doc/<module>/prd.md
  -> doc/<module>/design.md
  -> doc/<module>/project.md
  -> doc/<module>/prd.index.md

 doc/<module>/<topic>/<topic>.prd.md
  -> <topic>.design.md
  -> <topic>.project.md
```

专题级文档最少应满足：
- PRD 指向 Project；
- PRD 推荐指向 Design；
- Project 指向 PRD；
- Design 指向 PRD 与 Project。

注：本规范中的 `project.md` / `*.project.md` 是目标命名。当前仓库若仍存在 `project.md`，视为历史实现形式，不影响该规范作为未来建档标准。

## 8. 例外规则
以下情况允许偏离最小三件套，但必须说明原因：
- 纯索引文档：如 `README.md`、`prd.index.md`。
- 纯手册文档：对外或对内操作手册，不承载需求和设计。
- 纯运行手册：如发布、故障处理、回滚剧本。
- 历史保留文档：暂时保留旧命名，但不作为新增建档模板。

例外规则不应成为绕过 `*.design.md` 的常用手段。

## 9. 建档决策树
新增文档前，按以下顺序判断：
1. 这是哪个模块/专题对象？先确定目录。
2. 这份文档回答的是 Why/What/Done 还是 How/Structure/Contract 还是 How/When/Who？再确定后缀。
3. 如果同时涉及需求、设计、执行，拆成多份文档，而不是写成一份混合文档。
4. 如果是操作步骤，判断是 `manual` 还是 `runbook`，不要落到 `design`。

## 10. 推荐模板组合

### 10.1 模块级
```text
doc/<module>/
  README.md
  prd.md
  design.md
  project.md
  prd.index.md
```

### 10.2 常规专题级
```text
doc/<module>/<topic>/
  <topic>.prd.md
  <topic>.design.md
  <topic>.project.md
```

### 10.3 带操作文档的专题
```text
doc/<module>/<topic>/
  <topic>.prd.md
  <topic>.design.md
  <topic>.project.md
  <topic>.manual.md
  <topic>.runbook.md
```

### 10.4 超大专题级
```text
doc/<module>/<topic>/
  <topic>.prd.md
  <topic>.design.md
  <topic>.project.md
  design/
    <topic>-architecture.design.md
    <topic>-interfaces.design.md
    <topic>-state-machine.design.md
```

## 11. 裁定原则
当文档落位出现争议时，按以下优先级裁定：
1. 是否符合“目录按对象、文件按职责”。
2. 是否能让读者在最短路径内找到同一专题的配套文档。
3. 是否把需求、设计、执行、操作混在了一起。
4. 是否保留稳定入口，避免每次都重新学习命名。

如果一个方案同时满足以上四点，则视为合规。
