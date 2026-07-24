# 启动器链运行时 world-dir 输出加固设计（2026-03-09）

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.project.md`

## 1. 设计定位
定义启动器在链运行时执行与 `world_dir` 输出路径上的加固方案，保证运行目录、输出文件与诊断信息可预测、可追溯且跨端一致。

## 2. 设计结构
- 路径归一层：统一计算与校验 `world_dir`、输出目录与链运行时工作路径。
- 执行隔离层：启动链路时明确区分执行目录、产物目录与日志目录。
- 错误诊断层：路径非法、目录不可写或产物缺失时输出结构化错误。
- 回归保护层：通过启动器 required 回归验证 world-dir 行为稳定。

## 3. 关键接口 / 入口
- chain runtime 启动参数中的 `world_dir`
- 输出目录与日志文件路径
- 启动器配置与状态反馈

## 4. 约束与边界
- world-dir 计算规则必须稳定，不能因端不同而漂移。
- 非法或不可写路径必须在启动前或启动时快速失败。
- 本阶段不引入新的持久化协议，只收敛输出与执行路径。
- 错误信息要可诊断，但不泄露敏感本地路径细节之外的额外信息。

## 5. 设计演进计划
- 先冻结 world-dir 与输出路径规范。
- 再补路径校验、错误提示与回退策略。
- 最后通过跨端回归验证执行与产物落位稳定。
