# 启动器 chain runtime stale execution world 恢复设计（2026-03-12）

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.project.md`

## 1. 设计定位
在不削弱 runtime 一致性校验的前提下，把“默认 node id 命中 stale execution world”从底层日志问题提升为 launcher 可识别、可恢复的产品级错误。

## 2. 设计结构
- 错误识别层：匹配 `DistributedValidationFailed` / `latest state root mismatch` 签名，生成 launcher 侧结构化错误。
- 恢复决策层：为默认 node id 提供 fresh node id 恢复；保留受控清理作为后续增强能力。
- 表现层：Web 控制面、GUI Agent、桌面启动器共用恢复状态与 CTA 语义。
- 回归层：覆盖 stale 失败、fresh node id 恢复、恢复后 explorer 查询成功。

## 3. 关键接口 / 入口
- `oasis7_web_launcher` 进程/状态机与 `gui_agent_api`
- `oasis7_game_launcher` 链启动参数与错误反馈
- launcher UI 中的链状态区、引导/错误卡片、恢复 CTA

## 4. 约束与边界
- runtime 校验失败仍然必须失败，launcher 只负责识别与恢复引导，不改 runtime 判定。
- 任何破坏性清理都不得默认自动执行。
- fresh node id 恢复应只改变必要链配置，避免引入更多默认漂移。

## 5. 设计演进计划
- 先补错误签名分类与统一错误码。
- 再补 fresh node id 恢复链路和 GUI Agent 契约。
- 最后根据实际试玩/QA 数据决定是否要加受控清理默认目录的入口。
