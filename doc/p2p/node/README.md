# `p2p/node` 热点子域入口

更新时间: 2026-04-17

## 从这里开始
- 想确认节点奖励、贡献分、资产与结算口径：先读 `node-contribution-points.prd.md`、`node-redeemable-power-asset.prd.md` 或 `node-reward-settlement-native-transaction.prd.md`
- 想确认复制链路、net stack、signer binding 与 DistFS 节点网络闭环：先读 `node-replication-libp2p-migration.prd.md`、`node-distfs-replication-network-closure.prd.md` 或 `node-consensus-signer-binding-replication-hardening.prd.md`
- 想确认 PoS 时间、slot clock 与控制面对齐：先读 `node-pos-slot-clock-real-time-2026-03-07.prd.md`、`node-pos-subslot-tick-pacing-2026-03-07.prd.md` 或 `node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
- 想确认节点身份引导、keypair bootstrap 与初始化入口：先读 `node-keypair-config-bootstrap.prd.md`
- 想确认 `wasm32/libp2p` 编译约束或 builtin wasm fallback：先读 `node-wasm32-libp2p-compile-guard.prd.md` 或 `node-builtin-wasm-fetch-fallback-compile.prd.md`
- 想精确找某份专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `node/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 `p2p` 模块级 landing page，负责跨 `blockchain / distfs / node / observer / token / network / distributed / consensus / viewer-live` 分流。
- `../prd.index.md` 是 `p2p` 模块完整文件级索引，适合已知主题后按文件名查找。

## 密度快照
- 治理前快照（2026-04-17）:
  - `doc/p2p/node/`: 68 份 Markdown
  - `doc/p2p/`: 269 份 Markdown
- 当前子域属于 `p2p` 模块最高密度热点路径；本页的目标是压缩首读路径，而不是在本批直接减少文件数。

## 首读主题簇

### 1. 奖励、资产与结算
- 首读入口:
  - `node-contribution-points.prd.md`
  - `node-redeemable-power-asset.prd.md`
  - `node-reward-settlement-native-transaction.prd.md`
- 适合问题:
  - 节点奖励怎么计、贡献分如何结算
  - 可赎回 power asset 与治理签名阶段如何拆分
  - 奖励、执行验证与原生交易结算的关系是什么
- 说明: `runtime-closure`、`multi-node-closure-test`、`audit-hardening`、`signature-governance-phase3` 等文件是增量子文档，不应代替主文档成为默认首读入口。

### 2. 复制、网络与 signer binding
- 首读入口:
  - `node-replication-libp2p-migration.prd.md`
  - `node-distfs-replication-network-closure.prd.md`
  - `node-consensus-signer-binding-replication-hardening.prd.md`
- 适合问题:
  - 节点复制链路现在哪个专题是主文档
  - DistFS 节点复制与 libp2p migration 的当前边界是什么
  - 共识 signer binding 与复制硬化需要看哪里

### 3. PoS 时间与控制面对齐
- 首读入口:
  - `node-pos-slot-clock-real-time-2026-03-07.prd.md`
  - `node-pos-subslot-tick-pacing-2026-03-07.prd.md`
  - `node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
- 适合问题:
  - slot/epoch 真实时钟驱动的现行口径是什么
  - 槽内 tick phase 和 proposal pacing 怎么理解
  - runtime / launcher / script 控制面参数应该看哪份专题

### 4. 身份引导与初始化
- 首读入口:
  - `node-keypair-config-bootstrap.prd.md`
- 适合问题:
  - 节点 keypair 与 config bootstrap 的当前真值在哪
  - 新节点身份初始化和配置入口怎么对齐

### 5. WASM 编译与兼容护栏
- 首读入口:
  - `node-wasm32-libp2p-compile-guard.prd.md`
  - `node-builtin-wasm-fetch-fallback-compile.prd.md`
- 适合问题:
  - `wasm32/libp2p` 的编译守卫和限制是什么
  - builtin wasm fetch fallback 的编译闭环要看哪里

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`，不要指望本页替代完整索引。
- 如果你追的是某个奖励阶段、closure test、audit hardening 或 release 说明，允许直接进相应 supporting spec，但不要把它们重新当作默认首读入口。
- 如果某个主题已经出现“主文档 + 增量子文档”的主从关系，应优先进入主文档，而不是从子文档倒推现行口径。

## 维护约定
- 新增 `node/` 专题后，若改变了默认首读路径，应同步更新本页。
- 本页只维护簇级入口，不维护完整文件清单。
- 若未来 `node/` 内部继续分裂出更高密度簇，再另开簇内治理专题，而不是把本页扩写成长表。
