# 持久世界与节点生命周期实施记录

日期：2026-10-12。对应需求见 [正式网络分层 PRD](formal-network-tiers-testnet-mechanism.prd.md)，部署边界见 [现行 governed bootstrap runbook](public-testnet-governed-bootstrap.runbook.md)。

本轮依据同日《oasis7 正式持久世界与节点生命周期优化方案》实施。交付围绕同一部署优化目的，原子接通正式持久世界运行与恢复、节点接入、成员追踪与部署管理；根据 2026-10-14 用户快速合入要求，不按 crate 数量机械拆分。本文记录已接通的入口及未完成的验收，不把工程 prerequisite 解释为正式开放。

用户选择先完成代码和本机验证。本轮没有真实故障域、生产部署或公开开放的验收结论。

## 本机正式入口验收进度（2026-10-14）

这些结果来自不同的串行测试与本机进程运行，不能合并解释为整套部署计划已通过。历史失败只在下表注明的场景仍未重测时作为未解决项；已修复的旧错误不作为当前阻断。

| 范围 | 最新证据 | 状态 |
| --- | --- | --- |
| `oasis7` 正式库 | 2026-10-14 测试模块拆分后 canonical `v9`：80 项通过，109.45 秒，含 effect 身份、retention 拆分及 cursor 重基；严格 library clippy `v1` 通过，36.62 秒；日志 `/tmp/oasis7-lifecycle-formal-lib-canonical-20261014-v9.log`、`/tmp/oasis7-lifecycle-lib-clippy-canonical-20261014-v1.log` | 通过该测试组与 library clippy，binary 另验 |
| 正式读取及 cursor 恢复 | canonical `formal_read` 库组 `v7`：16 项通过，11.93 秒，含 33 个实际签名提交后的安全重基、fork/older/stale-CAS 拒绝；fixture `v31` 三项实际 CLI/IO 测试通过：正式 watch 1.14 秒、工程 watch 0.37 秒、替换后目录同步失败报告 0.04 秒；正式场景含断线重启、旧响应/旧切面及重复、重排和绑定篡改游标拒绝；日志 `/tmp/oasis7-lifecycle-watch-cli-fixture-20261014-v3.log` | 该库组及三项实际场景通过 |
| Launcher typed 开发配置 | 2026-10-14 canonical `v3`：176 项通过、1 项忽略，1.08 秒；含新适配器的 dev node/local-provider 参数及 typed 配置、formal synthetic 拒绝、路径穿越/原始空白 ID 拒绝；严格 launcher clippy `v2` 通过，4.13 秒；日志 `/tmp/oasis7-lifecycle-launcher-full-canonical-20261014-v3.log`、`/tmp/oasis7-lifecycle-launcher-clippy-canonical-20261014-v2.log` | 通过该 unit 测试组与 production clippy，真实入局未验 |
| 开发及脚本适配合同 | fixture `v29`：typed 配置两项、正式 signer 完整性一项、live/显式 P2P hint 两项、inspect 无副作用一项通过；node-start 与 observer-sync shell 测试通过，日志 `/tmp/oasis7-lifecycle-node-start-adapter-20261014-v1.log`、`/tmp/oasis7-lifecycle-observer-sync-adapter-20261014-v1.log` | 参数/适配与拒绝合同通过，shell stub 结果不作为真实节点部署证据 |
| 真实持久化格式探针 | 2026-10-14 canonical `shipping_compatibility_descriptor_` `v4`：4 项通过，30.63 秒；真实 driver 产生 World、checkpoint、Snapshot/Journal CAS，验证合法滞后缓存可读取、权威 CAS 替换拒绝、selector 0/999 拒绝、latest 低于更高异常记录拒绝，探针前后原目录和文件 SHA 不变；日志 `/tmp/oasis7-lifecycle-shipping-compat-probe-canonical-20261014-v4.log` | 通过该实际 decoder/CAS 集成测试组，Package 真实升级/回退仍待验 |
| full-sync 合同及离线探针 | 2026-10-14 canonical `formal_full_sync` `v2`：24 项通过、4 项忽略，5.87 秒；实际签名 HTTP 的在线备用端点/错误 pin 验证，以及在线取得 successor proof 后停服的完整离线 C0/CAS 重放探针；日志 `/tmp/oasis7-lifecycle-fullsync-contract-canonical-20261014-v2.log` | 通过此组合，忽略的普通 CLI 子进程另验；离线探针不声明联系过 provider |
| `oasis7_chain_runtime` formal 测试 | canonical no-run 构建所得 fixture `v31` 串行 formal 组：123 项通过、20 项忽略，16.73 秒；日志 `/tmp/oasis7-lifecycle-runtime-formal-fixture-20261014-v31.log`，含实际格式探针、正式 watch 和原有归档/grant 回归；忽略项不计入通过 | 通过该测试组 |
| `oasis7_distfs` canonical | 空子树、Business CAS 打包及 TCP 小包合并调整后完整库串行 `v7`：199 项通过、0 项失败、10 项忽略，428.79 秒，含真实 shared-index 非法第二边、可写 staging 拒绝及 SIGKILL 落盘边界；TCP 小包合并调整后 all-targets 严格 clippy `v10` 通过，3.20 秒。端点 release `v9` 已构建（52.01 秒），opt-level=3、test=false、debug-assertions=false | 通过该测试组及构建，忽略项另需实际运行 |
| `oasis7_node` canonical | clock 恢复后 `84646`：517 项通过，89.98 秒；定向 clock 回归 3 项通过 | 通过该库测试组 |
| Native TCP 传输 | 原 30 秒整次冷恢复预算的历史材料 RPC，调整前单独重测 37.48 秒、调整后 24.43 秒通过（含 setup）；9 项传输回归通过、1 个 helper 忽略，3.10 秒，含慢碎片不续期、签名与重放拒绝；仅配置 TCP_NODELAY，帧与超时不变 | 定向回归及调整后完整库 `v7` 通过：199 项通过、0 项失败、10 项忽略；原完整库 RPC 超时已关闭 |
| Native 双端实际进程 | release 端点 `v5` 加 canonical test harness：双端资格确认后 SIGKILL/重开 `v6` 通过，1.49 秒；恢复导入及连续 epoch 原请求续交 `v4` 两项通过，4.33 秒。受保护 provider 为本机协议模拟 | 通过这三项真实子进程场景 |
| Native 长期持久化 C10 | 业务 pack 收窄后的 canonical 显式运行 `v6` 通过，346.21 秒：71 条 Qualified、68256671 bytes、五次重开，当前 head 71 仍取回 position 2 的原结果；使用真实本地存储及原生库，不是 World/Wasmtime 长期进程测试 | 通过该原生库场景 |
| R-only Frozen/Prepared 实际运行 | `41571`：152.21 秒；最大结果延迟 57 ms、inclusion 18 ms、整轮 749 ms | 通过该场景 |
| Zero/genesis 实际运行 | `46981`：33.52 秒 | 通过该场景 |
| C03 持久重开 | 完整八阶段实际运行 `60429` 通过，536.86 秒；每阶段均由原请求恰好一次 Qualified，并精确重开两次。`primary_published` 另有强制删除本地 C0 材料缓存后的实际通过（80.95 秒） | 八阶段通过 |
| 损坏候选拒绝 | `corrupt_existing_candidate_or_frozen_artifact_refuses_prepare_without_promotion`，运行 `3416`：通过，3.69 秒 | 通过该定向回归 |
| World 长期历史 | `v13`：653.22 秒通过，70 次真实签名更新，捕获切面 73 条 Qualified、102233085 bytes，热 Tick 32 条；第 16/32/48/64 次及最终重开核验原请求、冷历史与恰好一次效果。端点使用 canonical release 构建，harness/runtime 为 debug test artifact，20 秒提交期限未变；debug 端点整轮仍受延迟限制 | 通过该构建组合，最新源码待重测 |
| World 初始激活 CLI | `1581`：实际 Prepare 中断后原请求重试、重复激活及签名冲突拒绝通过，40.98 秒 | 通过该场景 |
| Full-state sync CLI | 原 C0 加入/重开、移动 head、死 seed 转独立固定信任 alternate 三项实际通过；`v4`：合法 epoch 更新后停止、安装新 grant、从不变 C0 重开通过，6.81 秒 | 四项分别通过，完整脚本待验 |
| 单副本失联只读 | `v2`：SIGKILL R 后新提交暂停，签名状态报告 degraded；指定原已发布切面可读，未指定切面请求拒绝。R 原目录重开后原请求恰好一次提交，24.15 秒 | 通过该场景 |
| 健康集合转换及原生安装 | canonical `healthy_` 业务 pack 收窄后 `v2`：15 项通过，16.04 秒；另 257 层 canonical 哈希等价测试通过。含 native H 复制/读回、context 重验、权限边界、Awaiting 禁签禁追加、旧 P 封禁、安装/激活丢回复重开及首个 Normal。provider 为本机受保护签名协议模拟 | 原生库组合通过；实际基线 `v8` 读回 8233 ms（重复空哈希 7825 ms）；优化后 `v9` P 激活读回 386 ms、表构建一次，R 因 1054 个独立 inode 的同步仍耗时 4775 ms，最终激活 RPC 未完成。保持原 5000 ms 期限；R pack 修复后的 `v10` 在 arm 前因背景 Tick 原请求 pending 而拒绝，尚未取得切换成功结论 |
| 健康集合 RPC 边界恢复 | native install `v3`：1 项通过，3.50 秒；真实 TCP 受保护 checkpoint/CAS 恢复激活后的 H，旧恢复入口缺健康凭证拒绝，新入口核对 typed activation、C0、exact H 与 successor context；canonical all-targets clippy `v5` 通过，4.82 秒 | 原生 TCP 组合通过，管理安装 RPC 待接入 |
| 健康集合普通读取 | `healthy_durable_set_read_` `v3`：3 项通过，0.33 秒；验证 Qualified H/H-1、双 seal/双安装、inventory 一致、首个 Normal 和固定 C0 的 V2 重放；清除旧资格缓存后从当前签名索引恢复原结果，C0 引用保持恒定 | 读取协议夹具通过，完整实际进程组合待验 |
| Package upgrade 事务 | 旧本机脚本 `v10` 通过同 SHA 基线及不同 SHA 缺 chain 配置提前拒绝；发布链接及精确 selected-prior 调用已修复；当前 preimage 六项测试通过。实际失败启动回滚 fixture `v2` 在 stage 阶段失败，79.73 秒；prior/candidate 描述子进程均 exit 0，helper 的 `validate_runtime_compatibility_plan` 报 `current_binding` 未定义，尚未进入启动失败和回滚阶段。日志 `/tmp/oasis7-lifecycle-package-actual-fixture-20261014-v2.log` | 最新完整升级及失败回滚尚未通过 |
| 旧目录迁移 | `v16`：12 项迁移单元测试通过；实际脚本 `v2` 使用 128 MiB reward 数据并 SIGKILL 迁移进程后原请求续做通过，核对身份、原 head、历史、CAS、replication、pending/outbox。验证读取原始 persisted World，保留可选 simulator mirror 语义 | 通过该构建组合，最新完整源码待验 |
| 健康闭包读回复核 | canonical `healthy_copy_` 两项、`registered_` 三项通过；真实 walker 拒绝同一 Index 对象第二条路径上的非法深度，权限测试拒绝可被其他用户写入的目录/CAS，同时允许旧 0755/0644 读取 | 定向回归通过，完整安装进程待验 |
| 普通节点兼容生命周期 | `local-node-lifecycle.test.sh`：真实 start/restart、原动作重试、query-only 账本恢复通过；Node canonical clippy 通过，4.19 秒 | 通过上述验证 |
| Effect 身份与派发 | 三项 birth/receipt/legacy 字节回归通过，0.11 秒；扩大 effect 回归 95 项通过，2.64 秒；派发回归 22 项通过，0.83 秒；真实丢回复/SIGKILL/重开 `26533` 通过，22.41 秒；无合同 `47864` 通过，22.89 秒；丢账本 `92718` 通过，24.02 秒 | 通过上述场景 |
| Protected signer handoff | `57074`：40.28 秒实际运行通过，状态到达 `successor_qualified_readback_verified` / `succeeded` | 该 handoff 场景通过 |

之前报告的 ancestry/minimum 错误、初始 artifact CAS 缺失与 handoff RPC deadline/current-checkpoint 回滚均为旧轮结果；ancestry bridge、persistent raw-CAS 和精确签名切面缓存修复已有上述定向或实际证据。预热前移和外部派发接入已通过当前 formal 测试组；较早的 C03 八阶段尚未在这次接入后完整重跑。这些结果不外推为最新源码的长期历史、所有 handoff 交错或完整部署计划已通过。

长历史测试已将原请求的模块调用断言绑定到该请求的 exact Qualified Frozen/candidate Journal，避免把并发空 Tick 的 Wasmtime 调用错误归因于管理请求。启动优化从独立的受保护 P/R probe 获得一致 head 提示，正式恢复客户端的原最低锚保持不变；缓存安装仍要求从 C0 完整验证前缀及精确上下文，不扩大超时或缓存。

外部 effect 新出生身份包含 era，receipt 绑定完整 World、原 intent 与 payload；旧 `None` 的序列化和签名保持原字节。派发仅接受 Qualified outbox 中明确的 target 与 receiver contract 摘要，本机账本不授予资格。生产自动派发要求预授权发送者、幂等原身份 POST 和完整签名查询；本地账本丢失或节点更换不得改变永久 effect digest。外部回执独立于 World receipt，未确认结果不会直接改 committed World。真实回复丢失、SIGKILL、重开已通过：接收方执行与 POST 各一次，重开查回原签名回执，原 CommitRef 不变。无幂等合同场景从首次启动到重开均无 POST。丢账本场景用父进程内的生产 transport 与预授权 sender92 查询原 Qualified outbox，随后真实 runtime 仍以 sender90 重开；它不代表正式 Node 换密钥或授权集迁移已验收。

这仍是本机代码验证，未完成部署计划。最新源码长期历史、CLI 与完整 full-sync 组合，以及 handoff 的故障/交错负例仍待验；单次 handoff 成功不代表这些场景通过。没有真实故障域、生产部署或公开开放的验收结论。

全量同步仍有整次树/传输和本地历史容量界限，启动仍从 C0 验证完整保存历史；本轮回收不能被解释为任意大小世界或无限历史同步已完成。World CAS 的路径预检不外推为所有存储路径都有 syscall 级 O_NOFOLLOW。Node Package 已采用目录/文件描述符的 no-follow 读取、具名项身份重验和实际加载映像核验；跨重试工件身份仍绑定路径与 SHA，允许字节完全相同的 inode 替换。

有限 Tick 热尾上限为 32 条，每次候选最多归档 64 条旧证书；累计数使用 u64，不以扩大累计常量替代归档。恢复先绑定原已确认 Snapshot 和 CAS reader，再校验完整原前缀及冷热边界。缓存只保存内容寻址字节，不能替代 root、授权、签名或双端资格校验。

本轮集成的本机验收脚本为 `scripts/formal-live-local-tests.sh`，覆盖真实端点子进程及串行 Wasmtime 组合，且已纳入 required tier 调用路径。以上结果不代表脚本覆盖的所有流程均已在同一版本完整通过；长期历史与待验场景仍见上表及本节末尾状态。

## 后续 TODO 与验收边界（2026-10-14）

用户授权小遗留在实施文档跟踪；这些项不表示当前已通过，也不解除切换、回滚与恢复正确性及 required CI 的合入要求。

- TODO：在明确获准的真实部署环境验证 P/R 物理故障域、受保护 signer/HSM、服务管理器与部署回退。验收为原 C0/身份不变、故障后原请求恰好一次恢复及独立资格读回；本机协议模拟不代签。
- TODO：补充 macOS 以外平台的 executable-image/no-follow 路径与真实不同版本产物升级矩阵。验收为合法 prior/candidate 通过、错误加载映像及不兼容持久格式在停止前拒绝；shell stub 不代签真实升级。
- TODO：将本机诊断日志整理为可复用的操作故障排查步骤，保留阶段及原 operation 身份，避免仅凭超时判断故障原因。
- TODO（ST-02/ST-04，blocked）：验证正式玩家 Skill 支持的私有 Formal Game API 与 attached executor binding 后，组织独立 Agent 对 retained task/nonce/control 的目标、问答、断连回读和恢复自主试玩。现有普通合同测试不代签；当前 delivered Skill 指向 LocalProvider/LetAI，不能证明本轮 formal 消费者。本次仍交付最初定义的本机持久化与管理基础设施，不开放完整 formal 玩法。

## 本轮独立源码评审（2026-10-13）

`storage` 对非本人编写的历史 Published 缓存接线进行只读安全/兼容审查：精确 opaque cut、完整 Qualified proof、当前权限与双端 Query 保持；未发现阻断。随后增加的代码仅为测试计时，`storage` 核对的 SHA256 为 `34394860f6d9867a88516eba8834f967408eb9b54d82737341df9ec3591c9190`。编译时补入缺失的 `FormalRecordKindV1` 导入，当前 SHA256 为 `7166ca63d7148cb28a50db13b1a4dec98e1b1b99736ec49979dd1dae3d2fa09e`。损坏本地文件、聚合上限与双 Query 强制路径的专门运行证据仍待补齐。

`wasm_review` 对非本人编写的索引迭代、Recovery 历史资格及历史材料恢复进行只读审查，发现历史增量缺少原父根重建验证；补强后复审未发现 P0/P1 阻断。父根来自旧记录已承诺 parent ID 的精确记录体，不从当前 head 取根，不创建父记录资格。定向正例与错误 base、缺节点和换父根负例通过；评审不代替实际组合验收。

| 复审源文件 | SHA256 |
| --- | --- |
| `business_index.rs` | `02a728b0911abe8f1cfc042bae027e1b9c6880988084ccd94b713ef446742ca2` |
| `business_history_committed.rs` | `0cc561e771b8f0c1089316b2a44bc7340a3b0f8d98a7d330f666fe278db4ce13` |
| `business_material.rs` | `6b22951d4e697bfc859e9c0edec398c6a911ed75c0e9c1308301d4a2db11b7ca` |
| `endpoint_committed_material.rs` | `8a861e9a840afcd69b2ec174305a1b45cbb80ce91c6ef11a82c1bb8683d9e0c3` |
| `client_committed_history.rs` | `16e524aba99a3a1765560b64a481b05f72adc30d29bff8cf91faa980d1eb9ee8` |

`storage` 独立复核 CI 接线：required tier 保留原检查并追加正式验收脚本，真实 public ignored parents 显式执行，内部 child 不混入；端点与 runtime executable 从 cargo JSON 精确解析，串行构建/运行。范围规划 17 项测试通过。脚本 SHA256 `845ff46d2d6bfb0ed5e18415d6bc88b90a5b163e09d039877628766e68423186`，`ci-tests.sh` 为 `275d744a550a5f9d4830896e97588a8560ca129d0e53a18bb53a26e0947ea369`，scope 配置为 `0c5b1ef6afdade1ce1daaa14d44674850957f317464de055d6f748de07843359`。完整脚本和远程 CI 没有本轮通过结论。端点库 all-targets canonical clippy 已通过。

## 端点服务与旧工程配置

`oasis7_durability_endpoint` 同时提供旧 `replicated_durability_prerequisite` 工程协议和本轮 formal native profile 的独立持钥进程。正式 Node 通过经过校验的外部 C0、manifest v2 和完整初始物料运行；恢复必须具有原恢复权限和受保护端点证据。端点不生成世界、密钥或信任材料。正式配置及运行入口见 [本机节点生命周期 runbook](local-node-lifecycle.runbook.md)。下面保留的配置片段仅说明旧工程协议，不能用作正式信任锚或正式开放证据。

构建、运行入口：

```sh
env -u RUSTC_WRAPPER cargo build -p oasis7_distfs --bin oasis7_durability_endpoint
./target/debug/oasis7_durability_endpoint --config ./primary.endpoint.json
```

配置结构如下；所有信任值和 minimum_head 必须来自该工程协议的原始可信材料。示例中的占位符不是可用身份。程序不生成密钥、世界或信任材料。

```json
{
  "schema_version": 1,
  "trust": {
    "world_id": "original-world",
    "chain_id": "original-chain",
    "genesis_digest": "<original 64-character lowercase hex digest>",
    "authority_epoch": 4,
    "writer_key": "<original writer public key>",
    "primary_id": "primary",
    "primary_key": "<original primary public key>",
    "replica_id": "replica",
    "replica_key": "<original replica public key>"
  },
  "role": "Primary",
  "data_dir": "./primary-data",
  "signing_key_file": "./primary.key",
  "minimum_head": {
    "position": 0,
    "decision_hash": "<verified genesis anchor digest>",
    "qualified": false
  },
  "listen": "127.0.0.1:9101",
  "timeout_ms": 2000
}
```

Replica 使用自己的 role、目录、公钥和私钥。每个服务只读取自己的 endpoint key；RPC 客户端只持 writer key。端点拒绝 signer、角色、世界或 epoch 不匹配。相对路径以配置文件目录解析。私钥须为当前用户所有的私有普通文件；已有数据目录须为私有目录。新端点文件与目录分别以 0600/0700 创建。

服务和客户端均只接受 loopback 地址。跨主机使用必须由受认证、加密的隧道终止在本机 loopback；签名 RPC 本身不提供内容保密。不能直接绑定公网地址或把完整私有对象闭包开放为公共节点读取 API。

`transport::RemoteEndpoint` 与 `ReplicatedCoordinator<RemoteEndpoint>` 复用本地端点的 prepare、决定、proof 保存和原请求查询流程。每个 RPC 绑定原始信任、精确 endpoint role/identity、挑战、minimum anchor 和操作内容。响应由该端点签名并绑定原请求摘要。超时或响应校验失败不能成为 qualified 结果。

同一 slot 已准备候选后，另一个候选在端点持久入口被拒绝。原候选可重试。查询 qualified 必须同时验证原 request identity、候选签名和完整双端证据；响应封套签名不能替代内部证据。

guarded runtime 配置 v2 可填写 `remote_endpoints: {primary, replica, timeout_ms}`，并省略本地 primary/replica 的目录与 key 字段；混用远端和本地端点字段被拒绝。实际执行 driver 复用远端协议，协调进程不读端点私钥。它仍是明确的工程 profile，不能据此宣布正式游戏 runtime 已完成跨故障域提交。

v1 保持原完整配置摘要，已持久 continuation 不重新解释。v2 的执行绑定覆盖 node 身份、调度间隔、原激活与 release policy；目录、监听地址和私钥文件位置不参与绑定，实际公钥仍按固定 trust 验证。调度或 signer 变化不能借搬路径绕过。

v2 执行前将原 continuation、动作、pre-state 完整闭包和执行绑定保存到两端。writer 授权、双端冻结回执和闭合 FrozenCertificate 必须都持久保存，之后才运行实际 Wasmtime。候选证明 v2 绑定原冻结摘要，不能替换动作或 predecessor。已保存完整候选优先原样续交；仅冻结完成且无候选时从原输入重放，保留中断目录，不重新调用模型或选择动作。v1 缺候选的旧恢复拒绝仍保留。

冻结执行绑定还包含显式 Wasmtime policy（fuel、内存、输出与存储参数）及当前执行工件摘要。确定性重放不以 wall-clock 超时产生业务结果，仍实施有限 fuel/内存/输出预算。不同工件不能重放未完成冻结；完整候选则无需重新执行即可沿原证明续交。工件摘要按 `current_exe()` 所指文件读取，部署必须保留不可变工件，运行期间不得原地改写或替换该路径；这不是远程证明，也不能单独证明已映射代码与替换后的路径一致。正式升级和恢复仍须完成授权及工件保留合同。

## 存储与恢复边界

端点新增分段、校验链绑定的不可变 delta 帧。落盘顺序为私有临时文件、写入、文件同步、原子 rename、目录同步。写入不确定后保留 unknown/poisoned，须重新打开并查询原请求。

原 snapshot 保持可读，迁移后作为不可变基底。超过旧 64 条/16 MiB 的历史可恢复、查询原请求并继续 prepare；原请求不同内容继续拒绝。归档不裁剪，原请求与对象证据一直保留。

restore 允许相同证据历史的已完成重试和中断续做。目标已有历史必须是输入的完全相同 qualified 前缀；prepared 尾部、冲突、额外历史或更短输入均拒绝，不覆盖已有记录。

新增 v2 存储以端点签名的热 checkpoint 绑定当前 head、候选/frozen 引用、永久请求 radix trie 根和 revision。对象为不可变、按摘要寻址的有界 archive；历史输入按需读取。head 的签名归档节点绑定 binary-lifting 前序引用，minimum 与原请求的历史归属通过有界路径核对。未被 checkpoint 引用的孤儿文件不能成为权威或伪造去重。

旧分段 journal 的 segment seal 仅为边界 checksum；其原始文件保留，迁移为 v2 后不作为日常全历史 replay 路径。请求索引与归档永久保留，本轮不执行裁剪。`export_evidence` 显式物化完整历史，普通热状态仅保留一个候选/当前记录与冻结输入。历史 reader 的单次验证和缓存仍有有界预算；长期历史场景已在表中指定构建组合完成超过 64 条/16 MiB 的实际验证。全量同步仍受公开传输与本地容量预算约束，不能据此宣称无限历史或任意大小世界。

## 正式合同层与准入边界

`controlled_authority::formal` 使用独立签名域与类型。CanonicalRecord 的身份只覆盖逻辑记录体；Normal、CompletePrepared 和 ReplayFrozen 的签名/证明封套不会改写同一记录身份。Decision 只含双端 final receipt，不能提升为 Qualified：双端都保存完整 Decision 并签出 ProofComplete 后，Qualified 证明才成立，随后端点发布该 head。完整证明保存期间的单端状态不得显示 committed。

输入 Frozen 的 governing manifest 必须等于父记录的输出 governing manifest，执行策略也必须沿合法历史保持一致。每次输入闭包的 ExecutionManifest 摘要与 Cognition governing manifest 是两个不同承诺。实际重放 policy 明确启用 NaN canonicalization 与 deterministic relaxed SIMD，并绑定不可变执行工件；已有 WASM 执行器测试 31 项通过、1 项性能探针忽略。本机实际 formal 执行闭包组合仍在验证中，不作为正式准入收据。

普通读取使用外部指定 InitialActivationTrust 与 independently pinned read checkpoint，再验证 Qualified 历史与 audience/reader-key 绑定的投影 Merkle 证明。权限代数在投影叶内绑定，不能用物理 authority epoch 替代 World 的 capability generation。响应提供的服务签名或 issuer key 不能自行建立初始信任。原 engineering scope 的普通客户端拒绝规则保留；正式 Submit/Lookup/Changes 缺少各自证明合同的路径须显式拒绝。

fencing 的生产门槛包括实际持久实例、单调 floor 和受保护的签名表面。仅远程 oracle 声明强 capability，但端点仍持有可导出的本机 SigningKey，无法阻止旧进程离开 guard 使用复制的 key。此路径必须返回 `FENCING_UNPROVEN`；文件私有权限、目录 fingerprint、签名 enum 或本机测试均不能替代 nonexportable key custody。当前原生端点已接入受保护 signer 接口，只有其签名 custody 与单调 checkpoint 验证通过后才能进入正式 ready；本机 oracle 的协议测试不代替真实防回滚部署证据。issuer 的唯一接续同样要求独立、不可回滚且保留原证书的 witness；本机模拟 witness 只验证协议状态机。

## CLI 与成员查询

flat CLI 的 85 项 OptionSpec 目录统一 canonical/alias、类型、冲突规则、默认值与 help；assignment dispatch 与目录做精确覆盖验证。CLI 拒绝字符串缺值吞掉下一开关、重复标量、正反开关冲突和重复身份绑定。peer 等可重复列表继续可用。以 `--` 开头的字面值采用 `--node-id -- --literal`。

fetch requester 正名为 `--replication-fetch-requester-public-key`；wrapper/observer 配置使用 `REPLICATION_FETCH_REQUESTERS_CSV`。旧拼写暂为有警告的薄适配，冲突的新旧 env 被拒绝。权限仍仅为 fetch；该配置不能授予 writer 或 validator 身份。旧名退役边界见现行 bootstrap runbook。

`WorldGovernanceMembershipView::read` 从单一已验证 committed World 读取原候选、既有 resolver 选择、治理 epoch 和有效 registry。调用者先校验证明并将 CommitRef 的 CAS state_root_ref 解析到真实 state root，adapter 校验 root、world/runtime binding、manifest、finality 与 generation。解析错误直接返回，缺 registry 不回退到 node config。

`RuntimeMembershipApplicationView::startup_projection` 只报告配置角色和启动 registry 来源。当前没有安全 runtime install acknowledgement，`supports_safe_runtime_update=false`，`applied_consensus_membership=None`。治理有效不能显示为 BFT 已采用。现已提供 `validator list/status/wait/watch` 诊断入口，使用现有 `OASIS7_WORLD_SERVICE_ENDPOINT`、`PUBLIC_KEY`、`WORLD_ID`、`GENESIS_DIGEST`、`SCOPE`（必须 public）和 `READ_PRIVATE_KEY` 配置，所有字段采用 `OASIS7_WORLD_SERVICE_` 前缀。服务支持已独立证明的正式 public projection；guarded pinned history 只作为显式工程诊断，必须使用 `--allow-controlled-prerequisite`，默认客户端仍拒绝工程 profile。未知 capability/profile 明确拒绝，不读取本地 registry 冒充正式视图。

watch 为有界、只在 commit 变化时输出的 snapshot polling。`--cursor` 是 minimum CommitRef，不保证重放所有中间转换；`first_observed_in_this_run` 与实际首次生效位置区分，后者未证明时为 null。公开 candidate 只包含白名单字段，不返回 operator owner 或自由文本 revocation reason；原 WorldService projection 的私有字段过滤保留。旧 guarded 客户端遇到新的 capability 枚举可能 fail closed，本轮未交付版本协商。独立 node init/start/status/stop 已接通 compatibility observer、formal public observer 和只读 full-state sync；具体身份、grant 与 ready 边界见 [本机节点生命周期 runbook](local-node-lifecycle.runbook.md)。

每次成员读取使用 OS 随机生成的 32-byte observation nonce；调用者与服务签名都绑定包含该 nonce 的原始查询。旧响应不能用于新的查询。没有有效 registry 时，治理有效性保持 unknown/null，不解释为确定的未生效。

保存的正式 cursor 最多包含 32 帧、16 MiB。预算耗尽时，watch 可从当前配置的外部 C0 独立重读，完整验证后证明新历史包含原客户端的确切 head/authority 与已认证 minimum，并以缓存 compare-and-swap 原子安装新的 journal；失败保留旧缓存及文件。输出与写入文件必须对应同一提交。当前重基依赖单次 inline 历史，C0 到 head 超过 128 步或缺历史仍返回 `HistoryUnavailable`；调用方提供历史页的验证 API 不等于自动网络分页，不能宣称任意长度历史的 watch 重启续读已完成。33 个签名提交的库级重基测试及短历史断线重启的实际 CLI 子进程已通过；超过 inline 128 步的自动网络续页仍未交付。

## 剩余整体验收

| 功能 PR | 必须继续完成的能力 |
| --- | --- |
| PR 1 | 健康 durable set 替换的 protected install、双安装激活、跨 epoch Normal 读取与协调器；最新源码完整正式本机脚本；C01—C20 剩余故障交错及适用自主试玩。表中已通过的正式激活、提交、恢复和长期历史场景保留其实际版本范围。 |
| PR 2 | Package stage/promote 的 OperationRecord 与精确停止实例桥接；旧分散目录迁移的实际正常 binary SIGKILL/续做验证；O01—O19、U01—U10 剩余组合与调用者闭包核对。未实现的 BFT signer/成员写动作继续明确拒绝，不能以本地配置冒充生效。 |

这两个功能 PR 的完整验收尚未满足；当前代码不得合入后宣称方案完成。后续仍按原方案的两个完整交付目的集成，不把 prerequisite 子模块变成额外的用户审批链。

2026-10-14 独立试玩前置预检读取了候选正式玩家 Skill `site/skills/oasis7.md`，SHA-256 为 `c103ef66284e18d23cfc9b6d39ad4bcad9c922d7e18eeb5e93762d12caa3dc92`。官方入口 `./scripts/run-local-letai-game-test.sh --local-world-playtest` 使用私有本机 LocalProvider 世界，不连接 formal/public testnet，但需要真实 LetAI 凭据与付费推理，公开说明含默认自动充值；本轮没有新增付费预算授权，因此未启动该入口。预检子任务未读取实现，但环境仍能访问源码，隔离级别为 `instruction_only/source_visible`。此结果仅是前置预检，不能代签自主试玩；正式网络玩家 Skill 及真实执行 profile 仍未在本轮证明。

## 本机验证命令

```sh
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib --bin oasis7_durability_endpoint
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib governance_membership --no-default-features --features test_tier_required
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features node-libp2p,test_tier_required parse_options
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features node-libp2p,test_tier_required controlled_live -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor --features wasmtime
./scripts/p2p-triad-node-start.test.sh
./scripts/p2p-public-testnet-local-observer-sync.test.sh
./scripts/p2p-public-testnet-local-observer-runtime-preflight.test.sh
./scripts/p2p-public-testnet-observer-triad-rollout.test.sh
```

distfs CI 原 library 测试与 clippy 保留，并包含 endpoint binary。协议故障注入覆盖实际文件同步/rename 边界；多进程测试覆盖独立端点、原身份查询、退出/重启和断连。SIGKILL/进程退出不能证明磁盘断电或真实跨故障域隔离。完整正式游戏路径后续仍须按开发规范执行适用的实际自主试玩与环境验收。

2026-10-12 本机记录：最终端点 canonical 命令通过（library 109 passed / 4 ignored，binary 2 passed）；实际 runtime/Wasmtime 组合测试通过（7 passed / 1 ignored，ignored 为由测试启动的 SIGKILL helper）；WASM executor 30 passed / 1 ignored。此前 runtime 组合测试发现旧 schema-v2 拒绝断言失效及两次轮询等待超时；断言已按新增 v2 支持修正，协调器补充最后 reconciliation error 诊断，避免重负载并行运行后完整组合通过。等待超时的根因尚未独立证实，不能把单次通过解释为已消除所有时序风险。
