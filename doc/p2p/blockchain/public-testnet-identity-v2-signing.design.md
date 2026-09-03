# Public-testnet identity-receipt v2 signing design

Status: **PROPOSED / NOT IMPLEMENTED**

- Task: GitHub Issue `3607`, UID `task_174f0a5a87394012b071171cc4a52372`
- Frozen design context: HEAD `3fc05dec53312617a1d7c795c10548c22784b4f8`
- Scope: define the dedicated identity-v2 signing, assembly, verification, and
  trust-config contract; this document does not create keys, call a provider,
  or change the sidecar/planner/adapter.
- Authority: [`public-testnet-governed-bootstrap.runbook.md`](public-testnet-governed-bootstrap.runbook.md)
  and [`public-testnet-governance-trust-root-provisioning.md`](public-testnet-governance-trust-root-provisioning.md).

## 1. Decision and non-goals

Use a dedicated identity-receipt-v2 domain with a detached Ed25519 signature
produced by an explicitly approved external-custody provider, followed by an
independent verifier. The provider and verifier consume the same immutable
canonical payload bytes. Private key material stays outside the repository,
sidecar, planner, adapter, logs, and ordinary CLI arguments.

This is a new authority/tooling contract, not a reuse of an existing domain
key. The runtime `identity-receipt` command remains a raw metadata producer;
the rebuild-proof signer/verifier, node-consensus/feedback signer, finality
signers, and rollback signer are not identity-v2 signers. The current fixture
keys in `scripts/fixtures/oasis7-governance-root.v1.json` remain fixture-only
and must never be selected as production custody.

No live provider is configured. Until the approved provider, trust configuration,
and independent verifier exist, the executable path is `capability_blocked`; a
callback, copied signature, or `verified: true` field cannot satisfy admission.

This proposal does not supersede the active runbook's existing-root requirement.
The preferred authority migration is a dedicated identity-v2 profile authorized
under that existing governance root, not an automatic new root or reuse of a
node-consensus key. If that root cannot authorize the profile, a separate human
approved governance-authority change is required before implementation can be
activated. Tool development alone grants no signer or trust-root authority.

## 2. Options considered

| Option | Result | Reason |
| --- | --- | --- |
| Reuse runtime feedback/rebuild-proof signer and verifier | Reject | Its Ed25519 key and payload domain are node-consensus/rebuild-proof scoped. It cannot attest identity-v2 context or the exact raw-v1 byte stream. |
| Make the sidecar call OpenSSL or an offline callback directly | Reject | A generic detached primitive does not establish signer custody, allowlist, rotation/revocation, or an independent verifier receipt. Test callbacks also cannot be deployment evidence. |
| Dedicated prepare/sign/assemble/verify tools with external custody | Recommend | Separates deterministic payload construction, key custody, envelope assembly, and independent admission verification. It gives the adapter an auditable raw-byte and context bridge without placing secrets in this repository. |

## 3. Trust and authority boundary

The proposed deployment-bound regular file is:

`/operator/truth/identity-v2-trust-config.json`

Its repository-owned schema fixture, if separately approved, should live at a
future `scripts/fixtures/oasis7-identity-v2-trust-config.v1.json`; no such
production configuration is created by this slice. The existing
`/operator/truth/governance-root.json` provisioning and regular-file rules
remain applicable to the root that authorizes this profile.

The trust config has exact fields:

```json
{
  "schema_version": "oasis7.identity_v2_trust_config.v1",
  "network_id": "<governed-network-id>",
  "trust_root_id": "<governed-root-id>",
  "verifier_id": "governed-receipt-verifier",
  "algorithm": "ed25519",
  "rotation_epoch": "<governed-rotation-epoch>",
  "allowlist": [
    {
      "signer_id": "<dedicated-identity-v2-signer-id>",
      "public_key_ref": "<operator-managed-public-key-reference>",
      "public_key_sha256": "<64-lower-hex>",
      "status": "active",
      "valid_from": "<RFC3339 UTC>",
      "valid_until": "<RFC3339 UTC>"
    }
  ],
  "revocations": []
}
```

`public_key_ref` points only to public material. The verifier checks the trust
config and every ancestor for regular-file/no-symlink policy, ownership, mode,
exact schema, duplicate entries, and the deployment-truth file digest. Root,
network, verifier, algorithm, and rotation epoch are not caller-selectable.

The current-admission verifier resolves the code-owned path above and checks its
exact file SHA-256 against an independently installed deployment-authority pin,
itself anchored by the approved governance root. The caller cannot supply or
change that pin through a plan, template, manifest, environment override, or
verification receipt. A differing `--trust-config` path is rejected; the option
is an assertion of the configured path, not authority selection. Missing pins
or unapproved profile authorization fail closed. Recording a digest after
verification is evidence, not an independent trust anchor.

An active signer is accepted only when its validity interval covers the signed
receipt's `issued_at` and the signer is not revoked at that capture window.
Rotation adds a new signer identity and records the old signer as retired or
revoked; it never silently replaces a public key under the same identity.
Historical verification may use a retired key only when the trust ledger proves
that it was active at the receipt's capture window. Revocation reason, issuer,
effective time, and replacement identity are auditable public metadata.

Verification has two disjoint modes: `historical_audit` and `current_admission`.
Historical acceptance emits `historical_only=true, apply_authorized=false` and
cannot be consumed by destructive apply. Current admission rechecks the trusted
current effective signer status and rejects retired or revoked keys even when
they were active at issuance. Unknown or stale revocation state is a blocker.
Mode, evaluation time, trust-config digest and authorization result are bound
into the verifier receipt; apply independently revalidates current authority.

The production signer is an external custody/HSM/KMS authority selected by
blockchain-ops/governance. It attests signer identity, algorithm, and SHA-256
of the exact payload bytes. The CLI accepts no private-key path, secret, seed,
token, or provider credential.

`provider-ref` is a non-secret provider ID from a governance-approved registry,
not an endpoint or command. A pinned registry maps it to the permitted signer,
public-key digest, algorithm, authenticated transport and custody adapter digest.
Credentials are supplied exclusively by the operator's external custody runtime;
no CLI argument can override endpoint, command, trust anchor or authentication.
The authenticated provider receipt has the exact fields `schema_version`,
`provider_id`, `request_id`, `signer_id`, `public_key_sha256`, `algorithm`,
`canonical_payload_sha256`, `signature_sha256`, `context_digest`,
`rotation_epoch`, `capture_window_id`, and `issued_at`, plus a detached
provider-authentication proof. The independently pinned provider identity must
authenticate this receipt; exact payload, signature and request bindings are
checked again by the independent verifier. A self-reported JSON claim is not
custody attestation. Concrete registry/transport/proof profiles remain subject
to the authority migration above; absence blocks production signing.

## 4. Canonical payload and context

The signed payload is a finalized JSON object with exactly these fields and no
others:

```json
{
  "domain_separator": "oasis7.identity_receipt.v2/signature/v1",
  "schema_version": "oasis7.identity_receipt.v2",
  "signer_id": "<allowlisted-id>",
  "verifier_id": "governed-receipt-verifier",
  "trust_root_id": "<governed-root-id>",
  "task_uid": "<task-uid>",
  "head_oid": "<frozen-head-oid>",
  "frozen_head_oid": "<frozen-head-oid>",
  "plan_digest": "<pre-receipt-plan-intent-sha256>",
  "context_digest": "<canonical-context-sha256>",
  "capture_window_id": "<window-id>",
  "rotation_epoch": "<rotation-epoch>",
  "issued_at": "<RFC3339 UTC>",
  "expires_at": "<RFC3339 UTC>",
  "node_id": "<governed-node-id>",
  "peer_id": "<runtime-peer-id>",
  "key_sha256": "<64-lower-hex>",
  "key_size_bytes": 1,
  "key_mode": "0600",
  "key_uid": 0,
  "key_gid": 0,
  "signed_payload_sha256": "<sha256-of-exact-raw-v1-bytes>"
}
```

`plan_digest` is the digest of a separate, pre-receipt
`oasis7.clean_room_plan_intent.v1` document containing governed task UID,
frozen HEAD, capture window, network/adapter action, managed node names/roles,
and canonical reset-surface identifiers. It excludes identity receipts,
signatures, verification outputs, and final-plan receipt digests. Its canonical
JSON (UTF-8, sorted keys, compact separators, finite values) is hashed once;
the final receipt-bearing plan is never fed back into `plan_digest`.

The exact context schema is `oasis7.identity_v2_context.v1`: fields are
`schema_version`, `network_id`, `task_uid`, `head_oid`, `capture_window_id`,
`capture_start`, `capture_end`, `rotation_epoch`, `issued_at`, `expires_at`.
All are non-empty strings; timestamps use UTC `YYYY-MM-DDTHH:MM:SSZ`, HEAD is
40 lowercase hexadecimal characters, and timestamp ordering is checked.
The exact plan-intent fields are `schema_version` (the value above),
`context_digest`, `adapter_action`, and `nodes`. Each node has only `node_name`,
`node_id`, `peer_id`, `role`, and `reset_surface_ids`; identities are strings,
surfaces are unique strings, nodes are sorted by unique `node_name`, and surface
IDs are sorted. Node set, roles, action and surfaces must equal independently
approved deployment truth, not values invented by a receipt.
Both documents reject duplicate/unknown fields and noncanonical serialization;
their digests use the same sorted-key compact JSON encoding as the payload.
`context_digest` binds the exact canonical context and is signed. Shared fields
must agree across context, intent, raw identity and payload. Neither input may
contain receipt, payload, signature, final-plan or verification-output fields.

The one-way dependency is: canonical context -> context digest -> plan intent
-> plan-intent digest -> payload -> signature/envelope -> verification receipt
-> final plan. Raw-v1 bytes independently feed the payload's raw digest. No
downstream digest is allowed in an upstream input, including final-plan hashes.

The payload bytes are exactly:

```text
ASCII("OASIS7-IDENTITY-RECEIPT-V2\0")
+ UTF-8(JSON object above, ensure_ascii=true, sort_keys=true,
        separators=(",", ":"))
```

The signer signs those bytes with Ed25519. `canonical_payload_sha256` in the
prepare manifest is SHA-256 of the complete byte sequence above. The existing
wire field `signed_payload_sha256` retains its runbook meaning: SHA-256 of the
exact raw `oasis7.identity_receipt.v1` bytes, including whitespace and the
host-specific `key_path`. The v2 envelope does not expose `key_path`; the raw
digest binds it without making it a direct admission field.

Signed fields are every field in the payload object, including signer,
verifier, root, task/head/plan, freshness, node/peer, key tuple, and raw-byte
digest. `signature_hex`, `canonical_digest`, `authenticated`, and `verified`
are not signed. `signature_hex` is the lowercase 128-hex Ed25519 result;
`canonical_digest` is SHA-256 of canonical envelope JSON omitting only
`canonical_digest`, `authenticated`, and `verified` (and includes the
signature). It is an integrity aid, not cryptographic verification.

The template supplies only governed signed fields. It must not supply a
signature, digest, or verification verdict. If a compatibility reader sees
those fields in an older template, it must reject non-empty/non-default values
and regenerate them; it may not copy them into the final receipt.

## 5. Strict validation rules

- Parse UTF-8 JSON with duplicate-key rejection; reject BOM, non-finite
  numbers, unknown/missing fields, wrong types, malformed IDs, and
  non-canonical casing/serialization. Never normalize after payload selection.
- Require `head_oid == frozen_head_oid`, exact task UID, pre-receipt plan
  digest, capture window, rotation epoch, and trust-config signer/verifier/root.
- Require `issued_at < expires_at`, explicit UTC timestamps inside capture
  bounds, and an unexpired interval with configured clock-skew allowance.
- Require exact raw-v1 schema/fields and non-empty regular non-symlink bytes;
  hash before parse/projection. Match node/peer and the complete key tuple
  across raw v1, template, payload, and deployment inventory. Never reconstruct
  or open `key_path` during admission.
- Require the managed node set exactly once; reject duplicate nodes/peers or
  raw-map entries and cross-paired node/template/context files.
- Check provider payload digest, signer, algorithm, and signature shape, then
  independently verify Ed25519 over exact bytes. Recompute every digest and
  binding before setting either verdict true; never trust copied flags/signature.

## 6. Proposed executable contract (not present today)

The future dedicated executable is provisionally named
`scripts/p2p-public-testnet-identity-v2-signing-tool.py`. The path is a design
target, not an existing command. It has four subcommands and uses only regular
files plus an operator-approved provider bridge:

```bash
<identity-v2-tool> prepare \
  --raw-v1 <raw-v1.bin> --template <unsigned-template.json> \
  --context <context.json> --plan-intent <plan-intent.json> \
  --trust-config /operator/truth/identity-v2-trust-config.json \
  --payload-out <payload.bin> --manifest-out <prepare.json>

<identity-v2-tool> sign \
  --payload <payload.bin> --manifest <prepare.json> \
  --provider-ref <approved-custody-ref> \
  --signature-out <signature.hex> --attestation-out <sign-attestation.json>

<identity-v2-tool> assemble \
  --payload <payload.bin> --manifest <prepare.json> --signature <signature.hex> \
  --attestation <sign-attestation.json> --out <unsigned-envelope.json>

<identity-v2-tool> verify \
  --mode current_admission \
  --envelope <unsigned-envelope.json> --raw-v1 <raw-v1.bin> \
  --context <context.json> --plan-intent <plan-intent.json> \
  --trust-config /operator/truth/identity-v2-trust-config.json \
  --out <verified-envelope.json> --verification-out <verification.json>
```

`prepare` is deterministic and emits exact payload bytes plus raw/canonical
digests and sizes, template/context digests, all task/head/plan/window/rotation
bindings, signer/verifier/root IDs, and algorithm. `sign` asks only the approved
provider; it has no private-key option and fails closed without a provider.
`assemble` accepts no field overrides, checks the attestation, attaches only
the signature, and sets verdicts false. `verify` ignores incoming verdicts,
reconstructs and byte-compares the payload, verifies trust and Ed25519, then
writes freshly computed verdicts and an independent receipt. Only unsigned
verdict/digest fields may change after `prepare`.

`assemble` opens the payload as a regular non-symlink file and requires its
exact bytes, size and SHA-256 to match the prepare manifest, canonical payload
reconstruction and authenticated provider receipt. It verifies the detached
signature against these same bytes before emitting an envelope. Any mismatch
aborts without an output; a manifest alone cannot stand in for the payload.

The verification receipt records raw-v1, canonical-payload, and envelope
digests, signer/public-key and trust-config digests, verifier executable digest,
capture context, and result. It contains no private provider output.

## 7. Context and raw-byte executable bridging

The single-node sidecar should eventually receive explicit `--context`,
`--plan-intent`, `--trust-config`, `--signer-tool`, and `--verifier-tool`
arguments and orchestrate the four commands in a private temporary directory.
It passes the original `--raw-v1` unchanged. Missing tool, provider, trust
config, or verifier receipt is a capability blocker.

The adapter should eventually accept one exact raw-byte manifest:

```json
{
  "schema_version": "oasis7.identity_receipt.v2_raw_map.v1",
  "entries": [
    {
      "node_name": "<managed-node-name>",
      "node_id": "<governed-node-id>",
      "peer_id": "<expected-peer-id>",
      "raw_v1_path": "<operator-captured-regular-file>",
      "sha256": "<64-lower-hex>",
      "size_bytes": 1
    }
  ]
}
```

The manifest contains the exact managed node set once. The adapter passes bytes
from this map to each prepare/verify subprocess and retains the same association
in its transaction input. It never derives bytes from `key_path`, synthesizes
JSON, or pairs a receipt by list position. The map path is transport-only; the
verifier hashes and validates the opened bytes first.

The current sidecar CLI and adapter CLI do not expose these executable seams.
Their current callback-only/plan-only behavior must remain fail-closed until
this contract is implemented and approved; adding another mock callback is not
an implementation of this bridge.

## 8. Test and evidence contract

Offline tests cover deterministic repeated preparation; duplicate/unknown JSON;
malformed types/timestamps; plan-intent/final-plan cycle attempts; changed raw
or payload bytes; changed template/context; signer/public-key mismatch; wrong
domain; cross-node/peer/window pairs; stale/future receipts; revoked/retired
keys; copied signatures; and copied verdict flags. An end-to-end test may use
an ephemeral in-memory key/provider to prove byte identity across all four
steps, but it must be excluded from deployed allowlists and readiness evidence.

Production evidence requires custody/provider attestation; trust-config,
public-key, signer/verifier executable, and exact raw-v1 digests; prepare/sign/
verify manifests; independent verification receipt; all context bindings; and
an audit showing no secret emission. The final plan may reference these
digests, but cannot alter the signed payload or pre-receipt plan-intent digest.

This design has no tick, replay, checkpoint, or recovery-state mutation. Its
determinism guarantee is limited to canonical byte production and exact
admission pairing. Runtime integration, custody ceremony, trust-root approval,
and real provider evidence remain residual risks requiring blockchain-ops,
runtime, QA, and TPM follow-up.
