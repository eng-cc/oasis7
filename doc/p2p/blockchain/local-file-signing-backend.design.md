# Local file signing backend release and installation design

Status: source implementation under verification; release installer not deployed or accepted on a host.
Owner: `blockchain_ops_engineer`; interface/security: `runtime_engineer`; independent verification: `qa_engineer`.
Professional domain: [P2P design](../design.md); operator companion: [runbook](local-file-signing-backend.runbook.md).
Source baseline: `80efde80d0751e146233259e44c275d4120109a9`; source delivery evidence: [Issue 4200 dispatch](https://github.com/eng-cc/oasis7/issues/4200#issuecomment-5923478626).

## 1. 问题、目标与非目标

Merged M0 provides the three Rust executables and root-only post-install management. This source delivery implements a versioned release package and real macOS installation backend with explicit plan/apply, protected custody, external caller jobs and no initial signing authority. Source tests do not establish a host installation or permission acceptance.

This professional contract does not change product promises, blockchain authority admission, signing payloads, policy/grant semantics, key creation, backups/restore, identity bridge M1 or workflow policy. Original Downloads attachments are design inputs: V2 design SHA256 `68d73f88474d9ba0ea89020266d7063c729eaa3f050d913f495b01a971d8bdcd`; V2 runbook SHA256 `512908ebab4fc4ceedf6aa7e847a664c627fed16de3aa0bbf532e181c2cd3708`. Their proposed commands are not deployment evidence or permission grants.

## 2. 上游约束与相关角色

Ops owns host/release operations; Runtime owns installation v3, retained descriptors and sudo invocation; QA owns fixture/host acceptance distinction. Product AC is not applicable because this is bounded professional release/provisioning work with no new player promise. Task identity and execution scope remain in GitHub task truth.

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [professional_acceptance: authority isolation](../design.md#des-p2p-layer-authority) | Local custody must not admit blockchain/governance authority or let caller self-grant; install stays unready | [des-local-signer-install](#des-local-signer-install) | Ops / Runtime / QA; existing authority consumers | Host account, ACL and effective sudo acceptance requires separate approval and evidence |

## 3. 当前状态、目标状态与差距

| Capability | Current baseline | Delivery target | Evidence boundary |
| --- | --- | --- | --- |
| M0 signing/control | Rust crate implemented; installation CLI gated | Preserve protocol and root-only admin | Package tests demonstrate source behavior |
| Caller jobs | Installation v2 derives store/work | Explicit v3 external root with device/inode identity | No automatic installed-host migration |
| Release/installer | Absent | Immutable bundle, plan/apply and real macOS backend | No account or sudo change in this delivery task |
| Linux provisioning | Unverified | Explicit unsupported apply before mutation | Linux compile/fixtures do not claim host support |

## 4. 边界与结构

### des-local-signer-install

`package-release.py` produces bytes; `install-release.py` contains the fixed CLI and an independently approvable, stdlib-only inline bootstrap; `installer.py` validates the admitted release and `macos_host.py` provides real platform effects. Root plan/apply starts only through fixed `/usr/bin/python3 -I -c` with bootstrap source and digest supplied independently of the candidate stage. The bootstrap anchors the release path through retained nofollow directory descriptors and uses `fgetattrlist` on each same descriptor to require a valid extended-security-capable volume. It accepts only the exact returned-attribute omission with its zero reference or a bounded `kauth_filesec` carrying `KAUTH_FILESEC_NOACL`; zero-entry, populated, malformed, unsupported and failed ACL observations reject. Descriptor metadata is checked before and after capture, and the closed manifest, launcher, modules and binaries are captured once. It verifies every captured byte before executing the in-memory launcher and passes an immutable capsule through module loading, validation and installation copy. The candidate stage path is not reopened after admission; direct root execution of its `install-release.py` is unsupported because Python would open that candidate before its own guard could run. Native ACL or metadata uncertainty fails closed. Runtime Rust owns fixed binding load and caller job resolution. Fixture backends are importer-only injected objects, unavailable through production CLI/environment. Worker remains a fixed Rust executable, never a Python interpreter run as signer.

Approved release installer may execute as root only through a separately authorized host action. Caller cannot modify installed binding/code/control or access seeds; signer cannot alter control. External caller job content is ordinary untrusted input whose bytes are validated by the existing protocol and approved digest. Caller-owned ancestors of Documents remain caller-owned and provide no custody authority.

## 5. 关键运行流程

1. Package already-built release binaries plus installer modules; emit manifest digest for independent approval. Never execute build products as root during packaging.
2. Plan runs through the independently approved inline bootstrap and validates the exact captured manifest/member bytes against independently supplied expected digests, platform, host facts, paths, account allocation, effective ACL/sudo and conflicts. It produces canonical plan JSON without mutations. Import, release validation and file publication consume the same immutable capture; none reopen candidate-stage files by path.
3. Apply requires root, the independently approved bootstrap and release digests, plus the expected plan digest. The bootstrap repeats anchored same-descriptor ACL/metadata checks and complete capture before running installer code. For a fresh installation recompute live plan and refuse changed observations before effects. For a complete repeat, verify the supplied original plan digest against the protected completed journal/receipt and perform full installed readback instead of comparing installed objects with their pre-install observations. Partial journals return RECOVERY_REQUIRED. All subprocesses use fixed absolute executable paths, argument arrays, bounded output, timeout and clean environment; no shell or caller-selected executable.
4. Acquire protected installer lock; durably journal intent; allocate dedicated account/group; create protected directories; stage verified release bytes; publish root binding with observed store/jobs identities; verify keyless/grantless/policyless layout; publish narrow sudo include last; verify complete installation and publish report.
5. Success reports installed-but-unready. Exact repeat verifies every owned artifact and returns VERIFIED_UNCHANGED. Drift/conflict refuses repair. Interrupted apply leaves a durable RECOVERY_REQUIRED record; no automatic deletion, overwrite, rollback, key generation or activation.

## 6. 接口与数据合同

JSON is bounded UTF-8 with duplicate/unknown field rejection, canonical output `json.dumps(sort_keys=True,separators=(",", ":"),ensure_ascii=False)` plus one newline. Expected hashes cover exact file bytes, including newline. IDs use safe ASCII identifier grammar; SHA256 is lowercase 64 hex; source OID is lowercase 40 hex. Manifest maximum 64 KiB; each binary maximum 128 MiB; installer source files maximum 1 MiB; plan/report maximum 1 MiB.

Release `oasis7.local_signer_release.v1` exact fields: `schema_version`, `release_id`, `target`, `source_revision`, `installation_schema_version`, `control_schema_version`, `files`. `target` is `aarch64-apple-darwin` or `x86_64-apple-darwin` for apply. `files` is a sorted array of exact `{name,size_bytes,sha256}` entries: `oasis7_local_signer`, `oasis7_local_signer_worker`, `oasis7_local_signer_admin`, `install-release.py`, `installer.py`, `macos_host.py`. No optional arbitrary executable, subpath or extra bundle member. Manifest is not included in its own file digest array. Python runtime remains an explicit host dependency verified by absolute interpreter path and protected ancestry.

Plan `oasis7.local_signer_install_plan.v1` exact fields: `schema_version`, `release_id`, `manifest_sha256`, `platform`, `installation_id`, `deployment_id`, `store_dir`, `caller`, `signer`, `observations`, `actions`, `signing_enabled`. Caller exact fields: `name`, `uid`, `work_dir`; signer: `name`, `uid`, `gid`. `observations` is a sorted array of `{subject,facts}` where facts are canonical JSON host observations (path identity/mode/ACL, mount, account/group records, sudo validation); `actions` is ordered `{kind,target}` pairs from a fixed action enumeration. No arbitrary command or shell field. Identity IDs are supplied explicitly to plan, stable across retries; `signing_enabled` is always false. Missing evidence yields BLOCKED, not an inferred valid plan. Plan can run unprivileged when all required readonly evidence is available; otherwise operator runs a readonly root plan.

Production CLI forms:

```text
package-release.py --binary-dir DIR --installer-dir DIR --output-dir NEWDIR --release-id ID --target TARGET --source-revision OID
install-release.py plan --release-dir DIR --expected-manifest-sha256 HEX --store-dir ABS --work-dir ABS --caller-user NAME --signer-user NAME --installation-id ID --deployment-id ID --plan-out NEWFILE
install-release.py apply --release-dir DIR --expected-manifest-sha256 HEX --plan FILE --expected-plan-sha256 HEX
```

Root installer inputs are approved data, never executable paths selected through plan. The initial interpreter command is an inline bootstrap string copied from a separately reviewed source artifact; its exact captured bytes are checked against an independently supplied bootstrap digest before invocation. Command substitution trims trailing newline bytes, so the approved digest must cover the exact captured string that is passed to `-c`. This approval does not authenticate a publisher. The bootstrap retains the anchored directory chain, verifies native ACL and metadata on the same file descriptors used to capture every closed release member, and checks descriptor/name stability before handing an immutable byte mapping to the launcher. The launcher loads its Python modules and copies binaries from that mapping without reopening the stage. Backend interface: `observe(request,release)->facts`, `lock()`, `journal(record)`, `create_identity(identity)`, `create_layout(layout)`, `publish_release(verified_bytes)`, `publish_binding(config)`, `validate_installation(config,release)`, `publish_sudo(rule)`, `report(record)`. Production methods perform real effects; tests replace this object only by import. Apply derives actions internally and never dispatches unchecked plan action names.

Installation `oasis7.local_signer_installation.v3` preserves M0 top-level binding fields; callers become exact `{uid,work_dir,work_device_id,work_inode}`. Device/inode values come from retained descriptors after creation. v2 is rejected; no fallback to store/work. Fixed sudo invocation includes numeric `-u UID -g GID`, exact worker path and no worker arguments. Runtime owns detailed parser/descriptors and fixtures.

Existing `oasis7_local_signer_admin install --dry-run/--apply` stubs remain explicit BLOCKED compatibility forms; they do not implement this installer or delegate to editable Python paths. Their result must direct operators to the approved release installer, without claiming host mutation. The new real management entrypoint is the packaged `install-release.py`; post-install admin operations retain their Rust root-only boundary. Any command-help correction is a narrowly required Runtime CLI adapter edit, subject to the frozen implementation scope.

Reports use `oasis7.local_signer_install_report.v1` exact fields `schema_version,status,code,installation_id,deployment_id,release_id,manifest_sha256,plan_sha256,host_mutated,signing_enabled,completed_actions,remaining_actions`; action arrays use fixed names. Status is PLANNED, INSTALLED_UNREADY, VERIFIED_UNCHANGED, BLOCKED or RECOVERY_REQUIRED. No seed, credential or secret fields. BLOCKED before apply has host_mutated false; partial failure reports completed actions and retains journal.

## 7. 状态、事务与持久化

The protected journal progresses intent → identity → layout → release → binding → validated → sudo → complete, fsyncing data and parent directories at each boundary. Account/directory creation cannot be atomically rolled back as one transaction; failure explicitly exposes partial effects. Lock prevents concurrent applies. The complete journal/receipt binds the original manifest and plan digests, installation/deployment identity and resulting protected filesystem/account identities. Complete-repeat requires the same original approved plan and release plus full readback of code/config/store/jobs/account/ACL/sudo metadata and initial unready state; installed observations are expected to differ from pre-install absence, so this branch does not regenerate a fresh pre-install plan. Receipt alone is insufficient. Partial journal, missing receipt, foreign state or any readback drift returns RECOVERY_REQUIRED/BLOCKED without repair. Existing enabled policy, keys, grants or unknown installation block first-install apply.

Source files use nofollow regular-file descriptors with nlink=1; hash and copy the same bounded bytes. Destination operations use trusted retained parent descriptors, create-only writes and atomic publication. Recheck identity/ownership/ACL at use; replacing a parent or approved binary fails closed. Package output is create-only; release identities cannot overwrite prior versions. Jobs leaf is caller-owned 0700 and bound by device/inode; Runtime retains its descriptor while accessing jobs.

## 8. 部署、安全与运行约束

Store `/Library/Application Support/oasis7-local-signer` root:wheel (UID0/GID0) 0711; keys/state signer:signer 0700; state/records signer:signer 0700; control/grants/revoked-grants root:signer 0750. Existing Runtime layout also requires empty work root:wheel 0711 and empty backup-staging signer:signer 0700; these compatibility directories provide no backup capability or caller job mapping. No policy file, key file or grant exists initially. Doctor reports AUTHORIZATION_DENIED with ready false for absent policy; installer separately reports INSTALLED_UNREADY. Fixed config `/private/etc/oasis7/local-signer-installation.json` root-owned readable 0644. Code `/usr/local/libexec/oasis7-local-signer/<release-id>` root-owned 0755 and binaries 0555. Protected ancestors must be root-owned, nonwritable by caller/signer, symlink-free and free of ACL write grants. Reject unsupported/network filesystems and unknown ACL evidence; never remove unrelated ACLs. The implementation conservatively rejects any effective ACL entry and unsupported mounted filesystem anywhere in the mount inventory rather than interpreting permissive exceptions.

Selected external jobs `${CALLER_HOME}/Documents/keys/oasis7-local-signer` is caller-owned 0700; the exact user-selected absolute path is bound in Issue 4200 and the approved plan. Installer does not change HOME/Documents ancestry. No store/work symlink. Account `_oasis7_signer` must be dedicated, noninteractive, password-disabled, with distinct UID/GID from caller. Real backend uses fixed `/usr/bin/dscl` and `/usr/bin/dscacheutil`, validates unused numeric allocation and exact resulting records, and refuses shared/existing human identities.

Sudo include `/private/etc/sudoers.d/oasis7-local-signer` root:wheel 0440 allows only exact numeric caller as numeric signer UID:GID, fixed worker, empty argument constraint, NOPASSWD and NOSETENV. Validate candidate/effective policy using fixed `/usr/sbin/visudo` and `/usr/bin/sudo`. Broad NOPASSWD root/signer permissions invalidate isolation; generated narrow snippet cannot revoke broader policy. Do not edit unrelated rules. Real host acceptance must test extra argv, -E/SETENV, admin/shell/arbitrary executable refusal in actual caller/Codex context.

## 9. 质量与容量

| Stimulus/environment | Response | Metric / verification |
| --- | --- | --- |
| Isolated fixture, altered manifest/binary/plan | BLOCKED before effects | Zero backend mutation log entries |
| Isolated fixture, account/path/ACL collision | BLOCKED | Existing bytes/identity preserved |
| Fault at every durable stage | RECOVERY_REQUIRED | No authority activated; accurate journal/action report |
| Exact repeat fixture | VERIFIED_UNCHANGED | Zero additional effects; all artifact hashes verified |
| macOS real account/sudo/ACL | Separate host acceptance | Runbook negatives; fixture pass cannot satisfy |

## 10. 兼容、迁移与回滚

Explicit v3 installation breaks v2 binding compatibility. Existing installations need separately designed/approved migration; this installer rejects them. No private key/state schema migration, automatic upgrade, backup/restore or rollback is included. Preserve partial artifacts and diagnose with approved installation identity; recovery needs explicit owner review. Linux apply is unsupported and must fail before mutations.

## 11. 验证设计与可追溯性

Independent QA checks release tampering/approval, real-backend command generation, plan purity, ACL/path/account conflicts, sudo scope, fault points, repeat validation and initially absent signing authority. Runtime tests cover explicit v3 parser, external path identity, retained FD substitution and target GID. Installer behavior RED follows this design acceptance; import failure is not behavior proof.

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 验证目标 | 验证方法 / 精确来源 | owner / evidence | 未覆盖范围 |
| --- | --- | --- | --- | --- | --- |
| [authority isolation](../design.md#des-p2p-layer-authority) | [des-local-signer-install](#des-local-signer-install) | Fixed installation/control reject unapproved authority; future installer checks follow runbook | [existing negative behavior source](../../../crates/oasis7_local_signer/tests/qa_m0_negative.rs) | Ops / Runtime / QA; Issue 4200; installer RED/GREEN pending | Installer backend and actual host apply not proven; Linux permissions, keys/grants/signing deferred |

## 12. 决策、长期风险与未决问题

Independently approved bootstrap and release digests select exact captured bytes; neither is publisher authentication. Approval provenance remains an operator responsibility. A root-executed installer is trusted management code and must itself be approved; running editable workspace code as root is not the release contract. Approved package staging under `/private/var/db/oasis7-local-signer-approved/<release-id>` is a separately authorized administrator operation preceding installer startup; it is distinct from the final executable release directory so fresh provisioning has no circular prerequisite. The fixed isolated `/usr/bin/python3 -I -c` invocation admits only the externally approved inline bootstrap. That bootstrap uses anchored same-descriptor native ACL and metadata checks, captures and verifies the complete closed package, then passes immutable bytes to the candidate launcher; direct root execution of the candidate script is unsupported because interpreter script opening precedes any in-script check. Actual macOS ACL semantics, shell approval custody, account and sudo behavior still require separate operator/host acceptance. Broader caller privilege or unsupported Python/OS tools blocks installation rather than expands permissions. External communication/runbook review includes LiveOps under the repository role contract. Source delivery completion does not authorize a host apply.
