# Blockchain 子域入口

## 从这里开始

- 想确认链 HTTP 提交的请求身份、accepted 后的处理责任，以及 Explorer 生命周期、独立查询与组合过滤：先读 [chain HTTP / Explorer 专业需求](chain-http-submit-explorer-contract.prd.md)，实现时进入[配对设计](chain-http-submit-explorer-contract.design.md)。
- 想确认当前网络层级、`public_testnet`/`mainnet` 的 claim boundary、governed bootstrap 或 operator 路径：先读 `formal-network-tiers-testnet-mechanism.prd.md`；执行时再进入 `formal-network-tiers-testnet-mechanism.runbook.md` 和 `public-testnet-governed-bootstrap.runbook.md`。
- 想确认主链安全基线、剩余 mainnet-grade blocker、signer custody 或 genesis ceremony：先读 `p2p-mainnet-security-governance-readiness.prd.md`，再下钻到资产授权、registry drill 和 network-tier 专业证据。
- 想确认普通玩家的 `hosted_public_join`、邮箱登录、托管 player signer 和后续自托管升级：先读 `hosted-public-join-managed-identity-custody.prd.md`。它不覆盖 node、validator 或 governance signer custody。
- 想查看 `blockchain-p2pfs-hardening-phase*` 的历史合同与阶段追溯：先读 `p2p-blockchain-p2pfs-hardening.prd.md`，再按需进入同名 design/project；精确文件检索回到 `../prd.index.md`。

## 首读主题簇

| 主题簇 | 默认入口 | 入口边界 |
| --- | --- | --- |
| Chain HTTP / Explorer | [chain-http-submit-explorer-contract.prd.md](chain-http-submit-explorer-contract.prd.md) | 同一 Node 的 HTTP 接纳、请求关联和 Explorer 后端查询合同；目标要求与实现验收分开，界面由 Launcher 专题承接。 |
| Formal network tiers | `formal-network-tiers-testnet-mechanism.prd.md` | `local_devnet -> public_testnet -> mainnet` 的现行语义、promotion gate 与公开口径；operator 步骤见 companion runbook。 |
| Mainnet-grade security | `p2p-mainnet-security-governance-readiness.prd.md` | 当前仍是 `not_mainnet_grade` 的 custody、governance、genesis 和 QA gate；不以历史规格完成态升级结论。 |
| Hosted player identity | `hosted-public-join-managed-identity-custody.prd.md` | `hosted_public_join` 的玩家身份、邮箱登录与 managed player signer；不替代协议级 signer custody。 |
| P2PFS / blockchain hardening history | `p2p-blockchain-p2pfs-hardening.prd.md` | phase2~8、production-grade roadmap、Phase B 与 Phase C 已收敛到稳定三件套；Phase C 网络证明仍是 future gap。 |

## 待评审方案

- [免邮箱注册与多凭证账号体系 PRD](hosted-public-join-key-based-onboarding.prd.md)与[配套系统设计](hosted-public-join-key-based-onboarding.design.md)：`draft`，覆盖通行密钥、可选邮箱、多凭证与恢复、账号会话和单世界控制、事务存储、迁移及撤销。入库只供评审，不覆盖现行托管身份合同，不表示已实现或已发布；正式采纳前须完成 PRD §6.4 与系统设计 §18.3 的条款同步和适用评审。

## 现行与历史边界

- `doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md` 是旧 `shared_devnet -> staging -> canary` rehearsal 的 rollback/provenance 入口；它不能证明 `public_testnet`、`mainnet` 或公开大世界 readiness。当前 network-tier 真值和 checklist 以 `formal-network-tiers-testnet-mechanism.*` 为准。
- 历史 rehearsal 和已完成安全专题仍保留为审计/追溯材料；P2PFS hardening phase2~8 的合同与完成态已迁入 `p2p-blockchain-p2pfs-hardening.*`，原 phase 文件名仅保留在历史审计文字和 Git history 中。
- production-grade roadmap、Phase B commit-execution 与 Phase C DistFS proof-network 独立三件套也已迁入同一稳定入口；Phase C 的旧“完成”只作历史 provenance，不表示当前存在跨节点 challenge driver、topic/envelope、mainnet 或 production readiness。
- 根 [`README.md`](../../../README.md) 是当前公开状态权威，[产品层公开口径分册](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md) 定义长期沟通生命周期；network-tier PRD 与 runbook 只提供专业边界和证据，不单独升级公开状态。

## 维护规则

- 新增 blockchain 专题先归入链 HTTP / Explorer、现行网络层级、主链安全、hosted player identity 或可检索历史之一；只有改变默认首读路径时才更新本页和 `../README.md` / `../prd.index.md`。
- 需要淘汰历史专题时，先完成 successor 和活跃引用审计；保留审计证据不等于把它作为现行默认入口。
