# Local Ed25519 key management manual

Scope: the source extension in `oasis7_local_signer`; installation requires a separately reviewed release. The existing installed release does not gain these commands through a source checkout. Release operations follow the [operator runbook](local-file-signing-backend.runbook.md).

## Roles and supported capabilities

The administrator manages custody through `oasis7_local_signer_admin`. Ordinary callers submit approved signing jobs through `oasis7_local_signer`; they cannot create keys, change policy, issue grants or export private material.

Keys use Ed25519. Each key has an immutable signer ID, purpose and public key, plus a name, description and tags. Supported purposes are `rollback_strict_audit_v1` and `file_ed25519_v1`. Names and tags are descriptive: authorization uses the signer ID, exact context, payload digest, current policy and restricted grant.

| Operation | Command | Result |
| --- | --- | --- |
| Inventory | `key-list`, `key-show --signer-id ID` | Public metadata only |
| Create | `key-new --signer-id ID --purpose PURPOSE [--name NAME] [--exportable]` | Active key; export is disabled by default |
| Describe | `key-update --signer-id ID --name NAME --description TEXT --tags comma,separated` | Metadata update |
| Disable / archive | `key-state --signer-id ID --state inactive` or `archived` | Signing and replay authorization reject the key |
| Reactivate | `key-state --signer-id ID --state active` | Requires intact validated key material and separate signing authority |
| Rotate | `key-rotate --signer-id OLD --new-signer-id NEW` | Old key becomes inactive; new key records its lineage |
| Delete | `key-state --signer-id ID --state deleted --confirm-delete` | Requires inactive/archived state; removes seed, retains public tombstone |
| History | `key-audit`, `signing-audit` | Key events and public signing summaries |
| Authority inspection | `control-show`, `grant-list`, `grant-show --grant-id ID` | Current policy and grant inventory |

Rotation does not rewrite policy or grants. Review and publish replacement authority through the existing administrator policy/grant commands before callers use the new key. Deleted IDs and public-key tombstones cannot be reused. Existing legacy keys appear with conservative rollback-only, nonexportable metadata; update metadata to register them in the catalog.

## Encrypted import and export

`key-export --signer-id ID --output ABSOLUTE --passphrase-stdin` exports only keys created with `--exportable`. There is no command to turn an existing nonexportable key into an exportable one. `key-import --signer-id NEW --input ABSOLUTE --expected-sha256 SHA256 --passphrase-stdin [--exportable]` imports an encrypted envelope as an **inactive** key. Verify its public fingerprint and purpose before activation. Duplicate public keys and existing IDs are rejected.

Private exports use Argon2id and XChaCha20-Poly1305 authenticated encryption with random salt and nonce. Passwords must be 12–1024 bytes and arrive as a single newline-terminated line on non-terminal standard input. Use a password manager or protected secret source that supplies stdin. Do not put passwords in arguments, environment variables, shell history or this manual. The CLI rejects terminal stdin to avoid echoed input.

Encrypted input must be an absolute root-owned regular file with mode `0600`, a single link and the independently verified SHA256. Output paths must have protected root-owned ancestry; output is create-only `0600`. Preserve the envelope and its digest in the approved backup location. Losing the password prevents decryption.

## Backup and restore

Disable signing authority first: backup requires a policy with no enabled purposes. `backup-create --output ABSOLUTE --passphrase-stdin` creates an encrypted custody snapshot. If the snapshot contains nonexportable private keys, it requires the explicit `--include-nonexportable` option. This is a deliberate administrative disaster-recovery exception; nonexportable is an application rule, not hardware protection against the custody administrator.

Snapshots support up to 450,000 files and 1 GiB of raw custody data, including the signer’s supported 100,000 retained records. Encrypted backup input is bounded at 2 GiB; individual key envelopes remain bounded at 32 MiB. Backup and restore currently operate in memory, so large snapshots need several GiB of available RAM. A store exceeding the byte budget requires a separately designed archival workflow; records are never silently dropped.

Restore uses an empty, separately installed custody store with the same installation and deployment IDs:

1. Run `restore-plan --input ABSOLUTE --expected-sha256 SHA256 --output ABSOLUTE --passphrase-stdin`.
2. Review the plan's installation binding, encrypted input digest, tree digest and file count. Independently approve the plan digest.
3. Run `restore-apply --input ABSOLUTE --expected-sha256 SHA256 --plan ABSOLUTE --expected-plan-sha256 SHA256 --passphrase-stdin`.
4. Inspect keys, history and control inventory. Restored policy remains disabled and every restored grant is revoked. Issue fresh authority only after review.

Restore preserves consumed signing records; it does not reset their budgets or reactivate old grants. It does not overwrite a populated store, install executables or rewrite host configuration. An interrupted restore can resume only with the same encrypted input and approved plan; unrelated partial data blocks recovery.

## Generic file signatures

`file_ed25519_v1` signs a domain-separated canonical envelope binding the complete approved context, SHA256 and byte length of a file. It does not sign arbitrary raw bytes. The ordinary job path retains exact payload approval, caller binding, grant expiry and signing budget checks. A verifier must reconstruct the same envelope from the file and context and verify Ed25519 against the expected public key; a signature alone does not grant blockchain authority.

Prepare the payload as the ordinary caller:

```sh
oasis7_local_signer file-payload \
  --context /absolute/private-jobs/context.json \
  --expected-context-sha256 APPROVED_CONTEXT_SHA256 \
  --file /absolute/file-to-sign \
  --expected-file-sha256 APPROVED_FILE_SHA256 \
  --output /absolute/private-jobs/payload.bin
```

The output directory must be caller-owned `0700`; output is create-only `0600`. Review the emitted payload digest and use it in the restricted grant and ordinary job approval. This command only prepares public signing bytes and never invokes sudo or signs. The context retains the existing protocol-context field names; `rollback_ticket` is an opaque context identifier for this purpose, not a rollback action.

## Failure handling

`RECOVERY_REQUIRED` means an operation may have published durable intermediate state. Preserve the store and inspect the maintenance record; do not delete journals or recreate a key ID. `management-recover --expected-operation-key SHA256 --expected-target-sha256 SHA256` only completes an exactly bound, already published management target after validation. It cannot invent a missing target or replace restore's plan-bound recovery.

Deletion and rotation can stop after disabling a key. Check inventory and history before taking the next explicit action. Source tests use isolated fixtures; no test result implies that the installed host has been upgraded, contains keys or has signing enabled.
