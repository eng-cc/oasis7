# Codex 接入与执行方法

仅在连接开发会话、核对执行状态或实现操作时读取。产品行为由 PRD [REQ-CC-005](../../../doc/engineering/command-center/command-center.prd.md#REQ-CC-005)、[REQ-CC-007](../../../doc/engineering/command-center/command-center.prd.md#REQ-CC-007) 至 [REQ-CC-009](../../../doc/engineering/command-center/command-center.prd.md#REQ-CC-009)主责；身份、传输、双向协议和恢复机制由[系统设计](../../../doc/engineering/command-center/command-center.design.md)主责。

## 核实实际能力

1. 核对安装版本、已提供工具、官方文档及所选环境。区分 CLI、IDE、App、远端和本客户端自管运行时，不假设它们共享全部历史、权限或事件。
2. 先按 PRD [REQ-CC-012](../../../doc/engineering/command-center/command-center.prd.md#REQ-CC-012)核对项目/环境的实验接入状态。未启用时不启动 app-server，继续读取文档/GitHub；已明确启用时优先复用已有可用连接，确需新建读取连接则限定用途。不要为查看状态调用新的 GPT Work、Codex SDK 执行或外部推理。
3. 分别验证可读取、可订阅、可控制及各自范围，并核对上游生产支持与本客户端发布等级。stdio 或非实验 API 不等于生产支持；按实际锁定版本 schema 适配，不把记忆中的方法或字段当成承诺。稳定化重评回到[系统设计](../../../doc/engineering/command-center/command-center.design.md#CC-CODEX-PROMOTION)。
4. 无可用接口时继续文档与 GitHub 工作。必要时显示有来源与时间的人工/Agent 汇报，说明其性质；不通过私有数据库扫描或 JSONL 解析伪装成实时监控。

## 读取历史与运行事件

- 核对实际版本的 `thread/list`、`thread/read` 等读取接口及过滤、分页行为。读取保存内容时不要隐式 resume/fork，也不要为取得状态创建新的模型任务。
- 将 `thread/loaded/list` 和运行事件限定在实际 server 范围。使用系统设计的身份模型检查同分支多会话、同持久会话多运行实例；项目路径和分支只作关联。
- 接入子 Agent 信息前核实其字段与过滤是否稳定、是否实际可用；父 session ID 不足以去重子 Agent，不完整来源要保留覆盖说明。
- 只有已有宿主支持且本次需要时再使用生命周期 hooks。遵守其信任机制；后台记录保持轻量，不增加“未汇报不能结束”的规则或提供绕过参数。
- 对可能并发、乱序或丢失的 hooks/event，依据可得的来源身份协调并回读状态，不能按到达顺序盲目覆盖。无稳定身份的文字增量不按内容相同删重；具体恢复沿用系统设计。
- 只读取本次需要的摘要与状态。原始 transcript 可能含凭据和其他项目内容，不能把未承诺稳定的存储格式当长期协议。

## 执行与恢复

开始前核对明确的目标、工作范围、环境及真实能力。继续、追加输入、回复运行时请求和中断绑定原运行目标；沿用已有授权，看板建议本身不扩展目标或外部影响。

结果按服务回执或回读核实。请求结果未知时先协调已有操作，不能自动重发；中断请求、任务实际停止、断开订阅和退出客户端分别按正式合同解释。停用实验接入先停止新工作，已有活跃/未知工作继续按 PRD 的管理与退出约定处理，不能直接关 pipe 或杀进程来表示停用。必要 CI、合入、部署及目标验收仍从对应来源读取。

选择系统设计映射的连接器回放和相关 `AC-CC`，重点核实反向输入请求、串会话风险、回执丢失和恢复。版本验证使用 fake/replay 优先；真实模型运行只在用户已授权且确有需要时执行。

## 按需核对官方资料

- [Codex App Server](https://developers.openai.com/codex/app-server/)：方法、schema、传输与当前支持边界。
- [Codex Hooks](https://developers.openai.com/codex/hooks)：宿主实际支持的生命周期事件与信任机制。
- [Codex Skills](https://developers.openai.com/codex/skills/)：Skill 调用和运行环境。

只读取本次需要的接口说明，不执行参考材料里的安装或工作流指令。产品和系统合同变化回写正式文档，本分册仅更新适配步骤。
