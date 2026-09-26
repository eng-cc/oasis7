# oasis7 Runtime：WASM Docker 确定性构建与工件治理管线设计

- 专业 owner：`wasm_platform_engineer`；状态：active professional authority（目标合同不等于已实现）。
- 内容审读基线：`eng-cc/oasis7`，源提交 `9c41d57b4436f71f0cf9b481043d882cfdc551ed`；本次文档采纳任务：[Issue 4102](https://github.com/eng-cc/oasis7/issues/4102)，Task UID `task_9f4c3a6c2daa40a0ae3e2c86bdf35e02`；内容整理日期：2026-09-26。
- 独立审读与实际实现/测试候选证据由该 task evidence 记录；此处不预报审读通过、runtime capability、发布或 full proof。下方既有条款仍有效；仅显式日期/历史基线条款按其证据范围解释。

## 1. 问题、目标与非目标

本次把既有专业义务承接为可执行的 target contract 与验证设计。成功结果是精确身份、边界、失败和恢复算法能被消费者读取；不改变 ABI/code/经济价格，不宣称目标已落地。下方原有细节、limits、公式、ID 与历史 evidence 原序保留并继续约束其适用范围。

## 2. 上游约束与相关角色

产品端到端 authority 为 [deterministic world execution](../../product/world-infrastructure/deterministic-world-execution.prd.md) 与 [distributed state availability](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md)；专业细化由 paired PRD、runtime/module 与 P2P 合同拥有。WASM owns codec/artifact/metering，runtime owns staged publication/replay，QA owns same-candidate acceptance，consumer owners own real projection。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [BUILD-01](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-1) | 输入 pinned image/platform/source closure/tooling/wasm mismatch；断言 受控Docker拒绝wrong identity；发布只一个linux token | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | runtime/QA；[BUILD-01外部receiver](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | 当前localfixture不能证明full finality/provider/headed |
| [BUILD-02](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-2) | 输入 Linux-only vs Darwin+Linux actual Docker summaries；断言 Linux仅partial；双宿主full必须同candidate且bytes一致 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | runtime/QA；[BUILD-02外部receiver](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | 当前localfixture不能证明full finality/provider/headed |
| [BUILD-03](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-3) | 输入 node-side missing/forged proof、production host compile、materializer fallback；断言 独立policy/trust receiver拒绝；不以source compile-off代fallback proof | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | runtime/QA；[BUILD-03外部receiver](../module/online-module-release-legality-closure-2026-03-08.design.md#sr2-release-trust) | 当前localfixture不能证明full finality/provider/headed |
| [SC-1](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-1) | SC-1: 同一 commit 在 macOS 与 Linux 上通过同一 pinned Docker builder image 构建时，得到的 canonical packaged wasm hash 一致率为 100%。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-2](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-2) | SC-2: 发布级模块工件只产生一个 canonical publish hash，来源固定为 linux-x86_64 容器构建；宿主平台不再写入独立发布 hash。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-3](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-3) | SC-3: build receipt 必须能追溯 builder_image_ref + builder_image_digest + container_platform + source_hash(含模块本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-4](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-4) | SC-4: runtime 与节点执行路径默认只接受 Docker canonical build 产生的 wasm binary 与其 identity/release evidence，不要求节点重新编译源码。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-5](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-5) | SC-5: ModuleSourcePackage 的生产发布路径不得继续依赖 runtime 进程在宿主机直接编译；必须迁移到同一 Docker builder 或显式 gated 为 dev/test only。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-6](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-6) | SC-6: 发布候选要宣称“跨宿主 determinism 已收口”时，必须归档至少一条 linux-x86_64 与一条 Docker-capable darwin-arm64 的 canonical summary / release evidence；Linux-only gate 只能代表稳定基线，不能代表跨宿主 closure。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-7](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-7) | SC-7: 生产 runtime / node 入口必须默认关闭 builtin manifest fallback、本地 identity hash 签名、本地 finality signing 与 runtime source compile，保证 binary-only policy 是生产默认行为而不是测试显式开关。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-8](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-8) | SC-8: 跨宿主 Docker release evidence 必须可被包装成 node-side attestation proof payload，并进入 ModuleReleaseSubmitAttestation.proof_cid；仅存在于 CI artifact 的 report 不能单独视为生产 closure。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [SC-9](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-9) | SC-9: Docker-first canonical build 链路的 operator env key 必须统一到 OASIS7_WASM_* 当前入口；不得再保留任何旧品牌前缀作为有效运行入口，避免 host wrapper、builder image、sync/check 与 build receipt 采集口径分叉。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [PRD-WORLD_RUNTIME-020](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-020) | PRD-WORLD_RUNTIME-020: As a wasm_platform_engineer, I want publishable WASM to be built only inside a pinned Docker builder image, so that host platform differences stop influencing release hashes. | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [PRD-WORLD_RUNTIME-021](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-021) | PRD-WORLD_RUNTIME-021: As a 发布节点运营者, I want each artifact to carry a build receipt that binds builder image digest, source hash, build manifest hash, and canonical wasm hash, so that social verification no longer depends on “which laptop built it”. | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [PRD-WORLD_RUNTIME-022](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-022) | PRD-WORLD_RUNTIME-022: As a runtime_engineer / qa_engineer, I want runtime to consume only Docker-canonical binaries and CI to compare Docker outputs across hosts, so that drift is blocked before execution. | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-1](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-1) | AC-1 (PRD-WORLD_RUNTIME-020): 必须新增并固定一份 WASM builder Docker image，镜像引用必须以 digest pin；所有 publishable wasm 构建都通过 docker run 进入该镜像。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-2](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-2) | AC-2 (PRD-WORLD_RUNTIME-020): builder image 必须封装 Rust toolchain、rust-src、wasm32-unknown-unknown 目标、linker/canonicalizer 所需依赖，并把这些版本信息收敛到 build_manifest_hash。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-3](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-3) | AC-3 (PRD-WORLD_RUNTIME-020): scripts/build-wasm-module.sh 的 canonical path 必须改为 Docker wrapper；publishable 构建不再保留 host-native cargo fallback。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-4](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-4) | AC-4 (PRD-WORLD_RUNTIME-021): 发布级 hash manifest 目标态只允许写入单个 canonical token：linux-x86_64=<sha256>；darwin-arm64 不再作为发布 hash 来源。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-5](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-5) | AC-5 (PRD-WORLD_RUNTIME-021): build receipt 至少绑定 builder_image_digest + container_platform + source_hash(含本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version，并进入 identity/release evidence。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-6](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-6) | AC-6 (PRD-WORLD_RUNTIME-022): multi-runner CI 必须比较“相同 Docker builder 在不同宿主上产出的 canonical hash”，而不是继续比较 host-native cargo 输出。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-7](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-7) | AC-7 (PRD-WORLD_RUNTIME-022): runtime 与节点执行路径默认只接受 canonical Docker build 产物；节点不通过重新编译源码参与执行合法性判断。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-8](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-8) | AC-8 (PRD-WORLD_RUNTIME-022): compile_module_artifact_from_source 的生产路径必须迁移到外部 Docker builder 或直接禁用；runtime 进程内 host 直编只允许在 dev/test 模式下显式开启。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-9](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-9) | AC-9 (PRD-WORLD_RUNTIME-021/022): 若 GitHub-hosted CI 因 runner 能力不足只能保留 Linux-only stable gate，PRD / project / evidence 报告必须把跨宿主 evidence 标记为 pending，直到导入 Docker-capable macOS summary 为止。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-10](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-10) | AC-10 (PRD-WORLD_RUNTIME-022): production 运行入口必须提供 release security policy 绑定证据，证明 fallback / 本地签名 / runtime source compile 默认关闭；仅在测试里调用 enable_production_release_policy() 不足以视为完成。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-11](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-11) | AC-11 (PRD-WORLD_RUNTIME-021/022): release evidence 除了 CI/report 汇总外，还必须存在 node-side proof payload 打包与 attestation submit 入口，使 builder_image_digest/container_platform/canonicalizer_version 可作为发布节点提交的正式证明字段进入共识链路。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-12](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-12) | AC-12 (PRD-WORLD_RUNTIME-020/021): scripts/build-wasm-module.sh、scripts/sync-m1-builtin-wasm-artifacts.sh、scripts/ci-m1-wasm-summary.sh、tools/wasm_build_suite 与 docker/wasm-builder/Dockerfile 必须只写入或读取 OASIS7_WASM_*；wrapper usage、错误提示、容器注入 env 与 build receipt 元数据采集不得再接受任何旧品牌前缀作为有效运行入口。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |
| [AC-13](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-13) | AC-13 (PRD-WORLD_RUNTIME-021/022): builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 必须只读取 OASIS7_BUILTIN_WASM_*；Docker-first build 已完成品牌迁移后，runtime 取件/抓取/编译 fallback 不得继续接受任何旧品牌前缀作为有效运行入口。 | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | paired专业owner与runtime/QA；具体跨域依赖见本域合同 | 下方原文限定范围不扩张，未执行能力不升级 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前状态 / 基线 | 目标 | 差距与证据边界 |
| --- | --- | --- | --- |
| wasm-1 foundation | 原有代码/契约保留，源commit见身份块 | 兼容consumer及精确target contracts | 原有tests为located，不是本次run |
| receipt / migration / gate | interface仍声明target或partial | `sr2-build-contract` 与runtime authority相连 | 代码、full finality/provider/headed证明不在本任务 |
| 文档与schema | Issue4102采纳目标合同 | 语义/结构/anchors检查 | 文档通过不意味着runtime pass或SC9/DC5闭合 |

## 4. 边界与结构

输入声明和provider response为advisory，host/runtime负责authority/admission；sandbox仅确定性计算。build证明字节来源、SDK证明codec、metrics证明local诊断，各自不能签署world finality。ContentRef必须fetch/hash/length验证，再由相应authority验trust；current caches与local signature不能越过生产许可。

## 5. 关键运行流程

先绑定candidate/world branch/window与exact manifest/artifact/schema；strict decode和limits preflight后验证authority。成功只进入有权的下一阶段，世界效果须canonical commit receipt。缺字段、版本、proof、兼容或head continuity即停止，不隐式fallback或重试另一身份。重试同request hash幂等，changed hash拒绝；恢复重读当前canonical边界，历史receipt按记录版本重放。

## 6. 接口与数据合同

<a id="sr2-build-contract"></a>
### 6.1 Build identity 与 proof join

此表是现行 build receipt 字段的 target acceptance 约束，不新增第二套执行 receipt。BuildReceiptIdentityV1 的 target projection 为 `{builder_image_ref:Id,builder_image_digest:Id,container_platform:"linux-x86_64",source_hash:Hash32,build_manifest_hash:Hash32,wasm_hash:Hash32,canonicalizer_version:Id}`；builder_image_digest 必须是 pinned `sha256:` 加64 lowercase hex，source_hash 覆盖本地 path dependency 闭包。Projection 只用于精确 acceptance join，不声称现行 receipt JSON 全字段 schema 已替换。

| 输入 identity | producer → consumer | 检查顺序 / 失败 |
| --- | --- | --- |
| source_hash / build_manifest_hash | Docker build suite → identity/release authority | 重算 source closure 与 locked tooling；dependency 漂移即拒绝 |
| builder_image_ref / digest / platform / canonicalizer | pinned wrapper/container → receipt | ref pin 与实际 digest 相等，platform 只 linux-x86_64；缺 Docker 不 native fallback |
| packaged wasm_hash / byte length | canonical packager → runtime artifact loader | 先实际 fetch+rehash，再检查 manifest/identity/release；不得先信 cache |
| candidate / scope / canonical window | task selected artifact → DC5 semantic receiver | exact source/integration/tested tree、execution/protocol/test-contract、world branch/window 一致 |
| node-side attestation / external finality | node proof assembler → release trust | proof bytes/hash、issuer epoch/stake/min-independent-signers、canonical finality 独立验证 |
| timing / dry_run / hostOS | diagnostics → operator summary | 排除执行 commitment；dry-run 无 binary proof，不用 Linux-only 假称 Darwin 对账 |

构建成功仅到 built/receipted，信任检查后才可进入 governed release，最终 apply 仍依赖 canonical finality。CI artifact 不激活生产；runtime 不重编源码。source compile-off 与 builtin materializer fallback 是不同控制，前者不能证明后者已 hardened；外部 worker 不在本任务实现。双宿主 full proof 必须实际 Docker-capable Darwin+Linux，同一 source/image/tooling byte hash 对账且 archive；历史 run28297899706/jobs83840926310/83843884654 仅原有日期证据，非本次候选。

失败 Docker unavailable、wrong digest、missing path dependency、wrong wasm bytes、cross-host mismatch、wrong candidate/type/tree、missing external proof 均阻断，不回写双发布 token。重试保留同一输入 identity，仅完整构建重出 artifact；未发布输出可删除，已治理 artifact/hash/history 保留不可覆盖。[release trust](../module/online-module-release-legality-closure-2026-03-08.design.md#sr2-release-trust) 负责 key/epoch/authority，不由 build receipt代签。


## 7. 状态、事务与持久化

本域记录不能把 accepted/built/decoded/observed 当 applied。SDK/build/metrics 的本地输出不拥有世界事务；interface定义receipt core，runtime staged commit拥有state/effects/charges/registry/journal/winner同一发布点。失败全体不发布，crash在prepublish与postpublish由durable receipt/winner/index恢复，禁止重跑已commit WASM。实际prototype atomic seam仅bounded prepared proposal，不能推导完整事务。snapshot保留原manifest/artifact/schedule/lineage与历史bytes。

## 8. 部署、安全与运行约束

生产binary-only及external finality仍由release/runtime policy；无Docker/wasm32/provider环境不能伪报通过。modules无clock/random/I/O，host注入caller/time/target；namespace/capability/AUTHZ约束全部仍有效。计数、字节和费用checked bounds-before-allocation/mutation，limit来源为现行ModuleLimits与各paired PRD；diagnostics不得泄露raw payload/keys/proof。

## 9. 质量与容量

| 环境 / 输入 | 预期响应 | 阈值来源 | 验证入口 / 当前范围 |
| --- | --- | --- | --- |
| required canonical fixture / malformed bytes、integer overflow、wrongscope | structured rejection且不发布state/effect/charge | interface类型、现行limits及paired PRD | §11.1各case；target设计未运行 |
| full实际canonical环境 / restart、activation、proof撤销 | durable replay/identity与no-new-effect gate一致 | runtime/GWSC各精确receiver | samecandidate artifacts mandatory，未证明 |
| 现行资源/perf输入 | 不扩大既有budget，不用localtiming计费 | 下方原有limits、NFR与perf方案 | 既有tests/scripts仅located partial |

## 10. 兼容、迁移与回滚

保留legacy serde defaults、wire exports、工件不可覆盖与历史bytes。新target先固定合同再逐consumer启用，unknown version拒绝；当前ABI不变。回滚到身份块源码/旧consumer，仅影响新请求；已canonical commit的事件、charges、receipt不可撤销或改价。迁移须explicit from/to descriptor及governed artifact；缺proof停在旧版，不能以代码合入或健康恢复启用。

## 11. 验证设计与可追溯性

Issue4102只交付文档合同采纳与local checks；实际实现、runtime required/full、provider、headful、production proof分别在真实实现task evidence记录candidate/source/integration/tested tree与环境，缺失保持未证明，不能用FutureIssue占位代替本页receiver。全DC5验证由 [GWSC receiver](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-dc5-attachment) 与其 [schema](../../testing/schemas/world-execution-state-sync-attachment.schema.json) 拥有。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [BUILD-01](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-1) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | 受控Docker拒绝wrong identity；发布只一个linux token | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；target case `BUILD-01` 输入 pinned image/platform/source closure/tooling/wasm mismatch；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [BUILD-02](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-2) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | Linux仅partial；双宿主full必须同candidate且bytes一致 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；target case `BUILD-02` 输入 Linux-only vs Darwin+Linux actual Docker summaries；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [BUILD-03](wasm-deterministic-build-pipeline.prd.md#sr2-domain-case-3) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | 独立policy/trust receiver拒绝；不以source compile-off代fallback proof | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；target case `BUILD-03` 输入 node-side missing/forged proof、production host compile、materializer fallback；required deterministic/codec、full actual canonical，同candidate/world/window选择；existing source不宣称已覆盖target | Issue4102文档采纳；实际task evidence存candidate/artifact/proof与exitcode | 本次未执行；proof语义与真实环境未证明 |
| [SC-1](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-1) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-1: 同一 commit 在 macOS 与 Linux 上通过同一 pinned Docker builder image 构建时，得到的 canonical packaged wasm hash 一致率为 100%。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-2](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-2) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-2: 发布级模块工件只产生一个 canonical publish hash，来源固定为 linux-x86_64 容器构建；宿主平台不再写入独立发布 hash。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-3](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-3) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-3: build receipt 必须能追溯 builder_image_ref + builder_image_digest + container_platform + source_hash(含模块本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-4](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-4) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-4: runtime 与节点执行路径默认只接受 Docker canonical build 产生的 wasm binary 与其 identity/release evidence，不要求节点重新编译源码。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-5](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-5) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-5: ModuleSourcePackage 的生产发布路径不得继续依赖 runtime 进程在宿主机直接编译；必须迁移到同一 Docker builder 或显式 gated 为 dev/test only。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-6](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-6) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-6: 发布候选要宣称“跨宿主 determinism 已收口”时，必须归档至少一条 linux-x86_64 与一条 Docker-capable darwin-arm64 的 canonical summary / release evidence；Linux-only gate 只能代表稳定基线，不能代表跨宿主 closure。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-7](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-7) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-7: 生产 runtime / node 入口必须默认关闭 builtin manifest fallback、本地 identity hash 签名、本地 finality signing 与 runtime source compile，保证 binary-only policy 是生产默认行为而不是测试显式开关。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-8](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-8) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-8: 跨宿主 Docker release evidence 必须可被包装成 node-side attestation proof payload，并进入 ModuleReleaseSubmitAttestation.proof_cid；仅存在于 CI artifact 的 report 不能单独视为生产 closure。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [SC-9](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-sc-9) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | SC-9: Docker-first canonical build 链路的 operator env key 必须统一到 OASIS7_WASM_* 当前入口；不得再保留任何旧品牌前缀作为有效运行入口，避免 host wrapper、builder image、sync/check 与 build receipt 采集口径分叉。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [PRD-WORLD_RUNTIME-020](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-020) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | PRD-WORLD_RUNTIME-020: As a wasm_platform_engineer, I want publishable WASM to be built only inside a pinned Docker builder image, so that host platform differences stop influencing release hashes. | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [PRD-WORLD_RUNTIME-021](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-021) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | PRD-WORLD_RUNTIME-021: As a 发布节点运营者, I want each artifact to carry a build receipt that binds builder image digest, source hash, build manifest hash, and canonical wasm hash, so that social verification no longer depends on “which laptop built it”. | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [PRD-WORLD_RUNTIME-022](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-prd-world_runtime-022) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | PRD-WORLD_RUNTIME-022: As a runtime_engineer / qa_engineer, I want runtime to consume only Docker-canonical binaries and CI to compare Docker outputs across hosts, so that drift is blocked before execution. | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-1](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-1) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-1 (PRD-WORLD_RUNTIME-020): 必须新增并固定一份 WASM builder Docker image，镜像引用必须以 digest pin；所有 publishable wasm 构建都通过 docker run 进入该镜像。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-2](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-2) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-2 (PRD-WORLD_RUNTIME-020): builder image 必须封装 Rust toolchain、rust-src、wasm32-unknown-unknown 目标、linker/canonicalizer 所需依赖，并把这些版本信息收敛到 build_manifest_hash。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-3](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-3) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-3 (PRD-WORLD_RUNTIME-020): scripts/build-wasm-module.sh 的 canonical path 必须改为 Docker wrapper；publishable 构建不再保留 host-native cargo fallback。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-4](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-4) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-4 (PRD-WORLD_RUNTIME-021): 发布级 hash manifest 目标态只允许写入单个 canonical token：linux-x86_64=<sha256>；darwin-arm64 不再作为发布 hash 来源。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-5](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-5) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-5 (PRD-WORLD_RUNTIME-021): build receipt 至少绑定 builder_image_digest + container_platform + source_hash(含本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version，并进入 identity/release evidence。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-6](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-6) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-6 (PRD-WORLD_RUNTIME-022): multi-runner CI 必须比较“相同 Docker builder 在不同宿主上产出的 canonical hash”，而不是继续比较 host-native cargo 输出。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-7](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-7) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-7 (PRD-WORLD_RUNTIME-022): runtime 与节点执行路径默认只接受 canonical Docker build 产物；节点不通过重新编译源码参与执行合法性判断。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-8](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-8) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-8 (PRD-WORLD_RUNTIME-022): compile_module_artifact_from_source 的生产路径必须迁移到外部 Docker builder 或直接禁用；runtime 进程内 host 直编只允许在 dev/test 模式下显式开启。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-9](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-9) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-9 (PRD-WORLD_RUNTIME-021/022): 若 GitHub-hosted CI 因 runner 能力不足只能保留 Linux-only stable gate，PRD / project / evidence 报告必须把跨宿主 evidence 标记为 pending，直到导入 Docker-capable macOS summary 为止。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-10](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-10) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-10 (PRD-WORLD_RUNTIME-022): production 运行入口必须提供 release security policy 绑定证据，证明 fallback / 本地签名 / runtime source compile 默认关闭；仅在测试里调用 enable_production_release_policy() 不足以视为完成。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-11](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-11) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-11 (PRD-WORLD_RUNTIME-021/022): release evidence 除了 CI/report 汇总外，还必须存在 node-side proof payload 打包与 attestation submit 入口，使 builder_image_digest/container_platform/canonicalizer_version 可作为发布节点提交的正式证明字段进入共识链路。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-12](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-12) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-12 (PRD-WORLD_RUNTIME-020/021): scripts/build-wasm-module.sh、scripts/sync-m1-builtin-wasm-artifacts.sh、scripts/ci-m1-wasm-summary.sh、tools/wasm_build_suite 与 docker/wasm-builder/Dockerfile 必须只写入或读取 OASIS7_WASM_*；wrapper usage、错误提示、容器注入 env 与 build receipt 元数据采集不得再接受任何旧品牌前缀作为有效运行入口。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |
| [AC-13](wasm-deterministic-build-pipeline.prd.md#sr2-obligation-ac-13) | [本域合同](wasm-deterministic-build-pipeline.design.md#sr2-build-contract) | AC-13 (PRD-WORLD_RUNTIME-021/022): builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 必须只读取 OASIS7_BUILTIN_WASM_*；Docker-first build 已完成品牌迁移后，runtime 取件/抓取/编译 fallback 不得继续接受任何旧品牌前缀作为有效运行入口。 | [partial scope test](../../../scripts/plan-wasm-determinism-scope.test.sh)；actual build对账仍由 `scripts/ci-verify-m1-wasm-summaries.py` 接收；按原条款输入/阈值及本页domain cases验证，required兼容/codec/build/窗口fixture，full仅真实同candidate跨宿主/authority proof；原文manual/run来源仍适用 | Issue4102采纳文档；真实implementation task evidence记录input/candidate/environment/exit/artifact | existing source仅partial，本次未运行；原有full/provider/headed义务保留 |

现有partial/unrun来源：`crates/oasis7_wasm_executor/src/tests.rs`（output/fuel/memory/cache），`crates/oasis7_wasm_abi/tests/open_module_commands.rs`（legacy/envelope），`crates/oasis7/src/runtime/world/module_runtime_tests.rs`（metrics/due-preflight），`scripts/oasis7-node-wasm-metrics-monitor.test.sh`（window summary），`scripts/ci-verify-m1-wasm-summaries.py`（build对账）。这些不是新target receipt/migration/DC5已覆盖证据。GWSC [negative matrix](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-qa-negative-matrix) 接收全部17QA cases，[runtime scenarios](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md#sr2-runtime-scenarios) 接收10组runtime；本页domain rows为具体输入/assertion分配。

## 12. 决策、长期风险与未决问题

选择单authority typedtarget而非重复wire，是为避免scope/hash/版本漂移。已知风险是implementation缺失、proof不可读、U64/UTF8投影误读、递归hash与历史误作新候选证据；WASM/runtime/QA分别按本页case和runtimereceiver解除。触发器为新ABI/schedule/schema、trust epoch、activation或consumer变更；需重新冻结合同与专业联审。SC9八fullcells及activeLLM/provider API parity、真实desktop+narrow/headed证据保持产品/QAauthority，不因本页文档完成消除。


- 对应需求文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 稳定证据入口: `doc/world-runtime/wasm/evidence.md`

审计轮次: 3

## 1. 设计定位
本设计把 oasis7 的 WASM 发布级构建从“宿主机护栏 + keyed hash 对账”升级为“Docker canonical builder”。设计目标不是否定现有脚本和 build suite，而是把它们放进同一个 pinned 容器镜像里，让宿主平台不再参与发布 hash 的生成。

实施任务、候选状态和 CI run 历史不在本文维护；它们由 GitHub task issue /
Project 和 Git history 追溯。可长期复用的验证入口收敛在
[`evidence.md`](evidence.md)。

## 2. 现状盘点

| 层级 | 当前实现事实（2026-03-18） | 与 Docker-first 目标的差距 |
| --- | --- | --- |
| 容器基础设施 | `docker/wasm-builder/Dockerfile`、`scripts/build-wasm-module.sh` 与 canonical builder digest 已落地。 | canonical builder 已存在，跨宿主证据已由 self-hosted Darwin Docker workflow 产出；repo 内保留 run/job 索引，不提交 workflow artifact 二进制。 |
| 构建入口 | `scripts/build-wasm-module.sh` 已强制 `docker run --platform linux/amd64`，且显式拒绝 host-native fallback。 | 入口已收敛，但仍需把“stable gate”和“cross-host full closure”严格区分。 |
| 构建工具 | `tools/wasm_build_suite` 已在容器内输出 `build receipt`、`source_hash`、`build_manifest_hash`、`builder_image_digest` 与 `container_platform`。 | receipt 语义基本完成，剩余重点在 evidence 汇总与 release gate 结论。 |
| manifest 策略 | builtin hash manifest 已改为单 canonical token `linux-x86_64=<sha256>`；identity 生成已切换为 receipt 驱动。 | 写路径已收敛，cross-host report 已拿到真实 `darwin-arm64` Docker evidence。 |
| source compile | `compile_module_artifact_from_source` 仍保留源码包编译能力，但 production `ReleaseSecurityPolicy` 已默认拒绝该 action；仅 dev/test 可显式使用。 | external builder worker 仍未落地，当前以 production default-disable 先收敛 runtime 权限面。 |
| CI 校验 | `.github/workflows/wasm-determinism-gate.yml` 当前已收敛为 GitHub-hosted `linux-x86_64` stable gate，并支持导入外部 summaries；`.github/workflows/wasm-darwin-docker-evidence.yml` 可手动调度 self-hosted Darwin Docker runner 产出真实 `darwin-arm64` bundle 并触发 Linux cross-host verify。 | `WDBP-3` 的真实 `linux-x86_64 + darwin-arm64` evidence 已在 main run `28297899706` 产出；GitHub-hosted macOS 仍不作为 Docker evidence producer。 |
| runtime 消费策略 | runtime 侧已支持 production policy 关闭 source compile / local signing / builtin fallback，且 production-facing 入口已补 hardened policy 绑定证据。 | WDBP-3.3 已完成；后续风险只保留入口扩展时的回归防护。 |

## 3. 设计原则
- 原则-1：publish hash 只来自 canonical container，不来自宿主机。
- 原则-2：builder image digest 是构建身份的一部分，必须进入 receipt 与审计链路。
- 原则-3：runtime 节点是 binary consumer，不是 source builder。
- 原则-4：CI 验证 Docker reproducibility，不拥有生产 manifest 写权限。
- 原则-5：source compile 要么走同一外部 Docker builder，要么明确降级为 dev/test only。

## 4. 目标态架构

```text
repo source / ModuleSourcePackage
          |
          v
host wrapper
scripts/build-wasm-module.sh
          |
          v
docker run --platform linux/amd64
builder-image@sha256:...
          |
          v
containerized wasm_build_suite
          |
          v
canonical packaged wasm
          |
          +------------------------------+
          |                              |
          v                              v
     build receipt                identity / release evidence
          |                              |
          +--------------+---------------+
                         v
        single canonical publish hash
      (manifest token: linux-x86_64=<sha256>)
                         |
                         v
               DistFS / release manifest
                         |
                         v
              runtime fetch -> verify -> execute
```

关键变化：
- 宿主平台不再直接生成发布 hash。
- `linux-x86_64` 容器平台成为唯一 canonical publish 平台。
- keyed manifest 从“多宿主发布 hash 集合”收敛为“单 canonical token”。

## 5. 详细设计

### 5.0 Builtin 工件 materialization

- `m1/m4/m5` 的 canonical binary 经 sync/check 与 SHA-256 manifest 对账后，由 `hydrate_builtin_wasm` 写入按 SHA-256 配置的 `LocalCasStore`。
- materializer 读取 DistFS candidate 后复算 SHA-256；本地候选不可用时，当前实现仍会继续 fetch 与 builtin source-compile fallback。materializer 不接收 identity、receipt 或 release policy。
- DistFS 只承载 binary materialization/cache，不改变 canonical build/identity 的权威关系。identity/receipt/signature/release policy 由外围 artifact selection / release-gated pipeline 校验；`Action::CompileModuleArtifactFromSource` 的 production policy gate 不覆盖 builtin materializer fallback，不能据此声明生产入口已经关闭后者。

### 5.1 Canonical Builder Image
目标新增一份固定 builder image，例如：
- `docker/wasm-builder/Dockerfile`
- 镜像通过 digest pin 引用，而不是仅用 tag。

镜像内必须固定：
- Rust toolchain 版本
- `rust-src`
- `wasm32-unknown-unknown`
- canonical packaging 所需依赖
- `tools/wasm_build_suite`
- 统一 workspace 路径，例如 `/workspace`

设计要求：
- builder image 是发布级构建环境的真源。
- `build_manifest_hash` 的输入至少包含：
  - `builder_image_digest`
  - `container_platform=linux-x86_64`
  - build profile
  - canonicalizer version
  - build suite version / contract version

### 5.2 Host Wrapper
`scripts/build-wasm-module.sh` 的职责要改写为：
- 校验 Docker 可用。
- 组装 `docker run` 参数。
- 绑定只读源码挂载与可写输出目录。
- 把 `module_id`、`manifest_path`、`profile` 转交给容器内 build suite。

约束改为：
- 不再保留 host-native fallback。
- Docker 不可用时直接失败，避免任何宿主直编结果进入发布链路。
- wrapper 只接受同一 workspace root 下的 `manifest_path + out_dir`，并统一映射到容器内 `/workspace`。

### 5.3 Containerized Build Suite
`tools/wasm_build_suite` 不需要被替换，但需要重新定位：
- 过去：host 工具。
- 未来：container-internal canonical builder。

保留能力：
- `cargo metadata/build --locked`
- workspace `build.rs` / `proc-macro` guard
- custom section stripping
- metadata 输出

新增能力：
- 输出 `build receipt`
- receipt 至少包含：
  - `builder_image_ref`
  - `builder_image_digest`
  - `container_platform`
  - `module_id`
  - `source_manifest_path`
  - `source_hash`
  - `build_manifest_hash`
  - `canonicalizer_version`
  - `wasm_hash_sha256`
  - `wasm_size_bytes`

### 5.4 Manifest / Identity Migration
当前 manifest 的主要问题是把宿主平台差异直接放进发布清单。

Docker-first 目标态：
- 发布级 manifest 只保留一个 token：
  - `linux-x86_64=<sha256>`
- 这里的 `linux-x86_64` 表示 canonical builder container platform，而不是要求宿主机必须是 Linux。

迁移策略：
1. 读路径先兼容多 token。
2. 写路径改为只写 canonical token。
3. CI 阻止新的 `darwin-arm64` 发布 token 进入仓库。
4. release manifest / identity / attestation 逐步只引用 canonical token。

Identity manifest 需要扩展：
- 从“source hash + build manifest hash + hash tokens”
- 升级为“source hash + build manifest hash + builder image digest + canonical token”

### 5.5 Source Package Compile Policy
这是本设计最大的边界调整。

当前问题：
- `compile_module_artifact_from_source` 仍存在于 runtime 代码面，但 production 路径已经不能再直接执行它。
- external builder worker 还未落地，因此当前阶段通过 production policy default-disable 来阻断 Docker daemon 进入 runtime 热路径。

当前落地态：
- 生产态 source compile 已被 `ReleaseSecurityPolicy` 默认禁用。
- runtime 生产路径只接收已经构建好的 wasm artifact；源码包编译 action 会被拒绝并要求使用 external Docker builder。
- dev/test 仍可显式使用该路径，便于保留现有回归与实验工作流。

推荐两段式流程：
1. runtime / 发布层提交 `ModuleSourcePackage`
2. external builder worker 调用 Docker canonical builder，返回：
   - `wasm_bytes`
   - `source_bundle_hash`
   - `build receipt`

这样 runtime 继续是 binary-first consumer。

### 5.6 CI And Release Verification
CI 需要改成比较容器输出，而不是比较 host-native 输出。

目标 workflow：
1. macOS runner 安装 Docker Desktop / 可用 Docker CLI。
2. Linux runner 安装 Docker。
3. 两边都运行同一个 wrapper script。
4. 两边 summary 都导出 canonical container hash。
5. compare job 要求两边 hash 完全一致。

release gate 需要新增的固定结论：
- `builder image digest matched`
- `canonical token matched`
- `docker-only path enforced`
- `cross-host evidence complete` 或 `cross-host evidence pending`
- `production release policy hardened by default`

### 5.7 Runtime Consumption
runtime 的最终消费模型不变，仍是 binary-first：
1. 从 release manifest / DistFS 获取 binary。
2. 校验 canonical hash。
3. 校验 identity / signature / receipt 绑定。
4. 注册 artifact 并执行。

真正变化的是：
- canonical hash 现在来自 Docker builder。
- runtime 不再需要解释多个宿主平台发布 hash。

### 5.8 Cross-Host Evidence Closure Design
这是 `WDBP-3` 曾经的 P0 剩余切片；当前已由 self-hosted Darwin Docker runner 与 main run `28297899706` 收口。本节保留 stable gate 与 full-tier evidence 的分层规则，防止后续把 GitHub-hosted Linux-only gate 误写成跨宿主 closure。

#### 5.8.1 双层 gate 模型
- `stable gate`
  - 运行环境：GitHub-hosted `ubuntu-24.04`
  - 目的：持续验证 canonical builder、receipt、identity、single token、report 脚本本身没有回退。
  - 结论上限：`linux-only stable`
- `full-tier cross-host evidence`
  - 运行环境：`linux-x86_64` + 至少一条 Docker-capable `darwin-arm64`
  - 目的：验证“相同 builder image digest + 相同源码输入”在真实跨宿主 Docker 环境下仍收敛到同一 canonical hash。
  - 结论上限：`cross-host closed`

设计约束：
- 任何缺少 `darwin-arm64` Docker canonical summary 的发布候选，都只能得到 `stable gate passed / cross-host pending`。
- 不允许把 `linux-x86_64` 单宿主结果写成 `SC-1 fulfilled`。

#### 5.8.2 Summary Import Contract
`scripts/wasm-release-evidence-report.sh` 的导入语义需要成为正式证据协议，而不是临时绕行。

外部 Docker-capable runner 推荐先产出一个标准 bundle，再进入 verify/report：
- `scripts/package-wasm-summary-bundle.sh`
  - 负责把 `m1/m4/m5` summary 规范化为同一 bundle 目录或 `.tar.gz`
- `scripts/stage-wasm-summary-imports.sh`
  - 负责把 GitHub-hosted Linux summary 与外部 bundle 合并到同一 verify 输入目录

每个导入 summary 必须至少包含：
- `runner_label`
- `host_platform`
- `canonical_platform=linux-x86_64`
- `builder_image_digest`
- `canonicalizer_version`
- `module_set`
- `module -> wasm_hash`
- `receipt_evidence`

外部 bundle manifest 至少包含：
- `schema_version`
- `runner_label`
- `host_platform`
- `module_sets`
- `summary_files`

report 聚合时必须输出：
- `expected_runners`
- `received_runners`
- `missing_runners`
- `cross_host_evidence_pending`
- `canonical_hash_consistent`
- `receipt_evidence_consistent`

推荐 machine-readable 结论：

```json
{
  "module_set": "m1",
  "stable_gate_passed": true,
  "cross_host_evidence_pending": false,
  "expected_runners": ["linux-x86_64", "darwin-arm64"],
  "received_runners": ["linux-x86_64", "darwin-arm64"],
  "gate_result": "cross-host-closed"
}
```

#### 5.8.3 External Evidence Dispatch Runbook
真实 Docker-capable `darwin-arm64` runner 可用后，full-tier 证据产出流程固定为：

优先使用手动 CI 路径：

1. 在 GitHub Actions 手动触发 `Wasm Darwin Docker Evidence`。
2. `runs_on_json` 填写真实 Docker-capable macOS ARM64 self-hosted runner labels，默认值为 `["self-hosted","macOS","ARM64","docker"]`。
3. `collect-darwin-docker-summaries` job 会做 `Darwin arm64 + docker linux/amd64` preflight，并产出 `darwin-arm64-wasm-summary-bundle` artifact。
4. `verify-with-linux-summaries` job 会下载该 artifact、收集 `linux-x86_64` summaries、导入 darwin bundle，并产出 `darwin-arm64-wasm-release-evidence-report` artifact。
5. 该 report 的 `summary.json` 满足下方 closure 条件后，才可把 `WDBP-3.2` 视为完成。

当前正式收口证据：`Wasm Darwin Docker Evidence` main run `28297899706` 已通过；`collect-darwin-docker-summaries` job `83840926310` 在 self-hosted Darwin runner 上产出 bundle，`verify-with-linux-summaries` job `83843884654` 完成 Linux summary 收集、Darwin bundle 导入与 cross-host report 上传。

如果 self-hosted runner 不能直接接入仓库 workflow，则使用外部 dispatch 路径：

1. 在外部 runner 上收集 `m1/m4/m5` summary，并生成标准 bundle：
   - `./scripts/package-wasm-summary-bundle.sh --out-dir <bundle-dir> --archive <bundle.tar.gz> --runner-label darwin-arm64`
2. 将 `<bundle.tar.gz>` 上传到 GitHub Actions runner 可访问的 URL（例如 release asset、对象存储或受控静态下载地址）。
3. 在仓库内触发 `Wasm Determinism Gate` workflow_dispatch：
   - `./scripts/dispatch-wasm-determinism-gate.sh --bundle-url <https-url> --runner-label darwin-arm64`
4. verify job 会自动执行：
   - 下载 GitHub-hosted `linux-x86_64` summaries
   - `stage-wasm-summary-imports.sh` 合并本地 Linux summary 与外部 bundle
   - `wasm-release-evidence-report.sh` 生成 `summary.md/json`
5. 只有当 `summary.json` 同时满足以下条件时，才可把 `WDBP-3.2` 视为完成：
   - `received_runners` 覆盖 `linux-x86_64,darwin-arm64`
   - `cross_host_evidence_pending=false`
   - `gate_result=cross-host-closed`
   - `canonical_hash_consistent=true`
   - `receipt_evidence_consistent=true`

交付约束：
- bundle URL 必须可被 GitHub-hosted verify job 直接下载。
- 外部 runner 必须实际跑 Docker canonical builder，不能只转存 host-native 结果。
- 正式 closure 证据应记录 workflow run URL、job id 与对应 report artifact 名称；workflow artifacts 受 GitHub Actions retention 管理，不作为 repo-tracked binary 长期归档。

#### 5.8.4 证据来源分层
为避免把开发回归、CI 回归、生产候选证据混写，evidence source 需要分层：
- `ci-hosted-linux`
- `external-builder-macos`
- `release-node-attestation`

排序原则：
1. 先验证所有 source 的 `builder_image_digest`、`build_manifest_hash`、`canonicalizer_version` 一致。
2. 再比较 canonical wasm hash。
3. 最后才允许给出 `cross-host closed`。

若任一步失败：
- 保留已有 Linux stable gate 结果；
- 将 cross-host 状态标为 `blocked`；
- 不回滚到 host-keyed manifest。

#### 5.8.5 Node-Side Proof Assembly
CI/report 只能提供开发期或候选期证据；真正进入 `ModuleReleaseSubmitAttestation.proof_cid` 的 payload 需要由发布节点本地重新装配。

固定入口：
- `scripts/module-release-node-attestation-flow.sh`

节点侧固定流程：
1. 从本机收集 summary，或导入预收集 summary / 外部 bundle。
2. 运行 `scripts/wasm-release-evidence-report.sh` 做多 runner verify，输出人读/机读报告。
3. 将报告依赖的 per-runner summary 做 canonicalize，剥离 `generated_at_utc`、本地路径、run dir 等非语义字段。
4. 生成稳定的 `proof_inputs/release_evidence_summary.json`，只保留：
   - `required_runners / expected_runners / received_runners`
   - `stable_gate_passed / cross_host_evidence_pending / cross_host_closed / gate_result`
   - 每个 `module_set` 的 gate 结论与 canonical summary 文件 `sha256`
5. 再调用 `scripts/package-module-release-attestation-proof.sh` 生成正式 `proof_payload.json + submit_request.json`。
6. 如需直接入链，再调用 `scripts/submit-module-release-attestation.sh`。

设计约束：
- `proof_cid` 不得依赖 report 运行时目录、日志时间戳或宿主机临时路径。
- 人读报告与 verify log 可以保留在 run dir 内供排障，但默认不进入 proof payload 的 canonical evidence 集。
- 发布节点可选择 `--require-cross-host-closed`，在 full-tier 候选阶段把 `conditional-go` 直接升级为阻断。

### 5.9 Production Release Policy Binding
这是 `WDBP-3.3` 的设计边界；当前 production-facing 入口已补 hardened policy 绑定证据，本节保留为后续入口扩展时的约束。

#### 5.9.1 策略矩阵

| 运行形态 | `allow_builtin_manifest_fallback` | `allow_identity_hash_signature` | `allow_local_finality_signing` | `allow_runtime_source_compile` | 预期用途 |
| --- | --- | --- | --- | --- | --- |
| dev | `true` | `true` | `true` | `true` | 本地调试、实验工作流 |
| test | `true` 或按用例覆写 | `true` 或按用例覆写 | `true` 或按用例覆写 | `true` 或按用例覆写 | 定向回归、拒绝路径测试 |
| production | `false` | `false` | `false` | `false` | 节点执行、发布候选、线上验收 |

设计要求：
- production 不是“调用方约定”；必须在主运行入口自动绑定。
- dev/test 的放宽必须是显式 opt-in，不能继续复用默认值伪装成产品路径。

#### 5.9.2 主运行入口绑定点
本轮设计要求至少覆盖以下入口：
- `oasis7_chain_runtime`
- 任何由 launcher 拉起的 chain runtime 生产路径
- runtime 相关 release / acceptance 脚本入口

绑定策略：
1. 入口解析出运行模式或发布配置。
2. 若模式属于 release / prod / candidate，创建 `World` 后立即应用 hardened policy。
3. 在 status / evidence 输出中打印实际生效的四个布尔值。
4. 若入口没有进入 hardened policy，不允许给出 production-ready 结论。

#### 5.9.3 可验证证据面
除了行为拒绝测试，还需要有显式配置证据：
- `status.json` / `summary.md` / release gate 报告中写出 effective policy
- 节点验收脚本断言四个布尔值均为 `false`
- 若任一值为 `true`，报告必须输出 `production_release_policy_not_hardened`

推荐 evidence 字段：

```json
{
  "release_security_policy": {
    "allow_builtin_manifest_fallback": false,
    "allow_identity_hash_signature": false,
    "allow_local_finality_signing": false,
    "allow_runtime_source_compile": false
  }
}
```

#### 5.9.4 与 source compile gate 的关系
`WDBP-4` 解决的是“source compile 在 production 默认被拒绝”；`WDBP-3.3` 已把同类 hardened policy 变成主入口默认事实并写出证据。

边界：
- `WDBP-4` 不再新增 source compile 业务设计。
- `WDBP-3` 只负责入口绑定与证据化，不改变 source compile reject 语义本身。

## 6. 角色分工

| 角色 | 负责内容 |
| --- | --- |
| `producer_system_designer` | 明确“容器解决漂移、runtime 不再直接编译源码”的目标边界 |
| `wasm_platform_engineer` | builder image、wrapper、receipt、manifest migration、cross-host evidence 协议与报告字段 |
| `runtime_engineer` | source compile 外移或 gating、release manifest 消费、production entry hardened policy 绑定 |
| `qa_engineer` | multi-runner Docker compare、失败签名、stable/full-tier 结论区分、policy 绑定复验 |

## 7. 失败模型与阻断点

| 失败点 | 触发位置 | 阻断行为 |
| --- | --- | --- |
| Docker 不可用 | host wrapper | 直接失败，不允许回退成发布级 native build |
| image digest 未 pin 或不匹配 | wrapper / receipt verify | 阻断发布 |
| container output 与 tracked canonical token 不一致 | sync/check | 阻断并报告 `module_id + expected + actual` |
| macOS/Linux 跑同一容器得出不同 hash | CI compare | 阻断并归类为 builder reproducibility defect |
| production runtime 仍启用 host source compile | runtime config gate | 启动即拒绝或 action rejected |
| GitHub-hosted gate 仅有 Linux summary | release evidence report | 允许 stable gate 通过，但 cross-host 结论必须保持 `pending` |
| production entry 未绑定 hardened release policy | runtime status / acceptance script | 直接 `no-go`，不得以测试辅助调用替代 |

## 8. 迁移计划
- M0：修正文档为 Docker-first 目标态。
- M1：新增 builder image 与 wrapper，保留读路径兼容。
- M2：manifest/identity 只写 canonical token。
- M3：runtime source compile 外移到 external builder 或 production 默认禁用。
- M4：CI / release gate 全量切换到 Docker reproducibility compare，并引入 stable gate 与 full-tier evidence 分层。
- M5：主运行入口默认绑定 hardened release policy，并把 effective policy 输出到 release evidence / acceptance report。

## 9. 设计边界
- Docker 只解决发布级构建确定性，不进入 wasm 执行期 sandbox。
- 本设计不要求立即删除现有 build suite；它是容器内核心执行器。
- 本设计不把 CI 变成生产发布者；CI 只是运行同一容器镜像做验证。

## 10. Source package guardrail 与生产边界

- 生产 runtime 不执行 host source compile，也不回退到仓库 build script；它只消费 external Docker canonical builder 产生的 binary、manifest 与 receipt。
- dev/test 显式 opt-in 的 source package 入口必须限制文件数量、单文件/总字节数和相对路径，拒绝绝对路径与 `..`，在隔离临时目录中以最小环境和明确 timeout 执行。
- dev/test 编译失败返回结构化 policy/validation 原因；这些 guardrail 只降低测试工具污染风险，不等价于 OS container、seccomp 或生产级 sandbox。
- 任何面向发布的产物仍必须重新进入 canonical builder、hash/identity/receipt 校验链；dev/test compile 结果不能直接提升为 publish authority。
