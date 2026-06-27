# oasis7 Runtime：WASM Docker 确定性构建与工件治理管线设计

- 对应需求文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`

审计轮次: 3

## 1. 设计定位
本设计把 oasis7 的 WASM 发布级构建从“宿主机护栏 + keyed hash 对账”升级为“Docker canonical builder”。设计目标不是否定现有脚本和 build suite，而是把它们放进同一个 pinned 容器镜像里，让宿主平台不再参与发布 hash 的生成。

## 2. 现状盘点

| 层级 | 当前实现事实（2026-03-18） | 与 Docker-first 目标的差距 |
| --- | --- | --- |
| 容器基础设施 | `docker/wasm-builder/Dockerfile`、`scripts/build-wasm-module.sh` 与 canonical builder digest 已落地。 | canonical builder 已存在，但跨宿主证据仍未完整归档。 |
| 构建入口 | `scripts/build-wasm-module.sh` 已强制 `docker run --platform linux/amd64`，且显式拒绝 host-native fallback。 | 入口已收敛，但仍需把“stable gate”和“cross-host full closure”严格区分。 |
| 构建工具 | `tools/wasm_build_suite` 已在容器内输出 `build receipt`、`source_hash`、`build_manifest_hash`、`builder_image_digest` 与 `container_platform`。 | receipt 语义基本完成，剩余重点在 evidence 汇总与 release gate 结论。 |
| manifest 策略 | builtin hash manifest 已改为单 canonical token `linux-x86_64=<sha256>`；identity 生成已切换为 receipt 驱动。 | 写路径已收敛，但 cross-host report 还未拿到真实 `darwin-arm64` Docker evidence。 |
| source compile | `compile_module_artifact_from_source` 仍保留源码包编译能力，但 production `ReleaseSecurityPolicy` 已默认拒绝该 action；仅 dev/test 可显式使用。 | external builder worker 仍未落地，当前以 production default-disable 先收敛 runtime 权限面。 |
| CI 校验 | `.github/workflows/wasm-determinism-gate.yml` 当前已收敛为 GitHub-hosted `linux-x86_64` stable gate，并支持导入外部 summaries。 | 真实 Docker-capable `darwin-arm64` summary 仍未进入正式 evidence 报告，`WDBP-3` 未完成。 |
| runtime 消费策略 | runtime 侧已支持 production policy 关闭 source compile / local signing / builtin fallback，但主要通过显式启用 production policy 进入。 | 主运行入口尚缺“默认 hardened policy”绑定证据，binary-only 仍未完全提升为产品默认事实。 |

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
这是当前 `WDBP-3` 的 P0 剩余切片。设计目标不是让 GitHub-hosted CI 假装跨宿主完成，而是把“稳定 gate”和“最终证据”分层。

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
  "cross_host_evidence_pending": true,
  "expected_runners": ["linux-x86_64", "darwin-arm64"],
  "received_runners": ["linux-x86_64"],
  "gate_result": "conditional-go"
}
```

#### 5.8.3 External Evidence Dispatch Runbook
当真实 Docker-capable `darwin-arm64` runner 可用后，full-tier 证据归档流程固定为：

优先使用手动 CI 路径：

1. 在 GitHub Actions 手动触发 `Wasm Darwin Docker Evidence`。
2. `runs_on_json` 填写真实 Docker-capable macOS ARM64 self-hosted runner labels，默认值为 `["self-hosted","macOS","ARM64","docker"]`。
3. `collect-darwin-docker-summaries` job 会做 `Darwin arm64 + docker linux/amd64` preflight，并产出 `darwin-arm64-wasm-summary-bundle` artifact。
4. `verify-with-linux-summaries` job 会下载该 artifact、收集 `linux-x86_64` summaries、导入 darwin bundle，并产出 `darwin-arm64-wasm-release-evidence-report` artifact。
5. 该 report 的 `summary.json` 满足下方 closure 条件后，才可把 `WDBP-3.2` 视为完成。

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
- 正式 closure 证据应归档 workflow run URL 与对应 report artifact，供 `qa_engineer` 回归复核。

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
这是另一个 P0 剩余切片。当前代码已具备 hardened policy 结构体与 helper，但“默认生产入口启用”还缺显式绑定证据。

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
`WDBP-4` 解决的是“source compile 在 production 默认被拒绝”；`WDBP-3` 当前剩余的是“把同类 hardened policy 变成主入口默认事实并写出证据”。

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
