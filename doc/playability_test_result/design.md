# playability_test_result 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/playability_test_result/prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments
- 对应文件级索引: `doc/playability_test_result/prd.index.md`

## 1. 设计定位
本模块的 `design.md` 负责说明可玩性测试结果如何组织、记录、回流和归档。

## 2. 阅读顺序
1. `doc/playability_test_result/prd.md`
2. `doc/playability_test_result/design.md`
3. `doc/playability_test_result/prd.index.md`
5. 下钻卡片、手册与专题结果文档

## 3. 设计结构
- 结果层：测试卡、记录卡与专题结果的落位方式。
- 回流层：结果如何回流到 game / testing / world-simulator。
- 证据层：记录如何作为发布与整改证据使用。

## 4. 集成点
- `doc/game/prd.md`
- `doc/testing/prd.md`
- `testing-manual.md`

## 5. 专题导航
- 结果卡与手册保留在本模块下
- 后续若出现结构化测试方案，应补专题 `*.design.md`

## 设计目标
- 提供 `playability_test_result` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/playability_test_result/prd.md`
- 可变执行状态：GitHub task issue evidence comments
- 索引入口：`doc/playability_test_result/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。


<a id="feedback-score-calculation-design"></a>
## 评分计算与回流

依据[问卷评分需求](prd.md#feedback-score-product-contract)，计算按“保留原答案与来源 → 校验题目回答及评审归属 → 依唯一映射换算 → 检查维度完整性 → 未舍入聚合/判档 → 应用附加规则 → 展示/归档”执行。公式与常量只在[卡片评分合同](playability_test_card.md#feedback-score-contract)维护，输出同时保留原始分数和显示分数，避免显示舍入影响标签。这里定义实现/人工复核的共同算法，不声称已有脚本自动完成。

缺答、非法单选多勾或未评审会阻断相应计算，不补值、不重分配权重；运行未入局与证据缺失继续按 RunResult 表达。汇总器先固定当前候选及来源分组，再选择该组的完整已评卡片；排除理由和数量必须可见，旧卡、新卡及 AI/真人来源不暗中混合。保存新评审而保留旧答案/旧结论，不能用重算覆盖历史事实。数值建议、owner 结论、玩家杠杆和发布证据保持可分别审阅。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [评分与证据边界](prd.md#feedback-score-product-contract) | 同一答卷唯一复算；缺失不补值；历史及多卡来源可区分 | [计算与回流](#feedback-score-calculation-design) | QA/玩法 owner 评审答案、最终结论和发布输入 | 不实现业务评分器、不代签 L4/L5 或发布结论 |

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [评分与证据边界](prd.md#feedback-score-product-contract) | [计算与回流](#feedback-score-calculation-design) | 文档规则变更后离线复核原反例、四档映射、缺答、邻近阈值与覆盖规则 | 按[总手册](../../testing-manual.md)的卡片审阅流程，用本版合同的静态答卷 fixture 逐步复算并核对模板引用；无需启动游戏 | 同修订规则、fixture 输入、精确计算结果与条件标签 | 不证明实际试玩、已部署聚合器或历史卡片通过 |
