# Public Testnet Fresh Validator Host Bootstrap

Lifecycle: current reusable no-start subprocedure; the date is its establishment date, not an evidence window.

Current operator authority: `doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md`. This manual applies only beneath that runbook's current deployment truth and never replaces topology review, rollout, rollback, recovery, readiness, or release decisions.

Historical provenance task: `task_6afeb15f921a43bd971b1b2bf54222d4`.

## Purpose

Bootstrap only an erased sequencer host at `/opt/oasis7/p2p-testnet` from a
verified Linux package and a governed config/world stage. This procedure does
not start, enable, or unmask the validator. It creates the node identity with
the packaged runtime's no-start `provision-identity` command, and emits a
public-only receipt for later topology review.
Revalidate the package, governed config/world stage, node identity, and current
deployment truth on every invocation; the 2026-07-28 provenance does not make
later inputs or hosts ready.

## Required inputs

- `oasis7-linux-x64.deb` from the approved Linux package artifact.
- `oasis7-linux-x64-ops-tools.tar.gz` from the matching checksummed operator-tools artifact.
- A governed stage `config/` containing the bootstrap bundle, genesis,
  validator registry, `node.env`, and the production manifest and bootstrap-peer inputs.
- A governed stage `generated-world/` containing the canonical execution world
  at `world/`, plus `world-generation-provenance.json` and
  `generated-scenario-world/`. Pass the `generated-world/` directory itself to
  `--world-dir`; bootstrap consumes this exact stage output.

Run as the host operator:

```bash
./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh \
  --package-deb /srv/oasis7/oasis7-linux-x64.deb \
  --ops-tools-tar /srv/oasis7/oasis7-linux-x64-ops-tools.tar.gz \
  --config-dir /srv/oasis7/stage/config \
  --world-dir /srv/oasis7/stage/generated-world \
  --node-id triad-testnet-sequencer \
  --receipt /opt/oasis7/p2p-testnet/evidence/fresh-validator-host-bootstrap-receipt.json
```

For the governed third validator, use the triad inventory as the sole
operator authority and stage validator-47 separately from the existing pair.
The validator-47 identity is staged once by the identity ceremony and then
imported byte-for-byte; the final host bootstrap must never regenerate it. The
identity directory must contain regular `node-keypair.toml` and
`identity-receipt.json` files, both mode `0600`, with ownership matching their
mode-`0700` directory. The public receipt's finality key must match the
validator-47 entry in the governed registry.
The stage emits a complete `node.env` binding for validator-47, including the
validator role, provider capabilities, `0.0.0.0:6634` status and
`0.0.0.0:6834` gossip listeners, governed world, manifest, registry, and
inventory digest:

The inventory role is `validator`; the runtime mapping is the supported
`NODE_ROLE=storage` plus independent `P2P_NODE_ROLE=full_storage`. Do not
emit the unsupported runtime value `NODE_ROLE=validator`.
The bootstrap peer input is governed bytes, not a caller-selected topology:
use the exact evidence file
`doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt`
with SHA-256 `c7d0b977937adb5d27733ed0ad3e2212ccd0f3ac1b2273214e8cc57df110e5d6`.
Stage rejects wrong or stale peer files and source-registry digest drift before
creating the stage directory.
The source registry digest is only the immutable build-input link. The staged
deployment registry has its own exact-byte `generated_registry_sha256` and
canonical `generated_registry_semantic_sha256`; runtime status must report the
generated digest, never the source-input digest.

The following two commands are the exact stage-to-bootstrap handoff. They use
the stage root as the bootstrap input; do not manually copy
`generated-world/world/`, rewrite paths, or pair a snapshot with a different
sidecar or provenance file.

```bash
./scripts/p2p-public-testnet-build-deployment-stage.sh \
  --runtime-build-ref /srv/oasis7/oasis7_chain_runtime \
  --bootstrap-peers-file /srv/oasis7/bootstrap-peers.txt \
  --sequencer-finality-public-key <sequencer-finality-public-key> \
  --storage-finality-public-key <storage-finality-public-key> \
  --extra-validator triad-testnet-validator-47:<validator-47-finality-public-key>:100 \
  --validator-47-identity-dir /srv/oasis7/staged-validator-47-identity \
  --out-dir /srv/oasis7/stage/validator-47

./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh \
  --package-deb /srv/oasis7/oasis7-linux-x64.deb \
  --ops-tools-tar /srv/oasis7/oasis7-linux-x64-ops-tools.tar.gz \
  --config-dir /srv/oasis7/stage/validator-47/config \
  --world-dir /srv/oasis7/stage/validator-47/generated-world \
  --identity-dir /srv/oasis7/stage/validator-47/identity \
  --node-id triad-testnet-validator-47 \
  --service-name oasis7-triad-validator-47.service \
  --receipt /opt/oasis7/p2p-testnet/evidence/fresh-validator-host-bootstrap-receipt.json
```

Bootstrap flattens the canonical stage `generated-world/world/` into the
host's `staged-world/` while retaining the generated sidecar and provenance at
that same staged-world root. The receipt records `world.layout=nested_stage`.

The independent readback remains a separate observation command:

```bash
/opt/oasis7/p2p-testnet/current/bin/service-readback --read-only \
  --role validator-47 \
  --root /opt/oasis7/p2p-testnet \
  --service oasis7-triad-validator-47.service
```

This is an executable no-start staging step: the service is rendered but never
enabled or started. Readback must independently prove
`UnitFileState=disabled`, inactive/dead service state, `no_process=true`, and
`no_listener=true` for both validator-47 ports. A stale pair identity or any
wrong `node.env` field is rejected before materialization.
The no-start preflight and independent readback also reject a stack-local
`start-node.sh`/`oasis7_chain_runtime` orphan under the canonical root even
when systemd reports `MainPID=0`; unrelated processes outside that root are
outside this bounded check.
The bootstrap uses the runtime's read-only `identity-receipt` probe against the
imported key and records only public identities and SHA-256 digests; it does
not invoke `provision-identity` for validator-47.

Production execution must be root. The script creates or validates the fixed
no-login/no-home `oasis7-testnet` system account, then rejects a non-empty root, unsafe archive entries, symlinked target
paths, invalid JSON, missing C1 binaries, checksum mismatch, or a BUILDINFO /
governed runtime mismatch before creating the stack root. It installs the
systemd unit disabled and inactive; do not treat the receipt as a readiness or release claim.

## Handoff and recovery

Review the receipt's runtime hash, root/finality/libp2p public identities and
the rendered unit before any topology stage. To abandon a failed bootstrap,
do not start the service; preserve the public receipt and package input for
investigation. A failed post-materialization bootstrap safely removes only the
exact root it created during that invocation. The production receipt path is
fixed at `/opt/oasis7/p2p-testnet/evidence/fresh-validator-host-bootstrap-receipt.json`
and contains public identities and hashes only.

Keep the existing pair running on its current deployment truth while
validator-47 staging and readback are reviewed (pair preservation). If a cold cutover
is approved, make it staggered: stop and independently read back one
existing pair member, stage and verify it, then stop the second member; never
stop or start both pair members together, and never start validator-47 as part
of staging. Only after all three identities, provider roles, peers, and health
observations close may an operator perform the explicit launch.

If validator-47 fails preflight or health, leave its unit disabled and retain
the public receipt and readback. After proving no process and no listener,
perform a clean redeploy from a newly generated triad stage and a newly
ceremonied validator-47 identity; never reuse a pair identity or an unverified
staged key.
Do not reuse pair `node.env`, peerstore, runtime data, or world directories as
rollback material; clean redeploy is the durable rollback path.
