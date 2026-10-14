# Fresh WorldService Preview Bootstrap

- Owner role: `blockchain_ops_engineer`
- Review roles: `runtime_engineer`, `qa_engineer`
- Scope: a new, isolated DevLocal technical preview; never recovery of an existing world or formal authority activation.
- Authorities: [P2P identity/profile boundaries](../../p2p/prd.md#p2p-authority-profiles), [hosted player isolation](../../p2p/blockchain/hosted-player-access-operator-runbook.md), [design](../../p2p/design.md#des-p2p-fresh-preview-bootstrap).

## Inputs and trust

The operator independently fixes the exact world/node IDs, a protected node identity directory, a fresh absolute destination and a loopback WorldService endpoint. Generate an explicit 32-byte genesis initialization nonce once and retain it privately with the operation inputs. It is not a simulation seed, token, password or formal issuer authorization.

Use the existing `provision-identity --config-dir <absolute-private-directory> --node-id <exact-node-id>` only when creating a new node identity. When preserving a node identity, supply its existing, validated key configuration. The key directory must be owned by the invoking user with mode 0700; `node-keypair.toml` is a regular, non-symlink 0600 file. Generate and run on the same machine so the node signing seed need not be exported.

The initializer consumes these explicit inputs:

```bash
oasis7_chain_runtime initialize-preview-world \
  --root <fresh-absolute-root> \
  --world-id <new-world-id> \
  --node-id <exact-node-id> \
  --key-dir <absolute-private-key-directory> \
  --status-bind 127.0.0.1:5121 \
  --genesis-nonce <retained-64-hex-initialization-nonce>
```

Do not use an old world's ID or overwrite a partial old root. Stop/fence any old writer before switching the runtime to a new initialized root; keep its data and configuration separately. A new world is not a rollback or repair of old lineage.

## Generated materials

The command creates the actual canonical Runtime initial state and journal, binds the new world identity, and publishes real bounded node/execution commits into CAS. The initial commit must use the provisioned node's derived consensus signer and matching validator public-key binding; verify its actual signature separately from the service-signed initialization receipt. Its domain-separated genesis digest binds versioned initial material, explicit initialization inputs and the derived service public key. Retained initial bytes permit recomputation; an arbitrary genesis label or a hash of a directory path is insufficient.

Public material contains the world identity, initialization receipt, genesis material commitments, service public-key pin and immutable initial publication references. The service key is the node's derived feedback-submit signing identity, not an arbitrary node public key. Independent public-read credentials and Viewer configuration remain in private 0600 files; public receipts and CAS must not contain signing seeds.

The layout is `world/`, `records/`, `store/`, `replication/` and `state.json`, with `initialization-receipt.json`, `viewer.public.json` and `genesis/initial-artifact.json` as public material. Retain `replication/replication_commit_messages/` for the signed initial commit; its payload hashes must match the pinned execution record. `private/public-reader.key` and `private/viewer.env` contain private consumer material. The receipt is signed under its explicit preview-initialization domain; initializer source/schema commitments bind the original CLI version. A future mismatched initializer rejects a rerun instead of migrating or resealing the old root implicitly.

The root is staged in a private sibling on the same filesystem, synchronized, then published with an operating-system atomic no-replace operation. Existing targets are verified against their original receipt, identity, signer, retained genesis bytes and actual CAS/record/proof; they are not rewritten or silently repaired. Conflicting or incomplete roots fail closed.

## Starting and checking the consumer

Use the generated layout for the chain execution-world, records, CAS and execution-state paths, and keep the same node identity configuration. Do not let default `--runtime-root` resolution select a different store. Inject the private Viewer configuration through the service manager; do not paste it into logs, a PR, screenshots or shell arguments.

The consumer fixes `OASIS7_WORLD_SERVICE_ENDPOINT`, `OASIS7_WORLD_SERVICE_PUBLIC_KEY`, `OASIS7_WORLD_SERVICE_WORLD_ID`, `OASIS7_WORLD_SERVICE_GENESIS_DIGEST`, `OASIS7_WORLD_SERVICE_SCOPE` and `OASIS7_WORLD_SERVICE_READ_PRIVATE_KEY`. Public scope only permits reading. Agent writes require their separate signer, valid owner/executor delegation and generation; initialization does not synthesize those grants.

Verify on the deployment machine that the original receipt/genesis commitments still match, the node opens the recorded CAS and commit proof, and a signed WorldService read validates against the independently pinned service key and world. Verify the same after a controlled restart. Loading JSON or reaching a status port alone does not establish this chain.

## Regression and acceptance boundaries

Test fresh initialization and actual pinned read, matching rerun after state advancement, conflicting world/node/key/nonce, missing or modified genesis/receipt/CAS/proof, invalid key permissions and symlinks, and competing target publication. Failed initialization must not overwrite existing worlds, publish partial success or repair corrupt inputs.

Record the actual source and binaries used. Cross-platform generation is not presumed compatible: relevant schema/hash/persistence code must be equivalent and the target machine must actually open and verify the pin.

These materials prove isolated preview initialization only. Pending cognition binding and bounded local commits do not certify formal finality, activation/profile history, distributed consensus, independent failure-domain durability, production minting or invitation readiness. Complete account/world-control and applicable gameplay acceptance separately before expanding the player entry; deployment observations belong in the delivery issue.
