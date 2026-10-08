# Document Corpus Inventory v3：逐对象分片与无共享汇总写入设计

- 文档状态：proposed；本文定义待实施合同，不表示代码、迁移、CI 或权限已经生效。
- 建议落库路径：`doc/engineering/doc-governance/document-corpus-inventory-v3.design.md`。
- Canonical repository：`eng-cc/oasis7`。
- Owner role：`repository_health_engineer`；共同审读：`qa_engineer`，涉及已有语义结论时加入原 `decision_owner` / `domain_owner`。
- 审读日期：2026-09-30。
- 审读基线：`0bcb10dfa6874884c5879800e6c8836e7430f8fe`；这是本次代码观察的快照，不是要求后续任务永远绑定的 policy commit，也不要求每次 main 前进修改本文。
- 当前入口：[doc-governance README](README.md)。写作约束：[系统设计写作规范](system-design-writing-standard.design.md)。权限、任务及启用权威：[workflow source of truth](../workflow/source-of-truth.md)。
- 产品 AC：本次不改变玩家承诺、产品范围或游戏规则；实施任务使用 `professional_acceptance`，并按现行 traceability 合同填写 product requirement 的显式 N/A。不得同时把专业上游和产品上游都省略。
- 新命令、schema、模块及测试场景均为本设计新增接口；除明确标注为现有的命令外，不得当作当前仓库已经支持的功能。

## 1. 问题、目标与非目标

### 1.1 问题

现有 `doc/.governance/document-corpus-inventory.json` 是全量派生总表。普通文档的内容 SHA、语义复核总表的整文件 SHA、证据清单的整文件 SHA 和数量集中在同一文件。不同任务即使没有修改同一份业务文档，也会写入同一份总表，部分情况下还会修改同一行父级摘要。

只按四个产品模块或顶层目录拆成少量 JSON，仍会在同模块并发、公共 controls、证据委托摘要上竞争。将 JSON 改成 JSONL、使用 union merge、给每次生成加时间戳，均不解决共享写入根因。

### 1.2 目标与硬约束

<a id="DCI-01"></a>
**DCI-01：独立变更不共享清单写入。** 在规则未变化，且两个任务不共享源文档、语义 bundle 或证据原子组时，它们的 inventory 写集合必须不相交。源文档自身、真实共同 authority 和真实原子组的冲突不在消除范围内。

<a id="DCI-02"></a>
**DCI-02：父级静态。** 日常新增、修改、删除源对象不改根入口、委托入口、全局计数、全量文件列表或总摘要。只有 schema、固定集合位置及治理边界改变时才更新入口。

<a id="DCI-03"></a>
**DCI-03：不丢治理能力。** 保留全量覆盖、重复检测、owner/authority 路由、内容漂移、语义复核绑定、证据窗口和组完整性检查；不得把自动同步解释为审批。

<a id="DCI-04"></a>
**DCI-04：迁移无损且只切换一次。** 切换提交同时包含新读写器、转换数据、调用方及测试；正常运行不双读双写。旧格式只允许进入显式迁移工具。

### 1.3 非目标

不引入数据库、云服务、调度器、机器人自动回写 main、自定义 Git merge driver、跨 worktree 全局锁、新 PM 台账或审批系统。不改变四模块产品边界、既有 domain owner、证据的新鲜性定义以及现有 release/merge gate。此次不顺便重命名存量业务文档，不重写已完成的语义复核结论。

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | 独立源对象使用独立机器记录；任务证据仍是任务事实 | [DCI-01](#DCI-01) | repository_health_engineer；原始用户需求须在实施任务中回读绑定 | 不重构其他 README/index 热点 |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | 新增、修改或删除对象不更新动态父级汇总 | [DCI-02](#DCI-02) | repository_health_engineer；原始用户需求须在实施任务中回读绑定 | 不重构其他 README/index 热点 |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | 保留覆盖、复核、路由和证据检查；同步不能代表审批 | [DCI-03](#DCI-03) | repository_health_engineer、qa_engineer | 不改变既有 domain review 权限 |
| [目录登记](doc-structure-standard.design.md#35-一级物理目录-registry-与例外) | 保留结构登记与目录覆盖；治理分片不得制造递归登记或盲区 | [DCI-05](#DCI-05) | repository_health_engineer；现有 registry | 不增加产品模块 |
| [证据生命周期](doc-structure-standard.design.md#36-证据生命周期) | QA 保留对已有证据分类、留存和处置决定的权限 | [DCI-06](#DCI-06) | qa_engineer；原 decision/domain owner | 结构通过不等于当前验收通过 |
| [证据生命周期](doc-structure-standard.design.md#36-证据生命周期) | 保留证据字段、窗口、冻结 cohort 和组边界 | [DCI-07](#DCI-07) | qa_engineer；原 decision/domain owner | 不重签历史结论或声明 fresh run |
| [状态与持久化](system-design-writing-standard.design.md#7-状态、事务与持久化) | 不新增治理状态；工具操作状态只描述本地过程 | [DCI-08](#DCI-08) | repository_health_engineer；Git 工作树 | 不创建任务或审批状态 |
| [状态与持久化](system-design-writing-standard.design.md#7-状态、事务与持久化) | 明确局部写入、并发、失败恢复及不可逆边界 | [DCI-09](#DCI-09) | repository_health_engineer；Git 工作树 | 不声称文件系统跨文件事务 |
| [兼容与迁移](system-design-writing-standard.design.md#10-兼容、迁移与回滚) | 一次切换必须同时提供读写器、数据和消费者 | [DCI-04](#DCI-04) | repository_health_engineer、qa_engineer | 不在 main 保留双读双写 |
| [兼容与迁移](system-design-writing-standard.design.md#10-兼容、迁移与回滚) | 旧新身份映射、无损证明、受控适配和有界回滚 | [DCI-12](#DCI-12) | repository_health_engineer、qa_engineer | 不覆盖未声明的仓库外消费者 |

### 2.2 审读与权限

RH 负责存储、路径和覆盖合同；QA 负责证据规则和负向验收；已有语义记录的 owner 负责该记录内容变化的重新审读。工具只能计算、生成候选、写入明确选中的记录和检查一致性，不能通过 `--owner`、本地 reason、作者自报 digest 或 CLI 成功替代有效 review。

实际修改、源与测试版本、迁移结果和回滚决定记录在 PR 与真实验证输出中。本文只维护长期技术设计，不增加任务身份或批准凭据。

## 3. 当前状态、目标状态与差距

| 对象/能力 | 当前状态（上述审读基线） | 目标状态 | 差距/假设 | 证据或 owner |
| --- | --- | --- | --- | --- |
| Corpus | v2；`generated()` 全量生成，比较整个 `objects` 数组；父级保存 controls/delegate 摘要 | 静态 v3 入口＋每源对象一片 | 修改扫描、加载、写入及控制文件识别 | `scripts/document-corpus-inventory-check.py` |
| Semantic | v1 `entries` / `bundles` 单体表；绑定内容 SHA | 逐文件 entry＋逐真实集合 bundle | 不得机械重签复核 SHA | 同上 `check_semantic_overlay()` |
| Evidence | v1 单体 entries；部分字段来自生成器；有固定数量、triad 精确边界和五个窗口组 | 独立 entry / 原子 group；冻结基线与当前集合分开验证 | 不能简单删除数量断言或对新增对象沿用全局定额 | `scripts/doc-evidence-inventory-check.py` |
| CI | 有 baseline / doc_checker_contracts / workflow_governance；未知路径 full | 接入真实校验和对应回归，不误触无关重型能力 | 搜索到文件引用不等于已确认所有入口执行；实施时做调用链清点 | scope 配置、runner、capability inventory |
| 已有测试 | 覆盖 duplicate、delegate、registry、内容漂移、语义和产品边界负例 | 保留负例，并增加真实 Git 分支合并、迁移及 loop 测试 | 本次未执行这些仓库测试 | 两个现有 inventory `*.test.py` |

审读发现决定两个实施要求：不能只拆 `objects`；不能忽略分片与 product/system/code 路径所有权的冲突。用户反馈证明存在冲突需求，但本设计没有把未经统计的冲突次数或提速百分比写成事实。

## 4. 边界与结构

### 4.1 目标目录

```text
doc/.governance/
  document-corpus-inventory.json                # v3 静态入口
  document-corpus/
    objects/<hh>/<path-key>.json                # 普通源对象
    semantic/entries/<hh>/<path-key>.json        # 已复核单对象
    semantic/bundles/<bundle-id>.json           # 已复核真实集合
    evidence/entries/<hh>/<path-key>.json        # 非原子组证据
    evidence/groups/<group-id>.json             # 完整原子组

doc/testing/evidence/inventory.json             # v2 静态委托入口
scripts/document_corpus.py                     # 新增：公共模型、读取、验证、写入
scripts/document_evidence_policy.py             # 新增：由旧证据 checker 抽取的规则
scripts/document-corpus-inventory.py            # 新增：管理 CLI
scripts/document-corpus-inventory-check.py      # 保留命令，切换公共模型
scripts/doc-evidence-inventory-check.py         # 保留命令，切换公共模型
scripts/fixtures/document-corpus-v3/legacy/     # 新增：不可变旧格式输入与历史基线
.cache/doc-governance/                          # 显式忽略；报告、候选及导出
```

旧 `document-semantic-review-overrides.json` 在原子切换中删除。它的有效引用改为静态入口及 `locate`；不保留第二份活跃 JSON，也不放一个形似旧 schema 的跳转壳。

目录仅用于查找记录，不要求提交空目录占位文件。不维护 `<hh>/index.json`、根对象列表或文件数；空集合允许物理目录不存在。存在的未知文件必须报错，不使用 `.gitkeep` 冒充业务记录。

### 4.2 公共模块及信任边界

CLI 和两个 checker 使用同一个 `CorpusModel`，禁止各自实现一套拼接与去重。现有 evidence 生成、专用 triad/窗口/导航断言抽到 `document_evidence_policy.py`；checker 不再通过 subprocess 递归运行另一个全量 checker。

`CorpusModel` 至少含 `objects_by_path`、`semantic_entries_by_path`、`semantic_bundles_by_id`、`evidence_entries_by_path`、`evidence_groups_by_id` 和只读的 `path_to_group` / `path_to_bundle`。后两者只存在内存，不落库。

Trusted task admission loads shared parsing/classification only from the binding's exact immutable effective-tool-root commit and verifies module/import-closure bytes there. `validate_tool_root()` protects the shared module and imports and rejects tracked drift, target/untracked import shadow, symlink substitution and path escape; cold imports cannot write bytecode into the trusted source tree. It is unsafe to guard only `scripts/pm` and then import `scripts/document_corpus.py` from a candidate checkout. This restriction is for trusted admission: candidate-local checkers, tests and CI may execute code under test, but their results are not admission authority.

### 4.3 全量覆盖与元数据自递归

<a id="DCI-05"></a>
定义 `A` 为当前检查快照的全部 `doc/**` 文件，`M` 为两个合法静态入口及新分片存储中的合法元数据，`E` 为 `doc/testing/evidence/**` 下除精确路径 `doc/testing/evidence/inventory.json` 外的源文件，`D = A - M - E`。

必须满足：`D` 与 objects 的路径集合完全相等；`E` 与独立证据及组成员的路径并集完全相等；`D ∩ E = ∅`。Semantic 是对 `D ∪ E` 的可选复核层，不替代 coverage，也不要求每个文档都有语义复核记录。

`M` 不能仅凭“位于隐藏目录”认定合法。逐个文件验证其 schema、位置、key、对象身份和引用；未知文件、重复 key、非法路径、临时文件和孤儿记录都失败。`.governance` 中现有 registry/allowlist 仍是普通直接管理对象；不忽略整个 `.governance/**`。

源路径不得指向 `M`。分片不能给自身、其他分片或入口再生成 object。源 evidence 下子目录中另一个名为 `inventory.json` 的文件仍是证据源；禁止旧式按 basename 排除所有同名文件。

## 5. 关键运行流程

### 5.1 修改、增加与删除普通文档

作者修改源文件后运行显式 `sync --path`，工具读取源与 registry，计算期望 object，只重写实际变化的分片。已复核源发生变化时，机器同步只更新 object，随后 check 必须返回语义复核漂移，不能把 semantic SHA 顺手更新。

新增源没有旧 object 时创建一个；新文件不会自动取得语义批准。删除使用独立的 `--deleted-path`，必须确认源确实不存在才删除 object；已有 semantic、bundle 或 evidence 记录不得被普通 sync 自动删除。rename 显式提交旧端点删除、新端点新增；semantic 身份与 authority/backlink 修复另走复核，不猜测重命名。

默认只处理请求集合。`sync --all` 仅用于显式全库维护或已批准的规则变化，不作为每个任务的例行操作；即使全扫，也只写字节不同的对象，不改父级。

### 5.2 Semantic 与 evidence 复核

<a id="DCI-06"></a>
管理 CLI 的 `propose` 生成候选，不改 tracked 文件。候选保留原处置、owner、authority、note、retirement gate 及组成员；工具不得自动升级处置。审读者在现有 workflow 中确认内容与目标 SHA 后，显式 `apply-review` 写入被选记录。

候选必须绑定每个受影响源的内容 SHA、已有记录的精确字节 SHA/不存在标记、目标记录位置和拟写完整内容。apply 前重新读取；任一源、原记录或组成员改变，整体拒绝写入。候选 JSON 中的 owner 或 approval 文本不能证明权限；有效 review 和允许范围仍由现行 task/review gate 核验。

只更新复核结论而未修改源也允许，但必须显式声明该源的复核工作范围。工具成功只证明候选已按相同输入写入，不证明 owner 已批准，更不证明 evidence 现在有效。

### 5.3 并发合并

不同文档或不同原子组写不同文件，由 Git 正常合并。两个分支同改一份文档、同一复核记录或同一组时，保留冲突/复核阻断。即使 Git 自动拼好了业务文本，合并后的源 SHA 也必须重新匹配 object 与 semantic/evidence；不得因两个分支单独为绿就跳过候选合并树检查。

没有共同源/组但共享 registry 规则变更，不属于独立任务：规则变化可合法影响多个 object。写放大必须由任务声明，不用根总表掩盖。

### 5.4 检查与报告

`check` 先验证 schema/安全路径，再建立内存索引，随后做覆盖、逐对象、语义和证据检查。每次读取的源内容摘要缓存于该次运行，避免重复哈希。

`check --revision <OID>` 从单一 Git commit/tree 读取，不消费混合工作树；`check --worktree` 检查显式工作树，运行中检测集合/被读内容变化并报并发修改。现有无参数 checker 保留工作树语义。报告声明检查模式与快照，不能把工作树结果当成已提交或已合并结果。

全量 export 是衍生视图，只能输出 stdout 或显式报告路径，不得输出到 `doc/**`、tracked 文件或源存储目录。导出包含对象摘要、统计、控制文件摘要和可选快照 OID，但不反向成为正常 checker 的输入。

## 6. 接口与数据合同

### 6.1 路径身份和序列化

`path-key = sha256(repo_relative_posix_path.encode("utf-8")).hexdigest()`；`hh = path-key[:2]`。完整 64 位小写十六进制，不截短，不使用内容 SHA、owner、模块名或全局递增号命名。记录内保留完整可读 `path`。

路径必须为仓库相对 POSIX 路径；拒绝绝对路径、盘符、反斜杠、空段、`.`/`..`、控制字符、非 UTF-8 和符号链接/子模块等非普通源文件模式。不要静默 lower-case、裁剪或 Unicode 归一化；检测跨平台 casefold/规范化别名冲突并拒绝。bundle/group ID 限小写字母、数字、中划线，长度 1–96；存量不符合时迁移报错并由 owner 明确映射。

JSON 采用 UTF-8、两空格、LF、结尾一个 LF，固定字段排序；拒绝重复键、NaN/Infinity、错误类型及未声明字段。版本不支持立即失败，不猜测兼容。普通记录的 schema 包装与 `record` 分离，迁移保留原字段值。

源内容 SHA 保留现有含义：原始内容字节的 SHA-256，不进行 trim、Markdown 格式化或换行折叠。Git revision 模式读 blob；工作树模式读实际字节。生成后 staged blob 字节必须一致；受 autocrlf/filter 影响而不一致时报告 `hash-input-mismatch`，不得自动改复核 SHA。只为新元数据设置明确 LF 规则，不批量重写历史源文件换行。

### 6.2 静态 Corpus 入口

下面是完整的目标入口 schema 示例；路径和值固定，日常同步不能修改。

```json
{
  "version": 3,
  "scope": "all doc files through direct objects, delegated evidence objects, and controls",
  "decision_boundary": "routing is conservative heuristic input only",
  "product_modules": [
    "agents-world-simulation",
    "player-entry-distribution",
    "world-infrastructure",
    "world-rules-core-gameplay"
  ],
  "objects_root": "doc/.governance/document-corpus/objects",
  "semantic_entries_root": "doc/.governance/document-corpus/semantic/entries",
  "semantic_bundles_root": "doc/.governance/document-corpus/semantic/bundles",
  "delegates": [
    {
      "kind": "testing-evidence",
      "path": "doc/testing/evidence/inventory.json",
      "expected_version": 2
    }
  ]
}
```

移除动态 `objects`、`controls`、delegate `sha256` 和 `object_count`。`kind` 对应程序内白名单验证器，不能在 JSON 中携带要执行的 shell command。各 root 必须等于本版本允许的路径，不允许通过修改入口指向另一套弱约束数据。

### 6.3 Object 与 semantic entry

| 记录 | 包装 schema | `record` 必需字段 | 额外约束 |
| --- | --- | --- | --- |
| object | `oasis7.document-corpus-object/v1` | 原有 `path`、`content_sha256`、`registry_type`、`structural_owner`、`authority_entry`、`authority_layer`、`semantic_decision_owner`、`object_kind`、`lifecycle_candidate`、`inventory_disposition`、`routing_batch`、`routing_note` | 与同快照的既有生成规则逐字段相等 |
| semantic entry | `oasis7.document-semantic-entry/v1` | `path`、`content_sha256`、`disposition`、`decision_owner`、`current_authority`、`note` | 复用既有处置/owner 枚举；原 `retirement_gate` 作为可选字段保留；outstanding obligation 必填 |
| semantic bundle | `oasis7.document-semantic-bundle/v1` | 原 bundle 的 `id`、`paths`、`content_set_sha256`、`disposition`、`decision_owner`、`current_authority`、`note` | 一 bundle 一文件；原合法可选字段显式列入 schema |

以下为语法有效的说明性 entry，SHA 是占位值，不是实测值，不能用于迁移：

```json
{
  "schema": "oasis7.document-semantic-entry/v1",
  "record": {
    "path": "doc/world-runtime/design.md",
    "content_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "disposition": "retain_current_authority",
    "decision_owner": "runtime_engineer",
    "current_authority": "doc/world-runtime/prd.md",
    "note": "Schema example only; not review evidence."
  }
}
```

Semantic entry 的 path 与所有 bundle paths 必须互斥；bundle paths 必须排序、非空且无重复。集合 digest 保留旧算法：按 path 排序，连接 `path UTF-8 + NUL + sha256(source bytes) ASCII + LF` 后再算 SHA-256。禁止把整个 semantic 分片目录摘要存回根入口。

### 6.4 Evidence 入口与分片

`doc/testing/evidence/inventory.json` 的完整目标字段为 `version=2`、原 `scope`、固定 `entries_root`、固定 `groups_root`、`legacy_baseline_id`。roots 分别指向 §4.1 的 evidence 两个集合；`legacy_baseline_id` 固定为 `document-evidence-v1-at-v3-migration`，程序映射到版本化 legacy fixture，不能提供任意可执行规则位置。该入口没有 live 数量、live SHA、生成时间或最新 HEAD。

独立证据包装为 `schema=oasis7.document-evidence-entry/v1` 加 `record`，保留旧 entry 的全部字段。原 `REQUIRED_FIELDS`、枚举和有条件字段继续生效，合法可选字段由实施时的完整旧数据扫描＋专用规则穷举进入 schema，不得用无条件 `additionalProperties=true` 掩盖漏迁字段。

原子组包装为 `schema=oasis7.document-evidence-group/v1`，含 `id`、`group_sha256`、`entries`。成员保留旧字段但移除重复的 `atomic_group`、`group_sha256`；内存展开时把组 id 与 digest 注回每个成员。成员按 path 排序，不能在独立 entries 中重复出现。任何 `WINDOW_OBSERVATION` 都必须属于非空原子组。

组 digest 保留旧算法：按 path 排序，连接 `content_sha256 + "  " + path + LF` 的 UTF-8 字节后算 SHA-256。这与 semantic bundle 的算法不同，禁止为了统一代码而悄悄替换。

### 6.5 冻结证据基线与日常集合

<a id="DCI-07"></a>
旧 evidence 清单、旧 snapshot/freshness 字段和专用断言的成员集合保存在 `scripts/fixtures/document-corpus-v3/legacy/` 的不可变输入中；实际迁移 SHA、报告身份记在任务证据，不写进站立 workflow authority。fixture 不作为当前证据对象扫描，不随文档编辑刷新。

现有 `EXPECTED_SNAPSHOT`、`EXPECTED_LIFECYCLE_COUNTS`、legacy-source/batch 数量、五组窗口数量等，按以下规则拆解而非删除：

| 旧约束 | 新位置/检查范围 | 不允许的替代 |
| --- | --- | --- |
| 旧 snapshot 与 freshness 字节 | legacy fixture 和迁移等价性检查；保留历史含义 | 把旧 snapshot 改为每次 main HEAD |
| 冻结集合的 lifecycle/batch 计数 | 仅对 fixture 中明确的原始 path 集合计算 | 对整个增长中的 live 集合维持固定总数 |
| 原始五个窗口组成员数量 | 组内完整性＋与对应冻结成员集合比较 | 删除成员后只重新生成较小组 digest |
| triad 精确 path/类别/claim 边界 | 抽出的 evidence policy 继续逐项验证 | 仅校验枚举合法就放行 |
| authority/backlink、README 禁止当前化措辞 | live 专用检查继续执行 | 因 schema 正确而省略语义边界断言 |
| 当前内容 SHA 和逐项生成分类 | 当前分片展开后与现有 policy 输出比较 | 用快照中的旧内容 SHA 判定新内容已审读 |

新增证据不自动加入冻结 cohort，其当前记录仍须经过既有保守分类、字段/引用/窗口检查和 QA/domain 审读。新增同类别证据不修改原始数量常量。对冻结成员删除、移动、重新归类或重组，默认阻断；确需改变时是明确的 evidence policy/退休迁移，不是普通 `sync`，必须另有批准范围与替代关系。此次存储迁移不授权这类变化。

### 6.6 CLI 合同

所有新增命令显式接受 `--repo-root`，默认为脚本所在仓库。下面的 `--apply` 是本地文件写入开关，不是审批开关；没有 `--apply` 时只输出计划。

```bash
# 新接口：定位；--kind 可选 object / semantic / evidence，组成员返回组文件
python3 scripts/document-corpus-inventory.py locate \
  --kind object --path doc/world-runtime/design.md

# 新接口：同步声明集合中的机器 object；不改 semantic/evidence
python3 scripts/document-corpus-inventory.py sync \
  --path doc/world-runtime/design.md --apply

# 新接口：源已删除时，明确删除它的 object
python3 scripts/document-corpus-inventory.py sync \
  --deleted-path <deleted-source-path> --apply

# 新接口：只生成 review 候选；适用于 semantic/evidence
python3 scripts/document-corpus-inventory.py propose \
  --kind semantic --path doc/world-runtime/design.md \
  --output .cache/doc-governance/review-proposal.json

# 新接口：在现行任务复核完成后，验证候选输入仍相同，再写局部分片
python3 scripts/document-corpus-inventory.py apply-review \
  --proposal .cache/doc-governance/review-proposal.json --apply

# 现有入口：默认工作树全量检查；新增 --revision 可检查冻结 commit/tree
python3 scripts/document-corpus-inventory-check.py --repo-root .
python3 scripts/doc-evidence-inventory-check.py --repo-root .

# 新接口：导出报告；不能输出到 doc 或任意 tracked 文件
python3 scripts/document-corpus-inventory.py export \
  --output .cache/doc-governance/document-corpus.json
```

`sync` 支持重复 `--path` / `--deleted-path`，以及互斥的 `--all`；默认无路径是参数错误，不隐式全库写入。如增加 `--changed-from <OID>`，必须读取固定基线与当前目标的 `--no-renames` 差异并展示完整写集合，不能用浮动 main 和 shell 分词推断范围。

退出码统一：0 成功/只读计划有效；1 数据或合同验证失败；2 参数/schema 版本/输入格式错误；3 并发修改或候选过期。报错最少包含 `code`、`source_path`、`record_path`、违反条件及可执行的修复命令；不会为了打印错误而自动修改记录。

`--print-generated` 在两个旧 checker 中退役：返回非零并提示新 export。必须显式提醒，shell 的 `> old-path` 会在程序启动前截断目标文件；工具无法阻止这一行为，因此需要删除旧调用并通过严格入口 schema 抓出破坏，不能声称“错误返回即可防止截断”。


### 6.7 公共模块与局部操作接口

公共库实现下列固定职责；名字可以在独立审读后调整，但消费者不能复制算法。

```python
class CorpusView:
    # 固定 Git revision 或显式工作树；不从全局 cwd / latest main 偷取内容。
    def list_doc_paths(self) -> list[str]: ...
    def read_bytes(self, path: str) -> bytes: ...
    def file_mode(self, path: str) -> str: ...

def load_corpus(view: CorpusView) -> CorpusModel: ...
def expected_object(view: CorpusView, source_path: str) -> dict: ...
def validate_corpus(view: CorpusView, model: CorpusModel) -> list[Diagnostic]: ...
def validate_evidence(view: CorpusView, model: CorpusModel) -> list[Diagnostic]: ...
def record_path(kind: str, source_path: str) -> str: ...
def locate(model: CorpusModel, source_path: str) -> list[RecordLocation]: ...
def plan_sync(view: CorpusView, paths: list[str], deleted_paths: list[str]) -> WritePlan: ...
def apply_plan(plan: WritePlan) -> WriteResult: ...
def normalize_legacy(raw_corpus: dict, raw_semantic: dict, raw_evidence: dict) -> dict: ...
def normalize_current(model: CorpusModel) -> dict: ...
```

`locate` returns deterministically ordered exact object/semantic/evidence record endpoints for that model's immutable view, including the complete member set when the path belongs to a bundle or group. Importing the module performs no cwd/root autodetection, `sys.path` mutation, filesystem write, or candidate import; the shared module and its transitive implementation use only the standard library.

`WritePlan` 包含有序的源前置条件、每个目标的旧 bytes SHA/不存在条件、拟写 bytes 或显式删除、操作类别。`apply_plan` 只允许该集合，无命令执行器、远程网络动作或自动追加父级写入。`Diagnostic` 为 `code/source_path/record_path/detail/repair_command`，不得把验证失败当空列表吞掉。

`propose` 的 selector 支持 `--path`、`--bundle`、`--group` 三种互斥形式。path 若属于真实集合，返回确切集合 ID 并要求显式选择集合，不静默从一个成员扩成整组。全新 semantic/evidence 的 owner、处置和 rationale 必须由作者补全并按现行规则审读；自动生成的保守候选不是已有批准。

Review proposal 使用 `schema=oasis7.document-corpus-review-proposal/v1`，包括 `kind`、有序 `source_preconditions`、有序 `record_preconditions`、有序 `mutations`。每个 mutation 有 `record_path`、`operation=upsert|delete` 和 upsert 时的完整记录。不得包含具有授权含义的 approval 布尔值；本工具既不签发授权，也不通过编辑候选取得新范围。新建/删除条件与现有值均使用显式状态，禁止用空字符串混充不存在。

Evidence 的基本必填字段为 `path/lifecycle/semantic_role/retention_owner/domain_owner/required_followup_roles/authority/backlink/disposition/rationale/residual_risk`。已有 `evidence_window/observation_window/atomic_group/group_sha256/content_sha256/claim_boundary` 等条件字段继续按旧规则适用；delete candidate 的 `semantic_absorption/reference_repair/owner_approval/validation` 不能丢弃。迁移预检必须输出任何尚未列入 schema 的既有字段并拒绝有损转换。


## 7. 状态、事务与持久化

### 7.1 不新增治理状态机

<a id="DCI-08"></a>
分片只存对象快照或耐久复核结论，不新增 task status、CI status、review pending、last_sync 或 current main 字段。内存/日志中的 `planned → validated → written → checked` 仅描述一次本地文件操作，不是 PM 状态，也不赋予发布/合并资格。

对普通 sync，幂等键是源路径＋期望完整 object 内容。对 apply-review，幂等条件是候选所声明的输入仍匹配，或目标完整字节已经与候选一致。输入不同不是重试成功，必须重新生成候选并审读。

### 7.2 写入原子性与并发

<a id="DCI-09"></a>
一次变更先解析并验证全部请求，再开始写入；格式、路径、权限范围、缺失源、重复对象等可预检错误必须零写入。正常同步只改分片，不执行 `git add`、commit、push、rebase、reset 或 clean。

单文件使用同目录临时文件、flush、`os.replace`；内容相同直接跳过，mtime 也不刷新。跨文件操作没有文件系统事务；Git commit 才是对外的原子版本。进程在多个 replace 之间中止，工作树可能部分完成，checker 应明确失败；再次按同一声明集合同步可修复机器记录，review 类操作须重新验证全部候选前置条件。

同一 worktree 的写命令使用轻量进程级 OS advisory lock，锁文件由 `git rev-parse --git-path` 定位到该 worktree 的管理区。Unix 用 `fcntl`，Windows 用 `msvcrt`；不能降级成无锁写入。锁范围仅本 worktree、仅本工具写命令，竞争时立即返回 3，不排后台队列。进程退出释放锁；锁文件存在不代表被占用，不创建需要人工清除的 stale lock 状态。

锁内重新验证源与记录摘要。不能依赖 advisory lock 阻止任意编辑器；写后发现源再次变化时返回并发修改，不声称成功。故障遗留临时文件报告精确路径；只清理本次明确创建并仍由本次持有的临时文件，不使用宽泛通配删除。

### 7.3 已审读身份、删除与 rename

源路径是记录身份的一部分。rename 不自动延续旧 semantic/bundle 身份，也不允许仅修改 JSON 内 path 而保留旧 key 文件名。需要同时检查旧端点消失、新端点出现、引用修复以及现有 frozen evidence 约束。

删除源与 object 并不等于授权删除 evidence/semantic。当前树检查负责发现悬空；base/head 范围检查与现行专业 review 负责审查已被删除的记录。不能声称单纯检查最终树可以证明历史复核记录从未被恶意移除。

## 8. 部署、安全与运行约束

### 8.1 运行依赖

沿用仓库当前 Python 与 Git 环境，核心实现仅依赖标准库；不新增 pip 包。Markdown 内容门禁继续使用项目既有 `markdown-it-py`。测试必须覆盖 Linux、macOS，以及现有 Windows Git/Python 路径；缺少某平台实际结果时在任务证据中声明未验证，不以 Linux fixture 代替 Windows 验收。

公共模块导入不得在源目录生成未跟踪 `__pycache__` 并污染 trusted helper 检查；沿用项目已有 bytecode 隔离方式，测试必须包含 cold-source/untracked shadow 场景。

### 8.2 输入与资源限制

来源路径仅允许仓库内普通文件；没有 JSON 内自定义 shell、Python import、远端 URL 下载或可执行 validator。JSON 摘要是内容一致性，不是签名或 owner 身份证明。

元数据单文件默认上限 8 MiB、JSON 嵌套上限 32、单次解析对象预算 100,000；这些是新实现的防御性预算而非已测性能。超限报明确错误，不截断、不抽样放行；迁移预检必须先证明当前数据满足预算。源文件使用流式 SHA，不能因为 binary 不是 Markdown 就漏出 coverage。

### 8.3 Loop 与 write_scope 的附属分片规则

<a id="DCI-10"></a>
必须先在 workflow source of truth 增补窄范围的“document corpus sidecar”合同，再实现相同规则。现有 `doc/.governance/** → code` 不能简单改成全部 system；否则产品文档仍受阻，而且会把真正的治理控制交给普通文档任务。

| 路径/变更 | 生效后的归属 | 准入条件 |
| --- | --- | --- |
| `objects/<key>.json` | 由被绑定源路径的实际 loop 继承 | key/path 正确；输出与同快照生成规则一致；源在声明集合内 |
| semantic entry | 继承源路径 loop | 完整记录结构有效；源或纯复核动作在任务范围；保留 domain review |
| 单一 loop 的 semantic bundle / evidence group | 全体成员所属的唯一 loop | 全成员集合显式声明并受控；不能只声明其中一个源就扩大写范围 |
| 跨 loop 或含 unknown 成员的真实 bundle/group | 普通叶子不得写入 | 原子格式迁移逐字节保留现有复核记录与完整成员，不重分类、不机械拆组；其他改动走现有授权范围 |
| `doc/.governance/document-corpus-inventory.json`、`doc/testing/evidence/inventory.json`、schema、registry、allowlist、frozen fixture、公共脚本 | code | 原治理控制权限；普通文档任务不得经 sidecar 继承写入 |
| `doc/testing/evidence/**` 内普通证据源 | system 的专业证据资产 | 精确排除静态入口；复用已有文档类型，并为 JSON/JSONL/CSV/TSV 等数据类型加 evidence-only 校验；不扩大到任意脚本/二进制可执行物 |

解析顺序是“受信的窄 sidecar 规则 → 原有规则”，不是相信记录中的 `loop` 或 `decision_owner` 自报值。记录不存可指定权限的 `loop` 字段。

`validate_scope()` 继续以 ancestor scope base 和 head 检查真实任务差异，不把 integration target-only 改动算作作者写入。分片的 base/head 两端均需检查：删除从 base 解析，新增从 head 解析，修改同时验证，rename 按删除＋新增处理。旧记录缺失或不能解析时阻断，不能因为 head 删除了危险路径就跳过旧端点。

**write_scope 必须保持完整。** 冻结任务时把源路径及由 locate 推导出的精确分片路径一并登记；bundle/group 记录同时冻结被选成员集合。`out_of_scope` 对源和分片都生效。新对象或组在冻结后才出现且超出允许集合时，走已有 scope 更新流程，不由 sync 隐式扩权。

object 内容、组成员或来源集合变化均触发重新核对；普通任务不能以 `--all` 为由修改其他文档分片。纯 semantic 复核不要求伪造源文本变化，但必须有明确复核义务和批准范围。

原子迁移 PR 同时触达代码、系统文档和历史元数据时，采用 source of truth 已定义的、显式用户授权的 legacy/same-PR migration 包装。设计请求本身不代表已有执行授权；执行任务必须回读授权和冻结精确集合。候选 loop policy 不能给该迁移 PR 自我准入，合并后经实际 readback 才能按既有流程启用新规则。

### 8.4 CI 及本地入口

<a id="DCI-11"></a>
目标状态是启用后把 corpus/evidence 真实检查接入 author/PR-prep/required 路径并纳入 required baseline，单次入口只执行一次；当前树尚未证明这些检查已在这些入口实际执行。Checker 自身回归属于 `doc_checker_contracts`，不能仅在测试 inventory 登记一个文件名就声称真实检查已执行。

| 变化 | 应选择的额外能力 | 不应额外选择的能力 |
| --- | --- | --- |
| 普通文档及其有效分片 | 启用后由实际 corpus baseline 检查；保留源文件触发的现有专用规则 | 不因分片而新增 Rust/WASM/launcher/full |
| corpus CLI、共享库、evidence policy、schema/静态描述 | `doc_checker_contracts`；共享准入依赖同时选 `workflow_governance` | 不因未登记路径落入 accidental full |
| loop admission、其受信共享依赖或 policy | `workflow_governance`＋适用的 doc checker 回归 | 不把全部文档改动路由到 PM 重型回归 |
| planner、scope 配置、中央 runner、workflow 的既有 full 路径 | 保留现有 conservative/full required 覆盖 | 不为了迁移 PR 更快降低现有门禁 |

实现同步 `scripts/ci-required-scope.v2.json`、planner 路由回归、`scripts/ci-tests.sh` 的 capability dispatcher 及 `scripts/ci-required-capability-test-inventory.tsv`。既有 frozen legacy/versioned scope fixture 保持历史含义：新增 active-policy 测试，不为追平生产规则机械改写冻结 fixture 和已固定摘要。

核对 local renderer、receipt selector、full/full-core/full-support 各历史套件是否仍覆盖移动后的测试；不新建与 `doc_checker_contracts` 同义的能力。继承源场景专用检查：例如 scenario 文档原本需要 scenario 回归，不能以“文档只跑轻量”为名删掉它。

## 9. 质量与容量

### 9.1 写放大与确定性

对 k 份独立普通源，机器同步最多改 k 个 object；semantic/evidence 的写入只能来自额外明确批准的 s 个 entry 或 g 个组。两个静态入口的字节必须保持不变。不设“每个文档 PR 必须刷新一次所有记录”的流程。

相同输入两次 sync/propose/export 的逻辑结果确定；无变化时不得改分片字节、mtime 或 Git index。JSON 列表排序固定，运行时间、绝对路径和浮动分支名不进入持久记录。

### 9.2 读取复杂度与测试开销

实现采用 path/id 字典，不保留旧式在对象循环中反复线性查找 expected owner 的模式。目录扫描/读取/哈希一次、排序一次，目标复杂度 O(N log N + B)，B 为被读取源字节；不为每份分片再启动一个 checker。

回归使用小型独立 fixture，禁止每个负例都复制完整生产 `doc/` 再跑全库；生产快照全量检查只在集成验收中做一次或按测试场景需要执行。迁移验收报告记录 N、B、实际写文件数、峰值内存与耗时，不在没有测量时声明倍数提升。

### 9.3 错误诊断

至少提供 `schema-version`、`duplicate-json-key`、`unsafe-path`、`wrong-record-key`、`unknown-metadata-file`、`new-path-drift`、`missing-path-drift`、`object-content-drift`、`registry-routing-drift`、`semantic-content-drift`、`semantic-bundle-drift`、`evidence-group-drift`、`frozen-cohort-drift`、`loop-sidecar-scope`、`proposal-stale`、`concurrent-modification` 和 `legacy-format-disabled`。

兼容外部脚本消费的既有 OK/FAIL 前缀；已知调用方依赖旧错误码时提供明确映射，不以泛化的 `missing-delegate` 掩盖任意 KeyError。错误退出不能产生可用于正式 merge 的成功 receipt。

## 10. 兼容、迁移与回滚

### 10.1 交付组织

推荐一项已明确授权的原子格式迁移任务、一份最终合入 PR。PR 内可以有独立 review 的规范提交、实现提交、机械数据提交，但最终启用不能让 main 处于“新数据配旧 checker”或“双份表都有效”的状态。

已有 workflow 支持显式授权的同 PR source＋implementation 包装；必须先冻结并独立审读规范修订。授权未建立时，不执行混合写入，也不为了满足“一 PR”伪装成普通 code loop；按现行规范发布/实现流程包装同一格式切换。本文不扩大例外权限。

### 10.2 逐文件改动清单

| 文件/集合 | 实施动作 | 验收重点 |
| --- | --- | --- |
| 本设计、doc-governance README、engineering `prd.index.md` | 新增设计与导航；长期规则只维护本设计 | 十二段、需求与验证映射可解析；无任务台账 |
| `doc/engineering/workflow/source-of-truth.md` | 增补窄 sidecar 所有权、scope、trusted loader、启用边界和启用后的 corpus baseline obligation | 保留现有 gate、full fallback 和三 loop/可选 Issue 规则；候选策略不激活 |
| `documentation-governance.manual.md` | 替换旧整表刷新步骤为 locate/sync/propose/check | 不鼓励 blanket regenerate 或自动续签 |
| 新公共库、evidence policy、管理 CLI | 提供读取、严格模型、局部写入、转换与三方导入 | 单一算法、无 shell 注入、幂等 |
| 两个现有 checker 及其测试 | 使用公共模型，保留专用语义检查，退役旧生成输出 | 现有负例语义不丢失 |
| 两个入口与新分片集合 | 静态化、无损拆分、删除旧 semantic 表 | 无动态父级摘要 |
| CI scope、runner、capability inventory、调用链测试 | 接入实际检查与按需回归 | source/merge 检查都执行，未引入 accidental full |
| `.gitignore`、`.gitattributes` | 精确忽略报告目录；约束新增元数据换行 | 不大范围忽略 doc，不改变历史源 bytes |
| `scripts/fixtures/document-corpus-v3/legacy/` | 保留旧结构、字段全集、冻结 cohort、golden 等价结果 | fixture 不进入活跃 corpus、不当作批准证据 |
| 旧路径实际消费者 | `git grep` 完整查找后修复，包括 testing/templates、playability 入口等 | 只留明确历史或迁移专用引用 |

实施必须建立 base/head 下的精确消费者清单，不以本表代替仓库搜索。新增模块位置若调整，需同步 trusted-root 保护、CI 路由、测试与文档，不能只改 import。

### 10.3 迁移算法与命令

<a id="DCI-12"></a>
新增 `migrate-v2` 接受明确的旧源 commit `--source-revision`、报告路径及可选 `--apply`。旧记录从该不可变 Git 快照读取，不从带 conflict marker 的工作树大 JSON 猜测内容。

```bash
# LEGACY_SOURCE_OID 由实施任务固定为实际迁移起点的完整 OID
python3 scripts/document-corpus-inventory.py migrate-v2 \
  --source-revision "$LEGACY_SOURCE_OID" \
  --report .cache/doc-governance/migration-report.json

# 先确认 dry-run 的损失为零、路径和声明范围正确，再本地应用
python3 scripts/document-corpus-inventory.py migrate-v2 \
  --source-revision "$LEGACY_SOURCE_OID" \
  --report .cache/doc-governance/migration-report.json --apply

python3 scripts/document-corpus-inventory-check.py --repo-root .
python3 scripts/doc-evidence-inventory-check.py --repo-root .
git diff --check
```

转换顺序：加载旧三表并严格验证 → 生成规范化旧模型 → 计算新文件映射 → 在 staging 空间生成新结构 → 新 reader 回读 → 规范化比较 → 验证变更目标的前置摘要 → 应用文件替换 → 当前树检查。默认拒绝覆盖与迁移计划不一致的既有分片。

比较内容包括全部直接源、全部原字段值、semantic entry 与 bundle、evidence 字段、atomic group 成员和两套 digest 算法。只允许 schema 包装、控制元数据的存储方式及不再持久化的可计算汇总变化；忽略对象排序只可在规范化层显式执行，不能忽略字段、空值或重复条目。

迁移 PR 自己新增/修改的规范、README、脚本引用等，会使目标源集合不同于旧快照。必须把“旧模型的无损转换”与“PR 已声明的新源修改”分成报告的两个部分：先证明对同一旧快照 round-trip 等价，再对目标树的明确 source delta 正常 sync/review。不能把这部分差异静默加入等价比较的全局忽略名单。

报告至少包含：source OID、工具版本、旧新记录数、完整源路径差异、字段差异、bundle/group 差异、允许的包装差异、PR 声明的 source delta、拟写/删除清单、所有前置摘要、最终退出码。实际执行与 tested tree OID 归入 PR 与实际验证记录。

### 10.4 旧 PR 的有界三方适配

保留显式 `import-legacy-delta --base <B> --legacy-head <H> --target <N> --output <proposal>`。B 为旧 PR 的不可变 scope base，H 为旧 PR 原始 source head，N 为已切换后的目标快照。B/H 从 Git 对象读取旧模型，N 读取新模型；正常 checker 永远不自动回退旧格式。

业务源冲突先解决。机器 object 以最终源重新生成，不迁移旧总表的机器 SHA。Semantic 按 path/bundle-id，evidence 按 path/atomic-group-id，执行下列记录级三方比较；删除视为明确的不存在值：

| 条件 | 选择 |
| --- | --- |
| H 与 B 相同 | 保留 N |
| N 与 B 相同 | 提议 H 的变化 |
| H 与 N 相同 | 保留共同结果 |
| 其他 | 报该对象/组冲突，禁止字段级自动拼接处置结论 |

先统一解包和组表示再比较，不能因为新旧包装不同制造冲突。所选 H 记录仍须匹配最终源 SHA、scope、frozen evidence 与 authority 约束；合并后的内容不同即要求重新复核。禁止将 H 的整份旧表拆开覆盖 N，禁止 `ours`/`theirs` 整表取一边。

工具仅写 proposal；后续显式 apply 复核当前目标未变。如果旧实现不可读、身份有歧义或目标已变化，返回具体阻断，不读取“最新 main”替换缺失的输入。旧 PR 是否保留、重建或放弃由原任务处置，迁移工具不自动关闭 PR。

### 10.5 启用与回滚

格式切换的准入依据是一次实际验证的组合树及既有独立 review/CI/readback；不是“新文件已提交”。冻结 gate 需要的身份与报告从任务事实读取，正文不硬编码某个实现 PR 的编号。

切换后尚无后续新数据写入时，可按现有流程 revert 整个切换提交及对应 consumer，不能只恢复旧 JSON。已有新写入后，禁止恢复迁移前旧总表而丢掉后续变更；优先前向修复。确需回退，新增 `export-legacy` 将当前模型导出为匹配旧序列化的三表，并验证旧规则能表达全部当前记录；无法表达新增 cohort 或语义时必须拒绝，另行批准兼容处理。

回滚同样要求覆盖、复核 SHA、group、consumer、CI 与 loop policy 成对回退；不以临时关闭 checker 或忽略目录维持绿灯。工具不自动执行 revert/reset/force push。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

下表是待实施验证计划。测试文件中新增的 DCI-T 场景在实现时落入准确入口；表中没有声称这些新增场景当前已运行。证据 target 均为当前 PR 与实际验证记录＋对应 CI artifact，真实 source/integration/tested tree、环境及退出码在执行时记录，不填写虚构身份。

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | [DCI-01](#DCI-01) | 独立普通源写集合互斥 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T01、T18–T21；临时 Git 分支与双合并顺序 | task evidence＋Git transcript | 不证明同源或同组无冲突 |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | [DCI-02](#DCI-02) | 日常更新保持两个静态入口不变 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T01/T02；入口字节前后比较 | task evidence＋checker 输出 | 不证明未来 schema 版本无需更新入口 |
| [文档职责](doc-structure-standard.design.md#22-文件按职责) | [DCI-03](#DCI-03) | 覆盖、重复、路由、内容漂移和复核绑定均保留 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T03–T12；结构与语义负例 | task evidence＋checker 输出 | 不替代 domain review |
| [目录登记](doc-structure-standard.design.md#35-一级物理目录-registry-与例外) | [DCI-05](#DCI-05) | 源、证据与元数据覆盖闭合 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T03–T09；微型 fixture 和迁移候选 | task evidence＋checker 输出 | 不证明内容语义正确 |
| [证据生命周期](doc-structure-standard.design.md#36-证据生命周期) | [DCI-06](#DCI-06) | 同步不续签已有语义复核 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T10–T13；源变异和过期 proposal | task evidence＋schema diff | 不证明 owner 实际批准 |
| [证据生命周期](doc-structure-standard.design.md#36-证据生命周期) | [DCI-07](#DCI-07) | 冻结 cohort、专用分类、窗口组和导航边界保留 | [evidence 回归](../../../scripts/doc-evidence-inventory-check.test.py)：DCI-T14–T17；组和历史边界负例 | task evidence＋checker 输出 | 不证明 fresh run 或发布 readiness |
| [状态与持久化](system-design-writing-standard.design.md#7-状态、事务与持久化) | [DCI-08](#DCI-08) | 操作状态不成为任务或审批状态 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T22；检查和导出前后快照比较 | task evidence＋before/after 文件集 | 不新增任务状态机 |
| [状态与持久化](system-design-writing-standard.design.md#7-状态、事务与持久化) | [DCI-09](#DCI-09) | 写入幂等、限于声明集合并对并发/故障失败关闭 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T23–T25；故障注入和双写者 | task evidence＋before/after 文件集 | advisory lock 不阻止外部编辑 |
| [兼容与迁移](system-design-writing-standard.design.md#10-兼容、迁移与回滚) | [DCI-04](#DCI-04) | 读写器、格式数据、消费者和测试一次切换 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T26/T27/T30；完整 fixture 和旧格式拒绝 | task evidence＋migration report | 不覆盖未声明的外部消费者 |
| [兼容与迁移](system-design-writing-standard.design.md#10-兼容、迁移与回滚) | [DCI-12](#DCI-12) | 三方适配按记录比较且冲突时拒绝 | [corpus 回归](../../../scripts/document-corpus-inventory-check.test.py)：DCI-T28/T29；三快照导入 fixture | task evidence＋proposal and conflict output | 不自动裁决旧 PR 处置 |

### 11.2 必须实现的场景

| ID | 输入/动作 | 断言 |
| --- | --- | --- |
| DCI-T01 | 修改一份未复核普通文档并 sync | 只改该 object；两个入口 byte-identical |
| DCI-T02 | 同一输入重复 sync/propose/export | 分片字节、mtime、index 不变化；报告逻辑确定 |
| DCI-T03 | 新源缺 object | `new-path-drift` |
| DCI-T04 | 源删除但 object 仍在 | `missing-path-drift`；普通 sync 不偷删 review |
| DCI-T05 | 错 key、重复 path、重复 JSON key | 分别失败；不能被后读记录覆盖 |
| DCI-T06 | 分片指向自身/入口/另一分片 | 拒绝；不产生递归登记 |
| DCI-T07 | 存储目录放未知文件或 symlink | fail closed；不宽泛忽略隐藏目录 |
| DCI-T08 | evidence 子目录再有名为 inventory.json 的源 | 纳入 E，不能 basename 排除 |
| DCI-T09 | registry owner 改变、第五产品模块、delegate overlap | 保留对应既有负例语义 |
| DCI-T10 | 已复核源变更，仅 sync object | `semantic-content-drift`；旧 review bytes 未变 |
| DCI-T11 | bundle 任一成员内容变更/重排/重复 | 验证旧算法及成员约束；不自动重签 |
| DCI-T12 | obligation 缺 retirement gate；非法 owner/disposition/authority | 保留既有负例 |
| DCI-T13 | propose 后源或原记录变化，再 apply | 返回 3、零写入 |
| DCI-T14 | group 一成员变化、删除或移入独立 entry | 组 SHA/成员集合/互斥检查失败 |
| DCI-T15 | 新增冻结 cohort 外的独立证据 | 不改根入口、固定数量或旧组；仍需合法 review 与 policy |
| DCI-T16 | 删除/重分类冻结证据或篡改 triad 边界 | `frozen-cohort-drift` / 原专用边界失败 |
| DCI-T17 | 历史窗口伪装当前健康/发布入口 | 保留 README/claim 专用负例 |
| DCI-T18 | 两分支修改同模块的不同源 | inventory 无冲突；合并后 check 通过 |
| DCI-T19 | 两分支新增排序相邻文件 | 不共享总列表；两源、两分片都保留 |
| DCI-T20 | 一分支改普通源，另一分支改独立 evidence/group | 无父级摘要冲突；两种合并顺序逻辑模型一致 |
| DCI-T21 | 两分支改同源/同组；业务文本可自动合并 | 不静默丢失；冲突或 SHA/review 检查阻断 |
| DCI-T22 | check/export 前后比较 tracked/untracked 源状态 | 不写源、分片或 index；仅允许显式报告输出 |
| DCI-T23 | 第 n 次 replace 注入故障 | 不报告全成功；后续 scoped 修复可校验，无全库清理 |
| DCI-T24 | 同 worktree 两写者、不同 worktree 两写者 | 前者返回忙；后者互不因本工具锁阻塞 |
| DCI-T25 | 路径空格/Unicode/大小写碰撞/CRLF 及平台锁 | 路径无 shell 拆词，别名拒绝，哈希差异显式报错 |
| DCI-T26 | 全量 legacy fixture 拆分再规范化展开 | 全部业务字段、review SHA 和组语义相等 |
| DCI-T27 | 同一源重复迁移；已有分片被外部修改 | 首者无 diff；后者前置校验失败，不覆盖 |
| DCI-T28 | 旧 PR 自己改一条 review，main 改另一条 | 三方导入保留两边；未改的旧数据不覆盖 main |
| DCI-T29 | 旧 PR/main 同改同 review，或导入后源不匹配 | 明确冲突/过期，不整表 ours/theirs |
| DCI-T30 | v3 normal check 遇旧表；遗留生成命令；回滚不可表达 | 拒绝旧格式、错误调用和有损 rollback |
| DCI-T31 | product/system 文档＋准确附属 object/review | 正确 loop 和精确 scope 可通过；不要求 code 叶子 |
| DCI-T32 | product 任务写静态入口/registry/另一源分片 | 拒绝；不能靠 JSON 自报 owner 扩权 |
| DCI-T33 | 删除/rename 分片或源，旧端点越界 | 使用 base 内容验证并拒绝 |
| DCI-T34 | bundle 成员有越界或跨 loop | 普通叶子拒绝；不能只审一个成员取得整组写权 |
| DCI-T35 | candidate 改共享 loader、policy 或注入 import shadow | trusted admission 不消费候选 authority |
| DCI-T36 | 未声明新 sidecar、已有 source out_of_scope、纯复核动作 | 前两者拒绝；后者有明确复核 scope 时允许 |
| DCI-T37 | 纯文档 sidecar、checker、共享准入依赖、unknown 路径 | 能力集合符合 §8.4，unknown 保留 full |
| DCI-T38 | author/prepare/required 实际调用；checker 故意失败 | 所有适用入口阻断；无只登记不执行、无重复全库调用 |
| DCI-T39 | full/full-core/full-support 和 renderer/receipt | 移动测试不漏跑，selector 与证据保持一致 |

### 11.3 可执行验收顺序

先跑新增小 fixture 和两个 inventory 回归，再跑新 workflow/sidecar 回归、planner 回归、既有 doc governance/link 检查；最后在准确候选树执行完整 corpus/evidence 真实检查和现有 required gate。没有实际结果的项不能写 passed。

```bash
# 现有入口，实施后承载扩展场景
python3 scripts/document-corpus-inventory-check.test.py
python3 scripts/doc-evidence-inventory-check.test.py
bash scripts/plan-rust-required-scope.test.sh

# 新增入口，须在本次实现中提供并登记
python3 scripts/document-corpus-inventory-workflow.test.py

# 保留现有文档准入；OID 由当前任务的受信范围提供
OASIS7_PRODUCT_DOC_BASE="$BASE_OID" \
OASIS7_PRODUCT_DOC_HEAD="$HEAD_OID" \
  bash scripts/doc-governance-check.sh
bash scripts/doc-governance-check.test.sh
bash scripts/readme-link-check.sh
python3 scripts/document-corpus-inventory-check.py --repo-root .
python3 scripts/doc-evidence-inventory-check.py --repo-root .
git diff --check
```

发布前的成功条件同时包括：旧消费者已处置、两个入口静态、无损报告通过、独立并发合并通过、旧 review 不被续签、冻结证据断言保留、sidecar admission 通过、真实 required CI 与必要 live readback 完成。仅 schema 测试或迁移工具 dry-run 为绿不算完成。

## 12. 决策、长期风险与未决问题

### 12.1 设计决策

| 决策 | 采用理由 | 代价与边界 |
| --- | --- | --- |
| 一源对象一片；真实原子组一片 | 匹配独立变更的最小粒度，不依赖当前模块/owner 划分 | 小文件增多，人工定位依赖 locate；组内冲突仍保留 |
| 路径 SHA 命名，不做内容寻址 | 修改内容不换身份，无编号注册表，深层路径长度可控 | rename 是新身份；哈希名不如镜像路径直观 |
| 静态入口＋运行时聚合 | 消除全局 SHA、计数、文件列表的共享写入 | 消费者必须用 loader，不再直接 jq 单体表 |
| 机器与 reviewed 数据分开 | 防止修 hash 顺手把新内容变成已复核 | 内容变更仍可能需要明确复核，这是有效约束 |
| 专用规则＋冻结 cohort | 保留历史边界又不把 live 集合锁死在固定数量 | 冻结成员退休仍需要真实规则决策 |
| 全量只读、局部写入 | 先解决结构性冲突，不引入复杂增量缓存 | 全量读取仍有成本，后续按测量再优化 |
| 同步修 loop sidecar 归属 | 避免结构拆分后 product/system 任务仍无法维护记录 | 需要受信 admission、scope 和冷导入测试 |
| 一次原子切换，不双写 | 避免长期两套清单互相漂移 | 切换时旧 PR 有一次明确的适配成本 |

### 12.2 长期风险与复核触发

共享 registry 政策、大 bundle、本来共用的 README/index 仍可能冲突；本文不承诺全仓零冲突。某 bundle 长期包含互不依赖对象时，由原 owner 证明可独立复核后再拆，不能由迁移脚本擅自判断。

新增 evidence 类型或业务场景可能要求调整真实 policy，这种代码变化不是清单写放大。出现大量新规则进入共享 policy 时，再按规则职责拆策略模块，不把当前问题换成新的全局动态登记表。

如果导出报告开始被其他工具当作必须回写的权威输入，或新增父级指纹/总计数字段，就违反 DCI-02，应直接由回归阻断，而不是等冲突再次频发。

### 12.3 有界实施前核对项

以下是执行时必须用实际仓库消除的输入条件，不是把核心设计留作 TBD：完整旧字段集合及最大元数据大小由迁移预检输出，RH/QA 核对；实际消费者由 base/head `git grep` 清点，RH 负责；实际启用的 effective policy、允许的迁移包装与 readback 由现有 workflow authority 决定；平台验证缺口由 QA 在 task evidence 明示。

任何核对发现 schema 无法无损容纳已有字段、真实原子组跨越 loop、外部消费者依赖旧总表或迁移授权不足，都只能阻断对应范围并修订明确合同，不能用全局 ignore、force 或新的隐式兼容回退消除。

### 12.4 原始审读依据

本设计的“当前状态”来源于审读基线中的以下文件；新增结构、命令和验收项属于本文的设计决定，不是原文件已实现内容。

| 来源 | 本次采用的事实 |
| --- | --- |
| `scripts/document-corpus-inventory-check.py` | v2 生成/比较、controls/delegate、semantic entry/bundle SHA、四模块和覆盖检查 |
| `scripts/document-corpus-inventory-check.test.py` | 既有漂移、重复、delegate、semantic 等负例 |
| `scripts/doc-evidence-inventory-check.py` | v1 生成分类、冻结数量、精确 triad、窗口组和专用导航约束 |
| `doc/.governance/document-semantic-review-overrides.json` | 已有耐久语义复核记录及 testing templates bundle |
| `doc/engineering/workflow/source-of-truth.md` | 显式 same-PR 迁移包装、候选 authority 不启用、required baseline 与能力分工 |
| `doc/engineering/doc-governance/system-design-writing-standard.design.md` | 十二段结构、需求/验证映射及事实/设计/运行证据分离 |
| `doc/engineering/doc-governance/doc-structure-standard.design.md` | 文件职责、目录登记、四模块和证据生命周期权限 |

**最终完成定义：没有共同源文档、共同复核集合、共同证据组或共同规则变化的两个任务，不再因为 corpus inventory 体系而写入同一个持久文件；同时，覆盖、审读绑定、证据边界与有效准入没有被削弱。**
