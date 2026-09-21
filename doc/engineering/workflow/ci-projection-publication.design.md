# CI Impact Projection 一致发布与恢复系统设计

- 设计 ID：OASIS7-CI-PROJECTION-PUBLICATION-1
- 状态：Proposed；本文的 schema、CLI、错误码、测试和能力均未实现或验收
- Owner role：repository_health_engineer；独立验证角色：qa_engineer
- 模块：engineering/workflow；审读基线：`eng-cc/oasis7@bba55ffa83434518513cf08df1fad7ae950be491`
- Task truth：`task_e9f33b806d6040fe8136bbc62ce1a797`，GitHub Issue [#3871](https://github.com/eng-cc/oasis7/issues/3871)；用户提案见该任务的设计输入
- 交付边界：本地 Codex 客户端和现有 GitHub；手动 loop；系统文档 PR 与后续代码 PR 分离。本文不是 task 状态、运行时 authority 或能力发布声明。

## 1. 问题、目标与非目标

当前 `scripts/prepare-task-pr.sh` 对已有 PR 先发布代码并复用 PR，而 `.github/workflows/rust.yml` 从 PR 事件 body 读取 impact projection，同时从事件 head 读取源提交；因此新 head 可能与旧投影相遇。`scripts/pm/workflow-impact-projection.py` 对 source head 的严格拒绝是正确的，不能放宽。required-plan artifact 目前使用固定名称和 `overwrite: true`；只恢复一次 run 而不隔离 attempt 会混合证据。

目标：标准发布路径使投影与候选 head 对齐；对 body 晚发布、网络不确定和客户端退出提供同 head 恢复；每份被接受的投影精确绑定 Task、source、scope、可信 planner；run/attempt 证据闭合；正常路径不增加第二轮 required-gate。非目标：改变 Rust 测试矩阵、风险触发集成策略、合并权限、source review/integration 的既有身份划分，或引入服务、GitHub App、自动 loop、日常 PR close/reopen 和空提交恢复。

## 2. 上游约束与相关角色

以下固定引用均属于 `eng-cc/oasis7`；实施任务必须再冻结实际消费的发布提交。用户请求是纯工程缺陷修复，不产生产品 AC。专业接受的准确上游定位是 [Issue #3871 的冻结 acceptance evidence](https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462)：(1) 基于当前源码冻结 repo-owned 十二段设计与 professional acceptance；(2) 记录有序代码依赖、信任边界、恢复及 attempt-bound evidence gate，不声称尚未实现的能力。对应 typed 关系为 `trace.upstream_refs[type=professional_acceptance, applicability=required, required=true]`，其 evidence locator 为 `https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462`；`trace.system_design[applicability=required, required=true]` 指向本文 DES-CIP 条款。此处只是 S1 的设计映射，TPM 须在 task truth 中冻结/回读实际 typed 记录；不得把本段视为已写入机器合同。产品 N/A 要按写作规范记录 reason、范围、owner、evidence locator 和复核触发。TPM 管理任务、依赖和 PR 主链；repository health 审计文档/代码合同；QA 收口验证门禁；CI 实施专业角色由 TPM 派发。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 排除范围 |
| --- | --- | --- | --- | --- |
| [workflow source: PR source identity](source-of-truth.md#split-source-review-integration-contract) | 标准发布不得使新的 H 与旧 P 的 source-bound 义务混合 | [DES-CIP-01](ci-projection-publication.design.md#des-cip-01) | TPM 冻结 Issue #3871 的用户请求和 professional acceptance；代码 owner 待派发 | 不声称 hosted 已修复 |
| [workflow source: PR CI receipt](source-of-truth.md#split-source-review-integration-contract) | 过期事件只能在同 H/S、可信 authority 和 binding 下恢复 | [DES-CIP-02](ci-projection-publication.design.md#des-cip-02) | 可信 base verifier/planner、代码 owner 待派发 | 不放宽坏 digest 或错误 task |
| [workflow source: source/integration distinction](source-of-truth.md#split-source-review-integration-contract) | H/B/S/M/T/W 与 source-review、integration 的身份不得混用 | [DES-CIP-03](ci-projection-publication.design.md#des-cip-03) | 既有 workflow/review authority | 不修改复用政策 |
| [workflow source: attempt-bound integration CI](source-of-truth.md#split-source-review-integration-contract) | 每次被接纳的 receipt 必须证明同一 run/attempt 的 check 与 planner evidence | [DES-CIP-04](ci-projection-publication.design.md#des-cip-04) | receipt 与所有 artifact consumers 的代码 owner 待派发 | 不把 A1 成功借给 A2 |
| [workflow source: run/attempt and task gate](source-of-truth.md#split-source-review-integration-contract)；Issue `https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462` item 2，typed `professional_acceptance` required | 元数据修复仅对可信诊断、测试未开始的原 run 整轮新 attempt；不确定时最多一次请求 | [DES-CIP-05](ci-projection-publication.design.md#des-cip-05) | C3 实施、C2 可信协议、QA 独立验证 | 老协议/真实测试失败不自动 rerun |
| [writing standard: interface and trust](../doc-governance/system-design-writing-standard.design.md#6-接口与数据合同)；Issue `https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462` items 1/2，typed `professional_acceptance` required | task publication binding 可解析回读且错误输入/老协议 fail closed；CLI 尚是 Proposed | [DES-CIP-06](ci-projection-publication.design.md#des-cip-06) | C1 parser、C2 workflow、C3 publisher；QA 负例 | 不以自报 digest 或删除 marker 授权 |
| [writing standard: key flows](../doc-governance/system-design-writing-standard.design.md#5-关键运行流程) | 每个发布副作用后的失败、续接及冲突都要有可观察判定 | [DES-CIP-07](ci-projection-publication.design.md#des-cip-07) | repository health review、QA 验证 | 不声称 PR body PATCH 有 CAS |
| [writing standard: verification](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | 提供 fake 与 hosted 分层的可追溯验证，不以结构检查代替运行证据 | [DES-CIP-08](ci-projection-publication.design.md#des-cip-08) | QA 独立验收、TPM 写回任务证据 | 文档测试不证明生产能力 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前基线（上述 HEAD） | 目标 | 差距/证据 owner |
| --- | --- | --- | --- |
| 已有 PR | prepare helper push 后复用 PR URL | 先发布并回读 P(H1)，再精确推送 H1 | publisher；实施任务核对 helper 分支 |
| PR CI | 事件 body 提取，head 为事件 source | 事件优先，限定 stale/missing 情况同 H live 回读 | 可信 resolver；workflow 集成 |
| planner artifact | 固定名称、覆盖；receipt 按名称取唯一项 | R/A 独立的 plan 和完整 receipt 输入 | envelope v2、所有消费者审计 |
| 故障恢复 | 无发布 journal 或限定的同 run 恢复协议 | 回读 refs/body 后续接；整轮新 attempt | publisher/recovery、hosted 验证 |

这些目标均为待实施状态。环境为本地 macOS/Linux publisher、Ubuntu hosted CI；其他平台和真实网络行为尚未证明。

## 4. 边界与结构

本地 canonical task worktree 中的 `prepare-task-pr.sh` 委托 proposed publisher，读取冻结 Task evidence、可信 base planner、Git refs 和 PR，写 PR 机器块、精确 ref 与 git-common-dir journal。GitHub PR workflow 的 proposed resolver 从**事件 base 的固定 OID**提取执行依赖，读取事件投影或受限 live PR，冻结 `selected-impact-projection.json`；planner、required-gate、receipt 与 lifecycle/closeout 消费同一身份。Publisher 可使用现有本地用户 Git/PR/Actions 权限，却不能宣告 CI 成功；resolver 仅可读取，不能 push、PATCH 或 rerun。PR body、候选脚本及自报 digest 均是不可信输入；publication binding 是输入义务，不是通过凭证。

不新增 `edited`、`pull_request_target` 或自动 dispatch。pre-push 的临时 body P(H1) 对 live H0 不应触发测试；同 run 重跑保留原事件和 checkout，只有精确限定的 resolver 回读可更新投影输入。

## 5. 关键运行流程

<a id="des-cip-01"></a>

### DES-CIP-01：有序发布与幂等续接

1. 从 canonical mapping 确认 repo/UID/分支/PR 与独占分支锁，冻结干净工作树的 H1、事件比较基线 B、S=merge-base(B,H1)、D、可信 planner 配置；source 再移动则拒绝。现有 promotion/merge helper 必须遵循同一互斥边界。
2. 用 B 的 verifier/planner 完整验证 P(H1) 与 closure evidence；核对唯一 OPEN PR 的仓库、源/目标分支、Task UID 与远端 H0。先记录并回读 proposed publication binding；失败不继续。
3. 保留 PR 人工文本、Task 和 Refs，仅替换唯一机器 marker；重复 marker 或 Task 冲突拒绝。PATCH 后 GET 回读 H1/S/D，检查 PR 仍 OPEN、未合并、分支不变、远端仍 H0。
4. 用冻结 H1 的显式 refspec 和 `--force-with-lease=refs/heads/<branch>:<H0>` 发布；额外要求 fast-forward，除非既有任务政策明确授权改写。回读 ref/PR 必须为 H1/D，否则进入恢复而非宣告成功。
5. 查找对应 H1 的 run；未观察到时只返回“已发布，run 未观察到”。新 PR 的路径是先验证、绑定和 push H1，再创建携带 P(H1) 的 draft PR；create 响应丢失须按精确身份查询，不能重复创建。远端已是 H1 时只修复 body 并进入同 head 恢复。

<a id="des-cip-02"></a>

### DES-CIP-02：事件优先的严格 resolver

可信 Task/loop admission 先决定 projection 是否必需，marker 缺失不使新任务降级。事件 P 完整匹配 E=(repo, PR, UID, H, S)、可信 publication D 及完整 verifier 时直接冻结，不额外 GET。仅当结构及自身 digest 有效、Task 正确但事件 P 属于另一 H，或 projection-required 任务缺 marker/missing 时，允许 live 回读。重复 marker/key、非法 Base64、坏 digest、同 H 错 scope/path/config/closure、未知 schema 或身份冲突直接失败，不自动覆盖。

受限回读只访问 E 指定 repo/PR：GET live PR 并核对 OPEN、未合并、源/目标仓库 ID/分支、UID、live head=H；验证 Q 的 H/S/UID/D 和事件 B 的可信配置；再次 GET 确认机器块 fingerprint 与源身份不变，然后原子冻结 selected 文件。普通说明文字变化不影响 fingerprint。默认总预算 45 秒、最多三轮双读、每读 5 秒、退避 2/5 秒；权限错误不重试，网络/限流仅预算内重试。预算是设计值，不是实测。所有后续消费者只读冻结文件。

<a id="des-cip-03"></a>

### DES-CIP-03：恢复不漂移候选身份

H=事件 source head；B=事件 base commit；S=验证过的 merge-base(B,H)；M=原 run 固定 checkout/GITHUB_SHA；T=实际 `M^{tree}`；W=可信 workflow 定义；D=实际消费的 canonical 投影 digest；R/A=GitHub run/attempt。重跑不得把 H、B、M、T 或 W 换成 live ref，不要求 live base 恰等于 B。merge 父关系不一致交既有 tree/integration verifier，不借投影修复掩盖；最新 main 的集成复验仍由既有风险门禁决定。

<a id="des-cip-04"></a>

### DES-CIP-04：attempt 证据闭合

source projection v2 语义不变；外层 proposed `oasis7-required-plan-v2` envelope 包含 repo/PR/UID、H/B/S/M/T/W、R/A、publication ID、D、selected 文件 SHA-256、origin、planner/run selectors。artifact 名为 `oasis7-required-plan-v2-<R>-a<A>` 和诊断用 `oasis7-projection-resolution-v1-<R>-a<A>`；plan archive 成员固定为 `oasis7-required-plan-v2.json`、`selected-impact-projection.json`，`overwrite=false`。诊断 artifact 不授予通过。

Receipt 按 R/A jobs API 精确取得 required-gate job/check ID、可信 app/workflow、成功状态和实际所选测试执行证据；按同 A 选 plan、Cargo profile 及所有启用的结果/receipt artifact，核对 D 和候选身份。有新协议标识而缺 v2 必须失败，不 fallback v1；legacy v1 仅按旧单成员合同读取。A2 排队、失败或不确定时不得复用 A1 success；live source/review/publication 义务改变时也不得复用旧证据。

<a id="des-cip-05"></a>

### DES-CIP-05：同 head 元数据恢复

只在目标 run 实际支持新 resolver、可信诊断确认失败仅在投影获取/校验且测试未开始、live H 仍等于 run H、P(H) 已回读且 B/S/config 仍适用时，使用现有 `gh run rerun <R> -R eng-cc/oasis7` 整轮重跑，不能只重跑 planner/job。repair key 为 SHA-256(repo, PR, H, publication ID, R, previous A, reason)；每键最多一次请求，丢失响应先查 run attempt，仍不确定记 UNCERTAIN。H2 取代 H1 则 SUPERSEDED，不退回 H1；真正测试失败走普通修复。

## 6. 接口与数据合同

<a id="des-cip-06"></a>

### DES-CIP-06：Proposed 接口与错误

保留现有 `prepare-task-pr.sh --draft-candidate --create --impact-projection ... --review-change-class ...`；新增 `--resume-publication <id> --json` 和 `--recover-ci <run-id> --impact-projection ... --review-change-class ... --json`，实施前不可调用。helper 自行解析 UID/PR，并验证传入文件匹配候选，不能成为任意 run rerun 入口。

Proposed `oasis7-ci-publication/v1` 写入既有 Task evidence 通道并回读，而非另立台账：canonical repository/repository ID、task UID/合法 bootstrap epoch（legacy 显式 null）、源仓库 ID/源与目标分支、H/S、固定 planner authority OID/配置 SHA-256、D、`projection_required=true`、publication ID。ID 为以上身份字段 canonical JSON 的 SHA-256，不用时间戳；新 PR 创建前 number 可缺，创建后须经现有 reciprocal record-pr 对齐。同 H 不同 D 须由 Task 显式失效/重新冻结，不能选“最后一条”。实施时必须确定既有 evidence parser 的扩展及历史兼容。

| Proposed code | 默认处置 |
| --- | --- |
| `EVENT_PROJECTION_STALE_HEAD`、`PROJECTION_PUBLICATION_PENDING` | 仅在限定上下文有界回读；超时失败 |
| `EVENT_PROJECTION_INVALID`、`SCOPE_OR_AUTHORITY_MISMATCH`、`TASK_IDENTITY_CONFLICT` | 阻断，不能靠最新 live body 覆盖 |
| `SOURCE_SUPERSEDED`、`PUBLICATION_WRITE_CONFLICT` | 停止旧候选或有界重读，不盲写 |
| `UNSUPPORTED_RUN_PROTOCOL`、`RUN_ATTEMPT_MISMATCH` | 不无效重跑、不混合 artifact |
| `NETWORK_UNCERTAIN` | 记录 intent，先远端回读，不宣告成功 |

保留现有易检索的错误前缀，新增结构化 JSON diagnostic；恢复按 code 和验证上下文判断，不能用报错文本正则。PR body 用 event 文件/环境变量读取，拒绝重复 JSON key/marker、恶意路径及超限输入；初始预算 raw projection 32 KiB、UTF-8 body 60 KiB，超限报错不截断。

## 7. 状态、事务与持久化

<a id="des-cip-07"></a>

### DES-CIP-07：可恢复的有序提交

Git ref 与 PR body PATCH 非原子。journal 位于 `<git-common-dir>/oasis7/pr-publication/<repository-and-branch-hash>/<publication_id>/journal.json`，本机同 common-dir 分支级 flock 串行；不是跨机器锁。阶段 `PREPARED -> METADATA_CONFIRMED -> HEAD_CONFIRMED -> RUN_OBSERVED`，另有 `SUPERSEDED/CONFLICT/UNCERTAIN` disposition；这些都不是 GitHub task 状态或 ready/done。每次副作用前持久记录 intent，回读后记录 observation；临时文件 fsync/rename、当前用户权限、禁止保存 token。journal 丢失时只从 GitHub/refs 重建事实。

P(H1) 已 PATCH 而远端仍 H0，重验 H0 才能 push；push 响应丢失查询 ref，等 H1 续接、等 H0 才考虑重试、H2 冲突；create 响应丢失按精确身份找 PR；run 未观察到不制造空提交；recover 响应丢失查 attempt。PR 合并/关闭立即停止，不把未合入 H1 标为 done。PR body 只有读-改-回读，不具备 Git lease 等价 CAS；支持单 canonical publisher，人工并发 body 修改或多机无损并发不承诺，fingerprint 只是冲突探测。

## 8. 部署、安全与运行约束

CI 保留 read 权限，不增 `pull-requests: write`、`contents: write` 或 `actions: write`；手动本地 executor 使用既有 gh 权限发起 rerun。resolver、verifier、planner 和执行依赖从事件 B 提取至 runner temp，以 `python3 -I` 执行；候选同名模块不可 import。新 protocol/capability 输出用于核对实际运行 W；未支持协议的旧 run 不 rerun。错误投影必须使 required-gate 失败，不使用 skip/continue-on-error 掩盖；GitHub skipped job 的报告不能充当失败证明。权限、网络与 GitHub API 不确定时 fail closed。

## 9. 质量与容量

以下均为设计验收目标，非实测。正常路径在 fake GitHub 100 次 H0→H1 及 hosted 样本中事件 H/D 一致率 100%，且每次只需一轮 required-gate；有效事件快速路径不新增 PR GET。晚发布在 45 秒可见性预算内冻结同 H 的 Q，或有界失败后最多一次同 run 恢复；每一远端副作用 crash 后不重复 PR、不覆盖 H2、不无限重跑。A1/A2 混合接纳为零。恶意/超限输入在 toolchain 安装前拒绝，无候选代码执行。macOS publisher、Ubuntu hosted CI 分别验证 shell、base64、journal/lock 行为；真实网络可用性与跨机器并发未承诺。

## 10. 兼容、迁移与回滚

| 叶子 | PR/交付 | 依赖与退出条件 |
| --- | --- | --- |
| S1 | system 文档 PR：本设计、专业验收、CLI/错误/验证合同 | 独立 review，冻结实际文档提交和 Task 引用 |
| C1 | code PR：解析、resolver/publisher 核心、binding、v2 双读、fake tests | 消费 S1；先固定跨叶接口，不启用正式 emitter |
| C2 | code PR：可信 workflow resolver、attempt-bound 所有消费者与 receipt | C1 在可信 main；consumer first，emitter 后启用 |
| C3 | code PR：prepare helper 发布/journal/resume/recover | C2 实际支持协议；旧 workflow 不开放 recover |
| V1 | 现有验证 task：hosted 新建/续更/晚投影/重跑/负例 | C1-C3 冻结组合，记录 H/B/M/T/W/R/A/D |
| S2 | system/doc PR：manual 和 skill 操作说明 | V1 证据，不能以合并代替能力证明 |

这些是实施叶子标签，不伪造任务编号。当前 PM helper 是单 PR projection；同一 Task UID 第二个 PR 必须 fail closed，TPM 应以 coordinating Issue 和**有序 linked delivery tasks**映射这些叶子，每个 delivery task 各自有单 PR 主链，协调任务须等 required delivery 全部合并后才能完成；不能仅凭本表绕过 ordered multi-PR contract。所有 loop 手动触发，文档与代码 PR 分离。

拟新增 `scripts/pm/projection_publication_contract.py`、`pr_projection_resolver.py`、`pr_projection_publication.py` 及对应测试；拟修改 `scripts/prepare-task-pr.sh`、`.github/workflows/rust.yml`、`scripts/pm/workflow-impact-projection.py`、`scripts/plan-rust-required-scope.py`、`scripts/pm/ci-ready-receipt.py`、`ci_ready_receipt_identity.py`。实施前还需审计 PLAN_ARTIFACT/PLAN_MEMBER、Cargo profile、lifecycle/closeout 的全部间接消费者。legacy v1 仅按旧规则读取，缺新 resolver 的旧 run 返回 `UNSUPPORTED_RUN_PROTOCOL`；旧失败 PR 要有真实新 workflow 事件，必要时经现有任务授权一次 close/reopen 并证明新 W/B，绝非日常 recovery。回滚只能在新的可信基线禁用 live 读取，保留有序发布、严格校验、v2 reader 和历史 artifact；在途 run 行为仍由其 W/B 决定。

## 11. 验证设计与可追溯性

<a id="des-cip-08"></a>

### DES-CIP-08：事件捕获和 hosted 证据

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [workflow source: PR source identity](source-of-truth.md#split-source-review-integration-contract) | [DES-CIP-01](ci-projection-publication.design.md#des-cip-01) | 已有 PR H0→H1 和新建 PR 发布时，事件含对应 P(H1) | [prepare helper review-risk test](../../../scripts/pm/prepare-task-pr-review-risk.test.py)：现存 helper 回归入口；C3 增加 fake push 时刻捕获及新建/续更用例，macOS 本地和 Ubuntu hosted | C3 测试输出和 V1 Issue #3871 hosted H/D 记录 | 当前现存测试不证明新时序 |
| [workflow source: PR CI receipt](source-of-truth.md#split-source-review-integration-contract) | [DES-CIP-02](ci-projection-publication.design.md#des-cip-02) | 事件准确时不取 live，过期时只允许同身份恢复，坏输入拒绝 | [impact projection tests](../../../scripts/pm/workflow-impact-projection.test.py)：C1 增加 resolver stale/live/invalid 负例；固定事件 B runner 与 hosted V1 | C1 测试和 V1 解析来源、H/S/D 记录 | 现存测试不证明 proposed live resolver |
| [workflow source: source/integration distinction](source-of-truth.md#split-source-review-integration-contract) | [DES-CIP-03](ci-projection-publication.design.md#des-cip-03) | 恢复时 H/B/M/T/W 不漂移 | [receipt identity tests](../../../scripts/pm/ci-ready-receipt.test.py)：C2 增加原 run rerun 后源、checkout、workflow 对比；hosted 记录 H/B/M/T/W/R/A | C2 负例和 V1 身份矩阵 | 不声称 T 测过最新 main |
| [workflow source: attempt-bound integration CI](source-of-truth.md#split-source-review-integration-contract) | [DES-CIP-04](ci-projection-publication.design.md#des-cip-04) | A2 不取 A1 artifact、check 或成功 | [receipt tests](../../../scripts/pm/ci-ready-receipt.test.py)：C2 增加 A1/A2 混合、缺 v2、重复产物负例；Ubuntu hosted | C2 负例与 V1 R/A/check/artifact receipt | 当前固定名称 artifact 不具备隔离 |
| [workflow source: run/attempt and task gate](source-of-truth.md#split-source-review-integration-contract)；Issue `https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462` item 2，typed `professional_acceptance` required | [DES-CIP-05](ci-projection-publication.design.md#des-cip-05) | 可信 W/capability 和结构化诊断证明仅投影失败、测试未开始；同 R 整轮 A2，丢响应单次 repair key 不重复请求；旧 W、H2、真实测试失败均拒绝 | [prepare helper review-risk test](../../../scripts/pm/prepare-task-pr-review-risk.test.py)：C3 proposed fake `recover_ci_trusted_pretest_only`, `rerun_entire_workflow_same_run`, `lost_response_queries_attempt_once`, `legacy_protocol_denied`；fake API 断言 R/A 与所有 selected jobs，V1 Ubuntu hosted 同 R/A2 | C3 命令输出、诊断/capability、fake API log；V1 hosted run/jobs/attempt 与拒绝 evidence 写入 Issue #3871 | 当前测试未实现，CLI 不可运行；fake 不证明 hosted |
| [writing standard: interface and trust](../doc-governance/system-design-writing-standard.design.md#6-接口与数据合同)；Issue `https://github.com/eng-cc/oasis7/issues/3871#issuecomment-5751018462` items 1/2，typed `professional_acceptance` required | [DES-CIP-06](ci-projection-publication.design.md#des-cip-06) | binding parser/readback 精确匹配 UID/H/S/D/config、同 H 不同 D 不任取；重复 marker/key、超 32/60 KiB、恶意路径/同名候选模块在 toolchain 前拒绝 | [impact projection tests](../../../scripts/pm/workflow-impact-projection.test.py)：C1 proposed fake `binding_roundtrip_and_conflict`, `duplicate_marker_and_json_key`, `oversize_and_hostile_input_pretoolchain`；C2 runner 安装 toolchain 前哨兵和可信 base import 负例，V1 Ubuntu hosted 一例坏输入拒绝 | C1/C2 parser 负例、拒绝阶段日志和 V1 Issue #3871 hosted 记录 | 当前 parser/协议未实现；本地负例不证明 hosted |
| [writing standard: key flows](../doc-governance/system-design-writing-standard.design.md#5-关键运行流程) | [DES-CIP-07](ci-projection-publication.design.md#des-cip-07) | 每一远端副作用后 crash 能回读续接且不覆盖 H2 | [prepare helper review-risk test](../../../scripts/pm/prepare-task-pr-review-risk.test.py)：C3 扩展 crash/lease matrix，在 macOS publisher 验证 | C3 crash matrix 和 V1 恢复日志 | 不证明跨机器锁或 body CAS |
| [writing standard: verification](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | [DES-CIP-08](ci-projection-publication.design.md#des-cip-08) | fake 与 hosted 验证分层，坏投影必须拒绝 | [workflow impact consumers tests](../../../scripts/pm/workflow-impact-consumers.test.py)：C1/C2 回归并扩展负例；V1 Ubuntu hosted 新建/续更/晚投影/重跑/坏投影 | 各 leaf 的 test output 与 Issue #3871 hosted evidence | 本文及本地 fixture 不构成 QA 放行 |

| Proposed test / source | obligation | 方法与证据目标 | 未证明范围 |
| --- | --- | --- | --- |
| `projection-publication.test.py`: existing/create/crash/lease | CIP-01/07 | fake push 瞬间捕获 P(H1)；create 超时回读；每副作用后 crash；H2 拒绝 | fake 不证明真实 GitHub 或 body CAS |
| `pr-projection-resolver.test.py`: event/live/invalid | CIP-02/03 | 有效 P0 不取未来 P1；过期事件只取同 H/S/D；live H2、坏 digest/同 H 错配置拒绝 | 不证明最新 main 集成复验 |
| `ci-projection-publication.integration.test.py`: attempts/recover/missing/trust | CIP-02/04/05/08 | A1/A2 不混、缺 v2 不 fallback；整轮 attempt2；删 marker 不降级；候选恶意模块不执行 | fake 不证明 hosted |

C1/C2 后运行这三个 proposed Python 测试，并复跑现存 `bash scripts/plan-rust-required-scope.test.sh`、`python3 scripts/pm/workflow-impact-consumers.test.py`、`python3 scripts/pm/ci-ready-receipt.test.py`（执行前核对冻结候选路径）。Hosted V1 必须包含新建 PR、已有 PR H0→H1、晚投影、同 run A2 和坏投影拒绝；记录真实 source/integration/tested tree、workflow、check/app/artifact/projection 身份。V1 未完成前只写“实现已合入，hosted 验证待完成”，不宣告缺陷根治。

## 12. 决策、长期风险与未决问题

采用有序发布、严格限定 live resolver、同 run 新 attempt；仅 `gh pr edit` 不解决 push 时序与恢复；`edited` 自动跑会触发 pre-push 无用候选；无条件读最新 body 可能将 H2 配给 H1；放宽 head、删 marker、空提交恢复均破坏证据身份。close/reopen 仅是旧 run 授权迁移例外。若未来需要跨机器并发，再单独设计不可变发布存储或真正 CAS。

残余风险及复核触发：body PATCH 非原子（出现并发 publisher 时）、GitHub API 可用性（hosted 恢复失败时）、旧 run 无法原地升级（迁移请求时）、同 H 义务变更（Task revision 变化时）。repository health 跟踪语义与债务，QA 独立判断放行，TPM 在 Issue 记录 owner、延期与触发。最终专业验收必须同时证明标准 H/P 配对、晚发布同 H/B/M 恢复、坏证据阻断、全量 attempt 证据闭合，且没有新增服务、自动 loop、空提交或日常 PR close/reopen 依赖。
