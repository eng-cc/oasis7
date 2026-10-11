# 私有资料工具验证手册

- owner_role：`qa_engineer`
- module：`engineering`
- last_reviewed_at：2026-10-11
- 上游：[系统设计](private-context.design.md)、[产品验收](private-context.prd.md)

使用临时目录、虚构资料和测试口令。先运行设计的 canonical all-targets 与 caller integration 命令；下面操作用于复核测试证据，不表示已在生产环境完成。每项记录程序版本、平台、预期与实际回执；失败修复后复测。

<a id="TEST-PC-001"></a>
## 两入口与格式

在没有 signer 安装的用户目录运行独立 binary 和 caller context。同一 root 录入 machine、environment、runbook，比较 init/get/list/search 业务 JSON、错误码和退出码。未知子命令必须直接结束。重复 JSON 键、未知字段、非法 front matter、UTF-8 和身份不匹配必须拒绝；修改外部 Markdown 后 get 应反映实际字节 revision。

<a id="TEST-PC-002"></a>
## 并发与文件安全

读取 revision 后并发提交两候选，仅一次成功。打开 edit 草稿期间用 put 更新正式条目，再保存草稿，确认冲突且两方内容保留。在临时库构造 symlink、hardlink、FIFO 与越界 ID，确认拒绝且外部文件未访问；测试临近全库容量的不同 ID 写入。注入 staging、发布、目录同步故障，检查完整旧/新文件及 effect。

<a id="TEST-PC-003"></a>
## 事实检查

verify 登记 observation 与虚构 evidence-ref，读取状态。修改正文与凭据引用，旧 subject 必须失效；只变化保存时间不改变业务摘要。对过期与 subject mismatch 分别核对声明状态，运行摘要固定向量和 verification-only 更新测试。

<a id="TEST-PC-004"></a>
## 离线历史

初始化本地 Git fixture，提交资料，保存 commit OID，再修改提交。history 限制条数，restore 旧 OID 产生新工作区变化并保留 HEAD。对旧 commit 添加 replace ref，读取仍是原始对象。安装有标记副作用的 filter/fsmonitor，查询不得产生标记。浅库报告不完整，删掉 fixture 的一个 blob 后返回 HISTORY_INCOMPLETE，不获取远端；普通目录返回 HISTORY_UNAVAILABLE。

<a id="TEST-PC-005"></a>
## 快照与兼容

全库与 selection 分别 export、inspect、restore 到新目录，核对 store ID、所有字节与归档条目；本机配置、Git、锁及草稿不得包含。错误口令、kind、schema、摘要、重复 ID、缺项、多项、超限均在发布前拒绝。目标已存在或并发出现时不覆盖。运行旧 signer fixture 交叉解密、导入导出和备份回归。

<a id="TEST-PC-006"></a>
## 有界输出与平台

稳定库翻页应完整且不重复；页间修改后应 stale，损坏条目不得签发完整游标。管道保留有意资料输出，PTY 的 ESC/OSC 不执行，错误诊断不包含正文或测试口令。Mac/Linux 复核 init/clone 后缺少本机状态仍能只读查询，CI scope 与 runner 必须实际运行新包 all-targets 和 caller 集成测试。
