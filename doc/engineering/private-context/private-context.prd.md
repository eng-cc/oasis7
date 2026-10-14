# 私有资料管理工具产品需求

- 文档 ID：`PRD-ENGINEERING-PRIVATE-CONTEXT`
- 状态：`active`
- owner_role：`producer_system_designer`
- module：`engineering`
- last_reviewed_at：2026-10-11
- 上游：[工程域 PRD](../prd.md#PRD-ENGINEERING-035)
- 配对设计：[系统设计](private-context.design.md)

## 目标与边界

开发者与开发型 AI 使用同一份机器、环境和操作说明，能够辨别内容版本、来源和检查声明。资料保存在公开源码目录之外的独立私有 Markdown 库；真实密码、Token 和签名私钥继续由 signer 或外部凭据系统管理，条目只保存凭据定位引用。公开文档与测试使用虚构资料。

工具有等价入口 `oasis7_context` 与 `oasis7_local_signer context`。资料能力不依赖 signer 安装、worker、sudo、grant、任务 ID 或业务代码 HEAD。支持 macOS 与 Linux 普通用户；首版不承诺 Windows。

## 用户行为与验收

<a id="REQ-PC-001"></a>

### REQ-PC-001 共用资料

一个库对应一个 project 和一个共享范围，条目分 machine、environment、runbook。初始化、doctor、get、list、search、put、edit、archive、verify 都作用于完整单文件；默认列表与搜索排除 archived，读取仍可访问归档内容。

<a id="AC-PC-001"></a>

**AC-PC-001**：两 CLI 在无 signer 安装的 fixture 中输出相同业务结果与错误码；未知 context 命令不能回退 signer。中文正文、外部编辑和严格元数据错误可重复验证。

<a id="REQ-PC-002"></a>

### REQ-PC-002 防止覆盖

实际字节 SHA-256 是 revision；更新要求 expected revision，新建要求 if-absent。人工编辑使用私有草稿，检测冲突后保留草稿与正式内容。保存不等于提交或推送。

<a id="AC-PC-002"></a>

**AC-PC-002**：同 revision 的并发更新仅一次成功；edit 期间的另一个更新不被覆盖。目录异常、容量、故障注入不能留下半个正式文件，回执准确报告应用效果。

<a id="REQ-PC-003"></a>

### REQ-PC-003 检查声明

updated_at 是保存声明，verification 是事实检查声明。检查声明绑定业务内容摘要；正文、事实元数据、凭据引用或关系变化后失效。actor、owner、author 都是声明标签，不是认证身份。

<a id="AC-PC-003"></a>

**AC-PC-003**：内容修改导致 subject mismatch；只刷新保存时间不会使事实自动变成已检查；摘要固定向量、声明年龄与声明更新语义有测试。

<a id="REQ-PC-004"></a>

### REQ-PC-004 本地历史

查询不触网；本地 Git 提供已提交历史，浅克隆和缺失对象明确表示不完整。历史恢复形成当前工作区新修改，要求相同 store 身份和普通条件写。

<a id="AC-PC-004"></a>

**AC-PC-004**：恢复不 reset 或删除后续提交；replace refs 不改变指定对象含义；恶意 filter/fsmonitor 和 lazy fetch 不执行；原始字节状态与用户 porcelain 的区别明确展示。

<a id="REQ-PC-005"></a>

### REQ-PC-005 加密恢复

当前内容可以加密导出、检查，并恢复到不存在的新目录。快照包含配置与选定条目、归档条目和引用，不包含 Git、本机配置、锁、草稿或真实凭据。

<a id="AC-PC-005"></a>

**AC-PC-005**：口令、kind、schema、清单、摘要、重复项和容量错误在发布前失败；已存在的目标保持原样；旧 signer 加密格式继续互操作。

<a id="REQ-PC-006"></a>

### REQ-PC-006 有界与可维护

读取与搜索有界，分页变化返回 stale，损坏资料不宣称完整。诊断不泄露正文或口令，TTY 控制字符安全呈现。普通项目构建和 PR 不要求私有库可用。

<a id="AC-PC-006"></a>

**AC-PC-006**：Mac/Linux 文件系统、独立 binary/caller 集成测试在既有 CI 组实际执行；稳定分页完整、页间变化被拒绝；私有数据不进入公开 fixture。

## 首版排除范围

不创建或分享远端仓库，不自动提交、fetch、push、stash、merge、reset 或执行 Markdown 命令。不提供 GUI、MCP、远程服务、语义检索、附件、通用凭据库、完整多人 ACL 或条目级访问控制。标签与查询筛选不构成安全边界；不同共享范围使用不同目录、仓库与运行账号权限。

## 交付解释

源码、测试与文档具备交付条件，不代表真实机器已安装或真实私有库已录入。安装与首次使用步骤由[设计操作章节](private-context.design.md#操作与交付)承接；PR、实际 CI 与评审记录保存验证证据。
