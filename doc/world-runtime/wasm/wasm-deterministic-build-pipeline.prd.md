# oasis7 Runtime：WASM Docker 确定性构建与工件治理管线

- 对应设计文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`

审计轮次: 3

## 1. Executive Summary
- Problem Statement: 当前仓库已经有 host 侧 deterministic guard、canonical packaging、keyed hash manifest、identity manifest、DistFS 与 multi-runner 对账，但这些机制本质上仍是在“接受不同宿主平台会产出不同 wasm，然后用治理和对账去兜底”。如果目标是从源头解决 `darwin-arm64` / `linux-x86_64` 的 hash 漂移，仅靠 host 原生构建护栏不够，必须把发布级构建环境收敛到同一容器镜像。
- Proposed Solution: 把 publishable WASM 的 canonical build path 改为 Docker-first。所有可进入发布链路的 wasm 都必须在 pinned builder image 中构建，容器平台固定为 `linux-x86_64`，并由同一容器内的 build suite 产出 canonical packaged wasm。宿主机只负责调用 `docker run`；真正进入 module id、release manifest、identity 与 runtime 执行链路的 hash，只认容器产物，不认 host-native 构建结果。
- Success Criteria:
  - SC-1: 同一 commit 在 macOS 与 Linux 上通过同一 pinned Docker builder image 构建时，得到的 canonical packaged wasm hash 一致率为 `100%`。
  - SC-2: 发布级模块工件只产生一个 canonical publish hash，来源固定为 `linux-x86_64` 容器构建；宿主平台不再写入独立发布 hash。
  - SC-3: build receipt 必须能追溯 `builder_image_ref + builder_image_digest + container_platform + source_hash(含模块本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version`。
  - SC-4: runtime 与节点执行路径默认只接受 Docker canonical build 产生的 wasm binary 与其 identity/release evidence，不要求节点重新编译源码。
  - SC-5: `ModuleSourcePackage` 的生产发布路径不得继续依赖 runtime 进程在宿主机直接编译；必须迁移到同一 Docker builder 或显式 gated 为 dev/test only。
  - SC-6: 发布候选要宣称“跨宿主 determinism 已收口”时，必须归档至少一条 `linux-x86_64` 与一条 Docker-capable `darwin-arm64` 的 canonical summary / release evidence；Linux-only gate 只能代表稳定基线，不能代表跨宿主 closure。
  - SC-7: 生产 runtime / node 入口必须默认关闭 builtin manifest fallback、本地 identity hash 签名、本地 finality signing 与 runtime source compile，保证 binary-only policy 是生产默认行为而不是测试显式开关。
  - SC-8: 跨宿主 Docker release evidence 必须可被包装成 node-side attestation proof payload，并进入 `ModuleReleaseSubmitAttestation.proof_cid`；仅存在于 CI artifact 的 report 不能单独视为生产 closure。
  - SC-9: Docker-first canonical build 链路的 operator env key 必须统一到 `OASIS7_WASM_*` 当前入口；不得再保留任何旧品牌前缀作为有效运行入口，避免 host wrapper、builder image、sync/check 与 build receipt 采集口径分叉。

## 2. User Experience & Functionality
- User Personas:
  - `wasm_platform_engineer`：需要维护 pinned builder image、容器构建入口、build receipt 与 canonical hash 契约。
  - 发布节点运营者 / 模块发布者：需要在 macOS/Linux 上得到同一个 publish hash，并能向外证明“这个 hash 来自哪一个容器镜像和源码输入”。
  - `runtime_engineer`：需要让节点只消费合法 binary，而不是把源码编译变成执行前置依赖。
  - `qa_engineer` / 发布维护者：需要把漂移排查从“主机差异”收敛为“同一容器镜像是否稳定输出”。
- User Scenarios & Frequency:
  - 构建入口升级：每次涉及 Rust 版本、LLVM、canonicalizer、镜像 digest 或 build suite 的变更时执行。
  - builtin 模块更新：每次 `m1/m4/m5` 发生工件变更时执行容器构建与发布级 hash 更新。
  - 发布候选验证：每个候选版本至少执行一次跨宿主 Docker reproducibility 对账。
  - 源码包发布：每次玩家/Agent 源码包要进入正式模块市场时执行。
- User Stories:
  - PRD-WORLD_RUNTIME-020: As a `wasm_platform_engineer`, I want publishable WASM to be built only inside a pinned Docker builder image, so that host platform differences stop influencing release hashes.
  - PRD-WORLD_RUNTIME-021: As a 发布节点运营者, I want each artifact to carry a build receipt that binds builder image digest, source hash, build manifest hash, and canonical wasm hash, so that social verification no longer depends on “which laptop built it”.
  - PRD-WORLD_RUNTIME-022: As a `runtime_engineer` / `qa_engineer`, I want runtime to consume only Docker-canonical binaries and CI to compare Docker outputs across hosts, so that drift is blocked before execution.
- Critical User Flows:
  1. Flow-WDBP-001（builtin Docker 构建）:
     `编辑模块源码 -> host 调用 build wrapper -> docker run pinned builder image (linux-x86_64) -> 容器内运行 build suite + canonical packaging -> 产出 canonical wasm + build receipt -> sync manifest / identity / DistFS`。
  2. Flow-WDBP-002（源码包发布）:
     `提交 ModuleSourcePackage -> 发布节点或外部 builder worker 展开源码包 -> docker run 同一 builder image -> 输出 canonical wasm + build receipt -> 后续治理/安装只消费 binary + receipt，不在 runtime 热路径直接 host 编译`。
  3. Flow-WDBP-003（跨宿主验证）:
     `PR/候选发布触发 macOS + Linux workflow -> 两边都运行同一 Docker builder -> 导出 canonical hash summary -> compare job 要求 hash 完全一致，否则阻断`。
  4. Flow-WDBP-004（运行时消费）:
     `节点读取 release manifest / builtin manifest -> 获取 canonical wasm binary -> 校验 wasm hash 与 build receipt / identity -> 加载执行 -> 节点不重编源码`。
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| canonical Docker builder | `builder_image_ref`、`builder_image_digest`、`container_platform=linux-x86_64`、`workspace_mount`、`out_dir` | host wrapper 统一调用 `docker run`，所有 publishable 构建都走容器 | `requested -> container_started -> built/failed` | 同一源码与镜像 digest 必须输出同一 canonical hash | 发布级构建只能由受控 builder image 执行 |
| host wrapper script | `module_id`、`manifest_path`、`profile`、`out_dir`、`docker_binary` | 本地与 CI 都执行同一 wrapper，而不是各自直接跑 cargo | `raw_host -> wrapper -> containerized` | host OS 不进入 publish hash 计算 | publishable 构建只允许 Docker path；无 host-native fallback |
| build receipt | `source_hash`、`build_manifest_hash`、`builder_image_digest`、`container_platform`、`canonicalizer_version`、`wasm_hash` | 容器构建结束后写出 receipt 并供 identity / release evidence 消费 | `built -> receipted -> verified` | `source_hash` 必须覆盖模块源码与本地 `path` 依赖闭包；任一字段变化都必须改变 `build_manifest_hash` 或 receipt digest | receipt 为发布与审计必需，不是可选日志 |
| publish hash manifest | `module_id linux-x86_64=<sha256>` | sync/check 仅更新和验证 canonical container hash | `built -> checked/synced -> published` | 发布级 manifest 只允许一个 canonical token；`darwin-arm64` 不再写入发布清单 | 本地默认只读；CI 不写 manifest |
| identity / release evidence | `module_id`、`source_hash`、`build_manifest_hash`、`builder_image_digest`、`wasm_hash` | 生成 identity manifest、attestation、release manifest 引用 | `metadata-ready -> signed/verified` | identity 绑定容器镜像与源码输入，不绑定宿主平台 | 生产验证由 release/trust root 决定 |
| source package policy | `source_bundle_hash`、`compile_mode=external-builder|dev-only` | 生产路径提交到外部 builder；runtime 直编仅限 dev/test | `submitted -> queued_for_build -> built/rejected` | 生产不得继续走 runtime host compile；dev/test 路径显式 gated | 仅受控发布节点可产出 publishable artifact |
| production release policy | `allow_builtin_manifest_fallback`、`allow_identity_hash_signature`、`allow_local_finality_signing`、`allow_runtime_source_compile` | 生产入口启动时必须切到 fallback-off / local-signing-off / source-compile-off | `dev-default -> production-hardened` | 仅显式 dev/test 配置可放宽；不能依赖测试中手工调用作为生产证据 | `runtime_engineer` 负责入口绑定，`wasm_platform_engineer` / `qa_engineer` 负责 gate |
- Acceptance Criteria:
  - AC-1 (PRD-WORLD_RUNTIME-020): 必须新增并固定一份 WASM builder Docker image，镜像引用必须以 digest pin；所有 publishable wasm 构建都通过 `docker run` 进入该镜像。
  - AC-2 (PRD-WORLD_RUNTIME-020): builder image 必须封装 Rust toolchain、`rust-src`、`wasm32-unknown-unknown` 目标、linker/canonicalizer 所需依赖，并把这些版本信息收敛到 `build_manifest_hash`。
  - AC-3 (PRD-WORLD_RUNTIME-020): `scripts/build-wasm-module.sh` 的 canonical path 必须改为 Docker wrapper；publishable 构建不再保留 host-native cargo fallback。
  - AC-4 (PRD-WORLD_RUNTIME-021): 发布级 hash manifest 目标态只允许写入单个 canonical token：`linux-x86_64=<sha256>`；`darwin-arm64` 不再作为发布 hash 来源。
  - AC-5 (PRD-WORLD_RUNTIME-021): build receipt 至少绑定 `builder_image_digest + container_platform + source_hash(含本地 path 依赖闭包) + build_manifest_hash + wasm_hash + canonicalizer_version`，并进入 identity/release evidence。
  - AC-6 (PRD-WORLD_RUNTIME-022): multi-runner CI 必须比较“相同 Docker builder 在不同宿主上产出的 canonical hash”，而不是继续比较 host-native cargo 输出。
  - AC-7 (PRD-WORLD_RUNTIME-022): runtime 与节点执行路径默认只接受 canonical Docker build 产物；节点不通过重新编译源码参与执行合法性判断。
  - AC-8 (PRD-WORLD_RUNTIME-022): `compile_module_artifact_from_source` 的生产路径必须迁移到外部 Docker builder 或直接禁用；runtime 进程内 host 直编只允许在 dev/test 模式下显式开启。
  - AC-9 (PRD-WORLD_RUNTIME-021/022): 若 GitHub-hosted CI 因 runner 能力不足只能保留 Linux-only stable gate，PRD / project / evidence 报告必须把跨宿主 evidence 标记为 pending，直到导入 Docker-capable macOS summary 为止。
  - AC-10 (PRD-WORLD_RUNTIME-022): production 运行入口必须提供 release security policy 绑定证据，证明 fallback / 本地签名 / runtime source compile 默认关闭；仅在测试里调用 `enable_production_release_policy()` 不足以视为完成。
  - AC-11 (PRD-WORLD_RUNTIME-021/022): release evidence 除了 CI/report 汇总外，还必须存在 node-side proof payload 打包与 attestation submit 入口，使 `builder_image_digest/container_platform/canonicalizer_version` 可作为发布节点提交的正式证明字段进入共识链路。
  - AC-12 (PRD-WORLD_RUNTIME-020/021): `scripts/build-wasm-module.sh`、`scripts/sync-m1-builtin-wasm-artifacts.sh`、`scripts/ci-m1-wasm-summary.sh`、`tools/wasm_build_suite` 与 `docker/wasm-builder/Dockerfile` 必须只写入或读取 `OASIS7_WASM_*`；wrapper usage、错误提示、容器注入 env 与 build receipt 元数据采集不得再接受任何旧品牌前缀作为有效运行入口。
  - AC-13 (PRD-WORLD_RUNTIME-021/022): builtin wasm materializer、release manifest fallback 与 DistFS root override 的 runtime env key 必须只读取 `OASIS7_BUILTIN_WASM_*`；Docker-first build 已完成品牌迁移后，runtime 取件/抓取/编译 fallback 不得继续接受任何旧品牌前缀作为有效运行入口。
- Non-Goals:
  - 不在本专题中要求所有节点运行 Docker 再执行模块；Docker 只解决发布级构建，不进入 runtime 执行热路径。
  - 不在本专题中实现语义等价 canonical hash；当前仍以容器内 canonical packaging 的 byte hash 为准。
  - 不让 CI 变成生产发布者；CI 只验证同一 Docker builder 的 reproducibility。
  - 不保留“不同宿主平台分别产出不同发布 hash”作为长期目标。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用，本专题聚焦容器化构建、工件治理与 runtime binary-only 消费。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview:
  - 源码层：builtin 模块和 `ModuleSourcePackage` 都先形成可审计的源码输入集合。
  - 容器构建层：宿主机通过统一 wrapper 调用 pinned Docker builder image；容器平台固定为 `linux-x86_64`。
  - canonical packaging 层：容器内运行现有 build suite，产出 canonical packaged wasm 与 build receipt。
  - 工件治理层：发布级 manifest 只记录 canonical 容器 hash；identity / release evidence 绑定 build receipt。
  - 执行层：runtime 和节点只消费 canonical binary 与 identity/release evidence，不再依赖源码重编译。
- Integration Points:
  - 现有入口：`scripts/build-wasm-module.sh`
  - 现有构建工具：`tools/wasm_build_suite/src/lib.rs`
  - 现有源码包编译路径：`crates/oasis7/src/runtime/module_source_compiler.rs`
  - 现有工件同步：`scripts/sync-m1-builtin-wasm-artifacts.sh`
  - 现有工件同步：`scripts/sync-m4-builtin-wasm-artifacts.sh`
  - 现有工件同步：`scripts/sync-m5-builtin-wasm-artifacts.sh`
  - 现有 identity 生成：`crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs`
  - 现有对账脚本：`scripts/ci-m1-wasm-summary.sh`
  - 现有对账脚本：`scripts/ci-verify-m1-wasm-summaries.py`
  - 现有 runtime 消费：`crates/oasis7/src/runtime/builtin_wasm_materializer.rs`
  - 现有 release manifest 消费：`crates/oasis7/src/runtime/world/release_manifest.rs`
  - 计划新增：`docker/wasm-builder/Dockerfile`
  - 计划新增：`docker/wasm-builder/README.md`
- Edge Cases & Error Handling:
  - Docker 不可用：canonical build 直接失败，并明确提示“当前宿主不能产出 publishable wasm”；不允许自动回退为 host-native 发布构建。
  - builder image digest 漂移：若 wrapper 与 receipt 中的镜像 digest 不一致，必须阻断发布。
  - 容器内 lockfile / dependency 漂移：继续由 `cargo --locked` 失败并阻断。
  - 容器内 canonical hash 与 tracked manifest 不一致：`sync --check` 直接阻断，输出 `module_id + expected + actual`。
  - macOS/Linux Docker 输出不一致：multi-runner compare 直接阻断；此时问题归因到 builder image / container path，而不是允许回写两个平台 token。
  - runtime 仍尝试生产态 host 编译源码包：必须被配置门禁拒绝，并输出“source compile requires external Docker builder”。
  - Docker Desktop / engine 版本不同：只要同一 builder image 输出 hash 一致即可通过；engine 版本作为诊断信息而不是发布 identity 的一部分。
  - GitHub-hosted `macos-14` runner 无 Docker daemon：允许 CI 暂时退化为 Linux-only stable gate，但 release evidence 必须保留“cross-host pending”状态，并继续要求导入外部 Docker-capable macOS summary，不得静默降级目标态。
  - 外部 `darwin-arm64` evidence 只生成 workflow artifact、未进入 attestation proof payload：不得宣称 production release evidence 已闭环，只能算开发期对账完成。
  - production runtime / node 未启用 hardened release policy：必须在节点验收或 release gate 中直接阻断，因为此时 builtin manifest fallback / 本地签名 / runtime source compile 仍可能穿透 binary-only 契约。
- Non-Functional Requirements:
  - NFR-WDBP-1: 同一源码、同一 builder image digest、同一 `linux-x86_64` container platform 的 canonical wasm hash 可复现率必须为 `100%`。
  - NFR-WDBP-2: Docker canonical build 失败日志必须在一次执行内定位到 `builder_image_digest`、`module_id`、`expected`、`actual` 或容器入口失败原因。
  - NFR-WDBP-3: 发布级 hash manifest 不再依赖宿主平台集合扩容；宿主平台差异不应影响发布 hash 空间。
  - NFR-WDBP-4: 本地默认路径在无显式授权时不得修改 tracked manifest / identity；CI 也不得写入。
  - NFR-WDBP-5: 生产态 `ModuleSourcePackage` 不得要求 runtime 节点本地具备 Docker 才能执行共识路径。
  - NFR-WDBP-6: 发布证据报告若缺少 `darwin-arm64` Docker canonical summary，必须在机器可读输出中明确标示 `cross_host_evidence_pending=true` 或同等阻断状态，避免把 Linux-only 结果误判为 full closure。
  - NFR-WDBP-7: 生产入口 hardened release policy 的默认值必须可由自动化验证，不允许依赖人工约定或测试专用辅助调用。
- Security & Privacy:
  - 工件信任锚点升级为 `builder_image_digest + wasm_hash + artifact identity`，而不是开发机环境。
  - Docker socket 权限只应出现在受控 builder 节点或本地开发机，不应进入普通 runtime 节点的默认生产权限面。
  - 私钥、发布证书与阈值签名分片不得进入 builder image、构建日志或 receipt。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (WDBP-0): 修正文档目标态，明确 Docker-first canonical build、单 canonical publish hash 与 source compile 外移边界。
  - v1.1 (WDBP-1/WDBP-2): 新增 builder image、wrapper script 与 build receipt；把 publish manifest 收敛到单 canonical token。
  - v2.0 (WDBP-3/WDBP-4): 让 multi-runner CI 与 source package 发布流程全面切换到 Docker builder，并收敛 runtime host compile。
- Technical Risks:
  - 风险-1: Docker builder image 维护成本上升，需要明确镜像版本、digest 与升级节奏。
  - 风险-2: 现有 keyed manifest / runtime loader / source compile 路径存在兼容债务，迁移时需要过渡窗口。
  - 风险-3: macOS 开发机在 `linux-x86_64` 容器上构建会更慢，但这是为换取 canonical publish hash 的必要代价。
  - 风险-4（2026-03-18，P0 未收口）: GitHub-hosted workflow 当前仅稳定覆盖 `linux-x86_64`，真实 Docker-capable `darwin-arm64` evidence 仍需外部补证；如果不持续显式追踪，WDBP-3 会被误认为已完成。
  - 风险-5（2026-03-18，P0 未收口）: runtime 生产入口尚未形成“默认 hardened release policy”证据闭环；若只在测试中显式启用生产策略，会让 binary-only / no-fallback 契约停留在文档层。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-020 | WDBP-0/WDBP-1/WDBP-2 | `test_tier_required` | Docker builder image + wrapper script + canonical container output 验证 | publishable wasm 构建入口、容器环境收敛 |
| PRD-WORLD_RUNTIME-021 | WDBP-0/WDBP-2/WDBP-3 | `test_tier_required` | build receipt、single canonical token manifest、identity/release evidence 绑定验证 | 工件治理、发布证据与社会层可验证性 |
| PRD-WORLD_RUNTIME-022 | WDBP-0/WDBP-3/WDBP-4 | `test_tier_required` + `test_tier_full` | multi-runner Docker compare、source package 外部 builder / dev-only gate、runtime binary-only consumption 验证、production release policy 绑定验证 | build drift 阻断、源码包发布边界、执行前合法性 |
- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WDBP-001 | 用 pinned Docker builder image 产出发布级 canonical hash | 继续让宿主原生构建产出各自平台 hash，再靠 keyed manifest 兜底 | 你的目标是从源头解决跨平台漂移，容器固定环境比 host-keyed 兜底更直接。 |
| DEC-WDBP-002 | 发布清单收敛到单个 canonical `linux-x86_64` 容器 hash | 长期保留 `darwin-arm64` / `linux-x86_64` 双发布 hash | 容器平台已固定，宿主差异不应继续进入发布 hash 空间。 |
| DEC-WDBP-003 | runtime / 节点只认 Docker-canonical binary | 节点执行前重编源码并比对 hash | 共识层仍应是 binary-first；Docker 解决的是发布构建，不是执行时重编译。 |
| DEC-WDBP-004 | `ModuleSourcePackage` 生产构建外移到外部 Docker builder | 继续允许 runtime 进程在宿主机直接编译源码包 | 把 Docker daemon 和 host compile 留在 runtime 热路径会扩大权限面，也破坏 canonical publish 契约。 |
