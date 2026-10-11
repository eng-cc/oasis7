# 私有资料管理工具系统设计

- 文档 ID：`DESIGN-ENGINEERING-PRIVATE-CONTEXT`
- 状态：`active`
- owner_role：`runtime_engineer`
- module：`engineering`
- last_reviewed_at：2026-10-11
- 上游：[配对产品需求](private-context.prd.md)、[工程域 PRD](../prd.md#PRD-ENGINEERING-035)

### 2.1 需求承接与分配表

| 上游条款 | 独立义务与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖 |
| --- | --- | --- | --- | --- |
| [REQ-PC-001](private-context.prd.md#REQ-PC-001) | AC-PC-001；首版独立义务 | [DES-PC-001](#DES-PC-001) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |
| [REQ-PC-002](private-context.prd.md#REQ-PC-002) | AC-PC-002；首版独立义务 | [DES-PC-002](#DES-PC-002) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |
| [REQ-PC-003](private-context.prd.md#REQ-PC-003) | AC-PC-003；首版独立义务 | [DES-PC-003](#DES-PC-003) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |
| [REQ-PC-004](private-context.prd.md#REQ-PC-004) | AC-PC-004；首版独立义务 | [DES-PC-004](#DES-PC-004) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |
| [REQ-PC-005](private-context.prd.md#REQ-PC-005) | AC-PC-005；首版独立义务 | [DES-PC-005](#DES-PC-005) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |
| [REQ-PC-006](private-context.prd.md#REQ-PC-006) | AC-PC-006；首版独立义务 | [DES-PC-006](#DES-PC-006) | runtime_engineer；context 核心 | 真实数据与生产环境另行使用验收 |

<a id="DES-PC-001"></a>

## 模块与数据

`oasis7_context` 包含普通用户态文件、查询、Git、快照与 CLI。`oasis7_secure_envelope` 是不依赖环境、进程、文件或权限的纯字节加解密 crate。signer 依赖两者，context 不依赖 signer；caller 在固定安装检查前分发 context，成功、失败、帮助和未知子命令都结束在该分支。worker IPC 不增加资料请求。

库根包含 `context.json`、`entries/<id>.md`、空 `entries/.gitkeep`、`.gitignore`、`.gitattributes`。`.context-local/` 保存锁与草稿，不进入 Git；entries staging 也排除。store 配置字段为 schema_version、32 位小写十六进制 store_id、project_id、name。schema 为 `oasis7.context_store.v1`，配置上限 16 KiB。

条目使用限定 JSON front matter：独立 `---` 行、完整 JSON 对象、独立 `---` 行、Markdown 正文。schema 为 `oasis7.context_entry.v1`，字段为 project_id、id、kind、title、environment、owner、tags、state、updated_at、updated_by、verification、credential_refs、links。嵌套对象严格拒绝未知及重复键。ID 与文件名相同，语法 `[a-z0-9][a-z0-9_-]{0,63}`；普通更新保持 id/project。kind 可变，路径不变。

工具写 LF；读一致 LF/CRLF，以实际字节算 revision。BOM、NUL、非法 UTF-8、混合分隔及冲突标记不是合法资料。title 最多 256 字节，声明标签 128 字节，tags 最多 32 个且每个 64 字节，引用最多 32 个，关系最多 64 个。条目上限 1 MiB，全库扫描上限 64 MiB，快照条目原始字节上限 12 MiB、内层 JSON 明文 16 MiB、外层密文 32 MiB，不自动缩小结果范围。

显式绝对 `--root` 优先于 OASIS7_CONTEXT_DIR，再使用显式本机默认 root；不向上搜索源码目录或遍历 HOME。本机配置在 XDG_CONFIG_HOME/oasis7/context.json 或 HOME/.config/oasis7/context.json，schema `oasis7.context_local_config.v1`。可选 default_root、editor（绝对 program 与 args 数组）、verification_max_age_days（1–3650，默认 30）。不经 Shell 拼接 editor。

<a id="DES-PC-002"></a>

## 持久化与并发

根、entries 和本机状态目录通过目录句柄限定路径，拒绝 symlink、hardlink、特殊文件与越界 ID。只读查询不创建锁或状态文件。写入在 store 协调锁下核对容量、当前 revision 与身份，然后在同目录完整 staging、文件同步、原子发布和目录同步。新目录 0700、文件 0600，不递归改写已有目录权限。effect 区分 none、applied、unknown。

新库在同父目录私有 staging 构建，再 no-replace 发布；已有合法空 Git checkout 可继续初始化，context.json 最后发布。已有合法配置保持 store 身份。保留已有 Git 辅助规则，只补缺失行；未知文件不删除。失败返回 INIT_INCOMPLETE 和实际效果。

put 要求完整候选条目和 expected revision 或 if-absent。edit 将私有草稿交给本机 editor，保存前仍按原 revision 竞争；失败保留草稿。归档也是条件更新，不提供永久删除。搜索分页绑定当前视图摘要；变化返回 stale，格式失败不能签发完整游标。

<a id="DES-PC-003"></a>

## 版本与事实检查

revision 是实际文件字节的 `sha256:` 摘要。verification subject 对规范业务内容计算摘要，排除保存时间、保存者及检查声明自身。声明包含观察时间、方法、证据引用和声明者；内容不匹配与过期分别展示。声明不是凭据认证或签名替代。历史恢复重新声明保存时间，不能自动刷新观察时间。

<a id="DES-PC-004"></a>

## 离线 Git

Git checkout 根必须等于资料根，支持 linked worktree。Git 能力检查先在不访问库的命令上验证 `--no-lazy-fetch` 与 `--no-replace-objects`；不支持返回 GIT_CAPABILITY_UNSUPPORTED。清除继承 GIT 路径、trace 与自定义程序覆盖，固定程序与参数数组，禁用 pager、optional locks、fsmonitor、hooks。命令输出 2 MiB 上限、10 秒超时，退出回收子进程，不展示 remote URL。

只允许受约束 rev-parse、ls-tree、ls-files --stage、cat-file、log。读取 raw blob，不运行内容过滤器、textconv、外部 diff 或工作区 porcelain。status 自己比对当前 context.json 与 entries 字节、HEAD blob 与 index blob，标记 comparison=raw_bytes、remote_contacted=false。working_tree_dirty 仅表示资料范围。换行与 clean/smudge 转换可能使结果区别于用户 porcelain。

history 固定本地 HEAD，再查询固定条目路径。返回完整 commit OID、UTC commit 时间、作者声明、标题；限制条数，浅库明确不完整。缺对象返回 HISTORY_INCOMPLETE，无 Git 或无提交历史返回 HISTORY_UNAVAILABLE。历史 restore 只接受完整 commit OID，读取历史 config 和条目原始 blob，核对当前 store_id/project/id，然后走普通条件 put；不 reset 或丢弃后续历史。

Git fetch、提交与 push 属于用户授权的常规流程；查询永不触网，本地远端跟踪引用不证明远端当前状态。Git 更新不与 context 写并行。merge 后运行 doctor，文本合并成功不等于事实语义一致。

<a id="DES-PC-005"></a>

## 加密快照

共用封装保留既有 Argon2id v19 参数（64 MiB、3 次、1 lane）、16 字节随机 salt、24 字节随机 nonce 与 XChaCha20-Poly1305，header 全部认证。原 signer kind、header、容量及错误边界保持互操作；资料使用独立 kind，不能被 signer 旧命令当密钥读取。

快照是当前配置、清单与选定完整条目的可恢复内容，包括未提交及 archived；selection 必须显式标记。清单、摘要、重复键/ID、缺项多项、格式与容量全部先验证。排除 Git、本机 root/editor、锁、草稿、custody 和真实秘密。它不等于 Git 历史备份；历史可用私有远端或独立 Git bundle 保存。

export/inspect/restore 从隐藏 TTY 或显式 --passphrase-stdin 读取口令，不接受命令行或环境口令。KDF 不持有资料写锁。恢复只向不存在目录 no-replace 发布，重建白名单文件与默认辅助规则，保留 store ID，不恢复旧 UID/GID、ACL、可执行位或绝对路径。发布前失败目标不出现；发布后同步失败报告 effect，重试不覆盖旧目标。

<a id="DES-PC-006"></a>

## 命令接口

两个入口共享同一应用实现与 JSON 回执。子命令为 init、doctor、status、list、search、get、put、edit、archive、verify、history、restore、snapshot export/inspect/restore。用户数据输出与诊断分开；诊断不包含正文、口令或 Git remote 凭据，TTY 清理控制字符。snapshot inspect/restore 不要求配置当前 root。

```bash
oasis7_context --root /absolute/private/example init --project oasis7 --name "示例资料库"
oasis7_context --root /absolute/private/example doctor --json
oasis7_context --root /absolute/private/example put machine-example --file /absolute/private-drafts/machine-example.md --if-absent --actor human:example --json
oasis7_context --root /absolute/private/example search "开发机" --json
oasis7_context --root /absolute/private/example history machine-example --json
oasis7_context --root /absolute/private/example restore machine-example --from-commit FULL_COMMIT_OID --expected-revision sha256:CURRENT_DIGEST --actor human:example --json
oasis7_context --root /absolute/private/example snapshot export --output /absolute/private-backups/example.enc
oasis7_context snapshot inspect --input /absolute/private-backups/example.enc --json
oasis7_context snapshot restore --input /absolute/private-backups/example.enc --to /absolute/private/example-restored --expected-project oasis7 --json
```

示例是虚构值，完整 front matter 由 schema 字段约束。正文中的命令与关联引用是资料，不获得执行或授权含义。

## 操作与交付

普通用户安装独立 binary：`env -u RUSTC_WRAPPER cargo install --locked --path crates/oasis7_context`。源码更新不证明已安装 signer caller 更新；如需统一入口应明确安装对应 caller 版本。

首次使用依次选择公开源码外的目录、init、doctor，录入机器/环境/runbook，核对 get/search、条件更新、历史恢复与加密往返。真实资料和生产凭据不进入 CI。doctor 检查格式、身份、权限、链接、容量、残留及历史，但不自动修复或清理。暂停工具使用仍可通过编辑器与普通 Git 维护 Markdown；回退保留资料与历史。

canonical 验证：`env -u RUSTC_WRAPPER cargo test --locked -p oasis7_secure_envelope --all-targets`、`env -u RUSTC_WRAPPER cargo test --locked -p oasis7_context --all-targets`、`env -u RUSTC_WRAPPER cargo test --locked -p oasis7_local_signer --test context_cli`，加受影响 signer 加密回归。Linux workspace_support 执行 all-targets；macOS fleet_health 执行文件系统和 caller 测试，显式准备固定工具链。Cargo 与 CI 控制修改接受既有 required full 选择，安全与兼容边界独立评审。

性能用固定虚构 fixture 记录读取、搜索与快照耗时和内存，不设未经测量的 P95 门禁。容量到上限后分库或独立设计新格式，不借 signer 大 kind 绕过限制。后续 GUI、MCP、服务、语义索引分别验收。

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立义务与适用条件 | 验证方法与当前入口 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-PC-001](private-context.prd.md#REQ-PC-001) | [DES-PC-001](#DES-PC-001) | [AC-PC-001](private-context.prd.md#AC-PC-001) | [验证手册](private-context.manual.md#TEST-PC-001) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
| [REQ-PC-002](private-context.prd.md#REQ-PC-002) | [DES-PC-002](#DES-PC-002) | [AC-PC-002](private-context.prd.md#AC-PC-002) | [验证手册](private-context.manual.md#TEST-PC-002) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
| [REQ-PC-003](private-context.prd.md#REQ-PC-003) | [DES-PC-003](#DES-PC-003) | [AC-PC-003](private-context.prd.md#AC-PC-003) | [验证手册](private-context.manual.md#TEST-PC-003) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
| [REQ-PC-004](private-context.prd.md#REQ-PC-004) | [DES-PC-004](#DES-PC-004) | [AC-PC-004](private-context.prd.md#AC-PC-004) | [验证手册](private-context.manual.md#TEST-PC-004) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
| [REQ-PC-005](private-context.prd.md#REQ-PC-005) | [DES-PC-005](#DES-PC-005) | [AC-PC-005](private-context.prd.md#AC-PC-005) | [验证手册](private-context.manual.md#TEST-PC-005) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
| [REQ-PC-006](private-context.prd.md#REQ-PC-006) | [DES-PC-006](#DES-PC-006) | [AC-PC-006](private-context.prd.md#AC-PC-006) | [验证手册](private-context.manual.md#TEST-PC-006) | 实现 PR 的实际测试与角色评审回执 | 生产环境与真实资料可用性 |
