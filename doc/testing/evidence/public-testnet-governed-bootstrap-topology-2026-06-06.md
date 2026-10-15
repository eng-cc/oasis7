# Public Testnet Governed Bootstrap Topology (2026-06-06)

证据窗口边界：本文的 `current`、`live`、`fresh`、lane verdict 和后续建议仅指文中记录的采集窗口，不证明当前端点可用、节点健康、发布 readiness 或持续任务义务。当前部署/恢复以[governed bootstrap runbook](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md)为准，当前 readiness 以[formal network runbook](../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md)及新采集的同窗证据裁决；保留本次失败、通过与未验证范围，不以较晚结果覆盖。

本文件保留冻结 bootstrap 的 `2 validators + 2 observers` 身份与计划，不等于当前受管 fleet 清单；原 Follow-up gates 是当时的引导次序。不得改写配套 replay bundle 的 payload/hash 来伪造新部署。

## Scope
1. This artifact advances `public_testnet` from signer-truth wiring to a concrete fresh bootstrap candidate.
2. The governed validator set is intentionally frozen as `2 validators + 2 observers`, not `4 validators`.
3. This matches the currently recoverable repo/local truth:
  - two validator signer public keys are concretely frozen
  - two observer node identities are known from prior local expansion evidence

## Cold-start world
- `chain_id`: `oasis7-public-testnet-governed-20260606`
- `world_id`: `oasis7-public-testnet-governed-20260606`
- validator registry: `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
- bootstrap peers: `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`

## Node matrix
| node_id | planned role | validator set | bootstrap source | concrete truth |
| --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator | yes | yes | public ECS peer `/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp`; current deployment finality signer `65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16` |
| `triad-testnet-storage` | validator | yes | yes | public ECS peer `/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj`; current deployment finality signer `858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6` |
| `triad-testnet-local` | observer | no | no | existing local observer peer id `12D3KooWNphsGixZxpqmZf9RSVhCWHH7hWFYZBN8izmWfXQYAXTQ`; joins after validator pair is live |
| `triad-testnet-fourth-local` | observer | no | no | fourth-node peer id `12D3KooWAkDbJby8wGRhnESJYFR7q6DWfNXQ7Ea2ZrZvehezj47s`; joins after validator pair is live |

## Why the validator count is 2
1. Current concrete validator signer truth only exists for:
  - `triad-testnet-sequencer`
  - `triad-testnet-storage`
2. Current concrete observer truth exists as node identity / peer topology, not as validator admission truth.
3. Therefore the honest fresh bootstrap candidate is:
  - governed validator cold-start with 2 validators
  - four-node network rollout with 2 observers added after validator pair reaches healthy head

## Follow-up gates
1. Bring up validator pair from this registry with `--genesis-validator-registry`.
2. Re-attach `triad-testnet-local` as observer against the fresh world.
3. Re-attach `triad-testnet-fourth-local` as observer against the fresh world.
4. Only after these four nodes are healthy should `public_testnet` readiness be reconsidered.
