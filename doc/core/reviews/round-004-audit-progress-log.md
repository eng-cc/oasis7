# ROUND-004 审计进度日志（逐文档即时回写）

审计轮次: 4

- 规则：每读完 1 篇文档，立即回写文档 `审计轮次: 4`，并在本日志新增 1 条记录。
- 结论枚举：`pass` | `issue_open` | `blocked`

| 时间 | 审计人/代理 | 文档路径 | 结论 | 问题编号 | 备注 |
| --- | --- | --- | --- | --- | --- |
| 2026-03-06 11:42:02 +0800 | G4-002-Aristotle | doc/world-simulator/prd.index.md | issue_open | I4-001 | 索引未收录 `viewer-asset-pipeline-ui-system-hardening-2026-03-05` 的 PRD/Project，存在可达性断链。 |
| 2026-03-06 11:47:37 +0800 | G4-002-Aristotle | doc/world-simulator/prd.md | issue_open | I4-002 | 主 PRD 分册索引缺少 `viewer-asset-pipeline-ui-system-hardening-2026-03-05.prd.md`，与“唯一入口可追溯”目标不一致。 |
| 2026-03-06 11:48:07 +0800 | G4-002-Aristotle | doc/world-simulator/project.md | issue_open | I4-003 | 项目主入口仍宣称 `C2-002` phase 文档“归档”，但对应 phase8/9 文档已物理删除且表述与当前仓库状态不一致。 |
| 2026-03-06 11:48:51 +0800 | G4-002-Aristotle | doc/world-simulator/README.md | pass | - | 入口文档结构清晰，主入口与主题目录划分一致，未发现分工边界异常。 |
| 2026-03-06 11:49:25 +0800 | G4-002-Aristotle | doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md | issue_open | I4-004 | PRD 中含大段“Phase 8/9 增量任务记录/验收证据”，混入执行过程信息，越过 Why/What/Done 边界。 |
| 2026-03-06 11:50:04 +0800 | G4-002-Aristotle | doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.project.md | issue_open | I4-005 | 项目文档合并后复用 `T0~T4` 编号承载多个 phase，任务标识不唯一，削弱 PRD-ID 到任务的可追溯性。 |
| 2026-03-06 11:41:54 +0800 | codex | `doc/world-runtime/governance/audit-export.md` | issue_open | I4-001 | 文件未在 `doc/world-runtime/prd.index.md` 的专题清单中登记，存在可达性断点。 |
| 2026-03-06 11:42:42 +0800 | codex | `doc/world-runtime/governance/governance-events.md` | issue_open | I4-001 | 文件未在 `doc/world-runtime/prd.index.md` 的专题清单中登记，存在可达性断点。 |
| 2026-03-06 11:42:13 +0800 | codex | `doc/testing/ci/ci-wasm32-target-install.prd.md` | issue_open | I4-001 | SC-2 写为 scripts/ci-tests.sh required/full，验收命令不可直接执行。 |
| 2026-03-06 11:42:16 +0800 | codex | `doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md` | issue_open | I4-001,I4-002 | 输出文件路径写成 `site/site/doc/cn/index.html` 且验收命令写为裸 `cargo check`，可达性与可执行口径不一致。 |
| 2026-03-06 11:42:19 +0800 | codex | `doc/p2p/prd.md` | issue_open | I4-012 | PRD 闭环以治理检查为主，缺可执行验收命令映射。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/project.md` | issue_open | I4-011 | 模块级 project 缺 PRD-ID->TASK->验收命令->证据矩阵。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/prd.index.md` | issue_open | I4-010 | 索引未覆盖 README.md，可达性规则未显式声明。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/README.md` | issue_open | I4-007 | README 与 prd.index.md 同时承担入口角色，权威入口边界不清。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯表 T0~Tn 占位且缺命令证据，且混入迁移过程描述。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.project.md` | issue_open | I4-002,I4-013 | PRD-ID 粒度与 PRD 不一致，且无独立验收命令段。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.prd.md` | issue_open | I4-005 | phase 系列缺主从权威声明，存在双源漂移风险。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/distfs/distfs-production-hardening-phase1.project.md` | issue_open | I4-003 | 任务含回归描述但未落地为可执行验收命令清单。 |
| 2026-03-06 11:42:20 +0800 | codex | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.project.md` | issue_open | I4-008 | 已完成状态缺最近更新日期字段，时效性不可审计。 |
| 2026-03-06 11:42:26 +0800 | Codex-G4-001 | `doc/core/prd.md` | pass | - | 入口层级与术语口径清晰，未发现 D4 高中风险。 |
| 2026-03-06 11:42:43 +0800 | codex | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md` | issue_open | I4-003,I4-004 | 同文并存 `审计轮次: 4` 与列表项 `- 审计轮次: 2`，且脚本口径依赖 `$CODEX_HOME/.../agent-browser` 非仓内可达路径。 |
| 2026-03-06 11:42:43 +0800 | Codex-G4-001 | `doc/core/prd.index.md` | pass | - | 索引入口单一且与主文档互链明确，可达性正常。 |
| 2026-03-06 11:42:49 +0800 | codex | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md` | issue_open | I4-002 | 验证命令写为 cargo test/check + cargo run，命令表达不具可执行性。 |
| 2026-03-06 11:48:00 +0800 | codex | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md` | issue_open | I4-014 | Traceability 表将“对应任务”写为“文档内既有任务条目”，缺 PRD-ID -> TASK -> 验收证据的可反查链。 |
| 2026-03-06 11:48:31 +0800 | codex | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”段仅 T-MIG 行带 PRD-ID，T0~T5 未建立任务到需求映射与验收命令证据。 |
| 2026-03-06 11:49:05 +0800 | codex | `doc/world-runtime/integration/node-contribution-points-runtime-closure.md` | issue_open | I4-016 | 文档采用旧模板且未提供 PRD-ID/任务/验收命令链，且未在 `doc/world-runtime/prd.index.md` 收录。 |
| 2026-03-06 11:49:38 +0800 | codex | `doc/world-runtime/README.md` | pass | - | 主入口声明与目录分区清晰，未发现新增设计质量问题。 |
| 2026-03-06 11:50:18 +0800 | codex | `doc/world-runtime/prd.index.md` | issue_open | I4-001 | 索引声明“保证专题文档可达”，但未收录 `governance-events.md`、`audit-export.md`、`integration/node-contribution-points-runtime-closure.md`。 |
| 2026-03-06 11:50:53 +0800 | codex | `doc/world-runtime/prd.md` | issue_open | I4-017 | Validation 表为描述性“验证方法”，未给出可执行命令或证据路径，PRD->TASK->命令链条不闭合。 |
| 2026-03-06 11:51:19 +0800 | codex | `doc/world-runtime/project.md` | issue_open | I4-018 | TASK-WORLD_RUNTIME-001~005 未附验收命令/证据，只有 TASK-WORLD_RUNTIME-006 给出具体回归命令，追溯链不完整。 |
| 2026-03-06 11:51:46 +0800 | codex | `doc/world-runtime/testing/testing.md` | issue_open | I4-019 | 文档仍声明挂靠 `doc/world-runtime.prd.md` 且使用 `ModuleValidationFailed` 术语，与当前 `governance-events.md` 口径不一致。 |
| 2026-03-06 11:52:17 +0800 | codex | `doc/headless-runtime/README.md` | issue_open | I4-020 | 模块已更名 `headless-runtime` 但活跃文档与约定仍长期并存 `nonviewer-*` 命名，术语双轨易造成检索与口径漂移。 |
| 2026-03-06 11:52:46 +0800 | codex | `doc/headless-runtime/prd.index.md` | pass | - | 专题 PRD/Project 配对完整且入口单一，未发现新增可达性问题。 |
| 2026-03-06 11:53:20 +0800 | codex | `doc/headless-runtime/prd.md` | issue_open | I4-017 | Validation 表仍为描述性验证方法，未落到可执行命令与证据路径，追溯闭环不完整。 |
| 2026-03-06 11:53:46 +0800 | codex | `doc/headless-runtime/project.md` | issue_open | I4-018 | TASK-NONVIEWER-* 任务项未给出验收命令与证据链接，Project 层执行可复现性不足。 |
| 2026-03-06 11:54:15 +0800 | codex | `doc/README.md` | pass | - | 总入口路径矩阵与 legacy redirect 说明完整，未发现 D4-001~D4-008 新增问题。 |
| 2026-03-06 11:54:44 +0800 | codex | `doc/game-test.prd.md` | pass | - | redirect 角色边界与主入口指向清晰，未见多入口冲突。 |
| 2026-03-06 11:55:15 +0800 | codex | `doc/game-test.project.md` | issue_open | I4-021 | Project 文档未维护 PRD-ID 映射与验收命令字段，redirect 任务可追溯性不足。 |
| 2026-03-06 11:55:44 +0800 | codex | `doc/playability_test_manual.md` | pass | - | redirect 声明与主入口指向明确，未发现新增冲突。 |
| 2026-03-06 11:56:15 +0800 | codex | `doc/playability_test_card.md` | pass | - | redirect 声明与主入口指向明确，未发现新增冲突。 |
| 2026-03-06 11:56:43 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md` | issue_open | I4-014 | Traceability 表沿用 `PRD-ENGINEERING-006 + 文档内既有任务条目`，未建立本专题 PRD-ID 与任务/命令/证据链。 |
| 2026-03-06 11:57:17 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”仅迁移任务带 PRD-ID，T0~T3 未显式映射需求与验收命令。 |
| 2026-03-06 11:48:11 +0800 | codex | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md` | issue_open | I4-003,I4-006 | 同文并存 `审计轮次: 4` 与 `- 审计轮次: 2`；状态仍写“进行中/等待回归”且最近更新停在 2026-03-01，时效状态失真。 |
| 2026-03-06 11:48:17 +0800 | codex | `doc/scripts/precommit/pre-commit.prd.md` | issue_open | I4-004 | 接口示例含占位命令 `rustfmt --edition 2021 <staged .rs files>`，不属于可直接执行验收命令。 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.prd.md` | issue_open | I4-001,I4-004,I4-006,I4-005 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述；phase 系列缺主从口径声明 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.prd.md` | issue_open | I4-001,I4-004,I4-006,I4-005 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述；phase 系列缺主从口径声明 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.prd.md` | issue_open | I4-001,I4-004,I4-006,I4-005 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述；phase 系列缺主从口径声明 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.prd.md` | issue_open | I4-001,I4-004,I4-006,I4-005 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述；phase 系列缺主从口径声明 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.prd.md` | issue_open | I4-001,I4-004,I4-006,I4-005 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述；phase 系列缺主从口径声明 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/consensus/builtin-wasm-identity-consensus.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-builtin-wasm-storage.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-builtin-wasm-storage.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase1.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase2.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase2.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase3.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase3.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase4.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase4.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase5.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase5.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase6.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase6.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase7.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase7.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase8.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase8.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase9.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-production-hardening-phase9.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-runtime-path-index.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-runtime-path-index.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-standard-file-io.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distfs/distfs-standard-file-io.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-hard-split-phase7.prd.md` | issue_open | I4-001,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-hard-split-phase7.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-pos-consensus.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-pos-consensus.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-runtime.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/distributed/distributed-runtime.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/network/net-runtime-bridge-closure.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/network/net-runtime-bridge-closure.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/network/readme-p1-network-production-hardening.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/network/readme-p1-network-production-hardening.project.md` | issue_open | I4-008 | 完成状态缺日期字段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.project.md` | issue_open | I4-003,I4-008 | project 缺可执行命令片段；完成状态缺日期字段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.project.md` | issue_open | I4-003,I4-008 | project 缺可执行命令片段；完成状态缺日期字段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points-multi-node-closure-test.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points-multi-node-closure-test.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points-runtime-closure.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points-runtime-closure.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-contribution-points.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-distfs-replication-network-closure.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-distfs-replication-network-closure.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-execution-reward-consensus-bridge.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-execution-reward-consensus-bridge.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-keypair-config-bootstrap.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-keypair-config-bootstrap.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-net-stack-unification-readme.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-net-stack-unification-readme.project.md` | issue_open | I4-003,I4-008 | project 缺可执行命令片段；完成状态缺日期字段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.release.md` | pass | - | 未发现 D4 高中风险。 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-redeemable-power-asset.release.md` | pass | - | 未发现 D4 高中风险。 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-replication-libp2p-migration.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-replication-libp2p-migration.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-reward-runtime-production-hardening-phase1.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-reward-runtime-production-hardening-phase1.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-reward-settlement-native-transaction.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-reward-settlement-native-transaction.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-storage-system-reward-pool.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-storage-system-reward-pool.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-uptime-base-reward.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-uptime-base-reward.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-wasm32-libp2p-compile-guard.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-observability.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-observability.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-mode-runtime-metrics.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-source-dht-mode.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-source-dht-mode.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-source-mode.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/observer/observer-sync-source-mode.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/token/mainchain-token-allocation-mechanism.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/token/mainchain-token-allocation-mechanism.release.md` | pass | - | 未发现 D4 高中风险。 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.prd.md` | issue_open | I4-001,I4-004,I4-006 | 追溯任务写为 T0~Tn 占位；PRD 缺验收命令段；PRD 含过程化迁移描述 |
| 2026-03-06 11:48:20  | codex | `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.project.md` | issue_open | I4-003 | project 缺可执行命令片段 |
| 2026-03-06 11:48:36 +0800 | codex | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` | issue_open | I4-003 | 同文并存 `审计轮次: 4` 与列表项 `- 审计轮次: 2`，审计标记口径冲突会误导轮次统计。 |
| 2026-03-06 11:48:38 +0800 | codex | `doc/scripts/precommit/pre-commit.project.md` | pass | - | 项目文档中的核心验收命令（pre-commit/fmt/wasm check）均可直接执行。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/README.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/checklists/cross-module-impact-checklist.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/prd.index.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/project.md` | issue_open | I4-201 | 任务项存在未附验收命令/证据链条目，PRD-ID 追溯闭环不完整。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/reviews/consistency-review-round-001.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:54 +0800 | Codex-G4-001 | `doc/core/reviews/consistency-review-round-002.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/consistency-review-round-003.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/consistency-review-round-004.md` | issue_open | I4-203 | 受审基线仍为启动占位值（0 份），与当前已审进度存在状态时效漂移。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-001-archive-migration-plan.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-001-reviewed-files.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-002-dedup-merge-worklist.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-002-reviewed-files.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-003-filename-semantic-worklist.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-003-reviewed-files.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-004-doc-design-quality-worklist.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-004-reviewed-files.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/templates/prd-id-test-evidence-mapping.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/README.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md` | issue_open | I4-202 | 快照保留大量旧 .project.md 路径且被引用可达门禁豁免，存在中风险可达性债务。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.project.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-core.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-engineering.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-game.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-headless-runtime.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-p2p.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-playability_test_result.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-readme.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-root-legacy.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-scripts.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-site.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-testing.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-world-runtime.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/checklists/active-world-simulator.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd.index.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/prd.md` | pass | - | 结构与口径未发现需立即整改的 D4 高中风险问题。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/engineering/project.md` | issue_open | I4-201 | 任务项未形成 PRD-ID->TASK->验收命令->测试证据完整链，存在高风险断链。 |
| 2026-03-06 11:48:55 +0800 | Codex-G4-001 | `doc/core/reviews/round-004-audit-progress-log.md` | pass | - | 审计进度日志结构正常，已按逐篇即时回写机制持续记录。 |
| 2026-03-06 11:48:55 +0800 | codex | `doc/site/github-pages/github-pages-showcase.prd.md` | issue_open | I4-003 | 同文并存 `审计轮次: 4` 与列表项 `- 审计轮次: 2`，审计字段重复导致轮次统计口径不唯一。 |
| 2026-03-06 11:48:58 +0800 | codex | `doc/playability_test_result/game-test.prd.md` | issue_open | I4-004 | agent-browser 命令依赖 `$CODEX_HOME/.codex` 外部路径，当前仓库内未提供可直接执行的本地兜底入口。 |
| 2026-03-06 11:49:15 +0800 | codex | `doc/site/github-pages/github-pages-showcase.project.md` | issue_open | I4-003,I4-006 | 同文并存 `审计轮次: 4` 与 `- 审计轮次: 2`；状态“最近更新 2026-02-10”与当前轮次审计时间差较大，时效维护不足。 |
| 2026-03-06 11:49:16 +0800 | codex | `doc/playability_test_result/game-test.project.md` | issue_open | I4-003,I4-008 | 任务仅 G1~G5 未映射 PRD-ID，且依赖路径 `.codex/skills/playwright/SKILL.md` 在仓库内不可达。 |
| 2026-03-06 11:49:35 +0800 | codex | `doc/playability_test_result/README.md` | issue_open | I4-006 | 文档声明“仅保留最近一天样本”，但活跃卡片同时列出 2026-02-28 与 2026-03-01 多日样本，状态口径不一致。 |
| 2026-03-06 11:49:45 +0800 | codex | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md` | issue_open | I4-002,I4-003 | 里程碑验收命令写为裸 `cargo check`（未对齐 `env -u RUSTC_WRAPPER` 口径），且存在重复审计字段 `- 审计轮次: 2`。 |
| 2026-03-06 11:49:58 +0800 | codex | `doc/playability_test_result/prd.index.md` | pass | - | PRD 索引维持“专题 PRD 与 project 成对登记”且入口链路可达。 |
| 2026-03-06 11:50:09 +0800 | codex | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.project.md` | issue_open | I4-003,I4-006 | 存在重复审计字段 `- 审计轮次: 2`；状态最近更新停在 2026-02-26，缺 ROUND-004 审计后的状态说明。 |
| 2026-03-06 11:50:17 +0800 | codex | `historical removed viewer-tools doc set: capture-viewer-frame.prd` | pass | - | 提供了可直接运行的 fallback 命令（run-viewer-web / capture-viewer-frame）并给出参数语义。 |
| 2026-03-06 11:50:27 +0800 | Codex-G4-001 | `doc/core/reviews/consistency-review-round-004.md` | issue_open | I4-203 | `S_round004` 当前基线记为 18 份，但本分区已回写 45 份，统计口径与实时进度仍未对齐。 |
| 2026-03-06 11:50:35 +0800 | codex | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md` | issue_open | I4-003 | 文档内 `审计轮次` 字段重复（主字段与 `- 审计轮次: 2` 并存），造成轮次判读歧义。 |
| 2026-03-06 11:51:01 +0800 | codex | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.project.md` | issue_open | I4-003,I4-006 | 存在重复审计字段 `- 审计轮次: 2`；状态最近更新时间停在 2026-02-12，缺本轮审计后的时效说明。 |
| 2026-03-06 11:51:27 +0800 | codex | `doc/site/manual/site-manual-static-docs.prd.md` | issue_open | I4-001,I4-002 | 接口路径写成 `site/site/doc/cn/index.html` 不可达；里程碑验收命令写为裸 `cargo check`，与仓库执行口径不一致。 |
| 2026-03-06 11:51:46 +0800 | codex | `doc/site/manual/site-manual-static-docs.project.md` | issue_open | I4-001,I4-006 | 任务项仍使用 `site/site/doc/cn/index.html` 错误路径；状态更新时间停在 2026-02-15，缺本轮审计后的时效标注。 |
| 2026-03-06 11:51:46 +0800 | codex | `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` | issue_open | I4-004 | 验收标准要求“可直接复制命令”，但正文未给出任何可执行命令示例，无法直接按文复现。 |
| 2026-03-06 11:52:07 +0800 | codex | `doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md` | issue_open | I4-002 | 里程碑验收写为裸 `cargo check`，未与仓库统一命令 `env -u RUSTC_WRAPPER cargo check` 对齐。 |
| 2026-03-06 11:52:20 +0800 | G4-002-Aristotle | historical removed standard_3d viewer doc set: viewer-asset-pipeline-ui-system-hardening-2026-03-05.prd | pass | - | PRD 目标态与验证闭环完整，未发现本文件内新增 D4 高中风险分工问题。 |
| 2026-03-06 11:52:33 +0800 | codex | `doc/site/manual/viewer-manual-content-migration-2026-02-15.project.md` | pass | - | 任务、依赖、状态与验收命令口径一致，未发现 D4-001~D4-008 的新增高/中风险问题。 |
| 2026-03-06 11:52:38 +0800 | codex | `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md` | issue_open | I4-005 | WPCM-5 映射到 `PRD-TESTING-004`，与主 PRD 使用的 `PRD-TESTING-WEB-*` 编号体系不一致，追踪链路断裂。 |
| 2026-03-06 11:52:45 +0800 | G4-002-Aristotle | historical removed standard_3d viewer doc set: viewer-asset-pipeline-ui-system-hardening-2026-03-05.project | pass | - | 任务拆解含 PRD-ID 映射与状态时间，项目口径与 PRD 分工一致。 |
| 2026-03-06 11:53:03 +0800 | G4-002-Aristotle | historical removed standard_3d viewer doc set: viewer-web-closure-testing-policy.prd | pass | - | 闭环策略文档与配套 project 分工清晰，未发现新增 D4 高中风险问题。 |
| 2026-03-06 11:53:02 +0800 | codex | `doc/testing/manual/systematic-application-testing-manual.prd.md` | issue_open | I4-004 | 文档定义 required/full 口径与脚本入口，但未给出可直接执行的命令示例，验收复现需跨文档跳转。 |
| 2026-03-06 11:53:25 +0800 | G4-002-Aristotle | historical removed standard_3d viewer doc set: viewer-web-closure-testing-policy.project | issue_open | I4-006 | 依赖列表写为 `doc/world-simulator.project.md`（路径不存在），会导致主项目入口可达性断点。 |
| 2026-03-06 11:53:22 +0800 | codex | `doc/testing/manual/systematic-application-testing-manual.project.md` | issue_open | I4-005 | TMAN-5 使用 `PRD-TESTING-004`，与主 PRD 的 `PRD-TESTING-MANUAL-*` 编号体系不一致，任务追踪存在断链风险。 |
| 2026-03-06 11:53:46 +0800 | G4-002-Aristotle | doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md | issue_open | I4-007 | 主 PRD 合并了 phase8~10 的执行过程与验收产物清单，混入 project/devlog 属性内容。 |
| 2026-03-06 11:53:47 +0800 | codex | `doc/testing/README.md` | pass | - | 入口索引与目录结构一致，未发现 D4-001~D4-008 的立即整改项。 |
| 2026-03-06 11:54:02 +0800 | codex | `doc/site/README.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:54:02 +0800 | codex | `doc/site/github-pages/github-pages-architecture-svg-refresh.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:54:03 +0800 | codex | `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:54:04 +0800 | G4-002-Aristotle | doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md | issue_open | I4-008 | 标题声明“含 PRD-ID 映射”但任务条目仅列 VGRO/VRI 编号，未给出明确 PRD-ID 对应，追溯链不足。 |
| 2026-03-06 11:54:10 +0800 | codex | `doc/testing/prd.md` | pass | - | 模块主 PRD 的分层与追踪口径完整，未发现 D4-001~D4-008 的立即整改项。 |
| 2026-03-06 11:54:30 +0800 | codex | `doc/testing/project.md` | pass | - | PRD-ID 到任务映射完整且依赖可达，未发现 D4-001~D4-008 的新增问题。 |
| 2026-03-06 11:54:48 +0800 | codex | `doc/testing/prd.index.md` | pass | - | 专题 PRD/project 配对索引完整且路径可达，未发现 D4 级别问题。 |
| 2026-03-06 11:54:56 +0800 | codex | `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md` | issue_open | I4-002 | 验收命令未统一为 。 |
| 2026-03-06 11:54:56 +0800 | codex | `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:54:58 +0800 | codex | `doc/site/github-pages/github-pages-content-sync-2026-02-12.prd.md` | issue_open | I4-002 | 验收命令未统一为 。 |
| 2026-03-06 11:54:58 +0800 | codex | `doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:08 +0800 | codex | `doc/scripts/README.md` | pass | - | scripts 模块入口与子目录说明一致，未见 D4-001~D4-008 问题。 |
| 2026-03-06 11:55:30 +0800 | G4-002-Aristotle | doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.prd.md | pass | - | PRD 以规格与验收口径为主，未发现新增 D4 高中风险分工/可达性问题。 |
| 2026-03-06 11:55:32 +0800 | codex | `doc/scripts/prd.md` | pass | - | 模块主 PRD 的脚本分层与 fallback 约束清晰，未见 D4 即时整改问题。 |
| 2026-03-06 11:55:48 +0800 | G4-002-Aristotle | doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.project.md | issue_open | I4-009 | 标题声明“含 PRD-ID 映射”但任务列表未给出 PRD-ID 对应，PRD-ID→TASK 追溯链缺失。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-content-sync-2026-02-25.project.md` | issue_open | I4-001 | 存在 site/site/doc/cn/index.html 路径口径，发布可达性风险。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` | issue_open | I4-001 | 存在 site/site/doc/cn/index.html 路径口径，发布可达性风险。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md` | issue_open | I4-001 | 存在 site/site/doc/cn/index.html 路径口径，发布可达性风险。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-game-first-home-2026-02-25.prd.md` | issue_open | I4-001 | 存在 site/site/doc/cn/index.html 路径口径，发布可达性风险。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-motion-layer.prd.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-motion-layer.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-pointer-interaction.prd.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:49 +0800 | codex | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md` | issue_open | I4-001 | 存在 site/site/doc/cn/index.html 路径口径，发布可达性风险。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/github-pages/github-pages-lean-tech-refresh.prd.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/github-pages/github-pages-lean-tech-refresh.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/prd.index.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:50 +0800 | codex | `doc/site/project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:55:56 +0800 | codex | `doc/scripts/project.md` | pass | - | 任务映射与依赖链路完整，未发现 D4-001~D4-008 新问题。 |
| 2026-03-06 11:56:08 +0800 | G4-002-Aristotle | doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md | pass | - | PRD 目标态与验收口径完整，未发现新增 D4 高中风险分工问题。 |
| 2026-03-06 11:56:14 +0800 | codex | `doc/scripts/prd.index.md` | pass | - | 专题 PRD 与 project 配对完整，索引路径可达。 |
| 2026-03-06 11:56:24 +0800 | G4-002-Aristotle | doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.project.md | issue_open | I4-010 | 状态段缺少“最近更新时间”字段，完成态时效性无法在文档内直接审计。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/README.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap12-market-closure.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap12-market-closure.project.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap123-runtime-consensus-metering.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap123-runtime-consensus-metering.project.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.prd.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap3-install-target-infrastructure.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap3-install-target-infrastructure.project.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:41 +0800 | codex | `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.prd.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.project.md` | issue_open | I4-002 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/governance/readme-resource-model-layering.prd.md` | issue_open | I4-005 | 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/governance/readme-resource-model-layering.project.md` | issue_open | I4-005 | 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/governance/readme-world-rules-consolidation.prd.md` | issue_open | I4-005 | 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/governance/readme-world-rules-consolidation.project.md` | issue_open | I4-005 | 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/prd.index.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-llm-p1p2-production-closure.prd.md` | issue_open | I4-005 | 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-llm-p1p2-production-closure.project.md` | issue_open | I4-005 | 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-p0-p1-closure.prd.md` | issue_open | I4-005 | 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-p0-p1-closure.project.md` | issue_open | I4-002,I4-005 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-prod-closure-llm-distfs-consensus.prd.md` | issue_open | I4-002,I4-005 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-prod-closure-llm-distfs-consensus.project.md` | issue_open | I4-005 | 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.prd.md` | issue_open | I4-002,I4-005 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 缺少 ROUND-002 主从口径主入口声明。 |
| 2026-03-06 11:56:42 +0800 | codex | `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.project.md` | issue_open | I4-005 | 缺少 ROUND-002 主项目入口声明。 |
| 2026-03-06 11:56:44 +0800 | G4-002-Aristotle | doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.prd.md | pass | - | PRD 目标态、非目标与验证链条完整，未发现新增 D4 高中风险问题。 |
| 2026-03-06 11:56:45 +0800 | codex | `doc/scripts/precommit/precommit-remediation-playbook.prd.md` | pass | - | 提供了可直接执行的修复与复检命令序列，脚本路径在仓库内可达。 |
| 2026-03-06 11:57:02 +0800 | G4-002-Aristotle | doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.project.md | issue_open | I4-010 | 状态段仅写“已完成”未给最近更新时间，难以判断完成态时效。 |
| 2026-03-06 11:57:06 +0800 | codex | `doc/scripts/precommit/precommit-remediation-playbook.project.md` | pass | - | 校验步骤给出可执行命令 `./scripts/fix-precommit.sh`，依赖路径可达。 |
| 2026-03-06 11:57:21 +0800 | G4-002-Aristotle | doc/world-simulator/scenario/scenario-files.prd.md | issue_open | I4-011 | PRD 内包含大段执行型测试矩阵与脚本建议，混入 project/testing 手册属性信息。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/README.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-engineering-architecture.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.project.md` | issue_open | I4-002,I4-003 | 验收命令未统一为 env -u RUSTC_WRAPPER cargo check。 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md` | issue_open | I4-003,I4-006 | 存在重复审计字段 - 审计轮次: 2。 状态为进行中或active，需补充与当前审计轮次一致的时效说明。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-module-driven-production-closure.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:27 +0800 | codex | `doc/game/gameplay/gameplay-module-driven-production-closure.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-release-production-closure.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-release-production-closure.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-runtime-governance-closure.prd.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-runtime-governance-closure.project.md` | issue_open | I4-003 | 存在重复审计字段 - 审计轮次: 2。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-top-level-design.prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-top-level-design.project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/prd.index.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/prd.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:28 +0800 | codex | `doc/game/project.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:57:27 +0800 | codex | `historical removed viewer-tools doc set: capture-viewer-frame.project` | pass | - | 验证命令与脚本路径明确且可执行，fallback 使用边界清晰。 |
| 2026-03-06 11:57:38 +0800 | G4-002-Aristotle | doc/world-simulator/scenario/scenario-files.project.md | issue_open | I4-009 | 任务拆解标题声明“含 PRD-ID 映射”但条目未标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:57:51 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-art-capture-2026-02-28.prd` | pass | - | 脚本入口、参数与输出路径定义完整，未发现不可执行命令口径问题。 |
| 2026-03-06 11:58:00 +0800 | G4-002-Aristotle | doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.prd.md | pass | - | 设计文档以 Why/What/Done 为主，未发现新增高风险分工或可达性问题。 |
| 2026-03-06 11:58:18 +0800 | G4-002-Aristotle | doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.project.md | issue_open | I4-009,I4-010 | 标题声明“含 PRD-ID 映射”但任务未标 PRD-ID，且状态段缺最近更新时间。 |
| 2026-03-06 11:58:13 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-art-capture-2026-02-28.project` | pass | - | 任务拆解与依赖口径完整，未发现 D4-001~D4-008 新增问题。 |
| 2026-03-06 11:58:32 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-02-28.prd` | pass | - | 回归命令参数完整且产物路径明确，可直接复现框架验证结果。 |
| 2026-03-06 11:58:49 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-02-28.project` | pass | - | 任务拆解与依赖链路清晰，未发现命令可执行性相关异常。 |
| 2026-03-06 11:59:08 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-03-01.prd` | pass | - | 关键回归产物与参数结果可追溯，未见不可执行验收命令表达。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/agent-default-modules.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/agent-default-modules.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/module-lifecycle.md` | issue_open | I4-019 | 仍引用 legacy 主入口 `doc/world-runtime.prd.md`，与当前模块主入口口径不一致。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/module-storage.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/module-storage.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/module-subscription-filters.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/module-subscription-filters.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md` | issue_open | I4-017 | Validation 区仅描述性验证方法，缺可执行命令/证据路径。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/module/player-published-entities-2026-03-05.project.md` | issue_open | I4-018 | Project 任务缺可执行验收命令与证据链接。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/bootstrap-power-modules.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/bootstrap-power-modules.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-integration.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-executor.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-executor.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-interface.md` | pass | - | 未发现新增 D4-001~D4-008 高中风险问题。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sandbox-security-hardening.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sandbox-security-hardening.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sdk-no-std.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sdk-no-std.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.prd.md` | issue_open | I4-014 | Traceability 表沿用“文档内既有任务条目”，未建立专题 PRD-ID->TASK->命令/证据链。 |
| 2026-03-06 11:59:25 +0800 | codex | `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.project.md` | issue_open | I4-015 | “含 PRD-ID 映射”下任务未全量标注 PRD-ID，追溯链不完整。 |
| 2026-03-06 11:59:31 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-03-01.project` | pass | - | 项目任务与依赖边界清晰，未发现 D4 命令可执行性风险。 |
| 2026-03-06 11:59:55 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-material-recognizability-2026-02-28.prd` | pass | - | 参数与输出元数据定义清楚，未出现不可执行验收命令表达。 |
| 2026-03-06 12:00:19 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-material-recognizability-2026-02-28.project` | pass | - | 依赖中的 viewer 启动命令与脚本路径明确，未发现执行口径冲突。 |
| 2026-03-06 12:00:46 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-visual-detail-system-optimization-2026-02-28.prd` | pass | - | T3 回归命令可直接执行且证据路径清晰，未见命令可执行性缺陷。 |
| 2026-03-06 12:01:08 +0800 | codex | `historical removed viewer-tools doc set: viewer-texture-inspector-visual-detail-system-optimization-2026-02-28.project` | pass | - | 任务链与依赖路径完整，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:01:38 +0800 | codex | `doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md` | pass | - | 构建入口、环境变量与校验脚本定义完整，未见不可执行命令表述。 |
| 2026-03-06 12:02:05 +0800 | codex | `doc/scripts/wasm/builtin-wasm-nightly-build-std.project.md` | pass | - | `CI_VERBOSE=1 ./scripts/ci-tests.sh required` 等验收命令可执行且与依赖口径一致。 |
| 2026-03-06 12:02:31 +0800 | codex | `doc/playability_test_result/prd.md` | pass | - | 模块主 PRD 的证据流程与追踪口径完整，未发现 D4 即时问题。 |
| 2026-03-06 12:02:56 +0800 | codex | `doc/playability_test_result/project.md` | pass | - | 任务拆解含 PRD-ID 映射且状态链路清晰，未发现 D4 新问题。 |
| 2026-03-06 12:03:28 +0800 | codex | `doc/playability_test_result/playability_test_card.md` | pass | - | 反馈模板字段完整且可直接用于人工采集，未涉及不可执行命令口径。 |
| 2026-03-06 12:03:55 +0800 | codex | `doc/playability_test_result/playability_test_manual.md` | pass | - | 归档提示与当前主入口引用一致，未发现 D4 问题。 |
| 2026-03-06 12:04:22 +0800 | codex | `doc/playability_test_result/card_2026_02_28_19_22_20.md` | pass | - | 实测卡片证据链（访问地址/日志/录屏/指标）完整，未涉及命令可执行性缺陷。 |
| 2026-03-06 12:04:48 +0800 | codex | `doc/playability_test_result/card_2026_02_28_21_22_51.md` | pass | - | 长玩卡片给出完整量化指标与证据路径，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:05:09 +0800 | codex | `doc/playability_test_result/card_2026_02_28_22_47_14.md` | pass | - | 卡片包含完整采样与结论字段，未发现 D4-001~D4-008 命令问题。 |
| 2026-03-06 12:05:34 +0800 | codex | `doc/playability_test_result/card_2026_02_28_23_27_06.md` | pass | - | 该卡片回填了访问地址/证据/量化指标，审计未见 D4 命令问题。 |
| 2026-03-06 12:06:01 +0800 | codex | `doc/playability_test_result/card_2026_03_01_00_20_13.md` | issue_open | I4-006 | 量化指标 `TTFC` 记录为 `null`，该卡片关键可比字段缺失会削弱版本横向对比有效性。 |
| 2026-03-06 12:07:03 +0800 | codex | `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.prd.md` | pass | - | AC 中 `sync-m1 --check` 命令与 workflow 路径清晰；后续活跃入口已统一到 determinism gate 命名。 |
| 2026-03-06 12:07:27 +0800 | codex | `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.project.md` | pass | - | 任务映射与依赖链路可达，未发现命令可执行性异常。 |
| 2026-03-06 12:07:50 +0800 | codex | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.prd.md` | pass | - | required-check 注入脚本目标与 dry-run/幂等口径明确，未见不可执行命令表达。 |
| 2026-03-06 12:08:20 +0800 | codex | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.project.md` | pass | - | 任务映射与 `gh api` 依赖关系清晰，未发现 D4 命令问题。 |
| 2026-03-06 12:08:49 +0800 | codex | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.prd.md` | pass | - | 门禁变更范围与保留路径描述清晰，未见不可执行命令写法。 |
| 2026-03-06 12:09:29 +0800 | codex | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.project.md` | pass | - | 项目任务与依赖收敛到脚本/手册改造，审计未见 D4 问题。 |
| 2026-03-06 12:09:53 +0800 | codex | `doc/testing/ci/ci-test-coverage.prd.md` | pass | - | required/full 覆盖与降级路径定义明确，可执行口径无歧义。 |
| 2026-03-06 12:10:16 +0800 | codex | `doc/testing/ci/ci-test-coverage.project.md` | pass | - | 任务拆解包含 PRD-ID 映射且与主 PRD 口径对齐。 |
| 2026-03-06 12:10:43 +0800 | codex | `doc/testing/ci/ci-testcase-tiering.prd.md` | pass | - | required/full 命令与 case 标签策略一致，未发现执行口径异常。 |
| 2026-03-06 12:11:06 +0800 | codex | `doc/testing/ci/ci-testcase-tiering.project.md` | pass | - | 任务拆解与主 PRD 标签口径一致，未发现 D4 新问题。 |
| 2026-03-06 12:11:30 +0800 | codex | `doc/testing/ci/ci-tiered-execution.prd.md` | pass | - | 分级执行命令与触发器分流定义完整，未发现不可执行表达。 |
| 2026-03-06 12:11:54 +0800 | codex | `doc/testing/ci/ci-tiered-execution.project.md` | pass | - | 项目任务与分层规则主入口对应关系清晰。 |
| 2026-03-06 12:12:23 +0800 | codex | `doc/testing/ci/ci-wasm32-target-install.project.md` | pass | - | workflow 与脚本依赖链可达，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:12:51 +0800 | codex | `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md` | pass | - | 指标定义、计算口径与脚本输出链路完整，未见执行命令风险。 |
| 2026-03-06 12:13:13 +0800 | codex | `doc/testing/governance/llm-skip-tick-ratio-metric.project.md` | pass | - | 任务映射到指标实现链路完整，未发现 D4 新问题。 |
| 2026-03-06 12:13:37 +0800 | codex | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.prd.md` | issue_open | I4-002 | 边界处理里给出 `cargo build -p oasis7 --bin oasis7_chain_runtime`，未对齐仓库约定的 `env -u RUSTC_WRAPPER cargo ...` 可执行口径。 |
| 2026-03-06 12:14:03 +0800 | codex | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.project.md` | pass | - | 项目任务与回归产物链路清晰，未见新增命令可执行性问题。 |
| 2026-03-06 12:14:29 +0800 | codex | `doc/testing/governance/wasm-build-determinism-guard.prd.md` | pass | - | 构建护栏规则与 `sync --check` 验证链定义清晰，未见命令可执行性问题。 |
| 2026-03-06 12:14:55 +0800 | codex | `doc/testing/governance/wasm-build-determinism-guard.project.md` | pass | - | 项目任务与护栏落点可追溯，未见 D4 新增风险。 |
| 2026-03-06 12:15:19 +0800 | codex | `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.prd.md` | pass | - | 启动迁移与 fail-fast 边界定义清晰，未发现不可执行命令口径问题。 |
| 2026-03-06 12:15:49 +0800 | codex | `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.project.md` | pass | - | 迁移任务与脚本依赖链路可追溯，未见 D4 新增问题。 |
| 2026-03-06 12:16:14 +0800 | codex | `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md` | pass | - | 最小验收命令均可执行且与仓库 cargo 口径一致。 |
| 2026-03-06 12:16:38 +0800 | codex | `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.project.md` | pass | - | 任务拆解覆盖生命周期硬化主链路，审计未见命令可执行性问题。 |
| 2026-03-06 12:17:03 +0800 | codex | `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.prd.md` | pass | - | 鉴权来源优先级与回退路径定义清晰，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:18:25 +0800 | codex | `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md` | pass | - | 任务映射与依赖边界完整，未发现 D4 级命令问题。 |
| 2026-03-06 12:19:00 +0800 | codex | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md` | issue_open | I4-002 | 任务描述直接写 `cargo check`，未对齐仓库约定的 `env -u RUSTC_WRAPPER cargo ...` 可执行口径。 |
| 2026-03-06 12:19:34 +0800 | codex | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md` | pass | - | 迁移后的脚本入口与采样字段口径明确，未发现不可执行命令写法。 |
| 2026-03-06 12:20:04 +0800 | codex | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.project.md` | pass | - | 项目拆解与脚本依赖对齐，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:20:33 +0800 | codex | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.prd.md` | pass | - | 连续注入参数、统计字段与手册接线定义完整，未见命令可执行性问题。 |
| 2026-03-06 12:21:50 +0800 | codex | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.project.md` | pass | - | 项目任务覆盖持续注入实现与证据收口，未发现 D4 新问题。 |
| 2026-03-06 12:22:24 +0800 | codex | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.prd.md` | pass | - | 模板路径、执行命令口径与证据字段定义完整，未见命令可执行性缺陷。 |
| 2026-03-06 12:22:56 +0800 | codex | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md` | pass | - | 模板落地与手册接线任务可追溯，未见 D4 新问题。 |
| 2026-03-06 12:23:23 +0800 | codex | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.prd.md` | pass | - | feedback 注入参数与统计产物口径完整，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:23:55 +0800 | codex | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md` | pass | - | 项目任务覆盖注入执行与证据扩展，未见 D4 新问题。 |
| 2026-03-06 12:24:26 +0800 | codex | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md` | issue_open | I4-006 | 文档仍将 `oasis7_viewer_live` 作为长跑架构核心，和后续 `oasis7_chain_runtime` 迁移口径存在主入口冲突。 |
| 2026-03-06 12:26:17 +0800 | codex | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.project.md` | issue_open | I4-006 | 依赖仍大量指向 `oasis7_viewer_live` 旧链路，与已迁移到 `oasis7_chain_runtime` 的执行口径不一致。 |
| 2026-03-06 12:26:54 +0800 | codex | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.prd.md` | pass | - | bootstrap 幂等逻辑与 S10 指标恢复口径清晰，未见命令可执行性问题。 |
| 2026-03-06 12:27:56 +0800 | codex | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.project.md` | pass | - | 项目任务与证据样本路径完整，未发现 D4 命令问题。 |
| 2026-03-06 12:28:27 +0800 | codex | `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | issue_open | I4-006 | 文档架构仍以 `oasis7_viewer_live` 为核心，但同分区后续专题已迁移到 `oasis7_chain_runtime`，主入口口径冲突。 |
| 2026-03-06 12:28:56 +0800 | codex | `doc/testing/longrun/s10-five-node-real-game-soak.project.md` | issue_open | I4-006 | 依赖仍绑定 `oasis7_viewer_live` 路径，与 longrun 新口径 `oasis7_chain_runtime` 并行冲突，易造成执行歧义。 |
| 2026-03-06 12:29:30 +0800 | codex | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.prd.md` | pass | - | 性能采样与输出接线定义完整，未发现 D4 命令可执行性问题。 |
| 2026-03-06 12:30:02 +0800 | codex | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.project.md` | pass | - | 项目任务与性能观测接线完整，未发现 D4 新增问题。 |
| 2026-03-06 12:30:29 +0800 | codex | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.prd.md` | pass | - | decision/llm_api 拆分公式与边界处理规则完整，未见 D4 命令问题。 |
| 2026-03-06 12:30:58 +0800 | codex | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.project.md` | pass | - | 项目任务与解耦口径一致，未发现 D4 新增问题。 |
| 2026-03-06 12:31:29 +0800 | codex | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md` | pass | - | hotspot 规则与输出兼容策略明确，未见 D4 命令可执行性问题。 |
| 2026-03-06 12:32:00 +0800 | codex | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.project.md` | pass | - | 项目任务与脚本接线路径一致，未发现 D4 新问题。 |
| 2026-03-06 12:32:30 +0800 | codex | `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.prd.md` | pass | - | 性能方法论流程与门禁参数定义完整，未见 D4 命令可执行性问题。 |
| 2026-03-06 12:32:57 +0800 | codex | `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.project.md` | pass | - | 项目任务与 stress 脚本接线一致，未发现 D4 新增问题。 |

| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/viewer-manual.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-runtime.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-runtime.project.md` | issue_open | I4-008 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator.prd.md` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator.project.md` | issue_open | I4-008 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-hook-foundation.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-hook-foundation.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-readiness.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-readiness.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/rust-wasm-build-suite.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/rust-wasm-build-suite.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.project.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-agent-behavior.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-agent-behavior.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-async-openai-responses.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-async-openai-responses.project.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-dialogue-chat-loop.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-dialogue-chat-loop.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-factory-strategy-optimization.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-factory-strategy-optimization.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-industrial-mining-debug-tools.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-industrial-mining-debug-tools.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-lmso29-stability.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-lmso29-stability.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-multi-scenario-evaluation.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-effect-receipt.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-effect-receipt.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-system.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/llm/llm-prompt-system.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-industrial-economy-wasm.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-industrial-economy-wasm.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-power-system.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-power-system.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/material-multi-ledger-logistics.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/m4/material-multi-ledger-logistics.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/prd/acceptance/unified-checklist.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/prd/launcher/blockchain-transfer.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/prd/quality/experience-trend-tracking.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/asteroid-fragment-renaming.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/asteroid-fragment-renaming.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/chunked-fragment-generation.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/chunked-fragment-generation.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/frag-resource-balance-onboarding.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/frag-resource-balance-onboarding.project.md` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/fragment-spacing.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/fragment-spacing.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-power-facility-baseline.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-seed-locations.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/scenario-seed-locations.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/world-initialization.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/scenario/world-initialization.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-2d-3d-clarity-improvement.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-2d-3d-clarity-improvement.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-2d-visual-polish.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-2d-visual-polish.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-3d-commercial-polish.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-3d-commercial-polish.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-3d-polish-performance.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-3d-polish-performance.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-agent-module-rendering.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-agent-module-rendering.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-agent-quick-locate.project.md` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-agent-size-inspection.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-agent-size-inspection.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-auto-focus-capture.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-auto-focus-capture.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-auto-select-capture.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-auto-select-capture.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-bevy-web-runtime.prd` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-bevy-web-runtime.project` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-enter-send.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-enter-send.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-ime-cn-input.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-ime-cn-input.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-prompt-presets.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-right-panel-polish.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-right-panel-polish.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase1-asset-pipeline.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase1-asset-pipeline.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase2-visual-quality-gate.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase2-visual-quality-gate.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase3-material-style-layer.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase3-material-style-layer.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase4-texture-style-layer.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase4-texture-style-layer.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase5-advanced-texture-maps.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase5-advanced-texture-maps.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase6-material-variant-preview.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase6-material-variant-preview.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase7-theme-pack-batch-preview.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase7-theme-pack-batch-preview.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-copyable-text.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-copyable-text.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-dual-view-2d-3d.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-dual-view-2d-3d.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-egui-right-panel.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-egui-right-panel.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-frag-default-rendering.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-frag-default-rendering.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-fragment-element-rendering.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-generic-focus-targets.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-generic-focus-targets.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-i18n.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-i18n.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-industrial-visual-closure.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-industrial-visual-closure.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-location-depletion-visualization.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-location-depletion-visualization.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-location-fine-grained-rendering.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-location-fine-grained-rendering.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-manual.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-minimal-system.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-minimal-system.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-module-visual-entities.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-module-visual-entities.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-observability-visual-optimization.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-observability-visual-optimization.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.stress-report.template` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-overview-map-zoom.project.md` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-release-full-coverage-gate.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-release-full-coverage-gate.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-release-qa-iteration-loop.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-release-qa-iteration-loop.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-rendering-physical-accuracy.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-rendering-physical-accuracy.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-right-panel-module-visibility.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-right-panel-module-visibility.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-selection-details.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-selection-details.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-texture-inspector.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-texture-inspector.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visual-release-readiness-hardening-2026-03-01.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visual-release-readiness-hardening-2026-03-01.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visual-upgrade.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visual-upgrade.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visualization-3d.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visualization-3d.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visualization.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-visualization.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-wasd-camera-navigation.prd` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-wasd-camera-navigation.project` | issue_open | I4-002,I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-semantic-test-api.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.prd.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.project.md` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.project.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-webgl-deferred-compat-2026-02-24.prd` | issue_open | I4-002 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `historical removed standard_3d viewer doc set: viewer-webgl-deferred-compat-2026-02-24.project` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md` | pass | - | F4-002 补审覆盖：结构与口径未发现新增高/中风险。 |
| 2026-03-06 14:56:32 +0800 | codex-F4-002 | `doc/world-simulator/viewer/viewer-websocket-http-bridge.project.md` | issue_open | I4-006 | F4-002 补审覆盖：登记问题并进入整改阶段。 |
