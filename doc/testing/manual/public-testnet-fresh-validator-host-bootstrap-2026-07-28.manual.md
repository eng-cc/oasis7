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

- `oasis7-linux-x64-bundle.tar.gz` from the approved package artifact.
- A governed stage `config/` containing the bootstrap bundle, genesis,
  validator registry, `node.env`, and the production manifest and bootstrap-peer inputs.
- A governed stage `generated-world/` containing `snapshot.json`,
  `world-generation-provenance.json`, and `generated-scenario-world/`.

Run as the host operator:

```bash
./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh \
  --bundle-tar /srv/oasis7/oasis7-linux-x64-bundle.tar.gz \
  --config-dir /srv/oasis7/stage/config \
  --world-dir /srv/oasis7/stage/generated-world \
  --node-id triad-testnet-sequencer \
  --receipt /opt/oasis7/p2p-testnet/evidence/fresh-validator-host-bootstrap-receipt.json
```

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
