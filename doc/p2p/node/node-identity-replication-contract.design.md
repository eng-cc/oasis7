# Node identity and replication contract design

- PRD: `doc/p2p/node/node-identity-replication-contract.prd.md`
- Project record: `doc/p2p/node/node-identity-replication-contract.project.md`

## Design position

The design joins only the node-side contracts that must agree: local identity
bootstrap, optional validator signer binding, signed replication, persistence,
and fail-closed recovery. It consumes runtime-published status rather than
creating a second consensus, reachability, or topology truth.

## Flow

```text
config validation/bootstrap -> node identity + optional validator binding
  -> signature/source validation -> apply + persist guard state
  -> publish observed replication progress/status
```

Each arrow is ordered. In particular, unsuccessful validation or persistence
does not advance observed peer or committed progress. Startup load failure is a
blocked start, not a default-state recovery.

## Components and constraints

| Component | Current responsibility | Constraint |
| --- | --- | --- |
| Config bootstrap | Load a usable local node keypair on the verified current path | Explicit error for malformed/unwritable config; no silent replacement of invalid identity. |
| Signer binding | Validate normalized validator-to-ed25519 public-key mapping when configured | Missing/mismatched signing key rejects the message; governance/admission remains external. |
| Replication handle | Inject the node replication network and isolate its world/topic route | Injection is not a deployed libp2p or public-reachability claim. |
| Replication ingest | Verify source/signature, apply record, persist ordering guards, then observe progress | Invalid, stale, duplicate, or failed records cannot update status as success. |
| Recovery | Load required node/PoS and replication state | Corruption is surfaced and blocks the applicable start; restarting does not repair root cause. |
| Observability | Expose failure/progress to the runtime status and operations evidence path | The triad monitor only samples/projections; it does not redefine runtime truth. |

## Security and operating boundary

Node identity, transport/session identity, consensus signer, and governance
signer remain distinct. Local config generation supplies none of the custody,
rotation, revocation, registry, or ceremony guarantees required for production.
No private signer material is emitted to documentation or operational artifacts.

Historical UDP and `aw.*` transport wording is deliberately excluded. Likewise,
this design adds no automatic deploy/restart/rollback/restore/state-sync path;
such operations require separately authorized environment evidence.

## Evolution

Future transport or topology work may update this contract only with current
runtime evidence and the network authority. It must preserve apply-before-
observe ordering, explicit recovery failure, and the separation between a local
bootstrap key and production signer/governance truth.
