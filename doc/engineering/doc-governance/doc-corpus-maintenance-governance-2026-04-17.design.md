# 文档存量维护成本治理设计（2026-04-17）

- 对应需求文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`

## 1. 设计定位
`PRD-ENGINEERING-024` 解决的是“默认阅读面失控”问题。  
本设计解决的是“阅读面已经收紧，但文档存量本身已经形成新的维护成本”问题。

换句话说，上一轮把“入口看不完”压回去了，这一轮要处理“即使入口看得完，仓库仍然太重”。

## 2. 问题切换的判断标准

### 2.1 第一阶段: 阅读面噪音
特点：
- README 与根入口长期平铺长名单。
- 新读者必须顺扫大量专题才能找到当前真值。
- 主要症状是 `what / where / next / risk` 无法快速回答。

### 2.2 第二阶段: 存量维护成本
特点：
- 根入口与模块 README 已完成首读分流。
- 但文档总量、热点目录、历史 backlog 与近限长文件继续增长。
- 主要症状变成：
  - 搜索结果噪音增加；
  - `project.md` / `prd.index.md` 回写成本提高；
  - 审读与审计要处理更多旧专题与历史留痕；
  - 单文件越来越接近门禁上限。

当前仓库已进入第二阶段。

## 3. 维护成本模型

### 3.1 成本源
- 总量成本：`doc/` 文件总数过大，任何全局审查和检索都更慢。
- 热点路径成本：`world-simulator/viewer`、`p2p/node`、`testing/evidence` 这类热点目录持续累积，使对象边界越来越模糊。
- 历史 backlog 成本：`doc/devlog` 虽已退回历史归档层，但 57 份日文件仍在抬高回溯与去重成本。
- 近限文件成本：`project.md`、`prd.index.md` 等活跃入口文件逼近 1000 行门禁时，任何小变更都会变成结构性拆分压力。

### 3.2 本轮正式基线
- 2026-04-17 任务启动前 `doc/` Markdown 总数: 1730
- 高密度模块: `world-simulator` 549、`p2p` 269、`testing` 178
- 热点历史目录: `doc/devlog/` 57
- 最大单文件: `doc/devlog/2026-02-16.md` 3288 行
- 近限活跃文件示例: `doc/world-simulator/project.md` 998 行

## 4. 库存报告设计

### 4.1 输出内容
`scripts/doc-inventory-report.sh` 固定输出：
1. 总体统计: `doc/` Markdown 总数、`doc/devlog` 文件数、最大 Markdown 文件。
2. 模块密度: 按 `doc/<module>` 汇总计数并按降序排序。
3. 热点子目录: 至少统计到 `doc/<module>/<subdir>` 层级。
4. 历史 backlog: `doc/devlog` 目录的文件数与最大文件。
5. 近限文件: 非 `doc/devlog` 文档中接近 1000 行门禁的文件。

### 4.2 阈值
- `total_docs >= 1500`: 进入 corpus-pressure。
- `module_docs >= 200`: 进入 high-density。
- `subdir_docs >= 80`: 进入 hotspot-subdir。
- `devlog_docs >= 50` 或 `max_devlog_lines >= 2000`: 进入 archive-backlog。
- `non_devlog_lines >= 850`: 进入 near-limit。

### 4.3 输出格式
- Markdown 文本，便于直接贴进 execution log、项目文档、PR 描述或阶段评审。
- 每个告警项显式写出 `status=normal|action_required|split_required`，避免只有数字没有判断。

## 5. Follow-up 分类

### 5.1 历史压缩
适用对象：
- `doc/devlog` 这类已经退出运行态真值、但仍以高频日文件堆积的历史层。

处理顺序：
1. 先确认保留追溯要求。
2. 再补中间索引、月度/阶段摘要或压缩策略。
3. 最后才考虑把日文件进一步下沉或只保留引用入口。

### 5.2 路径级治理
适用对象：
- 模块入口已减重，但模块内部仍以热点子目录堆积大量专题。

处理顺序：
1. 先定位对象边界是否已经混叠。
2. 按对象/职责拆子域，不直接做随机迁移。
3. 需要时再用 redirect 保留历史路径兼容。

### 5.3 近限文件拆分
适用对象：
- `project.md`、`prd.index.md`、manual 等活跃文档接近 1000 行门禁。

处理顺序：
1. 先判断是历史回顾、长表索引还是活跃正文造成膨胀。
2. 把历史回顾退回归档层，把长表拆成定向索引。
3. 保持主入口继续只承担当前职责。

### 5.4 季度复核/门禁扩展评估
适用对象：
- 同一热点连续多轮增长，单靠人工建项已经不足以稳定收口。

处理顺序：
1. 先在季度治理复核中复用库存报告。
2. 若连续增长，再评估是否要把部分阈值升级为门禁或 baseline。

## 6. 当前优先级
1. `doc/devlog`
   - 已不再是运行态真值，但 57 份日文件和 3288 行单文件说明历史压缩债已经独立成立。
2. `world-simulator`
   - 总量 549，`viewer/` 热点依旧巨大，后续更适合路径级二次分层。
3. `p2p`
   - 总量 269，`node/`、`distfs/`、`blockchain/` 同时承载活跃主题与大量 supporting topic。
4. `testing`
   - 虽入口已减重，但 `evidence/`、`ci/`、`longrun/` 的维护边界仍容易继续抬高审计成本。
5. 近限活跃文件
   - 像 `doc/world-simulator/project.md` 这种 998 行文档不应等到门禁失败才拆。

## 7. 与现有治理的关系
- `doc-structure-standard` 决定“文档按什么对象、什么职责落位”。
- `doc-surface-area-governance` 决定“默认阅读面暴露什么”。
- `doc-corpus-maintenance-governance` 决定“入口已经收紧后，哪些文档存量仍需要继续治理”。

三者顺序是：
1. 先把文档放对位置。
2. 再把入口暴露面收紧。
3. 最后处理存量膨胀带来的维护成本。

## 8. 边界与例外
- 不因为体量大就默认删除历史；历史追溯仍需保留。
- 不因为入口干净就宣布治理完成；总量、热点路径和近限文件仍需跟踪。
- 不把库存报告直接当硬门禁；在连续多轮证明必要前，先保留为治理输入。
