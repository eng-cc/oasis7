# headless-runtime 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/headless-runtime/prd.md`
- 对应项目管理文档: `doc/headless-runtime/project.md`
- 对应文件级索引: `doc/headless-runtime/prd.index.md`

## 1. 设计定位
`headless-runtime` 模块的 `design.md` 负责描述无界面运行链路的总体设计，包括：
- 非 Viewer 运行模式的系统边界；
- 长时运行、认证、安全与归档相关设计；
- 与 world-runtime / p2p / testing 的集成位置。

## 2. 阅读顺序
1. `doc/headless-runtime/prd.md`
2. `doc/headless-runtime/design.md`
3. `doc/headless-runtime/project.md`
4. `doc/headless-runtime/prd.index.md`
5. 下钻 `nonviewer/` 等专题目录

## 3. 设计结构
- 运行形态层：定义 headless/nonviewer 模式与运行约束。
- 稳定性层：定义长时运行、内存、归档与恢复策略。
- 安全边界层：定义认证、协议与线上约束。

## 4. 集成点
- `doc/world-runtime/prd.md`
- `doc/p2p/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- `doc/headless-runtime/nonviewer/*` 承载细分专题
- 涉及状态机、协议或异常恢复的主题继续下钻到专题 `*.design.md`

## 设计目标
- 提供 `headless-runtime` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/headless-runtime/prd.md`
- 执行入口：`doc/headless-runtime/project.md`
- 索引入口：`doc/headless-runtime/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
