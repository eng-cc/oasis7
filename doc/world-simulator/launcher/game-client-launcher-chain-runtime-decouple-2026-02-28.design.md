# 启动器与链运行时解耦设计（2026-02-28）

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.project.md`

## 1. 设计定位
定义 launcher 与 chain runtime 的职责拆分、控制面边界、进程编排与状态同步方式。

## 2. 设计结构
- 职责层：launcher 负责配置与进程编排，chain runtime 负责链逻辑执行。
- 控制层：启动、停止、状态查询与错误传播协议。
- 目录层：输出路径、进程参数与日志位置解耦。

## 3. 关键接口 / 入口
- launcher 控制面 API 与 runtime 进程参数
- 状态轮询与错误回传结构
- 输出目录与执行世界目录约定

## 4. 约束与边界
- launcher 不得内嵌链逻辑。
- runtime 进程失败必须返回结构化错误给 launcher。
- 目录参数变更需保持向后兼容迁移。

## 5. 设计演进计划
- 先完成 Design 补齐与互链回写。
- 再沿项目文档推进实现与验证。
