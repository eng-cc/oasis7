# Public-testnet governance trust-root provisioning

The clean-room adapter consumes one structured JSON trust-root artifact. The
artifact is not a caller-selected identity and is not a replacement for the
independent provenance verifier. It is a deployment-bound regular file at:

`/operator/truth/governance-root.json`

The repository fixture is the schema and canonical-digest contract:

`scripts/fixtures/oasis7-governance-root.v1.json`

For the governed fixture deployment, provision the exact fixture bytes as the
code-owned path, then enforce the operator-local ownership contract:

```sh
mkdir -p /operator/truth
install -o "$(id -u)" -g "$(id -g)" -m 0600 \
  scripts/fixtures/oasis7-governance-root.v1.json \
  /operator/truth/governance-root.json
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 /operator/truth/governance-root.json
else
  sha256sum /operator/truth/governance-root.json
fi
```

The `mkdir` step must create only the governed parent path; do not replace the
artifact with a symlink or copy it from another node. `shasum -a 256` is the
portable macOS/Linux spelling when available, with `sha256sum` retained for
minimal Linux images.

For this fixture the expected file digest is
`f278bc8f060cd6777d68f086fc3131edc5d6b5a6080bde09208ba69a69e3ef66`; the
embedded canonical `root_digest` is
`5abd00f3e90a3e894f110f5a32ecab772e23e97ad7ec2cc9d675ae65282ae8ab`.

The adapter pins both the file SHA-256 and the semantic `root_digest` computed
by the repository provenance helper. It also rejects a symlink in the file or
any ancestor. `owner_scope=operator-local` means the account executing the
adapter must own the regular file; the numeric UID is deployment-local and is
never supplied by a plan or authority envelope. The fixture's public-key
values are non-secret test material and must not be treated as production
signing credentials.

## Identity-v2 admission profile (not provisioned)

Identity-v2 is a separate signing domain and authority profile. It does not
reuse the rebuild-proof signer, a node-consensus key, or the fixture keys
above. The profile may be activated only after a governance-root-approved
deployment record authorizes it; writing a tool or a digest does not grant
signer authority. This profile is currently **NOT PROVISIONED / CAPABILITY
BLOCKED**. No live provider, private key, credential, node, or production
identity configuration is created by this document.

The deployment must reserve these code-owned paths before activation:

| artifact | deployment path | independent admission requirement |
| --- | --- | --- |
| governance root | `/operator/truth/governance-root.json` | The fixed regular file and semantic `root_digest` must match the independently pinned governance-root values above. |
| identity-v2 trust config | `/operator/truth/identity-v2-trust-config.json` | Exact file SHA-256 is pinned by deployment authority under the governance root; the caller may assert this path but may not select another path or supply the pin. |
| provider registry | `/operator/truth/identity-v2-provider-registry.json` | Exact registry bytes and digest are pinned by deployment authority under the governance root; `provider-ref` selects only a non-secret allowlisted ID. |
| independent verifier | path recorded by the provider registry | The executable must be a deployment-owned regular executable with an independently pinned SHA-256; a self-reported digest or caller command/endpoint is not evidence. |

The identity-v2 trust config must use the exact schema
`oasis7.identity_v2_trust_config.v1` and fields `schema_version`, `network_id`,
`trust_root_id`, `verifier_id`, `algorithm`, `rotation_epoch`, `allowlist`,
and `revocations`. Each allowlist entry binds one immutable `signer_id` to a
public-only `public_key_ref`, its `public_key_sha256`, status, and validity
interval. Public key replacement requires a new signer identity. The exact
`revocations` entries contain `signer_id`, `effective_at`, and `reason`; issuer
and replacement metadata, where required by the governance ledger, must be
recorded outside the caller-supplied input. Unknown or stale revocation state
blocks current admission.

The provider registry must bind each approved provider ID to its signer,
public-key digest, Ed25519 algorithm, authenticated custody adapter, adapter
SHA-256, and the verifier executable/digest. The registry's trust-config digest
is a consistency check, not the independent root of trust: the deployment
authority must pin the registry and trust-config bytes separately under the
governance root. No registry entry may expose a private-key path, secret,
endpoint, arbitrary command, or credential.

All deployment artifacts listed above and every ancestor must satisfy the regular
non-symlink, ownership, and mode policy before use. Identity-v2 verification
must bind the exact raw-v1 bytes, context digest, pre-receipt plan-intent
digest, task, frozen HEAD, node/peer tuple, capture window, rotation, and
trust-config/provider-registry/verifier digests. A receipt with
`mode=historical_audit` is forensic only (`historical_only=true`,
`apply_authorized=false`), even when a retired key was valid at issuance;
destructive apply requires a fresh `current_admission` receipt with
`apply_authorized=true` and an independent current-authority recheck.

The sidecar bridge now invokes the four fixed commands and independently calls
the verifier seam, but deployment trust-config, registry, and verifier pin
values remain unset here. The planner requires `--identity-v2-evidence-map`; the
adapter accepts that exact map with `--identity-v2-mode current_admission` for
validation-only use, without `--apply`. The validator-pair executor still has
no v2 production admission input. Operators must not add guessed CLI flags or
pass identity-v2 material through the legacy identity-receipts input. The
governed bootstrap runbook documents the software interface as a
capability-gated prerequisite, not as evidence that production provisioning
has occurred.
