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
/usr/bin/python3 -I -c 'import hashlib,json,pathlib,sys; p=pathlib.Path(sys.argv[1]); raw=(p/"manifest.json").read_bytes(); assert hashlib.sha256(raw).hexdigest()==sys.argv[2]; m=json.loads(raw); entry=next(x for x in m["files"] if x["name"]=="install-release.py"); code=(p/"install-release.py").read_bytes(); assert len(code)==entry["size_bytes"] and hashlib.sha256(code).hexdigest()==entry["sha256"]' "$STAGING" "$EXPECTED_MANIFEST_SHA256" || exit 9
```

These fixed native tools copy data only. They leave a partial staging directory untouched on interruption. Treat it as `STAGING_RECOVERY_REQUIRED`: do not overwrite, delete, or execute it. An operator must separately review the exact partial inventory, quarantine it through an explicitly authorized action, and create a fresh staging directory from the approved bundle. There is no automatic cleanup, retry, rollback, or resume.

Before root startup, obtain the stdlib-only trusted bootstrap as a separate approved regular-file artifact. Its bytes are the reviewed `TRUSTED_BOOTSTRAP` string literal from the release's `install-release.py`, materialized outside the candidate stage before privileged execution. `APPROVED_BOOTSTRAP_SOURCE` must be outside `$STAGING`; `EXPECTED_BOOTSTRAP_SHA256` must come from independent review of the exact bootstrap bytes. Do not derive either value from the staged package or compute the expected digest in the same shell block. Shell command substitution removes trailing newlines from the source; approve the exact captured value. These are additional human approval inputs, and this source workflow does not provide publisher authentication. Verify `/usr/bin/python3` is the approved host interpreter and staging ancestors are protected; actual macOS ACL, sudo, account, and host acceptance remains separate. Staging does not create the signer account, sudo rule or custody store.

## Installation plan and apply

The approved release entrypoint accepts `plan` and `apply` as defined in [interfaces](local-file-signing-backend.design.md#6-接口与数据合同). Plan binds release digest, explicit installation/deployment IDs, caller/signer identities and paths. Example selected locations are private store `/Library/Application Support/oasis7-local-signer` and ordinary jobs `${CALLER_HOME}/Documents/keys/oasis7-local-signer`; exact absolute paths belong in the approved host plan. They are agreed locations, not evidence that an installation exists.

Plan performs readonly checks and writes only the requested new plan artifact. If readonly host evidence is inaccessible, report BLOCKED; a host administrator can run an approved readonly plan. Review exact actions and canonical plan bytes, then independently approve its SHA256. Apply requires root and both expected digests, checks the live plan again, journals effects and publishes sudo last.

```sh
APPROVED_BOOTSTRAP_CODE="$(/bin/cat "$APPROVED_BOOTSTRAP_SOURCE")"
ACTUAL_BOOTSTRAP_SHA256="$(/usr/bin/printf '%s' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk '{print $1}')"
test "$ACTUAL_BOOTSTRAP_SHA256" = "$EXPECTED_BOOTSTRAP_SHA256" || exit 9
/usr/bin/python3 -I -c "$APPROVED_BOOTSTRAP_CODE" plan \
  --release-dir "$STAGING" --expected-manifest-sha256 "$EXPECTED_MANIFEST_SHA256" \
  --store-dir '/Library/Application Support/oasis7-local-signer' \
  --work-dir "$APPROVED_WORK_DIR" --caller-user "$APPROVED_CALLER" \
  --signer-user _oasis7_signer --installation-id "$APPROVED_INSTALLATION_ID" \
  --deployment-id "$APPROVED_DEPLOYMENT_ID" --plan-out "$NEW_PLAN_FILE"
/usr/bin/python3 -I -c "$APPROVED_BOOTSTRAP_CODE" apply \
  --release-dir "$STAGING" --expected-manifest-sha256 "$EXPECTED_MANIFEST_SHA256" \
  --plan "$NEW_PLAN_FILE" --expected-plan-sha256 "$INDEPENDENTLY_APPROVED_PLAN_SHA256"
```

The bootstrap reads the release only through anchored nofollow directory and member descriptors. On each retained descriptor, `fgetattrlist` must prove that the volume's extended-security capability is valid and enabled. The returned-attribute bitmap must show either an absent ACL with the exact zero-filled reference framing, or a filesec with the exact `KAUTH_FILESEC_NOACL` sentinel and bounds. Zero-entry and populated ACLs, unsupported volumes, query errors, and malformed or truncated responses reject. Descriptor metadata is checked before and after capture. The bootstrap verifies the independently approved manifest and exact closed package inventory, captures the manifest, launcher, modules and binaries once, and executes only the in-memory snapshot. `install-release.py` receives the immutable captured-byte capsule; installer imports, release validation and installation copy consume those exact bytes without reopening candidate staging paths. Direct root execution of `$STAGING/install-release.py` is unsupported and returns `TRUSTED_BOOTSTRAP_REQUIRED`; it cannot protect the interpreter's initial script open. The supported root entry is the independently approved inline `-c` invocation above.

The approved work parent must already exist and belong to the caller; the installer creates only the selected leaf and does not create or change HOME/Documents/keys ancestors. Unknown sudo-policy output, any effective ACL entry, a mounted unsupported/network filesystem, or an existing foreign installation blocks this first delivery. Keep the protected approved staging bundle for exact repeated verification.

Successful installation reports INSTALLED_UNREADY with signing_enabled false. It creates no key, policy or grant. Existing doctor reports AUTHORIZATION_DENIED with ready false for missing policy, the intended initial unready state. Installation does not admit any blockchain/governance signer. Key creation, policy, grant and enabling require separate content-specific authorization through existing root admin commands.

## acceptance-checklist

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
