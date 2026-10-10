# Local file signing backend operator runbook

Status: source implementation under verification; host staging, installation and acceptance pending.
Owner: `blockchain_ops_engineer`. Authority: [release/installation design](local-file-signing-backend.design.md). Professional entrance: [P2P design](../design.md).

## Release preparation and approval

Build the three `oasis7_local_signer` Rust executables in release mode using the repository Cargo contract. `scripts/local-signer/package-release.py` packages existing binaries and reviewed installer modules into a new version directory. It records target, full source OID, schema versions, exact file sizes and SHA256 values. Review source/binary provenance and independently approve the exact manifest digest. A digest printed by the untrusted package is not approval by itself.

No source command in this task changes host accounts, ACLs, sudo, keys or signing. The operator must separately authorize root execution and approve the host plan. macOS is the first supported apply target; Linux apply is explicitly unsupported.

An approved administrator stages the approved bundle as data before root Python imports. The following is a reviewable future host action, not executed source verification. `APPROVED_BUNDLE` is the reviewed regular-file package location, `EXPECTED_MANIFEST_SHA256` comes from independent approval, and `RELEASE_ID` must match its manifest. Verify `/private/var/db` is protected and the new staging parent has no ACLs or symlinks. Refuse an existing release staging directory; do not overwrite it.

```sh
case "$RELEASE_ID" in ''|*[!A-Za-z0-9_-]*) exit 9 ;; esac
test "$(/usr/bin/shasum -a 256 "$APPROVED_BUNDLE/manifest.json" | /usr/bin/awk '{print $1}')" = "$EXPECTED_MANIFEST_SHA256" || exit 9
STAGING="/private/var/db/oasis7-local-signer-approved/$RELEASE_ID"
test ! -e "$STAGING" && test ! -L "$STAGING" || exit 9
/usr/bin/install -d -o root -g wheel -m 0755 /private/var/db/oasis7-local-signer-approved
/usr/bin/install -d -o root -g wheel -m 0755 "$STAGING"
for name in manifest.json oasis7_local_signer oasis7_local_signer_worker oasis7_local_signer_admin install-release.py installer.py macos_host.py; do
  /usr/bin/install -o root -g wheel -m 0444 "$APPROVED_BUNDLE/$name" "$STAGING/$name" || exit 9
done
test "$(/usr/bin/shasum -a 256 "$STAGING/manifest.json" | /usr/bin/awk '{print $1}')" = "$EXPECTED_MANIFEST_SHA256" || exit 9
```

These fixed native tools copy data only. They leave a partial staging directory untouched on interruption. Treat it as `STAGING_RECOVERY_REQUIRED`: do not overwrite, delete, or execute it. An operator must separately review the exact partial inventory, quarantine it through an explicitly authorized action, and create a fresh staging directory from the approved bundle. There is no automatic cleanup, retry, rollback, or resume.

Before root startup, obtain the native runtime gate and stdlib-only trusted bootstrap as separate approved regular-file artifacts, both staged outside the candidate directory under root-protected, ACL-free ancestry. The gate is not a member of the candidate manifest. `APPROVED_RUNTIME_GATE` and `APPROVED_BOOTSTRAP_SOURCE` identify those artifacts; `EXPECTED_RUNTIME_GATE_SHA256` and `EXPECTED_BOOTSTRAP_SHA256` come from independent review of their exact bytes. Do not derive any expected digest from the candidate package, the gate, or the bootstrap in the same shell block. The approved gate is built separately with the pinned Rust runtime crate workflow and requires fresh independent digest approval whenever its code changes. Materialize the bootstrap literal without a trailing line terminator: shell command substitution removes trailing newlines, while the gate hashes the source file bytes directly, so both inputs must cover the same exact bytes. This source workflow does not authenticate a publisher. Staging does not create the signer account, sudo rule or custody store.

## Installation plan and apply

The approved release entrypoint accepts `plan` and `apply` as defined in [interfaces](local-file-signing-backend.design.md#6-接口与数据合同). Plan binds release digest, explicit installation/deployment IDs, caller/signer identities and paths. Example selected locations are private store `/Library/Application Support/oasis7-local-signer` and ordinary jobs `/private/var/db/oasis7-local-signer-jobs/${CALLER_USER}/oasis7-local-signer`; exact absolute paths belong in the approved host plan. They are agreed locations, not evidence that an installation exists.

Plan performs readonly checks and writes only the requested new plan artifact. If readonly host evidence is inaccessible, report BLOCKED; a host administrator can run an approved readonly plan. Review exact actions and canonical plan bytes, then independently approve its SHA256. Apply requires root and both expected digests, checks the live plan again, journals effects and publishes sudo last.

A caller with no existing sudo grants is a valid initial state. The root plan distinguishes native sudo's explicit denial for the exact caller and local host from authentication or policy-query failures, and binds the exit code and both output streams into the observation digest. Post-installation checks still require the exact worker rule. Do not grant the caller administrator access to make preflight pass.

```sh
ACTUAL_RUNTIME_GATE_SHA256="$(/usr/bin/shasum -a 256 "$APPROVED_RUNTIME_GATE" | /usr/bin/awk '{print $1}')"
test "$ACTUAL_RUNTIME_GATE_SHA256" = "$EXPECTED_RUNTIME_GATE_SHA256" || exit 9
APPROVED_BOOTSTRAP_CODE="$(/bin/cat "$APPROVED_BOOTSTRAP_SOURCE")"
ACTUAL_BOOTSTRAP_SHA256="$(/usr/bin/printf '%s' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk '{print $1}')"
test "$ACTUAL_BOOTSTRAP_SHA256" = "$EXPECTED_BOOTSTRAP_SHA256" || exit 9
/usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C "$APPROVED_RUNTIME_GATE" \
  --bootstrap-source "$APPROVED_BOOTSTRAP_SOURCE" --expected-bootstrap-sha256 "$EXPECTED_BOOTSTRAP_SHA256" -- plan \
  --release-dir "$STAGING" --expected-manifest-sha256 "$EXPECTED_MANIFEST_SHA256" \
  --store-dir '/Library/Application Support/oasis7-local-signer' \
  --work-dir "$APPROVED_WORK_DIR" --caller-user "$APPROVED_CALLER" \
  --signer-user _oasis7_signer --installation-id "$APPROVED_INSTALLATION_ID" \
  --deployment-id "$APPROVED_DEPLOYMENT_ID" --plan-out "$NEW_PLAN_FILE"
/usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C "$APPROVED_RUNTIME_GATE" \
  --bootstrap-source "$APPROVED_BOOTSTRAP_SOURCE" --expected-bootstrap-sha256 "$EXPECTED_BOOTSTRAP_SHA256" -- apply \
  --release-dir "$STAGING" --expected-manifest-sha256 "$EXPECTED_MANIFEST_SHA256" \
  --plan "$NEW_PLAN_FILE" --expected-plan-sha256 "$INDEPENDENTLY_APPROVED_PLAN_SHA256"
```

The shell verifies both externally approved digests before the gate launch. `/usr/bin/env -i` clears inherited environment variables before the gate is loaded. The native gate checks its fixed direct Command Line Tools Python identity and bootstrap source digest, then starts Python with `-I -S -B -c`; Python rejects missing or malformed fixed-FD runtime attestation before touching the candidate stage. The bootstrap reads the release only through anchored nofollow directory and member descriptors. On each retained descriptor, `fgetattrlist` must prove that the volume's extended-security capability is valid and enabled. The returned-attribute bitmap must show either an absent ACL with the exact zero-filled reference framing, or a filesec with the exact `KAUTH_FILESEC_NOACL` sentinel and bounds. Zero-entry and populated ACLs, unsupported volumes, query errors, and malformed or truncated responses reject. Descriptor metadata is checked before and after capture. The bootstrap verifies the independently approved manifest and exact closed package inventory, captures the manifest, launcher, modules and binaries once, and executes only the in-memory snapshot. `install-release.py` receives the immutable captured-byte capsule; installer imports, release validation and installation copy consume those exact bytes without reopening candidate staging paths. Direct Python execution of `$STAGING/install-release.py` is unsupported and blocks because it has no native gate attestation. The supported root entry is the externally approved gate invocation above.

The approved work parent must already exist and belong to the caller; the installer creates only the selected leaf and does not create or change HOME/Documents/keys ancestors. Unknown sudo-policy output, any effective ACL entry, a mounted unsupported/network filesystem, or an existing foreign installation blocks this first delivery. Keep the protected approved staging bundle for exact repeated verification.

Caller-side custody identity checks use a search-only directory descriptor on macOS (`O_SEARCH`), with nofollow traversal, protected owner/mode and descriptor ACL validation. They compare device/inode without requiring directory listing permission; the custody root remains 0711. Content operations traverse ancestors with search-only descriptors and retain their separate readable-final-directory contract.

Successful installation reports INSTALLED_UNREADY with signing_enabled false. It creates no key, policy or grant. Existing doctor reports AUTHORIZATION_DENIED with ready false for missing policy, the intended initial unready state. Installation does not admit any blockchain/governance signer. Key creation, policy, grant and enabling require separate content-specific authorization through existing root admin commands.

## acceptance-checklist

For this host, use the approved external jobs layout: `/private/var/db/oasis7-local-signer-jobs` root:wheel 0711, an exact caller-owned 0700 child, and the installer-created caller-owned 0700 leaf. Resolve and approve the actual caller UID/GID before provisioning the parent. This is a proposed installation location, not evidence that provisioning has occurred. Keep HOME/Documents and its ACL unchanged.

Filesystem admission examines each installation path and existing ancestor through retained nofollow descriptors. Only local, ownership-enforcing APFS/HFS is admitted; a target on autofs, a remote/unknown filesystem or `noowners` is blocked. Unrelated mounts such as default `auto_home` do not block installation. Filesystem identity is bound in the plan and rechecked before apply; this does not authorize host mutation or expand the documented trusted-root race boundary.

| Check | Isolated source fixture | Required separate macOS host evidence |
| --- | --- | --- |
| Trust | Changed manifest/binary/plan, wrong target, unknown members reject before effects | Independently approved release provenance and exact digests |
| Protected paths | Symlink/hardlink/parent substitution, owner/mode/ACL conflicts reject | Effective ACLs/groups/mount and root ancestry of config/code/store |
| Caller jobs | External v3 path and device/inode, nofollow retained FD | Actual caller jobs access without seed/control access; no HOME/Documents permission change |
| Identity | Caller/signer collision and account drift reject | Dedicated disabled-login account and exact UID/GID |
| Sudo | Exact numeric RunAs UID:GID, empty argv, NOSETENV, admin excluded | Effective policy and actual caller/Codex negatives for extra argv, -E, shell/admin/arbitrary root/signer commands |
| State | No keys/policy/grants; disabled/unready; exact repeat unchanged | No broad privilege or inherited write ACL invalidating boundary |
| Failure | Fault injection at durable stages, accurate RECOVERY_REQUIRED | Approved interruption/recovery exercise without data loss |

Run source tests only in temporary fixture roots with importer-injected fake backend. Tests must not select the real host backend through environment or CLI. Fixed absolute production command-generation tests verify real backend operations without executing them. Fixture success proves source behavior and does not pass the macOS column.

On macOS, run fixture tests with `TMPDIR=/private/tmp python3 -m unittest discover -s scripts/local-signer/tests -v`; `/var` is an OS symlink alias and strict release path validation rejects it. This selects a canonical temporary test path and does not relax production nofollow validation.

## Failure and recovery

BLOCKED with host_mutated false means apply did not begin effects. RECOVERY_REQUIRED means a durable journal identifies retained partial effects and remaining actions. Preserve account/directory/release/config artifacts; do not delete private data, regenerate installation identity, overwrite policy or widen sudo to resume. Return the exact approved plan/release digest and nonsecret report to Ops/Runtime/QA for an explicitly approved recovery action.

An exact complete repeat revalidates live bytes, ownership/ACL, config/store/jobs identity and sudo, then returns VERIFIED_UNCHANGED. Unknown existing state, v2 binding, changed release identity, enabled policy or private data blocks first installation. Upgrades, migrations, rollback, backup/restore, Linux host installation and M1 remain deferred.

### Installer lock and ACL observations

The installer uses the fixed root-owned `/private/var/db/oasis7-local-signer-install.lock`; macOS `/private/var/run` can be group writable and is not an admitted protected parent. Plan checks the lock parent; apply opens a regular, single-link root:wheel0600 lock with nofollow/nonblocking flags and verifies ACL admission before locking. Preserve a retained lock file; it is preparation state, not an installation journal.

ACL observation hashes bind the admitted absence of ACLs, rather than the entire `ls -lde` display (timestamps, directory sizes, link counts and extended-attribute markers). Descriptor inode/device, owner/group/mode and filesystem identity remain separately bound and rechecked. ACL entries remain rejected. Releases with the revised observation encoding require a fresh plan and exact approval; do not reuse older plans.

### Terminal completion recovery

A separately reviewed recovery package may use the trusted gate's `apply --completion-check` entry with an explicit original installed release/Manifest and original root-owned plan/digest. This observes under the existing protected lock and performs no journal write. The returned journal preimage digest is independently approved before `apply --completion-only --expected-journal-sha256 ...`. The recovery package Manifest authenticates recovery code separately; it must not replace the original installed release binding.

Only canonical stage `recovery` with exactly identity/layout/release/binding/validated/sudo completed is eligible. Original IDs, digests, runtime, installation config and fresh installed bytes/layout/inodes/empty authority state/sudo policy must match. Completion writes only the bound complete journal through the atomic durable writer; it does not replay earlier actions or enable signing. A failed write reports RECOVERY_REQUIRED and attempts to retain the original recovery journal. Preserve all artifacts and do not retry automatically.

Effective sudo observation uses bounded `sudo -ll` structured entries, exact numeric signer UID/GID, `!setenv`/`!authenticate`, fixed worker and empty argv. Only the explicitly evidenced macOS default environment names and lecture/log options are admitted; unknown defaults, loader/search/interpreter/identity variables, extra entries or privilege options reject. The worker does not select code, custody/config paths or external commands from those presentation/session variables. Actual caller negative tests remain required for host acceptance.


## Custody search permission binding repair

The dedicated binding repair (`oasis7.local_signer_binding_repair_plan.v1`) fixes the caller/worker directory-identity read-permission defect only. It is not an initial install, general upgrade or permission migration. The administrator must independently approve the new release Manifest and freshly generated protected repair plan before apply. Plan generation uses the approved gate/bootstrap and explicit `apply --binding-repair-plan-out`; no initial-install action is executed. It binds the original completed journal and plan, current config/include/effective sudo digests, attested runtime, unchanged installation/deployment IDs, accounts, store/jobs device/inode, empty authority state and a fresh release ID. Its only config differences are release ID, worker path and worker digest.

Apply uses explicit `--binding-repair-apply` with the exact repair plan SHA256. Under the installer lock it reobserves the original binding, writes a separate root-owned 0600 `/private/var/db/oasis7-local-signer-binding-repair.json` intent, disables the worker include and verifies effective denial before creating the new version directory. It preserves the old release, original plan and original installation journal. Every completed action has durable progress evidence. It replaces the config, checks layout and bytes, publishes the new exact numeric UID:GID/empty-argv/NOSETENV worker rule, and requires the actual bound caller's installed CLI doctor to return `AUTHORIZATION_DENIED`, `ready:false`, exit 3. Only durable completion reports `BOUND_REPAIRED_UNREADY`; no keys, policies, grants or signing are enabled.

A caught failure attempts to disable worker authority and reports `RECOVERY_REQUIRED`; failed revocation remains an unresolved host state. A process interruption can bypass this handler: durable progress is evidence, not a promise that a published worker rule was automatically revoked. Do not retry or roll back automatically. Preserve all effects. An independently approved, receipt-preimage-bound `--binding-repair-quarantine` only revalidates known old/new config and sudo states, revokes worker authority, verifies denial and writes a separate quarantined receipt. It never runs release code or replays an installation/repair action. `QUARANTINED_UNREADY` is containment, not repaired readiness; further repair needs a separately reviewed contract.

Once any repair receipt exists, the current installer rejects historical initial-install repeat/completion routes. Current repaired validation must use `--binding-repair-check`, with the approved repair plan/release, and recheck the completed repair receipt, unchanged original journal, current installed bytes/config/layout/sudo and actual caller doctor. It reports `REPAIRED_VERIFIED_UNREADY` without installed-state writes. Historical original complete evidence does not authenticate the new binding.


### Failed binding repair: execute-only system sudo recovery

A repair that reached exactly `intent, sudo_disabled, release, binding, validated, sudo` but failed the actual caller doctor may be recovered with the separately approved `execute-only-system-sudo-open` flow. Ordinary install/apply and the failed repair apply must not be rerun. This flow accepts only the protected failed receipt at `stage=recovery`, the exact failed repair plan/release, unchanged original installation journal, exact failed configuration, empty authority directories and a disabled own sudo include with no effective caller grants. It does not adopt arbitrary interrupted stages.

Use the trusted native gate and a newly staged fixed release. In addition to the original protected installation plan and release digest inputs, supply `--failed-repair-plan`, `--expected-failed-repair-plan-sha256`, `--failed-release-dir` and `--expected-failed-manifest-sha256`. Read-only `--binding-repair-recovery-plan-out` creates a new protected recovery plan. Independently review that plan and obtain exact human approval before `--binding-repair-recovery-apply` with `--expected-recovery-plan-sha256`.

The flow preserves `/private/var/db/oasis7-local-signer-install.json` and `/private/var/db/oasis7-local-signer-binding-repair.json`. Its separate progress receipt is `/private/var/db/oasis7-local-signer-binding-recovery.json`. It revokes before publishing a fresh release and replacing only release/worker bindings, validates the disabled state, publishes the narrow worker sudo rule, validates again, and requires the actual caller doctor to prove `AUTHORIZATION_DENIED` with `ready:false`. Each step is durably recorded. Any failure attempts revocation and reports `RECOVERY_REQUIRED`; never retry automatically or edit receipts. Successful `BOUND_RECOVERED_UNREADY` keeps signing disabled. Explicit `--binding-repair-recovery-check` consumes the current completion receipt read-only and reports `RECOVERED_VERIFIED_UNREADY`; historical install verification is not current repaired-release evidence.
