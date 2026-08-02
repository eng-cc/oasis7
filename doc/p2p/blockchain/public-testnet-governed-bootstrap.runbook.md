# Public Testnet Governed Bootstrap Runbook (2026-06-06)

- 对应项目文档:
  - GitHub Issue / GitHub Project
- 关联证据:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md`
  - `doc/testing/evidence/public-testnet-five-node-inventory-2026-06-23.md`

审计轮次: 2

## Stable authority and evidence boundary

- This is the stable operator authority for public-testnet topology, deployment truth, rollout, rollback and recovery drills. The frozen bootstrap topology remains historical provenance; the managed-five-node inventory is an execution target only after a fresh Phase A truth capture. Neither records present fleet health or release readiness by itself.
- Dated manifests, bundles, peer lists, package hashes, status snapshots and evidence files remain evidence-window inputs. Before each deployment, recovery update, or readiness assertion, operators must recapture the current deployment truth and bounded same-window health evidence; QA owns release-blocking verdicts and LiveOps/community owns external status or recovery wording.
- Restart, port cleanup, reseed, data isolation and waiting for recovery may collect evidence or provide temporary recovery only. For peer-head drift, execution/resource divergence, signer/peer identity mismatch, or degraded replication, identify and repair the code, configuration, package, manifest, deployment artifact, or state-contract root cause before clean redeploy/rebuild; never treat an operational restart as the durable resolution.

## 1. Purpose
这份 runbook 的目标不是描述理念，而是定义一条可以重复执行的 `public_testnet` 部署、补更、重建和恢复流程：

1. 从冻结的 governed bootstrap truth 出发。
2. 先拉起 2 个 validator。
3. 再以 observer 身份接入当前 operator inventory 中的非 validator 节点。
4. 将每一步的失败边界压缩为可判定的 gate，而不是运行中再猜。

这份 runbook 明确吸收了本轮 live rebuild 的几个教训：

1. `node-keypair.toml` 是受保护真值，删除它会连带改变派生 signer truth。
2. libp2p bootstrap peer id 是部署真值，不是可以长期硬编码的常量。
3. observer 超过低高度阈值后不能从 genesis 硬 replay；正常路径应由节点自动拉取受验证的高位 replication checkpoint 并补尾。
4. 这里的自动路径是 observer/light-node 的 high-head checkpoint catch-up，不是完整 execution snapshot restore，也不替代正确的 deployment truth。
5. recovery 只可消费 signed V2 checkpoint protocol 产物；该产物必须绑定同一 snapshot/status/latest identity 并携带可验证的闭包。直接从运行中的 validator 拷贝目录（包括 `seed-from-remote`、`cp`、`tar`、`rsync`）已禁用：它们不能保证 `world`、`execution-records`、`store/blobs` 与 replication/bridge metadata 属于同一提交。

## 2. Scope and Topology
冻结的 governed bootstrap 拓扑仍是两台 validator 加两台 observer；当前 operator 部署清单在此基础上扩展为五个受管节点。部署/补更时必须以本节的当前 operator inventory 为准，不能从旧 `.tmp` bootstrap 目录推断节点数量。

### 2.1 Governed bootstrap topology

| node_id | role | location | in validator set |
| --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator / sequencer | ECS `39.104.204.172` | yes |
| `triad-testnet-storage` | validator / storage | ECS `39.104.205.67` | yes |
| `triad-testnet-local` | observer | Linux LAN / local observer family | no |
| `triad-testnet-fourth-local` | observer | macOS local observer family | no |

### 2.2 Current operator inventory

| node_id | role | host / lane | stack root | service manager | status endpoint |
| --- | --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator / sequencer | `root@39.104.204.172` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-sequencer.service` | `http://127.0.0.1:6631/v1/chain/status` |
| `triad-testnet-storage` | validator / storage | `root@39.104.205.67` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-storage.service` | `http://127.0.0.1:6632/v1/chain/status` |
| `triad-testnet-local` | observer | Linux LAN observer | `/opt/oasis7/p2p-testnet-local` | `oasis7-testnet-observer.service` | `http://127.0.0.1:6633/v1/chain/status` |
| `triad-testnet-windows-observer` | observer | Windows observer | `C:\oasis7-deploy` | scheduled task `Oasis7Observer` | `http://127.0.0.1:5121/v1/chain/status` |
| `triad-testnet-fourth-local` | observer | macOS local observer | `$OASIS7_TESTNET_FOURTH_ROOT` | launchd `oasis7.testnet.fourth` | `http://127.0.0.1:19083/v1/chain/status` |

Credential files may be used by an operator as local access aids, but this runbook only records target identities and never records secret values.

### 2.3 Stack roots and deprecated bootstrap dirs

- ECS validator stack root: `/opt/oasis7/p2p-testnet`
- Linux LAN observer stack root: `/opt/oasis7/p2p-testnet-local`
- Windows observer stack root: `C:\oasis7-deploy`
- macOS observer stack root: `$OASIS7_TESTNET_FOURTH_ROOT`
- Old `.tmp/testnet-local-node-bootstrap` and `.tmp/testnet-fourth-node-bootstrap` directories are bootstrap staging artifacts only. If they have no runtime binary, `CURRENT_VERSION`, and service definition, they are not managed installs and must not be counted as deploy targets.

`$OASIS7_TESTNET_FOURTH_ROOT` is the operator-local path for the managed macOS observer root; do not hard-code a user home path in repo docs.

## 3. Truth Model
必须先区分两类真值：

### 3.1 Frozen repo truth
仓库里冻结的治理/启动真值：

- manifest
- genesis
- validator registry
- canonical bootstrap world
- bootstrap peers
- release bundle schema

这些文件定义“网络应当是什么”。

### 3.2 Deployment truth
真正部署时动态生成或动态确认的真值：

- validator host 上实际存在的 `config/node-keypair.toml`
- 从 root node key 派生出的 consensus signer public key
- validator 实际 libp2p `local_peer_id`
- 当前 runtime package sha256
- 当前可用于 observer attach 的 state-sync / seed bundle

这些值定义“当前这一轮 live rebuild 实际用了什么”。

结论:

1. repo truth 冻结后，不代表 deployment truth 永远不变。
2. 只要 validator host key 变化，deployment truth 就必须重新生成。
3. observer 接入必须消费 deployment truth，不能继续使用旧的 bootstrap peer id / signer / seed artifact。

## 4. Hard Rules
下面这些规则必须作为强约束执行：

1. 禁止对 `config/` 目录使用会删除未列出文件的同步方式。
   - 特别是禁止裸用 `rsync --delete` 覆盖整目录。
2. 禁止删除 validator 的 `config/node-keypair.toml`，除非明确要做 key rotation。
3. 一旦发生 key rotation，必须把以下真值一并重建：
   - derived validator signer truth
   - validator registry
   - governed bootstrap world
   - observer `REPLICATION_REMOTE_WRITERS_CSV`
4. libp2p bootstrap peer id 必须在 validator 启动后实时读取，不能沿用旧证据文件中的值。
5. 当 validator 高度超过低高度阈值后，observer 不得使用空 world 从 genesis replay 追链；正常 attach 必须依赖自动 high-head checkpoint catch-up 与 tail gap sync。
6. 自动 high-head checkpoint catch-up 只覆盖 observer/light-node 边界；execution-required 节点不能用它跳过历史执行，也不能把它当作完整 snapshot state-sync。
7. 自动 checkpoint catch-up 失败后，observer 必须先 `reset-state`，再只消费 signed V2 checkpoint protocol 的 recovery 产物；live raw copy（包括 `seed-from-remote`）不得作为 recovery 路径。产物必须包含 execution restore 所需 blob 闭包，否则 observer 会在 restore snapshot/journal 时落入 `BlobNotFound`。
8. 禁止把手工复制 validator 数据目录、手工拷 checkpoint、或从一台 validator 直接覆盖另一台 validator 的 `data/` 目录当作 testnet 同步/恢复流程。validator 恢复只能走两条路径：
   - 先让节点按 manifest bootstrap peers、replication fetch、peer-head exchange 自动恢复同步。
   - 自动恢复失败且根因是 deployment truth 漂移或本地状态污染时，从当前 deployment truth 从零重建 validator pair。
9. 若曾经执行过手工 checkpoint/data copy，该状态只能作为被隔离的故障现场或无效恢复尝试记录；不能作为 readiness 证据、不能继续接在正式 testnet world state 上运行，也不能对外宣称为“已同步”。
10. 遇到多节点分叉、execution/resource 状态不一致、peer head 长期 stale、validator pair 互相拒绝 commit、或类似状态不对的问题，不得把临时运维动作当作最终修复结论。标准处理顺序必须是：
   - 先保留现场并查清根因，至少覆盖代码路径、deployment truth、world/resource snapshot、replication/head exchange 和节点身份。
   - 修复根因对应的代码、配置、包、manifest、部署产物或重建脚本。
   - 再按修复后的 canonical deployment truth 重新部署或从零重建受影响节点。
   - 重启服务、清理端口、手工 reseed、复制数据、或等待自愈只能作为取证/验证手段，不能替代根因修复，也不能作为“分叉已解决”的对外口径。

### 4.1 Runtime high-state sync contract
`public_testnet` observer 的标准 attach 设计是不要求操作者预先提供 seed/state-sync 目录。新 observer 在发现远端链头远高于本地高度时，必须先尝试从 P2P replication 网络拉取一个受验证的高位 execution checkpoint，再从该 checkpoint 后继续 tail gap sync。

运行时实现必须满足以下约束：

1. 候选 checkpoint 不能只看 advertised head 或最近一个对齐边界；必须覆盖 storage profile 保留窗口内的多个 checkpoint 边界。
2. `release_default` 当前按 64 高度间隔保留 8 个 execution checkpoint，因此 observer gap sync 至少要回看 8 个 64-height checkpoint windows。
3. checkpoint commit payload 的 `execution_block_hash`、`execution_state_root` 与 checkpoint descriptor 必须完全匹配，blob 内容必须按 content hash 和 size 校验后才能安装。
4. 如果 advertised head 不是 checkpoint 高度，observer 可以安装 head 之前最近仍被保留且可验证的 checkpoint；安装后再由正常 gap sync 追尾。
5. storage/full-storage provider 即使不是原始 sequencer writer，也必须能在 fetch-commit 响应中为本地 retained execution checkpoint 动态附加 checkpoint descriptor，并以自身 authorized replication writer 身份重新签发增强 commit message；否则只连到 storage peer 的冷 observer 无法获得可安装的高状态入口。
6. 若保留窗口内没有可获取的 checkpoint，状态接口应继续报告 `state_sync_fallback_required=true`，此时才进入 break-glass seed/state-sync recovery。

## 5. Required Inputs
开始前，操作者必须准备好以下输入：

### 5.1 Repo artifacts

- `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world`
- `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`

### 5.2 Runtime/package artifact

必须是 Linux package truth，不允许用本机 macOS binary 直接替代 ECS runtime。

最少校验项：

```bash
./scripts/release-candidate-bundle.sh validate \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json
```

### 5.3 Host access

- validator ECS SSH 可写权限
- local observer stack 可写权限

## 6. Phase Overview
执行顺序固定为：

1. Phase A: Preflight and truth capture
2. Phase B: Build deployment truth
3. Phase C0: Routine CI package update
4. Phase C: Preflight, reset, stage, and rebuild validators when state or signer truth requires it
5. Phase D: Verify validator pair
6. Phase E: Optional recovery seed/state-sync bundle
7. Phase F: Attach or reseed observers
8. Phase G: Final health verification
9. Phase H: Failure handling and rollback

任何 phase 未通过，不进入下一 phase。

## 7. Phase A - Preflight and Truth Capture
### Goal
在任何 destructive 操作前，把本轮 live rebuild 真值先读出来并记下。

### Required checks
1. 优先使用标准 truth capture 脚本：

```bash
./scripts/p2p-public-testnet-capture-truth.sh \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --sequencer-ssh-host root@39.104.204.172 \
  --storage-ssh-host root@39.104.205.67 \
  --out .tmp/public-testnet-deployment-truth.json
```

2. 若只做最小手工校验，至少要覆盖：

```bash
./scripts/release-candidate-bundle.sh validate \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json
```

3. 记录当前 ECS host key material 是否已存在：

```bash
ssh root@39.104.204.172 'ls -l /opt/oasis7/p2p-testnet/config/node-keypair.toml'
ssh root@39.104.205.67 'ls -l /opt/oasis7/p2p-testnet/config/node-keypair.toml'
```

4. 记录当前远端 runtime hash：

```bash
ssh root@39.104.204.172 'sha256sum /opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime'
ssh root@39.104.205.67 'sha256sum /opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime'
```

5. 若 validator 已在运行，读取当前实际 libp2p peer id：

```bash
ssh root@39.104.204.172 'curl -s http://127.0.0.1:6631/v1/chain/status | jq -r .replication.local_peer_id'
ssh root@39.104.205.67 'curl -s http://127.0.0.1:6632/v1/chain/status | jq -r .replication.local_peer_id'
```

### Pass criteria
1. bundle validate 通过
2. 已知每台 validator 的当前 runtime hash
3. 已知每台 validator 的 `node-keypair.toml` 是否会被保留
4. 若节点当前可读，已记录 live `local_peer_id`

## 8. Phase B - Build Deployment Truth
### Goal
把本轮 live deploy 需要的动态真值显式生成出来。

### When to rebuild deployment truth
满足任一条件就必须重建 deployment truth：

1. `node-keypair.toml` 不存在
2. `node-keypair.toml` 被替换
3. validator 实际 `local_peer_id` 与旧 bootstrap peers 不一致
4. validator 派生 signer 与旧 registry 不一致

### Required outputs
至少生成或确认以下内容：

1. current validator signer truth
2. current bootstrap peer ids
3. current deployment-only validator registry
4. current deployment-only bootstrap world
5. current deployment-only manifest/bundle wiring

### Required checks
1. 若 key rotation 发生，先从 host 当前 `node-keypair.toml` 派生 signer truth。
2. 用新的 signer truth 生成 deployment-only validator registry。
3. 用新的 registry 重建 deployment-only governed bootstrap world。
4. 读取 validator 实际 `local_peer_id`，更新当前 deployment truth 中的 manifest/bootstrap peer artifact；manifest-backed public_testnet 启动不再要求 observer env 维护 `REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV`。
5. 更新 observer 的 `REPLICATION_REMOTE_WRITERS_CSV`，对齐当前 deployment truth 中所有 authorized replication writers；除 validator signer 外，必须包含会提供 retained execution checkpoint 的 storage/full-storage provider signer。

### Pass criteria
1. deployment-only registry 与 host 当前 signer truth 一致
2. deployment-only bootstrap world 已重新生成并验证
3. manifest/bootstrap peer artifact 里的 peer ids 与 validator live `local_peer_id` 一致

manifest/bootstrap peer truth 刷新应写回 deployment artifact，例如：

```bash
curl -fsS http://39.104.204.172:6631/v1/chain/status | jq -r '.replication.local_peer_id'
curl -fsS http://39.104.205.67:6632/v1/chain/status | jq -r '.replication.local_peer_id'
$EDITOR doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt
```

`scripts/p2p-public-testnet-refresh-bootstrap-peers.sh` 只适用于 legacy/non-manifest observer env 维护；不得把它作为 formal manifest-backed public_testnet 的必要同步路径。

## 8.5 Phase C0 - Routine CI Package Update
### Goal
在不需要重建 signer/genesis/world 的情况下，把当前五节点升级到同一代码线的 CI artifact，并保留可回滚 release。

### CI package scope selection
1. 只升级 ECS Linux validators 和 Linux LAN observer 时，可使用 `Testnet Packages` 的 Linux artifact。
2. 同时升级 macOS local observer 时，使用包含 `macos-arm64` 的 package run，并分别校验 Linux 与 macOS runtime hash/DMG native identity；当前 arm64 observer 不得由 `macos-x64` artifact 覆盖。
3. 需要升级 Windows observer 时，必须使用包含 Windows artifact 的 CI scope；如果先前 run 只有 Linux/macOS，不得把 Linux artifact 复制到 Windows。
4. 最终 fleet 允许存在 package version 后缀差异，但必须说明原因。当前 workflow 中 `all_existing` 仍只生成 `linux-x64`、`macos-x64`、`windows-x64`；完整 managed-fleet packaging 必须对同一 requested ref/commit 协调执行 `all_existing` plus `linux_macos_arm64` runs，并记录两组 run ID、BUILDINFO 与 SHA256SUMS。验收以 runtime hash、commit lineage、status health 和高度对齐为准。

### Safe update order
1. 记录五节点 `CURRENT_VERSION`、runtime hash、service manager、status endpoint。
2. 先升级两个 validators，并确认 validator pair `ready`、高度推进、互相有 fresh peer head。
3. **Observer checkpoint gate（硬门）**：在任何 Linux、Windows 或 macOS observer 的 mutation command 前，两个 canonical provider 必须从其 manifest `status_url` 同时返回 `chain_proof.latest_execution_checkpoint`。每个对象必须有 `schema_version >= 2`、非空 `checkpoint_id`、64 位 hex `manifest_hash` 和正 `height`；两端 `checkpoint_id`、`manifest_hash` 必须相同，且 `abs(sequencer.height-storage.height) <= 1`。`null`、v1、malformed identity、仅一个 provider、identity mismatch 或 height 不兼容都 fail closed。`p2p-public-testnet-package-rollout.py` 生成的 observer wrapper/PowerShell/macOS script 内置此 gate；不得绕开 wrapper 直接调用 Linux primitive、Windows installer 或 macOS promotion script。
4. gate 失败时，停止 observer rollout；先修复 provider package/config/deployment truth 或等待 pointer+manifest validation 后的 checkpoint 发布，再重新生成并执行 observer plan。不得用 restart、旧 seed 或 validator-to-validator 手工 copy 作为 gate 的替代。
5. 再升级 observers；每个 observer 升级后单独验证，不要把上一个 observer 的 ready 当作整个 fleet ready。
6. 如果 observer 从空状态或旧高度启动后报告 `consensus_peer_head_unavailable` 或 network head 暂不可用，macOS rollout 会在 120 秒启动窗口内重试；持续未就绪会触发 `readiness_timeout` rollback，但不会仅因这两个 transient 信号触发 state-sync escalation。rollback 后若仍有 `replication no connected providers`、长期不追高或再次启动仍无法取得 network head，使用 Phase E/F 的 recovery seed 路径。`execution driver peer mismatch` 仍是立即终止启动等待的 authority failure。
7. 如果 validator pair 自身出现 `execution driver peer mismatch`，不要继续 reseed observers；先恢复 validator pair，再用恢复后的 storage/sequencer fresh state 重新 reseed observers。
8. validator clean rebuild 后，本地 observer 不能只升级 runtime/package 并隐式保留旧 `world` / `world-simulator-mirror` / `execution-records` / `replication-root` / `runtime-root` / `store`。`scripts/p2p-public-testnet-local-node-install.sh` 遇到既有状态默认 fail-closed；同链普通升级必须显式 `--preserve-state`，clean rebuild/redeploy 必须显式 `--reset-state`，随后走自动 checkpoint catch-up 或已验证的 signed V2 checkpoint recovery。不得以 `seed-from-remote` 恢复。
9. 如果 observer 本地 `committed_height` 高于 clean-rebuilt validator pair 的 fresh head，视为旧链状态接入新链；不要用重启作为修复结论，先修正部署/状态合同，再从 clean state 或受信 checkpoint 重建 observer。

### Platform-specific update entrypoints
1. Linux nodes use `scripts/p2p-public-testnet-package-node-upgrade.sh` with the node root and Linux bundle.
2. macOS local arm64 observer uses the generated `macos-arm64` operator script and an explicit `system/oasis7.testnet.fourth` or `gui/<numeric-uid>/oasis7.testnet.fourth` target; do not verify it with the Linux runtime hash or a verification-only DMG check.
3. Windows observer uses the Windows installer artifact, updates `C:\oasis7-deploy\CURRENT_VERSION` and deploy metadata, and restarts scheduled task `Oasis7Observer`.

### macOS arm64 observer fail-closed contract

The package rollout helper generates a plan-only macOS script for the managed observer. It accepts only a concrete `system/<label>` or `gui/<numeric-uid>/<label>` target, derives the matching bootstrap domain (`system` or `gui/<uid>`), and rejects generic `user`, non-numeric GUI identities, or incomplete targets. Before bootout it executes the fail-closed canonical-provider checkpoint gate through native `curl` and `plutil`; this macOS contract does not require Python. It then re-verifies the exact planning-time DMG SHA-256, mounts the DMG at an explicit temporary mountpoint, requires Darwin/arm64 host capability and native arm64 payload identity, and creates a verified attempt-local preflight backup of the active runtime, every declared config, `CURRENT_VERSION`, and `DEPLOYED_BUILDINFO`—but not persistent state while the observer is running. Declared state roots must be normalized, unique, non-nested descendants of the node root and may not equal the node root or overlap/contain the runtime, config, `CURRENT_VERSION`, `DEPLOYED_BUILDINFO`, or launchd plist. The generated shell rejects symlink state roots again before backup, failed-state capture, and restore, before any rollback `rm -rf`.

Next it boots out the original target. If bootout fails, it ensures the untouched original service is loaded and verifies `/healthz` contains `ok=true` and `/v1/chain/status` contains `running=true`; requiring `running=true` from healthz is invalid. After successful bootout—and while the observer is stopped—it copies and verifies every declared persistent-state path. A stopped-state backup failure follows the same original-service restart/health path. Runtime or provenance promotion begins only after this state closure passes, then the script bootstraps the derived domain and checks the same endpoint-specific fields while continuing to inspect the status payload for authority failures.

Any post-promotion failure first preserves the failed active state under the attempt root for evidence. Rollback then replaces every active state path with its stopped pre-upgrade snapshot, restores runtime/config/`CURRENT_VERSION`/`DEPLOYED_BUILDINFO`, bootstraps the original domain, and verifies health. Rollback is idempotent and the attempt root is retained; EXIT cleanup only detaches and removes the explicit DMG mountpoint. During the bounded 120-second startup window, `network_head_available=false` and `consensus_peer_head_unavailable` are transient unready signals: the script keeps retrying, and persistent absence ends in `rollback "readiness_timeout"` without `state_sync_escalation_required=true`. Explicit `state_sync_fallback_required=true`, `authority_failure`, and `execution_driver_peer_mismatch` remain fatal authority signals; the status classifier scans the full alert array and fatal status error fields so a transient alert cannot mask a later fatal signal. An authority failure always rolls back and emits `state_sync_escalation_required=true`, even when failed-state capture or rollback itself cannot complete. It is not permission to restart repeatedly or to use validator data copying as recovery.

The script validates package identity but does not add a signing/notarization requirement; release policy remains the authority if it already supplies one.

## 9. Phase C - Clean Rebuild Validators
### Goal
从零重建 validator pair。

本 phase 的顺序是硬约束：`consumer-impact record gate -> preflight both -> reset both -> stage both -> sequencer liveness -> storage`。任一步未通过，都不得提前执行后一步；特别是 consumer-impact record 未通过时不得建立 SSH/network connection 或执行 host preflight，且不得在只 reset 一台 validator 后向任一 host staging。

### C0. Consumer-impact record (hard gate)

调用标准重建脚本必须传入 `--consumer-impact-record <path>`。脚本在创建 SSH control connection、执行 host preflight、process cleanup、reset 或 staging 前验证该文件。记录必须是 JSON，格式如下：

```json
{
  "impact": "active | none | unknown",
  "evidence_source": "status/traffic/operator/community evidence reference",
  "timestamp": "2026-07-15T10:30:00+08:00",
  "validators_already_stopped": true,
  "outage_update_channel": "channel/reference or n/a when impact=none",
  "recovery_update_checkpoint": "Phase G snapshot/update reference or n/a when impact=none",
  "producer_wording_approval": "approval reference or n/a when impact=none",
  "decision": "proceed"
}
```

这个 record 是 preflight/reset/redeploy continuation 的硬 gate：

1. `impact` 只能是 `active`、`none` 或 `unknown`；`evidence_source` 必须非空；`timestamp` 必须是带 `Z` 或显式 UTC offset 的 RFC3339 时间；`validators_already_stopped` 必须是 JSON boolean；`decision` 必须严格等于 `proceed`。缺失、malformed JSON、字段类型/值无效或 `decision=hold` 都必须 fail closed。
2. `impact=none` 时，三个 communication 字段仍须存在且非空，但允许使用 `n/a`。
3. `impact=active` 或 `unknown` 时，按存在外部 consumer 处理。TPM 与 LiveOps/community 共同负责 outage 和 recovery updates，`producer_system_designer` 批准对外 wording；`outage_update_channel`、`recovery_update_checkpoint` 和 `producer_wording_approval` 必须非空且不能是 `n/a`。
4. 如果 validator 在 record 生成前已经停止，也不能跳过 determination：在任何 host preflight、目录删除、reset、staging、启动或其他 redeploy continuation 前补齐同一记录。已停止这一事实本身不能推断 `active`、`none` 或恢复完成。
5. 本 record 只授权继续执行受治理的 testnet reset/redeploy，不改变 `testnet`、`resettable`、`non-mainnet` 边界，也不授权承诺旧 chain state、testnet asset 或 mainnet continuity。

### C1. Preflight both validators

consumer-impact record 通过后，才在两台 validator 上执行非变更 preflight：确认当前 runtime、repair-rebuild helper、governance registry importer 均可执行，repair helper 提供 `--generated-world-dir` 合同，且远端 Python、tar、systemd 与 process inspection 工具可用。任一 host preflight 失败时，两台 host 都不得进入 reset。

### C2. Reset both validators
停止服务并 destructive reset 旧链状态：

```bash
systemctl stop oasis7-triad-sequencer.service
systemctl stop oasis7-triad-storage.service
```

必须先 quiesce 并 reset 完两台 validator，之后才能向任一 host stage config/world。不要在 sequencer reset 后立即 staging，再处理 storage；该交错会让旧 storage runtime 与新 sequencer staging 短暂共存。

从第一台 validator 停止开始，到 Phase G full-fleet health criteria 全部通过为止，必须按 testnet outage 窗口处理：validator 服务不可用，依赖 validator 的 public RPC、explorer 和 guarded faucet 可能不可用或返回 stale data。不得把缓存可读、单个 endpoint 恢复或 sequencer 单机存活当作网络恢复。

必须清理旧链数据目录，但保留受保护的 `config/node-keypair.toml`，除非本轮明确要轮换 key。

标准重建脚本必须清理以下运行态，以保证“从零重建”不会继承旧 peerstore、旧 runtime root 或未释放端口：

- `data/execution-records`
- `data/execution-world`
- `data/execution-world-simulator-mirror`
- `data/storage`
- `data/runtime-root`
- `data/replication-root`
- `output/chain-runtime`
- `output/node-distfs`

在删除目录前，脚本必须等待或终止 stack-local 残留 `start-node.sh` / `oasis7_chain_runtime` 进程，避免旧进程继续占用 gossip/status/replication 端口。若 `systemctl stop` 后端口仍被旧 runtime 占用，本轮 rebuild 必须视为未清干净，不能继续把后续 readiness 失败归因于链同步慢。

### C3. Stage both validators

reset 两台 validator 完成后，才把以下内容放到两台 host：

- current release package/runtime
- manifest
- genesis
- validator registry
- bootstrap peers
- deployment-only bootstrap world

建议直接使用标准重建脚本，而不是临时拼接远程命令。该脚本已经处理了一个真实踩过的流程坑：不能把同一条 tar 流连续喂给两个远端 `tar -xf -`，否则第二次解包只会读到 EOF 并报 `This does not look like a tar archive`。脚本会先把 world 解到 `staged-world/`，再在远端复制到 `data/execution-world/`：

```bash
./scripts/p2p-public-testnet-rebuild-validators.sh \
  --config-dir .tmp/public-testnet-ci-rebuild-stage/config \
  --world-dir .tmp/public-testnet-ci-rebuild-stage/generated-world-from-rotated-signers/world \
  --consumer-impact-record .tmp/public-testnet-consumer-impact.json \
  --sequencer-ssh-host root@39.104.204.172 \
  --sequencer-sshpass-env PUBLIC_TESTNET_SEQUENCER_SSHPASS \
  --sequencer-service oasis7-triad-sequencer.service \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --storage-ssh-host root@39.104.205.67 \
  --storage-sshpass-env PUBLIC_TESTNET_STORAGE_SSHPASS \
  --storage-service oasis7-triad-storage.service \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --out-dir .tmp/public-testnet-validator-rebuild
```

两台 host 的 package、config 和 deployment-only world staging 都成功后，才进入启动步骤。

### C4. Start sequencer and confirm liveness

1. start `triad-testnet-sequencer`
2. confirm sequencer liveness（`running=true` 且 `last_error` 为空）

sequencer liveness 未通过时不得启动 storage，也不得开始恢复 observer 或对外 endpoint。

### C5. Start storage after sequencer liveness

1. start `triad-testnet-storage`
2. confirm storage joins sequencer

### Validator launch invariants
1. `NETWORK_TIER_MANIFEST_PATH` 必须指向 governed bootstrap manifest
2. `GENESIS_VALIDATOR_REGISTRY_PATH` 必须指向当前 deployment truth registry
3. `EXECUTION_WORLD_DIR` 必须预先放好 deployment truth world
4. runtime binary hash 必须与当前 staged package 一致
5. world staging 必须是“单次传输 + 远端复制”，不能复用同一 stdin tar 流做双重解包

### Deployment artifact retention and cleanup
标准 Linux package 升级入口是 `scripts/p2p-public-testnet-package-node-upgrade.sh`。该脚本默认在每次成功升级后执行 release retention：

1. 永远保留 `<node-root>/current` 解析到的真实 release 目录。
2. 本次升级前的 previous current release 目录也会被保留，作为当前升级尝试的回滚保护。
3. 额外保留 `<node-root>/releases` 下按 mtime 排序最新的 3 个 release 目录。
4. 删除其余非隐藏旧 release 目录；不会把 `<node-root>/data`、`<node-root>/config`、`<node-root>/backups`、service 文件、journal 或 live logs 当作 package 残留清理。
5. 如需临时扩大回滚窗口，可用 `--release-retention-count <N>` 调整最新 release 保留数量；该参数不取消 current realpath 和 previous current 的强保留。

外层 operator/自动化负责清理脚本作用域之外的上传和 driver 临时目录：

```bash
tmp_upload_dir="/tmp/oasis7-upgrade-${run_id}"
cleanup_upload_dir() {
  if [[ "${OASIS7_KEEP_DEPLOY_TMP:-0}" != "1" ]]; then
    rm -rf "$tmp_upload_dir"
  fi
}
trap cleanup_upload_dir EXIT
```

上传目录如 `/tmp/oasis7-upgrade-<run_id>`、per-run driver 目录如 `/tmp/oasis7-run*-<node>` 必须在部署流程成功或失败收口时清理；只有设置明确 debug keep 开关时才允许保留。`<node-root>/tmp` 仅用于本轮 staging/bundle/bootstrap 临时产物，部署成功后应清空本轮产物。`<node-root>/backups` 不属于 package cleanup，必须按单独的备份保留策略处理。

## 10. Phase D - Verify Validator Pair
### Goal
确认两台 validator 形成可持续推进的 governed chain。

### Required checks
对两台 validator 分别检查：

```bash
curl -s http://127.0.0.1:6631/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers,local_peer_id:.replication.local_peer_id}'
curl -s http://127.0.0.1:6632/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers,local_peer_id:.replication.local_peer_id}'
```

### Pass criteria
1. `running=true`
2. `last_error=null`
3. sequencer:
   - `committed_height > 0`
   - `last_execution_height > 0`
4. storage:
   - `committed_height > 0`
   - `network_head.height >= committed_height`
   - `connected_peers` 至少包含 sequencer
5. sequencer 和 storage 互相出现在 `connected_peers`
6. 记录本轮 live `local_peer_id`
7. 若 storage `last_execution_height == 0`，本轮不得把 storage 当作 observer seed source；但只要 storage 已验证并传播 sequencer 的 execution binding，就不应把这件事本身视作 validator rebuild 失败

### Hard stop conditions
出现以下任一项立即停止并回到 Phase B：

1. signer binding mismatch
2. libp2p `WrongPeerId`
3. empty-world `height 1` execution mismatch
4. validator 自身 `BlobNotFound`

## 11. Phase E - Optional Recovery Seed/State-Sync Bundle
### Goal
正常 observer 接入不再要求预先导入 seed。自动 high-head checkpoint catch-up 失败时，先保留故障现场并 `reset-state`；只有 signed V2 checkpoint protocol 的 recovery artifact 可用于恢复。运行中 validator 的目录 raw copy 已禁用，不能作为离线加速或手工恢复来源。

### Required contents when this recovery path is used
`scripts/p2p-export-state-sync-bundle.sh` 的 snapshot-only export 只有在 snapshot、status 与 execution-records `latest` 明确绑定同一 identity 时才是 recovery-capable；否则它只能作为诊断产物，不能恢复 observer。即使 identity 已绑定，它也不是 signed V2 protocol 的完整 closure 替代品。recovery-capable export 至少要包含：

1. checkpoint manifest
2. validator-set manifest
3. state-sync bundle manifest
4. snapshot file / `snapshot_sha256` / `state_root`

signed V2 checkpoint protocol 的完整 recovery closure 还必须覆盖：

1. `world/`
2. `execution-records/`
3. `store/`
4. restore snapshot/journal 所需全部 `store/blobs/`
5. 若运行时依赖存在，还要包含:
   - simulator mirror
   - execution bridge state
   - replication head metadata

### Important rule
不要把“最小 snapshot state-sync bundle”和“完整 seed/restore artifact”混成同一个合同。前者可以没有 journal；后者如果只复制 `world/` 与 `execution-records/`，但没有复制 restore 需要的 blob 闭包，observer 会在如下阶段失败：

- `restore snapshot ref ... BlobNotFound`
- `restore journal ref ... BlobNotFound`

所以完整 recovery export 必须保证 storage closure 完整、checkpoint/status/latest identity 绑定且签名可验证；不能做“最小猜测拷贝”，也不能从运行中 validator 的活目录逐项复制。

### Pass criteria
1. signed V2 checkpoint protocol 产物通过 `p2p-upgrade-preflight.sh --require-state-sync-bundle --verify-state-sync-bundle-semantics`，并验证 snapshot/status/latest identity 与签名。
2. 若使用完整 recovery artifact，对单个 observer 恢复后 runtime 不报 `BlobNotFound`、`authority_failure` 或 execution-driver mismatch。
3. 若使用完整 recovery artifact，可被所有当前 observers 重复消费；raw live copy 不得作为替代。

在 observer 启动前，完整 seed/restore artifact 必须执行闭包校验：

```bash
./scripts/p2p-verify-state-sync-closure.sh \
  --world-dir <seed-world-dir> \
  --execution-records-dir <seed-execution-records-dir> \
  --store-dir <seed-store-dir>
```

## 12. Phase F - Attach Observers with Automatic High-Head Checkpoint Catch-Up
### Goal
把当前 operator inventory 中的 observers 接进 validator 网络，或在旧状态分叉后从恢复好的 validator/storage fresh state 重新 seed。

### Required prep
1. observer manifest 使用当前 deployment truth bootstrap peer ids；legacy/non-manifest observer env 若仍存在，必须标记为旁路兼容信息而不是 formal startup source
2. observer env 使用当前 deployment truth writer allowlist；除 validator signer 外，必须包含会提供 retained execution checkpoint 的 storage/full-storage provider signer。fetch requester 不再需要逐个 observer 手动加 allowlist，只要 providers 运行的 runtime 对 `public_testnet` + `allow_observer_nodes=true` 开启开放签名读取策略
3. observer manifest 指向当前 deployment truth genesis/manifest
4. observer 的 `WORLD_ID`、governed registry/manifest、manifest bootstrap peers、remote writer allowlist、node identity、listen/status ports 必须全部来自当前 deployment truth
5. systemd service unit 必须是当前 testnet observer unit；不得让旧 devnet/triad observer service 继续占用状态端口或被误当作 public testnet 节点
6. observer state 可先 reset；正常路径不导入 seed bundle，启动后由 runtime 自动拉取受验证的 high-head replication checkpoint boundary，再执行 tail gap sync。若该路径 fail closed，则保持 no-start，消费 signed V2 checkpoint recovery artifact；不得从运行中的 validator live directory 导入。
7. local observer 使用的 runtime/package hash 必须对齐当前本机 runtime 真值；不要直接复用 validator Linux package hash 去校验本地 macOS debug/release binary
8. detached local observer 启动时不要让 `start-node.sh` 自己作为长期父进程驻留；若需要后台常驻，应直接启动 `logs/last-command.sh` 里展开后的 runtime binary 命令

### Start / reseed order
1. `triad-testnet-local` on Linux LAN, then verify.
2. `triad-testnet-windows-observer` on Windows, then verify.
3. `triad-testnet-fourth-local` on macOS, then verify.
4. If all observers were seeded before a validator recovery, reseed all affected observers again from the recovered storage/sequencer state.

### Required checks
```bash
ssh <linux-lan-observer> 'curl -fsS http://127.0.0.1:6633/v1/chain/status' \
  | jq '{node_id,running,last_error,readiness:.readiness.status,failed_gates:.readiness.failed_gates,committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers}'
ssh <windows-observer> 'powershell -NoProfile -Command "Invoke-RestMethod -UseBasicParsing http://127.0.0.1:5121/v1/chain/status | ConvertTo-Json -Compress -Depth 8"' \
  | jq '{node_id,running,last_error,readiness:.readiness.status,failed_gates:.readiness.failed_gates,committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers}'
curl -fsS http://127.0.0.1:19083/v1/chain/status \
  | jq '{node_id,running,last_error,readiness:.readiness.status,failed_gates:.readiness.failed_gates,committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers}'
```

Use each node's actual `STATUS_BIND` from its env/deploy metadata when it differs from the legacy examples above.

### Pass criteria
1. `running=true`
2. 所有当前 observers 都能看到 validator peer
3. `last_error=null`
4. `committed_height` 和 `last_execution_height` 向 validator 高度收敛

### Hard stop conditions
出现以下任一项，observer 接入失败；先保留现场排查自动同步原因，确认需要人工恢复时再回到 Phase E：

1. `WrongPeerId`
2. `fetch requester is not authorized`；这通常表示 validator 仍在旧 runtime、manifest 不是 `public_testnet`、`allow_observer_nodes` 未开启，或 requester 签名无效，而不是需要手动登记每个 observer
3. `BlobNotFound`
4. `height 1 peer commit execution mismatch`
5. `network tier runtime bundle hash mismatch`

## 13. Phase G - Final Health Verification
### Goal
确认“节点已跑起来”和“整个 fleet 真正 ready”不是一回事，并分别打结论。

### Required final snapshot
需要同时保留所有 current operator inventory 节点的状态快照，至少包括：

1. `running`
2. `last_error`
3. `committed_height`
4. `network_committed_height`
5. `last_execution_height`
6. `connected_peers`
7. `readiness.ready`
8. `readiness.failed_gates`
9. `consensus.network_head.decision`

在作出 `Fleet healthy` 或 recovery update 结论前，使用 collector 生成唯一的 bounded same-window artifact；它会逐节点记录 UTC capture timestamp、全局 UTC start/finish timestamp 和 capture span，并在 span、head、readiness、failed gate、last error 或 network-head decision 不满足时 fail closed：

```bash
./scripts/p2p-public-testnet-fleet-health.py \
  --managed-five-node \
  --sequencer sequencer \
  --node sequencer=http://39.104.204.172:6631/v1/chain/status \
  --node storage=http://39.104.205.67:6632/v1/chain/status \
  --node linux-lan-observer=http://<linux-lan-observer>:6633/v1/chain/status \
  --node windows-observer=http://<windows-observer>:5121/v1/chain/status \
  --node macos-observer=http://127.0.0.1:19083/v1/chain/status \
  --max-capture-span-seconds 30 \
  --output .tmp/public-testnet-fleet-health.json
```

`--managed-five-node` is mandatory for this deployment closure. It fail-closes unless the supplied identities are exactly `sequencer` (ECS 204 provider), `storage` (ECS 205 provider), `linux-lan-observer`, `windows-observer`, and `macos-observer`, once each. The collector without that flag remains only for explicitly non-closure diagnostics and cannot be used for a `Fleet healthy` or recovery conclusion.

### Verdict rules
1. **Fleet live**
   - 当前 operator inventory 中所有节点都在跑
   - validator 与 observer 都已接入
   - 高度在收敛或已收敛

2. **Fleet healthy**
   - 在满足 fleet live 的基础上
   - `last_error=null`
   - 不存在阻断性 gate
   - observer 不再依赖缺失 blob / 缺失 state-sync artifact
   - `readiness.status=ready`
   - `readiness.failed_gates=[]`
   - `consensus.committed_height`、`consensus.network_committed_height`、`consensus.last_execution_height` 与 validator head 对齐或在允许 lag 内
   - `consensus.network_head.decision=ready`
   - 没有 repository-defined lag threshold 时，所有 current operator inventory 节点必须在**同一个 same capture window** 的快照中，使 `consensus.committed_height`、`consensus.network_committed_height`、`consensus.last_execution_height` 都与 sequencer validator head **exactly equal**；不得把不同采样时间的“分别已追平”拼成 fleet healthy 结论

如果只满足第一条，不得对外宣称 healthy、恢复完成或“完全健康”。

### Operator communication boundary

1. **Merge announcement: n/a.** 合并 runbook、代码或 package 变更不等于 live deployment 已执行，不发布“网络已恢复”或“部署已完成”的 merge announcement。
2. Phase C0 的 consumer-impact record 是 deployment/outage messaging 的来源。`active` 或 `unknown` 都必须按存在外部 consumer 处理：TPM 与 LiveOps/community 负责 outage 和 recovery updates，`producer_system_designer` 批准 wording，并在任何 host preflight/reset/redeploy continuation 前记录 update channel、recovery checkpoint 和 approval reference。只有 `none` 且 evidence source 与 timestamp 已留存时，才可只保留内部 operator 记录。validator 已停止也不豁免这项 determination 或沟通 gate。
3. 需要对外说明时，必须使用 `testnet`、`resettable`、`non-mainnet` 边界，例如：`Oasis7 public testnet is undergoing a governed clean rebuild. This testnet is resettable and non-mainnet. Validators are temporarily unavailable; RPC, explorer, and guarded faucet may be unavailable or stale until full-fleet verification completes.`
4. validator pair、单个 observer、RPC、explorer 或 faucet 单独恢复都不能触发 healthy announcement。只有本 Phase 的 `Fleet healthy` 全部满足、每个节点的 `consensus.network_head.decision=ready`、并留存满足上述 exact-equality same-capture-window head criterion 的 full-fleet snapshot 后，才可由已记录的 recovery update checkpoint 说明服务恢复；恢复说明仍必须保留 `testnet`、`resettable`、`non-mainnet` 边界，不得承诺旧 chain state、testnet asset 或 mainnet continuity。

## 14. Phase H - Failure Handling and Rollback
### Deployment rollback
如果 validator 新包启动失败：

1. 切回上一版 `current` symlink
2. 恢复上一版 manifest/config
3. 保留失败现场日志和当前 deployment truth 快照

上述 package/config rollback 只恢复 binary 与配置选择，不会恢复 Phase C 已删除的 chain state。只要 destructive reset 已发生，就不得把 symlink/config 回切描述为链状态回滚或完整恢复；canonical recovery 是先修正 package/config/deployment truth，再按 Phase C clean rebuild validator pair，并按 Phase F reset/reseed 所有受旧链状态影响的 observers。

### Observer rollback
如果 observer 接入失败：

1. 不改 validator 链真值
2. 保留 observer 当前 seed/state-sync 失败现场
3. reset 受旧链状态影响的 observer，再从 clean-rebuilt storage/sequencer state 自动 catch-up 或 signed V2 current-chain recovery artifact 恢复
4. 不得使用绑定旧链状态的 seed bundle，也不得保留旧 observer state 重新接入 clean-rebuilt validator pair

### Windows package rollback 后的人工 handoff

Windows package rollout 只要输出 `rollback_required=true`，就进入仍在进行中的 incident；`rollback_applied=true` 只证明已尝试恢复 known-good binary/config/deployment provenance，不代表 observer 已恢复，也不得触发 recovery messaging。值班 operator 必须先保留 rollout 输出列出的 `attempt_stdout`、`attempt_stderr`、`attempt_exit_marker` 及 `C:\oasis7-deploy\logs\package-rollout-attempts\` 下对应 attempt 文件；不得在 incident closeout 前删除 attempt staging、覆盖日志或只摘录最后一行替代原始证据。随后采集 `staged_sha_closure_complete=true`、`promotion_begin=true`、`promotion_complete=true` 三个 marker 是否存在，以及 `Get-ScheduledTask` 的实际 state/action，并按下列分支 handoff：

1. **Preflight / closure failure, no restart.** 若缺少 `staged_sha_closure_complete=true` 或 `promotion_begin=true`，则 promotion/stop boundary 未获证明；即使 task 当前为 `State=Ready`，也不得执行 `Start-ScheduledTask`。保留 preflight/closure diagnostics，修复 staging closure、路径、hash、rollback root 或 task-action precondition 后重新生成并执行一次受控 rollout；若观察到 `State=Running`，也只保留现场并排查，不得以手工 restart 覆盖 preflight failure。
2. **Promotion entered but not completed.** 若有 `promotion_begin=true` 但没有 `promotion_complete=true`，保留所有 marker 和 attempt diagnostics，并把 observed task state 作为 stop 的事实来源：生成器没有独立的 stop marker，不能从 `promotion_begin=true` 推断 task 已停止。只有同时得到 `rollback_required=true`、`rollback_applied=true`，并确认 task 为已停止的 `State=Ready` 时，才进入下一分支；`State=Running`、缺失 state 或任何其他 state 都继续 incident，禁止手工 start。
3. **Restored post-stop rollback.** 仅当 `promotion_begin=true` 已记录、`rollback_applied=true` 已记录、installed/known-good runtime SHA-256、active governed config、`CURRENT_VERSION` / `DEPLOYED_BUILDINFO` 和 scheduled task action 都已闭合，且当前观察到 `State=Ready` 时，Windows node operator 才可人工执行 `Start-ScheduledTask -TaskName Oasis7Observer`。rollout 的“不自动重启”是安全边界；不得用修改脚本或删除诊断绕过。若已观察到 `State=Running`，不得再次 Start，先采集状态和日志。
4. 在任何手工 start 后，运行本 runbook `Current five-node status` 的现有 Windows/fleet status checks，并记录 scheduled task `State=Running`、RPC `running=true`、`last_error=null`、`readiness.status=ready`、`readiness.failed_gates=[]`、每个节点 `consensus.network_head.decision=ready`，以及满足本 Phase exact-equality same-capture-window criterion 的 `consensus.committed_height`、`consensus.network_committed_height`、`consensus.last_execution_height`。任一项不满足时，`rollback_required=true` incident 继续 active。
5. 在 task running、error clear、readiness、network-head decision 和 full-fleet same-window head checks 全部通过且证据已留存前，禁止发布“已恢复”“healthy”或同义 external messaging。外部 recovery update 仍遵循上文 `Operator communication boundary`：TPM 与 LiveOps/community 准备并发布 update，`producer_system_designer` 批准 wording；Windows operator 的本机检查结果本身不构成对外发布批准。

最小 Windows handoff 命令：

```powershell
Get-ScheduledTask -TaskName Oasis7Observer | Select-Object TaskName,State,Actions
Get-Content -LiteralPath '<attempt_stdout>','<attempt_stderr>','<attempt_exit_marker>'
Get-ScheduledTask -TaskName Oasis7Observer | Select-Object TaskName,State
Invoke-RestMethod -UseBasicParsing http://127.0.0.1:5121/v1/chain/status |
  ConvertTo-Json -Depth 8
```

### What not to do
1. 不要为 observer attach 问题去修改 validator registry 真值
2. 不要在 peer id 已变的情况下继续使用旧 bootstrap peer list
3. 不要在缺少完整 blob 闭包时宣布 state-sync 已可用
4. 不要手工把 sequencer 的 checkpoint、execution world、execution records、storage 或 replication root 拷到 storage validator 上来“同步”。这种做法绕过了 manifest/bootstrap peer/replication 协议和 validator signer binding，不能作为 testnet 恢复路径。
5. 不要在一次手工 copy 后继续等待它“自然变 ready”并把结果记为自动同步；live raw copy 已禁用，必须隔离既有状态，回到自动恢复或 signed V2 checkpoint recovery。

### Allowed recovery choices
当 validator pair 出现 `readiness=not_ready`、peer-head stale、state-sync fallback 或 replication transport degraded 时，operator 只能按下面顺序处理：

1. **自动恢复**：保持 current deployment truth 不变，修复 manifest bootstrap peers、端口占用、runtime 进程残留、network reachability 或 provider discovery 后，让节点通过 replication/head exchange 自己追平。
2. **从零重建**：如果自动恢复被 signer drift、runtime bundle hash drift、stale bootstrap peer id、stale peerstore 或本地链状态污染阻断，则重新生成 deployment truth，清空 validator runtime data/root，按 Phase C 重建 validator pair。
3. **signed V2 checkpoint recovery**：只适用于 observer/light-node recovery 或经验证的 break-glass restore drill；必须有签名 checkpoint、validator-set proof、snapshot/status/latest identity binding、bundle manifest 和闭包校验。它不是 validator-to-validator 手工同步，也不允许 live raw copy。

## 15. Standard Command Checklist
### Preflight
For current five-node fleet preflight, first load each managed node's real env/deploy metadata and use its actual status bind. The legacy `.tmp/testnet-*-bootstrap/node.env` paths below are only valid when intentionally rebuilding those bootstrap staging directories.

```bash
./scripts/p2p-public-testnet-preflight.sh \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --sequencer-ssh-host root@39.104.204.172 \
  --sequencer-sshpass-env PUBLIC_TESTNET_SEQUENCER_SSHPASS \
  --storage-ssh-host root@39.104.205.67 \
  --storage-sshpass-env PUBLIC_TESTNET_STORAGE_SSHPASS \
  --observer-env <current-linux-lan-observer-node.env> \
  --observer-env <current-macos-observer-node.env> \
  --out-dir .tmp/public-testnet-preflight
```

### Package version replacement
```bash
./scripts/p2p-public-testnet-package-rollout.py \
  --manifest .tmp/public-testnet-package-rollout/nodes.json \
  --package-dir .tmp/testnet-packages/<run-id> \
  --out-dir .tmp/public-testnet-package-rollout \
  --readiness-policy rpc-running
```

默认模式只生成计划，不改节点；脚本会校验各平台 `*-BUILDINFO` 和 `*-SHA256SUMS`，并输出 Linux/Windows operator 命令和 `rollout-plan.json`。本地 Linux 节点需要显式加 `--apply-local` 才会调用 `p2p-public-testnet-package-node-upgrade.sh` 执行替换；远端 SSH、Windows PowerShell 和凭据注入仍由 operator 在脚本外执行，manifest 不写密码。

Windows 计划要求 `windows-x64-SHA256SUMS` 覆盖并传输受治理的 bundle、genesis、network manifest、bootstrap peers、`governance_bootstrap_refs` 中全部非空 evidence 源，以及 public-testnet 必需的 world snapshot、递归 generated-world sidecar 和 world-generation provenance。每个带 `host` 的远端 Windows node manifest entry 必须显式声明非空 `rollback_backup_root`，并指向该节点已验证的时间戳备份目录；计划生成不得为远端节点隐式回退到 `backups\known-good`。每个计划按 node/version/commit/run 派生唯一 attempt id，installer、完整 governed closure 和 upgrade script 只能传到 `C:\oasis7-deploy\staging\package-rollout\<attempt-id>`；pre-apply 命令不得写 `config/`、install root 或其他 active deployment 路径。每次传输前先在远端创建并确认 staging 父目录，传输后按精确预期 SHA-256 远端复核并输出 `staging_transfer_ack`；全部传输闭合后，apply 必须使用精确的 `user@host` SSH target，并通过同步的 `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <staged-script>` 子进程从该 attempt staging root 执行脚本，再把子进程 `$LASTEXITCODE` 原样返回给 SSH。远端 PowerShell 在停止 scheduled task 或执行 installer 前初始化唯一、只追加的 stdout、stderr、exit-marker 诊断，完成 staged 文件与目录树完整性、来源路径、basename 冲突及已配置 known-good rollback root 预检，并依次输出 `staged_sha_closure_complete=true`、`promotion_begin=true`；只有此后才能停止 task、安装 runtime 或把 staged config 通过临时文件提升到 active tree，全部提升完成后输出 `promotion_complete=true`。

rollback runtime 优先从唯一的 `backup-manifest.json` 或 `backup-provenance.json` runtime path 解析，并在声明 SHA-256 时闭合校验；没有 manifest/provenance 时仅支持按顺序检查 `runtime/oasis7_chain_runtime.exe`、`bin/oasis7_chain_runtime.exe`，两者同时存在或都不存在均 fail closed，且最终路径必须保持在 rollback root 内。该 root 还必须提供与 node-local 相同相对路径的 `config/` bundle/genesis/manifest，以及 `CURRENT_VERSION`、`DEPLOYED_BUILDINFO`。脚本把所有已知 path-bearing bundle/manifest 字段改写为 active node-local 路径，并通过结构化路径分类拒绝残留 Unix/build-worktree 或非 deploy/install root 路径，随后用无 BOM UTF-8 临时文件原子替换本地化配置。常规替换使用 `rpc-running`，在 60–300 秒有界窗口内同时要求 runtime process 与 RPC `running=true`；`strict-ready` 另要求可用的 readiness truth 为 true。scheduled-task child 一旦终止且结果非零（包括 `255`），脚本立即保留并输出本次诊断文件的精确路径、传播 child exit code，并以 `rollback_required=true` 进入 known-good rollback。rollback 在任何 runtime 解锁等待前先比较 installed 与 known-good SHA，并输出 `rollback_runtime_restore_required`：hash 相等时跳过 runtime unlock/replacement；hash 不等时才执行有界解锁和 runtime 恢复。config 与 deployment provenance 也逐项比较 SHA，只恢复发生变化的 component，并输出 `rollback_component_restored` 或 `rollback_component_unchanged`。整个路径保留 identity/state；锁仍阻断确需执行的 runtime 恢复时才以 `lock_remains=true` fail closed，且不自动重启。

### Validator status
```bash
ssh root@39.104.204.172 'curl -s http://127.0.0.1:6631/v1/chain/status'
ssh root@39.104.205.67 'curl -s http://127.0.0.1:6632/v1/chain/status'
```

### Current five-node status
```bash
ssh root@39.104.204.172 'curl -fsS http://127.0.0.1:6631/v1/chain/status'
ssh root@39.104.205.67 'curl -fsS http://127.0.0.1:6632/v1/chain/status'
ssh <linux-lan-observer> 'curl -fsS http://127.0.0.1:6633/v1/chain/status'
ssh <windows-observer> 'powershell -NoProfile -Command "Invoke-RestMethod -UseBasicParsing http://127.0.0.1:5121/v1/chain/status | ConvertTo-Json -Compress -Depth 8"'
curl -fsS http://127.0.0.1:19083/v1/chain/status
```

For each output, record `CURRENT_VERSION`, runtime hash or artifact lineage, `running`, `last_error`, `readiness.status`, `readiness.failed_gates`, `consensus.committed_height`, `consensus.network_committed_height`, `consensus.last_execution_height`, and `consensus.network_head.decision`.

### Peer id truth
```bash
curl -fsS http://39.104.204.172:6631/v1/chain/status | jq -r '.replication.local_peer_id'
curl -fsS http://39.104.205.67:6632/v1/chain/status | jq -r '.replication.local_peer_id'
cat doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt
```

### Seed closure
```bash
./scripts/p2p-verify-state-sync-closure.sh \
  --world-dir <seed-world-dir> \
  --execution-records-dir <seed-execution-records-dir> \
  --store-dir <seed-store-dir>
```

## 16. Open Design Follow-Ups
这份 runbook 可以让部署更稳，但它也明确保留两项后续硬化方向：

1. 自动 high-head checkpoint sync 已覆盖 observer/light-node 的正常接入；execution-required 节点仍不能跳过历史执行
2. snapshot-only export 仅在 snapshot/status/latest identity 绑定后才可作为 recovery-capable input；完整 signed V2 checkpoint recovery closure 必须作为标准 artifact 提供，不能再依赖 live raw copy

所以后续必须补两类自动化：

1. 空 world observer 自动 high-head checkpoint sync 对 validator truth 的回归测试
2. signed V2 checkpoint recovery artifact 的闭包与 identity-binding 回归测试，至少覆盖 restore snapshot/journal 所需 blob 闭包和并发写入拒绝

## 17. Completion Boundary
这份 runbook 的交付物是“标准流程”，不是单次 live 执行记录。

每次真实执行仍然必须额外记录：

1. 当前 runtime/package hash
2. 当前 validator signer truth
3. 当前 validator live peer ids
4. 自动 high-head checkpoint catch-up 状态快照；若使用 recovery path，再记录 observer seed bundle 来源与 hash
5. 最终 current operator inventory 全节点状态快照
