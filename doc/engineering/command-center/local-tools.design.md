# 指挥中心本机工具集成

- 文档 ID：`DESIGN-ENGINEERING-COMMAND-CENTER-LOCAL-TOOLS`
- 状态：`draft`；实现及验收以实际 PR 为准
- owner_role：`runtime_engineer`
- module：`engineering`
- 上游：[指挥客户端产品设计](command-center.prd.md)、[系统设计](command-center.design.md)
- last_reviewed_at：2026-10-12

## 1. 目标和当前范围

在现有 macOS 指挥中心增加“本机工具”，使用普通用户 Rust 宿主复用 context 资料库，管理同包 CLI，接入已有兼容 signer。用户提供的 2026-10-12 本机工具设计是完整目标；本文记录当前实现合同及实际剩余范围，不将目标写成完成事实。

用户于 2026-10-12 明确暂停 Apple 代码签名和管理员助手工作，待背景讨论后继续。因此 SMAppService／认证 XPC／Authorization Services、signer 初装、密钥与 policy/grant 管理、受保护加密密钥导入导出和 signer 备份计划恢复均未开放。当前 Mac 未检测到可用 Apple 代码签名身份；此观察不等于所有 Mac 均不支持。已有兼容安装的普通 caller 不受该暂停影响。通用升级、自动更新、其他 purpose UI、远程安装和动态插件仍不在首版范围。

<a id="DES-CC-TOOLS-001"></a>

## 2. 普通用户能力

context 的 `application` 应用接口共享显式资料 root、完整原文校验、条件保存、历史恢复身份校验及原快照实现；CLI 不在 GUI 进程中运行。`info --json` 固定 schema 1，未知构建来源返回 null；它不打开资料库或 custody。caller 的 info 和其 context info 分别表示各自能力。

资料绑定持久保存真实 root、设备号/inode、store ID、原 store 项目 ID、面板关联项目 ID 和单调版本。所有读写按原绑定核对目录及 store 身份。普通项目和私有资料目录双向包含均拒绝，普通索引、缓存回退、搜索、阅读历史访问和来源打开都检查私有隔离；用户历史数据保留而不经普通入口暴露。正文只按需读取，不持久复制到普通文档索引。

简化表单修改完整原文中的负责字段，保留其余合法元数据。新建是 create-only，修改提交读取时 revision；CLI 与 GUI 使用原跨进程写锁。冲突保留草稿，重新读取后可再次保存；已确定未写入的错误不能锁死目标。资料落盘、Git 提交和远端同步是独立事实。

加密资料快照沿用 context 的明确选择范围、create-only 输出和新目录恢复。口令通过绑定窗口／操作／用途的随机短时句柄进入宿主内存，一次消费、两分钟过期、取消/退出清除，不持久化到任务或日志。快照在独立阻塞 worker 中执行，不持普通项目索引锁。

可选 CLI 与 App 同次构建，catalog 固定提交、release、target、大小和 SHA-256；仅复制核对后的资源到用户受管版本目录，不覆盖不同内容的已有文件。实际绝对路径可调用，PATH 可见性单独显示；不临时下载或从项目执行安装脚本。

### 2.1 需求承接与分配表

| 上游条款 | 独立义务与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖 |
| --- | --- | --- | --- | --- |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | 本机资料、保存、历史、快照和同包 CLI | [DES-CC-TOOLS-001](#DES-CC-TOOLS-001) | context 领域库、runtime_engineer | 不将落盘当作 Git 远端同步 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | 已有兼容 signer 的普通文件签名 | [DES-CC-TOOLS-002](#DES-CC-TOOLS-002) | signer caller、原 worker | 用户暂停特权初装/管理 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | 未知操作、资源占位及冻结退出 | [DES-CC-TOOLS-003](#DES-CC-TOOLS-003) | ToolJobs、原 context/signing 持久记录 | 不从进程消失推导回滚 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | 显式私有输入和持久线程属性 | [DES-CC-TOOLS-004](#DES-CC-TOOLS-004) | 实验 runtime、agent_engineer | 不保证原 runtime/外部 CLI 全部转存追踪 |

<a id="DES-CC-TOOLS-002"></a>

## 3. 已有 signer

后端只读取固定安装描述并核对普通 caller UID、受保护实际路径、release manifest 和 executable SHA-256。未知或不兼容安装保留原状；不经普通 App 读取 custody。

文件签名要求明确 file、job、signer、grant 和完整合法 SignContext；不自动填示例值。原 caller 第一次 prepare 建立私有 job，原 Rust 类型生成 context、file envelope 和 canonical JobInput，第二次 prepare 固定原请求。提交和 inspect 绑定原 job；准备不等于 worker 已授权。独立验签重建相同 context 的 envelope，并核对用户明确的预期公钥。选中文件必须是有界普通文件，不接受 FIFO、符号链接或读取中变动。

缺少密钥、policy、grant 或 purpose 配置时明确显示未配置；当前暂停的管理员集成不能用 shell/sudo 透传代替。

<a id="DES-CC-TOOLS-003"></a>

## 4. 操作与退出

工具操作先在 SQLite 事务中原子登记 operation ID、输入指纹、不可变目标资源及动作。相同 ID/输入回读既有记录，不重放；不同输入拒绝；已有 running/unknown 占位防止重复副作用。终态不倒退，崩溃中的任务转 unknown，不由进程消失推导回滚。

完整退出冻结同一准入机制，并检查所有项目的 Codex 和 ToolJobs；运行中或未知效果阻止安全退出声明。窗口关闭仍留后台。Codex 新动作与冻结使用同一锁协调，已登记的后台任务继续回读。口令和正文不作为恢复参数。

<a id="DES-CC-TOOLS-004"></a>

## 5. 显式私有资料输入

在原自管会话选择资料并预览来源修订与整条或明确行范围。发送时后端重读同一 binding/version/revision，变动要求重新预览。发送正文前持久登记线程私有来源；失败不发送。原线程后续普通轮次、恢复、模型派生请求与审批仍保持私有属性。

事件写入及历史读取从独立私有来源表恢复标记，迟到 snapshot 不能清除。普通 runtime 投影过滤私有线程、请求及操作，专用运行界面保留必要批准详情。既有数据库可能包含由私有输入派生的请求内容；不能直接复制整份 SQLite 并称为普通配置备份。原运行时或外部 CLI 的转存不在本 App 完整追踪范围内。

## 11. 验收与验证

| 合同 | 验证入口 | 仍需真实环境核对 |
| --- | --- | --- |
| 资料条件保存、未知元数据和外部修改 | `core/tests/context.rs`、context CLI/core tests、LocalTools UI tests | 同包 App 原生选择、重开、实际 CLI 并发 |
| 双向隔离、目录替换和旧索引/历史 | `core/tests/context.rs`、foundation/document_lifecycle tests | 已有用户数据库的有界迁移观察 |
| 快照范围、错误口令、新目录恢复 | context snapshot regressions、native finite command checks | 同包 App 真实导出和恢复 |
| CLI catalog、create-only、拒绝替换 | tools CLI regressions、打包脚本 | App 资源内实际 binary/info 与用户安装路径 |
| 文件签名输入和预期公钥验签 | tools 文件边界测试、signer detached tests | 已有专用 signer 的真实 submit/inspect；首次配置受用户暂停 |
| 幂等、资源占位、冻结和终态 | tools jobs/secrets regressions、native quit tests | App 长操作/退出交互 |
| 线程私有标记及后续轮次 | privacy unit tests、codex stdio fault replays | 用户显式启用的实验 runtime；本次不启动付费模型任务 |
| 管理员桥和 signer 初装/管理 | 用户明确暂停，未开放 | Apple 签名、系统批准、认证 IPC、gate 和专用空安装 |

普通 CI 不能代替原生系统授权、真实 signer 或 Finder 验收。验证结果、候选版本和未执行项目在实际 PR 记录；不新增通用流程台账。

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立义务与适用条件 | 验证方法与当前入口 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | [DES-CC-TOOLS-001](#DES-CC-TOOLS-001) | 独立库、CAS、元数据、快照及 CLI | [context tests](../../../tools/oasis7-command-center/core/tests/context.rs) 数据隔离和条件保存；context snapshot/CLI 回归；真实 App 新建与重开 | 实现 PR source/build/机器和实际结果 | 未执行原生场景不得计通过 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | [DES-CC-TOOLS-002](#DES-CC-TOOLS-002) | 同一请求 prepare/submit/inspect 和公钥验签 | [signer 授权反例](../../../crates/oasis7_local_signer/tests/qa_m0_negative.rs) 精确 purpose/context/hash；detached 单元测试验证 envelope；tools 输入反例和已有专用 signer 回读 | 实现 PR request/job 和真实结果 | 无测试安装时不证明 worker 实际提交；管理员用户暂停 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | [DES-CC-TOOLS-003](#DES-CC-TOOLS-003) | 幂等、冻结、未知效果、口令句柄 | [runtime 故障回放](../../../tools/oasis7-command-center/core/tests/codex_runtime.rs) 未知/退出；tools jobs 单元测试覆盖事务/冻结/查询可用性；secrets 和 native quit tests | 实现 PR 故障与退出回放 | 强退不保证后台继续 |
| [REQ-CC-013](command-center.prd.md#REQ-CC-013) | [DES-CC-TOOLS-004](#DES-CC-TOOLS-004) | 私有标记先于发送及后续轮次/恢复 | [runtime fault tests](../../../tools/oasis7-command-center/core/tests/codex_runtime.rs) 登记失败不发 frame、后续普通轮次和重开过滤；PrivateContextInput UI tests | 实现 PR 原线程/来源和回放结果 | 本次不启动模型任务，不证明模型输出质量 |
