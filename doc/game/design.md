# game 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/game/prd.md`
- 对应项目管理文档: `doc/game/project.md`
- 对应文件级索引: `doc/game/prd.index.md`

## 1. 设计定位
`game` 模块的 `design.md` 负责承接玩法层的总体设计入口，重点回答：
- 玩法规则、经济系统、协作治理和发行可玩性如何分层；
- 玩法目标如何映射到运行时、测试与发行门禁；
- 哪些能力留在模块级设计，哪些能力继续下钻到专题 `*.design.md`。

## 2. 阅读顺序
1. `doc/game/prd.md`
2. `doc/game/design.md`
3. `doc/game/project.md`
4. `doc/game/prd.index.md`
5. 下钻 `gameplay/` 等专题目录

## 3. 设计结构
- 规则层：定义核心循环、资源与治理边界。
- 系统层：定义 gameplay、发行可玩性、长期在线 P0 的结构分层。
- 验证层：通过 `testing-manual.md`、`doc/testing/*` 和可玩性结果对玩法质量做闭环验证。

## 4. 集成点
- `doc/testing/prd.md`
- `doc/playability_test_result/prd.md`
- `doc/world-runtime/prd.md`
- `doc/world-simulator/prd.md`

## 5. 专题导航
- 玩法专题继续放在 `doc/game/gameplay/`
- 规则和经济相关设计进入对应专题 `*.design.md`
- 发布与可玩性闭环通过 testing / playability 侧文档联动

## 设计目标
- 提供 `game` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/game/prd.md`
- 执行入口：`doc/game/project.md`
- 索引入口：`doc/game/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
