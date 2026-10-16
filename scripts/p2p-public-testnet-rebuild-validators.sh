#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Governed transaction contract (current path):
  ./scripts/p2p-public-testnet-rebuild-validators.sh plan \
    --execution-mode triad_staggered \
    --package-dir <verified-package-dir> \
    --provenance <verified-pair-provenance.json> \
    --trust-root <provenance-trust-root.json> \
    --identity-receipts <validator-identity-receipts.json> \
    --sequencer-rebuild-proof <signed-204-rebuild-proof.json> \
    --consumer-impact-record <path> \
    --human-direct-ssh-request <executor-bound-human-stop-request.json> \
    --known-hosts <pinned-known-hosts> \
    --quiescence-transaction-id <bounded-quiescence-id> \
    --capacity-json <per-node-capacity.json> \
    --node storage-205=local:<stopped-node-root> \
    --node sequencer-204=local:<stopped-node-root> \
    --sequencer-proof-url <bounded-proof-endpoint> \
    --out-dir <transaction-dir>
  (A persisted --stopped-quiescence-proof may be supplied only as an
   audit-routing locator when the direct request flags are unavailable; it is
   never an admission authority.)
  ./scripts/p2p-public-testnet-rebuild-validators.sh apply \
    --execution-mode triad_staggered \
    --transaction <transaction-dir>/transaction.json \
    --host-adapter <governed-host-adapter>
  OASIS7_VALIDATOR_PAIR_NONCE_LEDGER=/var/lib/oasis7/p2p-public-testnet/validator-pair-nonces.jsonl \
  ./scripts/p2p-public-testnet-rebuild-validators.sh resume \
    --execution-mode triad_staggered \
    --transaction <transaction-dir>/transaction.json \
    --host-adapter <governed-host-adapter> \
    --human-direct-ssh-request <fresh-live-human-stop-request.json> \
    --known-hosts <pinned-known-hosts> \
    [--credential-env <temporary-env-name> | --credential-fd <temporary-fd>] \
    [--adapter-credential-fd <temporary-fd> | \
      --adapter-storage-credential-fd <temporary-fd> \
      --adapter-sequencer-credential-fd <temporary-fd>]
  quiesce --host-adapter is retired and fails closed; use the direct mode:
  ./scripts/p2p-public-testnet-rebuild-validators.sh human_direct_ssh \
    --request <human-stop-request.json> \
    --known-hosts <pinned-known-hosts> \
    --out-dir <quiescence-dir> \
    [--credential-env <temporary-env-name> | --credential-fd <temporary-fd>]
  ./scripts/p2p-public-testnet-rebuild-validators.sh rollback \
    --execution-mode triad_staggered \
    --transaction <transaction-dir>/transaction.json \
    --host-adapter <governed-host-adapter>

Plan is non-mutating but performs bounded live GitHub/SSH read-only
re-observation and emits a stable oasis7.validator_pair_rebuild_plan.v1
digest. It does not call systemd or modify validator state. Apply/rollback
require the governed host adapter and emit transaction-bound phase receipts.
Resume is a governed recovery operation: it requires an explicit transaction,
host adapter, fresh live human-direct-SSH request, canonical pinned known-hosts,
a temporary credential seam (direct observation environment/FD or triad adapter
FD), and an already provisioned absolute
`OASIS7_VALIDATOR_PAIR_NONCE_LEDGER` environment binding. The nonce environment
path must match the request's exact `nonce_ledger_path`; the wrapper never
infers, creates, or falls back to a transaction/output ledger. Persisted proofs
are audit routing only and cannot authorize resume.
The plan contract binds per-node/platform inventories, exact destructive
targets, quiescence, forensic non-seed backup metadata, post-delete absence
targets, new-epoch provenance digests, reset/stage/start and same-window
health gates, and a rollback boundary that forbids restoring deleted chain
state.
`--plan`/`--test-mode`, `--apply`, and `--rollback` are accepted as aliases
for the subcommands.
`quiesce --host-adapter` is retired and fails closed because caller-supplied
JSON cannot establish host-state authority. Use `human_direct_ssh` for the
bounded, read-only stop evidence path. It never preflights, resets, stages,
starts, or mutates observer state.

The historical positional SSH rebuild path is retired and fail-closed. Its
former `--config-dir`, `--world-dir`, `--sequencer-ssh-host`,
`--sequencer-sshpass-env`, `--storage-ssh-host`, and
`--storage-sshpass-env` arguments remain source-visible only so old receipts
and audit tooling can identify the retired contract; this wrapper will never
execute that destructive path. Historical receipts are forensic evidence only.

Description:
  Safely rebuild the validator pair with a staggered cutover:
  consumer-impact gate -> preflight both/live baseline ->
  storage reset/stage/start/readback while sequencer remains live ->
  sequencer reset/stage/start/readback while storage remains live ->
  final pair readiness.
  At most one existing validator is stopped at any point. A failed member is
  left stopped after target-only cleanup; its live peer is never stopped as
  rollback. Capture live status evidence after every member transition.
  The governed triad_staggered receipt must prove
  max_simultaneously_stopped_validators=1 and never falls back to historical
  SSH arguments.

  The consumer-impact record must be valid JSON with impact set to active,
  none, or unknown; evidence_source; an RFC3339 timestamp
  with explicit timezone; boolean validators_already_stopped; and
  decision=proceed. Active/unknown records also require governed outage and
  recovery communication references plus producer wording approval.

  This script assumes the validator hosts already have the intended runtime
  package installed. It rebuilds chain state from the provided config/world
  truth and preserves protected host assets such as config/node-keypair.toml.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_command() {
  local name=$1
  command -v "$name" >/dev/null 2>&1 || die "missing command: $name"
}

require_dir() {
  local path=$1
  [[ -d "$path" ]] || die "missing directory: $path"
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || die "missing file: $path"
}

require_resume_inputs() {
  local transaction=""
  local execution_mode=""
  local host_adapter=""
  local direct_request=""
  local known_hosts=""
  local credential_env=""
  local credential_fd=""
  local adapter_credential_fd=""
  local adapter_storage_credential_fd=""
  local adapter_sequencer_credential_fd=""
  while (($#)); do
    case "$1" in
      --execution-mode)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --execution-mode"
        [[ -z "$execution_mode" ]] || die "resume received duplicate --execution-mode"
        [[ "$2" = "pair" || "$2" = "triad_staggered" ]] || die "resume --execution-mode must be pair or triad_staggered"
        execution_mode=$2
        shift 2
        ;;
      --transaction)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --transaction"
        [[ -z "$transaction" ]] || die "resume received duplicate --transaction"
        transaction=$2
        shift 2
        ;;
      --host-adapter)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --host-adapter"
        [[ -z "$host_adapter" ]] || die "resume received duplicate --host-adapter"
        host_adapter=$2
        shift 2
        ;;
      --human-direct-ssh-request|--direct-request|--request)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for $1"
        [[ -z "$direct_request" ]] || die "resume received duplicate live authority request"
        direct_request=$2
        shift 2
        ;;
      --known-hosts)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --known-hosts"
        [[ -z "$known_hosts" ]] || die "resume received duplicate --known-hosts"
        known_hosts=$2
        shift 2
        ;;
      --credential-env)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --credential-env"
        [[ -z "$credential_env" && -z "$credential_fd" ]] || die "resume accepts exactly one credential seam"
        credential_env=$2
        shift 2
        ;;
      --credential-fd)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --credential-fd"
        [[ "$2" =~ ^[0-9]+$ ]] || die "resume --credential-fd must be numeric"
        [[ -z "$credential_env" && -z "$credential_fd" ]] || die "resume accepts exactly one credential seam"
        credential_fd=$2
        shift 2
        ;;
      --adapter-credential-fd)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --adapter-credential-fd"
        [[ "$2" =~ ^[0-9]+$ ]] || die "resume --adapter-credential-fd must be numeric"
        [[ -z "$adapter_credential_fd" && -z "$adapter_storage_credential_fd" && -z "$adapter_sequencer_credential_fd" ]] || die "resume accepts one shared adapter descriptor or both role-specific descriptors"
        adapter_credential_fd=$2
        shift 2
        ;;
      --adapter-storage-credential-fd)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --adapter-storage-credential-fd"
        [[ "$2" =~ ^[0-9]+$ ]] || die "resume --adapter-storage-credential-fd must be numeric"
        [[ -z "$adapter_credential_fd" && -z "$adapter_storage_credential_fd" ]] || die "resume received duplicate or mixed adapter credential descriptors"
        adapter_storage_credential_fd=$2
        shift 2
        ;;
      --adapter-sequencer-credential-fd)
        [[ $# -ge 2 && -n "$2" ]] || die "resume requires a value for --adapter-sequencer-credential-fd"
        [[ "$2" =~ ^[0-9]+$ ]] || die "resume --adapter-sequencer-credential-fd must be numeric"
        [[ -z "$adapter_credential_fd" && -z "$adapter_sequencer_credential_fd" ]] || die "resume received duplicate or mixed adapter credential descriptors"
        adapter_sequencer_credential_fd=$2
        shift 2
        ;;
      *)
        die "resume rejects unsupported or positional input: $1"
        ;;
    esac
  done
  [[ -n "$transaction" ]] || die "resume requires --transaction"
  [[ -n "$host_adapter" ]] || die "resume requires --host-adapter"
  [[ -n "$direct_request" ]] || die "resume requires --request for fresh live GitHub authority"
  [[ -n "$known_hosts" ]] || die "resume requires --known-hosts for the canonical SSH pin"
  if [[ -n "$adapter_storage_credential_fd" || -n "$adapter_sequencer_credential_fd" ]]; then
    [[ -n "$adapter_storage_credential_fd" && -n "$adapter_sequencer_credential_fd" ]] || die "resume requires both role-specific adapter descriptors"
    [[ -z "$adapter_credential_fd" ]] || die "resume accepts one shared adapter descriptor or both role-specific descriptors"
    [[ -z "$credential_fd" ]] || die "resume cannot mix role-specific adapter descriptors with --credential-fd"
  fi
  if [[ -n "$adapter_credential_fd" && ( -n "$adapter_storage_credential_fd" || -n "$adapter_sequencer_credential_fd" ) ]]; then
    die "resume accepts one shared adapter descriptor or both role-specific descriptors"
  fi
  if [[ "$execution_mode" = "triad_staggered" ]]; then
    [[ -n "$credential_fd" || -n "$adapter_credential_fd" || -n "$adapter_storage_credential_fd" ]] || die "triad resume requires a shared or role-specific adapter descriptor"
  else
    [[ -n "$credential_env" || -n "$credential_fd" ]] || die "resume requires a direct-observation credential seam"
  fi
  local nonce_ledger=${OASIS7_VALIDATOR_PAIR_NONCE_LEDGER:-}
  [[ -n "$nonce_ledger" ]] || die "resume requires OASIS7_VALIDATOR_PAIR_NONCE_LEDGER"
  [[ "$nonce_ledger" = /* ]] || die "resume requires an absolute OASIS7_VALIDATOR_PAIR_NONCE_LEDGER path"
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

# The historical SSH entrypoint remains available for audit fixtures, but all
# governed pair transactions must go through the local-first executor.  Keep
# this dispatch in the shell entrypoint so operators and automation have one
# stable command surface while the executor owns the signed provenance and
# direct-SSH read-only gate.  The envelope below is intentionally derived only from
# the executor's JSON; the plan path performs only the bounded, direct
# read-only re-observation required by the human_direct_ssh contract and never
# invokes systemd or deletes a path.
receipt_contract_envelope() {
  local mode=$1
  shift
  local executor="$repo_root/scripts/p2p-public-testnet-validator-pair-rebuild.py"
  require_file "$executor"
  require_command python3

  local raw_output
  raw_output=$(mktemp "${TMPDIR:-/tmp}/o7pt-receipt.XXXXXX")
  trap 'rm -f "$raw_output"' RETURN
  if ! python3 "$executor" "$mode" "$@" >"$raw_output"; then
    cat "$raw_output"
    return 1
  fi

  python3 - "$raw_output" "$mode" "$@" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import sys
import tempfile
from pathlib import Path


raw_path = Path(sys.argv[1])
mode = sys.argv[2]
args = sys.argv[3:]
value = json.loads(raw_path.read_text(encoding="utf-8"))


def digest_json(item: object) -> str:
    return hashlib.sha256(
        json.dumps(item, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def durable_write(path: Path, payload: str) -> None:
    """Publish a receipt atomically, with file and parent-directory barriers."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_fd = -1
    temporary: Path | None = None
    try:
        temporary_fd, temporary_name = tempfile.mkstemp(
            prefix=f".{path.name}.", dir=str(path.parent)
        )
        temporary = Path(temporary_name)
        with os.fdopen(temporary_fd, "w", encoding="utf-8") as handle:
            temporary_fd = -1
            handle.write(payload + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        directory_fd = os.open(str(path.parent), directory_flags)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    except OSError as error:
        if temporary_fd >= 0:
            try:
                os.close(temporary_fd)
            except OSError:
                pass
        if temporary is not None:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
        raise SystemExit(f"durable receipt write failed: {error.__class__.__name__}")


REPOSITORY_EXECUTABLE_RELATIVE = "scripts/p2p-public-testnet-validator-pair-rebuild.py"
REPOSITORY_EXECUTABLE = Path.cwd() / REPOSITORY_EXECUTABLE_RELATIVE
if REPOSITORY_EXECUTABLE.is_symlink() or not REPOSITORY_EXECUTABLE.is_file():
    raise SystemExit("repository-owned validator-pair executor is not a regular file")
REPOSITORY_EXECUTABLE_IDENTITY = {
    "schema_version": "oasis7.validator_pair_rebuild_repository_executable.v1",
    "path": REPOSITORY_EXECUTABLE_RELATIVE,
    "sha256": sha256_file(REPOSITORY_EXECUTABLE),
}


def entry(path: Path, relative: str) -> dict[str, object]:
    info: dict[str, object] = {"path": relative}
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return {"path": relative, "kind": "missing", "exists": False}
    info.update(
        {
            "exists": True,
            "mode": stat.S_IMODE(metadata.st_mode),
            "uid": metadata.st_uid,
            "gid": metadata.st_gid,
        }
    )
    if stat.S_ISLNK(metadata.st_mode):
        info.update({"kind": "symlink", "target": os.readlink(path), "size_bytes": 0})
    elif stat.S_ISDIR(metadata.st_mode):
        info.update({"kind": "directory", "size_bytes": 0})
    elif stat.S_ISREG(metadata.st_mode):
        info.update({"kind": "file", "size_bytes": metadata.st_size, "sha256": sha256_file(path)})
    else:
        info.update({"kind": "other", "size_bytes": metadata.st_size})
    return info


def tree_inventory(root: Path) -> dict[str, object]:
    """Inventory without following symlinks, including metadata and hashes."""
    if not root.exists() and not root.is_symlink():
        entries = [entry(root, ".")]
    else:
        items = [root]
        if root.is_dir() and not root.is_symlink():
            items.extend(sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()))
        entries = [
            entry(item, "." if item == root else item.relative_to(root).as_posix())
            for item in items
        ]
    counts = {"entry_count": 0, "link_count": 0, "dir_count": 0, "file_count": 0, "total_bytes": 0}
    for item in entries:
        if not item.get("exists"):
            continue
        counts["entry_count"] += 1
        if item["kind"] == "symlink":
            counts["link_count"] += 1
        elif item["kind"] == "directory":
            counts["dir_count"] += 1
        elif item["kind"] == "file":
            counts["file_count"] += 1
            counts["total_bytes"] += int(item.get("size_bytes", 0))
    counts["entry_class_equation"] = (
        counts["entry_count"]
        == counts["link_count"] + counts["dir_count"] + counts["file_count"]
    )
    return {
        **counts,
        "sha256": digest_json(entries),
        "entries": entries,
    }


RESET_TARGETS = [
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
]

TRIAD_REMOTE_ROOT = "/opt/oasis7/p2p-testnet"
TRIAD_REMOTE_HOSTS = {
    "storage-205": "root@39.104.205.67",
    "sequencer-204": "root@39.104.204.172",
}
TRIAD_REMOTE_BACKUP_SCHEMA = "oasis7.validator_pair_rebuild_remote_backup_receipt.v1"
TRIAD_REMOTE_BACKUP_PHASES = (
    "staggered-preflight",
    "staggered-storage-backup",
    "staggered-storage",
    "staggered-sequencer-backup",
    "staggered-sequencer",
)
TRIAD_REMOTE_BACKUP_NUMERIC_FIELDS = (
    "available_bytes",
    "free_bytes",
    "required_bytes",
    "free_inodes",
    "required_inodes",
)


def reset_target_digest() -> str:
    return digest_json(RESET_TARGETS)


def cli_value(flag: str) -> str | None:
    try:
        return args[args.index(flag) + 1]
    except (ValueError, IndexError):
        return None


def triad_remote_backup_entry(role: str, observed: object, transaction_id: object) -> dict[str, object]:
    """Validate a remote FixedSSH receipt without reading its remote path locally."""
    if not isinstance(observed, dict):
        raise SystemExit(f"missing remote forensic backup receipt for {role}")
    if observed.get("schema_version") != TRIAD_REMOTE_BACKUP_SCHEMA:
        raise SystemExit(f"unsupported remote forensic backup receipt schema for {role}")
    if (
        observed.get("remote_target") is not True
        or observed.get("role") != role
        or observed.get("transaction_id") != transaction_id
        or observed.get("credential_transport") != "fd-only-v1"
        or observed.get("remote_host") != TRIAD_REMOTE_HOSTS[role]
        or observed.get("remote_root") != TRIAD_REMOTE_ROOT
    ):
        raise SystemExit(f"remote forensic backup host/transaction binding mismatch for {role}")
    if not isinstance(transaction_id, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.+:~-]*", transaction_id) or ".." in transaction_id:
        raise SystemExit("remote forensic backup transaction id is unsafe")
    backup_root = f"{TRIAD_REMOTE_ROOT}/backups/{transaction_id}"
    manifest = f"{backup_root}/manifest.json"
    if observed.get("backup_root") != backup_root or observed.get("manifest") != manifest:
        raise SystemExit(f"remote forensic backup path binding mismatch for {role}")
    manifest_sha256 = observed.get("manifest_sha256")
    if not isinstance(manifest_sha256, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", manifest_sha256):
        raise SystemExit(f"remote forensic backup manifest digest is malformed for {role}")
    if observed.get("reset_surface_manifest_sha256") != manifest_sha256:
        raise SystemExit(f"remote reset-surface manifest digest mismatch for {role}")
    if observed.get("reset_surfaces") != list(RESET_TARGETS):
        raise SystemExit(f"remote forensic backup reset-surface binding mismatch for {role}")
    non_seed = observed.get("backup_non_seed")
    if (
        not isinstance(non_seed, dict)
        or non_seed.get("forensic_only") is not True
        or non_seed.get("seed_eligible") is not False
        or non_seed.get("restore_deleted_chain_state") is not False
    ):
        raise SystemExit(f"remote forensic backup non-seed binding mismatch for {role}")
    capacity = observed.get("capacity")
    if (
        not isinstance(capacity, dict)
        or capacity.get("verified") is not True
        or capacity.get("same_filesystem") is not True
    ):
        raise SystemExit(f"remote forensic backup capacity is not verified for {role}")
    for field in TRIAD_REMOTE_BACKUP_NUMERIC_FIELDS:
        field_value = capacity.get(field)
        if isinstance(field_value, bool) or not isinstance(field_value, int) or field_value < 0:
            raise SystemExit(f"remote forensic backup capacity field is malformed for {role}: {field}")
    planned = value.get("capacity", {}).get(role) if isinstance(value.get("capacity"), dict) else None
    if not isinstance(planned, dict):
        raise SystemExit(f"code-owned remote backup capacity plan is missing for {role}")
    required_bytes = planned.get("required_bytes")
    required_inodes = planned.get("required_inodes")
    if (
        isinstance(required_bytes, bool)
        or not isinstance(required_bytes, int)
        or required_bytes <= 0
        or isinstance(required_inodes, bool)
        or not isinstance(required_inodes, int)
        or required_inodes < 128
        or capacity.get("required_bytes") != required_bytes
        or capacity.get("required_inodes") != required_inodes
        or capacity.get("available_bytes", 0) < required_bytes
        or capacity.get("free_bytes", 0) < required_bytes
        or capacity.get("free_inodes", 0) < required_inodes
    ):
        raise SystemExit(f"remote forensic backup capacity threshold mismatch for {role}")
    return observed


def triad_backup_phase_envelope() -> None:
    """Bind backup phases and peer-live proof without local backup authority."""
    if value.get("execution_mode", "pair") != "triad_staggered":
        return
    expected = list(TRIAD_REMOTE_BACKUP_PHASES)
    if mode == "rollback":
        expected.append("staggered-rollback")
    supplied = value.get("backup_phase_order")
    if supplied is not None and supplied != expected:
        raise SystemExit("triad remote backup phase order is not deterministic")
    value["backup_phase_order"] = expected
    transaction_id = value.get("transaction_id")
    backups = value.get("backup") if isinstance(value.get("backup"), dict) else {}
    if mode == "plan" and not backups:
        return
    if set(backups) != {"storage-205", "sequencer-204"}:
        raise SystemExit("triad receipt must contain both remote forensic backup entries")
    for role in ("storage-205", "sequencer-204"):
        observed = triad_remote_backup_entry(role, backups.get(role), transaction_id)
        phase_name = "staggered-storage-backup" if role == "storage-205" else "staggered-sequencer-backup"
        phase_receipt = value.get(f"{role}_staggered_backup_receipt")
        if not isinstance(phase_receipt, dict):
            # Plan-only envelopes may carry the durable remote entries without
            # having executed a callback. Apply/resume/rollback must carry the
            # executor-owned phase envelope as well. A supplied explicit
            # backup_phase_order is that durable envelope for adapters that
            # publish phase receipts separately from the transaction body.
            if mode != "plan" and supplied is None:
                raise SystemExit(f"missing {phase_name} receipt for {role}")
            continue
        if (
            phase_receipt.get("phase") != phase_name
            or phase_receipt.get("staggered_phase") != "remote_backup"
            or phase_receipt.get("backup_role") != role
            or phase_receipt.get("backup_before_stop") is not True
            or phase_receipt.get("target_stopped_before_reset") is not False
            or phase_receipt.get("reset_started_after_target_stop") is not False
        ):
            raise SystemExit(f"{phase_name} receipt ordering/role binding failed for {role}")
        nodes = phase_receipt.get("nodes")
        target = nodes.get(role) if isinstance(nodes, dict) else None
        peer_role = "sequencer-204" if role == "storage-205" else "storage-205"
        peer = nodes.get(peer_role) if isinstance(nodes, dict) else None
        if not isinstance(target, dict) or not isinstance(peer, dict):
            raise SystemExit(f"{phase_name} receipt must cover target and live peer")
        triad_remote_backup_entry(role, target, transaction_id)
        if any(target.get(key) != observed.get(key) for key in ("manifest", "manifest_sha256", "backup_root", "transaction_id", "role")):
            raise SystemExit(f"{phase_name} target receipt differs from durable backup entry for {role}")
        if target.get("backup_verified") is not True:
            raise SystemExit(f"{phase_name} target backup verification is missing for {role}")
        if (
            peer.get("active") is not True
            or peer.get("running") is not True
            or peer.get("service_state") != "running"
            or peer.get("independently_observed") is not True
            or peer.get("healthz_ok") is not True
            or peer.get("nrestarts") != 0
            or peer.get("oom_panic_segfault") is not False
        ):
            raise SystemExit(f"{phase_name} live peer proof failed for {peer_role}")


def node_contracts() -> dict[str, object]:
    nodes = value.get("nodes") if isinstance(value.get("nodes"), dict) else {}
    result: dict[str, object] = {}
    for role in value.get("mutation_order", []):
        node = nodes.get(role, {}) if isinstance(nodes, dict) else {}
        root = Path(str(node.get("root", ""))).resolve()
        root_inventory = tree_inventory(root)
        targets: list[dict[str, object]] = []
        for relative in RESET_TARGETS:
            target = root / relative
            resolved = target.resolve(strict=False)
            target_info = entry(target, relative)
            targets.append(
                {
                    **target_info,
                    "resolved_path": str(resolved),
                    "resolution_proof": {
                        "root": str(root),
                        "relative_path": relative,
                        "lexical_target": str(target),
                        "resolved_target": str(resolved),
                        "no_follow_symlink": True,
                        "target_under_root": str(resolved).startswith(f"{root}{os.sep}"),
                        "destructive": True,
                    },
                }
            )
        backup_entries = [
            item for item in root_inventory["entries"]
            if item.get("path") != "backups" and not str(item.get("path", "")).startswith("backups/")
        ]
        backup_manifest = {
            "schema_version": "oasis7.validator_pair_rebuild_forensic_backup_manifest.v1",
            "root": str(root),
            "entries": backup_entries,
            "entry_count": len(backup_entries),
            "sha256": digest_json(backup_entries),
        }
        post_delete_targets = [item["resolved_path"] for item in targets]
        post_delete_proof = {
            "schema_version": "oasis7.validator_pair_rebuild_post_delete_absence.v1",
            "phase": "reset-after-quiesce-before-stage",
            "required": True,
            "target_set": list(RESET_TARGETS),
            "target_set_sha256": reset_target_digest(),
            "targets": post_delete_targets,
            "indexed_sidecars_archives_module_stores_execution_records_bridge_runtime_replication": True,
            "observer_equivalents_not_mutated_by_pair_transaction": True,
            "status": "required_at_apply" if mode == "plan" else "receipt_bound",
        }
        triad_mode = value.get("execution_mode", "pair") == "triad_staggered"
        if triad_mode:
            triad_backup_phase_envelope()
        completed_roles = value.get("staggered_completed_roles", [])
        if not isinstance(completed_roles, list):
            completed_roles = []
        if mode not in {"plan", "quiesce", "human_direct_ssh"}:
            staged = value.get("staged", {}) if isinstance(value.get("staged"), dict) else {}
            observed = staged.get(role, {}).get("post_delete_absence") if isinstance(staged.get(role), dict) else None
            if not isinstance(observed, dict):
                if not triad_mode or role in completed_roles:
                    raise SystemExit(f"missing post-delete absence receipt for {role}")
                post_delete_proof["status"] = "not_run"
            elif observed.get("absent") is not True or observed.get("target_set") != list(RESET_TARGETS) or observed.get("target_set_sha256") != reset_target_digest():
                raise SystemExit(f"post-delete absence target binding mismatch for {role}")
            else:
                post_delete_proof["receipt"] = observed
        backup_receipt = None
        backups = value.get("backup", {}) if isinstance(value.get("backup"), dict) else {}
        if mode not in {"plan", "quiesce", "human_direct_ssh"}:
            observed_backup = backups.get(role) if isinstance(backups.get(role), dict) else None
            if not isinstance(observed_backup, dict):
                if not triad_mode or role in completed_roles or role in staged:
                    raise SystemExit(f"missing forensic backup receipt for {role}")
            else:
                if triad_mode:
                    # The FixedSSH receipt is remote authority.  The wrapper
                    # must not stat/hash a path on the local transaction host
                    # or turn local inventory into permission to mutate.
                    remote = triad_remote_backup_entry(role, observed_backup, value.get("transaction_id"))
                    backup_receipt = {
                        "authority": "fixedssh_remote_target",
                        "remote_target": True,
                        "schema_version": remote["schema_version"],
                        "backup_root": remote["backup_root"],
                        "manifest": remote["manifest"],
                        "backup_manifest_sha256": remote["manifest_sha256"],
                        "reset_surface_manifest_sha256": remote["reset_surface_manifest_sha256"],
                        "backup_capacity": remote["capacity"],
                        "backup_non_seed": remote["backup_non_seed"],
                        "local_audit_only": True,
                    }
                else:
                    manifest_path = observed_backup.get("manifest")
                    backup_root = observed_backup.get("backup_root")
                    manifest_sha256 = observed_backup.get("manifest_sha256")
                    if not isinstance(manifest_path, str) or not isinstance(backup_root, str) or not isinstance(manifest_sha256, str):
                        raise SystemExit(f"incomplete forensic backup receipt for {role}")
                    manifest_file = Path(manifest_path)
                    backup_root_path = Path(backup_root).resolve()
                    if manifest_file.is_symlink() or not manifest_file.is_file() or manifest_file.resolve().parent != backup_root_path:
                        raise SystemExit(f"forensic backup manifest path binding mismatch for {role}")
                    if sha256_file(manifest_file) != manifest_sha256.lower():
                        raise SystemExit(f"forensic backup manifest digest mismatch for {role}")
                    if backup_root_path.parent != root / "backups":
                        raise SystemExit(f"forensic backup root binding mismatch for {role}")
                    backup_receipt = {
                        "backup_root": str(backup_root_path),
                        "backup_manifest_sha256": manifest_sha256,
                        "backup_inventory": value.get("capacity", {}).get(role, {}).get("inventory", {}),
                        "backup_capacity": value.get("capacity", {}).get(role, {}),
                        "backup_non_seed": {
                            "forensic_only": True,
                            "seed_eligible": False,
                            "restore_deleted_chain_state": False,
                        },
                    }
        result[role] = {
            "role": role,
            "platform": node.get("transport", "unknown"),
            "root": str(root),
            "inventory": {
                key: root_inventory[key]
                for key in ("sha256", "entry_count", "link_count", "dir_count", "file_count", "total_bytes", "entry_class_equation")
            },
            "state_roots": targets,
        "destructive_target_resolution": targets,
            "observer_equivalents": {
                "status": "hold",
                "mutation": False,
                "state_roots": [item["path"] for item in targets],
                "absence_proof_required_before_observer_mutation": True,
            },
            "post_delete_absence_proof": post_delete_proof,
            "forensic_backup": {
                "authority": "local_audit_only" if triad_mode else "local_transaction_backup",
                "local_audit_only": triad_mode,
                "remote_authority_required": triad_mode,
                "manifest": backup_manifest,
                "manifest_sha256": digest_json(backup_manifest),
                "capacity": value.get("capacity", {}).get(role, {}),
                "metadata_bound": True,
                "non_seed_proof": {
                    "schema_version": "oasis7.validator_pair_rebuild_backup_non_seed.v1",
                    "forensic_only": True,
                    "seed_eligible": False,
                    "seed_source_paths": [],
                    "staged_from": "package-and-governed-provenance-only",
                    "restore_deleted_chain_state": False,
                    "machine_checkable": True,
                },
            },
            "stopped_quiescence_proof": {
                "required": True,
                "remote_activity": "executor-owned-direct-ssh-read-only" if mode in {"plan", "human_direct_ssh"} else False,
                "observation_state": value.get("proof", {}).get(
                    "baseline_observation_state",
                    value.get("observation_state", "stopped"),
                ),
                "proof": (
                    "repository-owned executor identity binds fixed direct-SSH read-only live baseline to role, digest, active=true,running=true before each target reset"
                    if value.get("execution_mode", "pair") == "triad_staggered"
                    else "repository-owned executor identity binds fixed direct-SSH read-only quiescence to role, digest, active=false,running=false before reset"
                ),
            },
        }
        if backup_receipt is not None:
            result[role]["forensic_backup"]["receipt"] = backup_receipt
    return result


contract = value.setdefault("receipt_contract", {})
contract.update(
    {
        "schema_version": "oasis7.validator_pair_rebuild_receipt_contract.v1",
        "mode": mode,
        "execution_mode": value.get("execution_mode", "pair"),
        "mutation_order": value.get("mutation_order", ["storage-205", "sequencer-204"]),
        "startup_order": value.get("startup_order", ["sequencer-204", "storage-205"]),
        "nodes": node_contracts(),
        "repository_executable": REPOSITORY_EXECUTABLE_IDENTITY,
        "provenance_epoch": value.get("provenance", {}).get("epoch", "plan-bound"),
        "provenance_digests": {
            "package": {
                "path": value.get("package", {}).get("directory"),
                "sha256": digest_json(value.get("package", {})),
                "runtime_sha256": value.get("package", {}).get("runtime_sha256"),
                "runtime_size_bytes": value.get("package", {}).get("runtime_size_bytes"),
            },
            "governed": value.get("network", {}).get("governed", {}),
            "network_id": value.get("network", {}).get("network_id"),
            "chain_id": value.get("network", {}).get("chain_id"),
        },
        "phase_receipts": {
            "reset": "quiesce-and-post-delete-receipt-required",
            "stage": "staged-governed-inventory-receipt-required",
            "start": "sequencer-then-storage-host-receipt-required",
            "same_window_fleet_health": "same host-receipt captured_at window required",
        },
        "executor_live_observations": value.get("staggered_live_observations", {}),
        "executor_rollback_observations": value.get("staggered_rollback_observations", {}),
        "rollback_boundary": {
            "schema_version": "oasis7.validator_pair_rebuild_rollback_boundary.v1",
            "restore_deleted_chain_state": False,
            "restore_only_forensic_snapshot": True,
            "requires_quiescence_before_restore": True,
            "observer_mutation": False,
        },
    }
)
if contract["execution_mode"] == "triad_staggered":
    requested_execution_mode = cli_value("--execution-mode")
    if requested_execution_mode and requested_execution_mode != contract["execution_mode"]:
        raise SystemExit("executor receipt execution mode differs from requested execution mode")
    phase_order = value.get("phase_order", contract.get("phase_order"))
    if phase_order is None:
        phase_order = ["staggered-preflight", "staggered-storage", "staggered-sequencer"]
    expected_phase_order = ["staggered-preflight", "staggered-storage", "staggered-sequencer"]
    if mode == "rollback":
        expected_phase_order.append("staggered-rollback")
    if phase_order != expected_phase_order:
        raise SystemExit("triad staggered phase order is not deterministic")
    max_stopped = value.get(
        "max_simultaneously_stopped_validators",
        value.get("pair_preservation", {}).get("max_simultaneously_stopped_validators", 1),
    )
    if max_stopped != 1:
        raise SystemExit("triad staggered receipt violates max_simultaneously_stopped_validators=1")
    contract["pair_preservation"] = {
        "max_simultaneously_stopped_validators": 1,
        "member_order": ["storage-205", "sequencer-204"],
        "live_peer_readback_before_each_reset": True,
        "rebuilt_member_readback_before_next_reset": True,
        "rollback": "target_only_cleanup_and_preserve_live_peer",
        "historical_ssh_fallback": False,
    }
    contract.update(
        {
            "max_simultaneously_stopped_validators": 1,
            "phase_order": phase_order,
            "backup_phase_order": value.get("backup_phase_order", []),
            "historical_ssh_fallback": False,
        }
    )
elif cli_value("--execution-mode") and cli_value("--execution-mode") != contract["execution_mode"]:
    raise SystemExit("executor receipt execution mode differs from requested execution mode")
# Keep the provenance names machine-checkable even when the signed receipt's
# governed map uses a different internal key order.
governed = value.get("network", {}).get("governed", {})
if isinstance(governed, dict):
    aliases = contract["provenance_digests"]
    aliases["new_epoch"] = value.get("provenance", {}).get("epoch", "plan-bound")
    for name in ("package", "genesis", "world", "registry", "manifest", "bootstrap"):
        if name == "package":
            aliases[name] = aliases["package"]
        else:
            aliases[name] = governed.get(name) or governed.get(f"{name}_artifact") or None
if mode == "plan":
    # Plan output is deliberately timestamp-free and therefore byte-stable.
    contract["deterministic"] = True
    # ``plan`` performs the executor-owned live GitHub authority read and
    # direct SSH quiescence observation before emitting the plan.  The plan
    # remains non-mutating, but reporting no remote activity is false.
    contract["remote_activity"] = "executor-owned-direct-ssh-read-only"
    contract["systemd_activity"] = False
    contract["destructive_activity"] = False
elif mode in {"quiesce", "human_direct_ssh"}:
    # Quiescence is an evidence-only transition.  It may not preflight,
    # reset, stage, start, or mutate any observer or validator state.
    contract["deterministic"] = False
    contract["remote_activity"] = "executor-owned-direct-ssh-read-only" if mode == "human_direct_ssh" else "governed-host-adapter-quiesce-only"
    contract["systemd_activity"] = False
    contract["destructive_activity"] = False
    contract["quiescence_only"] = True
    contract["preflight"] = "forbidden"
    contract["reset"] = "forbidden"
    contract["stage"] = "forbidden"
    if mode == "human_direct_ssh":
        contract["authority"] = "human_direct_ssh"
        contract["provider"] = value.get("provider")
        contract["fixed_command_allowlist"] = value.get("commands")
        contract["host_key_pins"] = value.get("host_fingerprints")
        contract["credential_seam"] = "temporary-fd-or-environment; secret-free argv/log/receipt"
else:
    contract["captured_at"] = value.get("host_receipt", {}).get("captured_at")
    contract["deterministic"] = False
    contract["remote_activity"] = "governed-host-adapter"
    contract["systemd_activity"] = "governed-host-adapter"
    contract["destructive_activity"] = "reset-targets-only"
    contract["host_receipts"] = {
        key: value.get(key)
        for key in ("quiesce_receipt", "backup_receipt", "host_receipt", "rollback_receipt", "rollback_quiesce_receipt")
        if key in value
    }
    contract["same_window_fleet_health"] = {
        "required": True,
        "status": "verified" if isinstance(value.get("host_receipt"), dict) else "not_run",
        "captured_at": value.get("host_receipt", {}).get("captured_at"),
        "full_204_chain_status_called": False,
    }

if mode == "plan":
    value["plan_digest"] = digest_json({key: item for key, item in value.items() if key != "plan_digest"})
else:
    value["canonical_digest"] = digest_json({key: item for key, item in value.items() if key != "canonical_digest"})

output = json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
target = cli_value("--transaction")
if target is None:
    out_dir = cli_value("--out-dir")
    target = str(Path(out_dir).resolve() / "transaction.json") if out_dir else None
if target:
    destination = Path(target)
    durable_write(destination, output)
print(output)
PY
}

# New governed modes are explicit subcommands.  The former positional SSH
# rebuild path is retained below only as unreachable legacy source for audit
# fixtures; fail closed before its parser so it cannot stop, reset, stage, or
# start a live node.
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

case "${1:-}" in
  plan|apply|rollback|quiesce|human_direct_ssh)
    receipt_contract_envelope "$@"
    exit $?
    ;;
  resume)
    require_resume_inputs "${@:2}"
    receipt_contract_envelope "$@"
    exit $?
    ;;
  --plan|--test-mode)
    shift
    receipt_contract_envelope plan "$@"
    exit $?
    ;;
  --apply)
    shift
    receipt_contract_envelope apply "$@"
    exit $?
    ;;
  --quiesce)
    shift
    receipt_contract_envelope quiesce "$@"
    exit $?
    ;;
  --rollback)
    shift
    receipt_contract_envelope rollback "$@"
    exit $?
    ;;
esac

if [[ $# -eq 0 ]]; then
  die "a governed subcommand is required; the legacy destructive SSH rebuild path is retired"
fi
die "legacy destructive SSH rebuild invocation is retired and fail-closed; use the governed plan/apply/resume/rollback path"

CONFIG_DIR=""
WORLD_DIR=""
CONSUMER_IMPACT_RECORD=""
SEQUENCER_SSH_HOST=""
SEQUENCER_SSHPASS_ENV=""
SEQUENCER_SERVICE=""
SEQUENCER_STATUS_URL=""
STORAGE_SSH_HOST=""
STORAGE_SSHPASS_ENV=""
STORAGE_SERVICE=""
STORAGE_STATUS_URL=""
STACK_ROOT="/opt/oasis7/p2p-testnet"
OUT_DIR=""
POLL_ATTEMPTS=20
POLL_SLEEP_SECONDS=3
DISABLE_SSH_MULTIPLEX=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-dir)
      CONFIG_DIR=${2:-}
      shift 2
      ;;
    --world-dir)
      WORLD_DIR=${2:-}
      shift 2
      ;;
    --consumer-impact-record)
      CONSUMER_IMPACT_RECORD=${2:-}
      shift 2
      ;;
    --sequencer-ssh-host)
      SEQUENCER_SSH_HOST=${2:-}
      shift 2
      ;;
    --sequencer-sshpass-env)
      SEQUENCER_SSHPASS_ENV=${2:-}
      shift 2
      ;;
    --sequencer-service)
      SEQUENCER_SERVICE=${2:-}
      shift 2
      ;;
    --sequencer-status-url)
      SEQUENCER_STATUS_URL=${2:-}
      shift 2
      ;;
    --storage-ssh-host)
      STORAGE_SSH_HOST=${2:-}
      shift 2
      ;;
    --storage-sshpass-env)
      STORAGE_SSHPASS_ENV=${2:-}
      shift 2
      ;;
    --storage-service)
      STORAGE_SERVICE=${2:-}
      shift 2
      ;;
    --storage-status-url)
      STORAGE_STATUS_URL=${2:-}
      shift 2
      ;;
    --stack-root)
      STACK_ROOT=${2:-}
      shift 2
      ;;
    --out-dir)
      OUT_DIR=${2:-}
      shift 2
      ;;
    --poll-attempts)
      POLL_ATTEMPTS=${2:-}
      shift 2
      ;;
    --poll-sleep-seconds)
      POLL_SLEEP_SECONDS=${2:-}
      shift 2
      ;;
    --disable-ssh-multiplex)
      DISABLE_SSH_MULTIPLEX=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

require_command jq
require_command curl
require_command ssh
require_command sshpass
require_command tar
require_command mktemp
require_command shasum

[[ -n "$CONFIG_DIR" ]] || die "--config-dir is required"
[[ -n "$WORLD_DIR" ]] || die "--world-dir is required"
[[ -n "$CONSUMER_IMPACT_RECORD" ]] || die "--consumer-impact-record is required"
[[ -n "$SEQUENCER_SSH_HOST" ]] || die "--sequencer-ssh-host is required"
[[ -n "$SEQUENCER_SSHPASS_ENV" ]] || die "--sequencer-sshpass-env is required"
[[ -n "$SEQUENCER_SERVICE" ]] || die "--sequencer-service is required"
[[ -n "$SEQUENCER_STATUS_URL" ]] || die "--sequencer-status-url is required"
[[ -n "$STORAGE_SSH_HOST" ]] || die "--storage-ssh-host is required"
[[ -n "$STORAGE_SSHPASS_ENV" ]] || die "--storage-sshpass-env is required"
[[ -n "$STORAGE_SERVICE" ]] || die "--storage-service is required"
[[ -n "$STORAGE_STATUS_URL" ]] || die "--storage-status-url is required"

require_file "$CONSUMER_IMPACT_RECORD"
if ! jq -e . "$CONSUMER_IMPACT_RECORD" >/dev/null 2>&1; then
  die "consumer-impact record must contain valid JSON: $CONSUMER_IMPACT_RECORD"
fi
consumer_impact_decision=$(jq -r '.decision // empty' "$CONSUMER_IMPACT_RECORD")
if [[ "$consumer_impact_decision" != "proceed" ]]; then
  die "consumer-impact decision must be proceed"
fi
if ! jq -e '
  def nonempty_string:
    type == "string" and ((gsub("^[[:space:]]+|[[:space:]]+$"; "")) | length) > 0;
  def governed_reference:
    nonempty_string
    and ((gsub("^[[:space:]]+|[[:space:]]+$"; "") | ascii_downcase) != "n/a");
  def leap_year($year):
    ($year % 4 == 0)
    and (($year % 100 != 0) or ($year % 400 == 0));
  def days_in_month($year; $month):
    if $month == 2 then
      if leap_year($year) then 29 else 28 end
    elif $month == 1 or $month == 3 or $month == 5 or $month == 7
      or $month == 8 or $month == 10 or $month == 12 then
      31
    else
      30
    end;
  def valid_calendar_date:
    (.[0:4] | tonumber) as $year
    | (.[5:7] | tonumber) as $month
    | (.[8:10] | tonumber) as $day
    | $day <= days_in_month($year; $month);
  type == "object"
  and (.impact == "active" or .impact == "none" or .impact == "unknown")
  and (.evidence_source | nonempty_string)
  and (.timestamp | type == "string")
  and (.timestamp | test("^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](\\.[0-9]+)?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$"))
  and (.timestamp | valid_calendar_date)
  and (.validators_already_stopped | type == "boolean")
  and (.decision == "proceed")
  and (.outage_update_channel | nonempty_string)
  and (.recovery_update_checkpoint | nonempty_string)
  and (.producer_wording_approval | nonempty_string)
  and (
    .impact == "none"
    or (
      (.outage_update_channel | governed_reference)
      and (.recovery_update_checkpoint | governed_reference)
      and (.producer_wording_approval | governed_reference)
    )
  )
' "$CONSUMER_IMPACT_RECORD" >/dev/null 2>&1; then
  die "consumer-impact record is invalid"
fi

require_dir "$CONFIG_DIR"
require_dir "$WORLD_DIR"

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$repo_root/.tmp/public-testnet-validator-rebuild-$(date +%Y%m%d-%H%M%S)"
fi
mkdir -p "$OUT_DIR"
CONTROL_DIR="$(mktemp -d "/tmp/o7pt-ssh.XXXXXX")"
cleanup() {
  if [[ -n "${SEQUENCER_CONTROL_PATH:-}" && -S "${SEQUENCER_CONTROL_PATH:-}" ]]; then
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -S "$SEQUENCER_CONTROL_PATH" -O exit "$SEQUENCER_SSH_HOST" >/dev/null 2>&1 || true
  fi
  if [[ -n "${STORAGE_CONTROL_PATH:-}" && -S "${STORAGE_CONTROL_PATH:-}" ]]; then
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -S "$STORAGE_CONTROL_PATH" -O exit "$STORAGE_SSH_HOST" >/dev/null 2>&1 || true
  fi
  rm -rf "$CONTROL_DIR"
}
trap cleanup EXIT

config_files=()
while IFS= read -r path; do
  config_files+=("$path")
done < <(find "$CONFIG_DIR" -maxdepth 1 -type f | sort)
[[ ${#config_files[@]} -gt 0 ]] || die "no top-level config files found in $CONFIG_DIR"

evidence_files=()
if [[ -d "$CONFIG_DIR/doc/testing/evidence" ]]; then
  while IFS= read -r path; do
    evidence_files+=("$path")
  done < <(find "$CONFIG_DIR/doc/testing/evidence" -maxdepth 1 -type f | sort)
fi

network_manifest_path=""
for file in "${config_files[@]}"; do
  if jq -e '(.schema_version // "") == "oasis7.network_tier_manifest.v1"' "$file" >/dev/null 2>&1; then
    network_manifest_path=$file
    break
  fi
done
[[ -n "$network_manifest_path" ]] || die "missing oasis7.network_tier_manifest.v1 config"
WORLD_RESOURCE_WORLD_ID=$(jq -r '.network_id // empty' "$network_manifest_path")
WORLD_RESOURCE_CHAIN_ID=$(jq -r '.chain_id // .network_id // empty' "$network_manifest_path")
[[ -n "$WORLD_RESOURCE_WORLD_ID" ]] || die "network tier manifest missing network_id"
[[ -n "$WORLD_RESOURCE_CHAIN_ID" ]] || die "network tier manifest missing chain_id"

control_path_for() {
  local host=$1
  local label
  label=$(printf '%s' "$host" | shasum | awk '{print substr($1, 1, 10)}')
  printf '%s/%s.sock\n' "$CONTROL_DIR" "$label"
}

open_master_connection() {
  local host=$1
  local sshpass_env_name=$2
  if [[ "$DISABLE_SSH_MULTIPLEX" -eq 1 ]]; then
    printf '\n'
    return 0
  fi
  local control_path
  control_path=$(control_path_for "$host")
  if [[ -S "$control_path" ]]; then
    printf '%s\n' "$control_path"
    return 0
  fi
  local sshpass_value=${!sshpass_env_name:-}
  [[ -n "$sshpass_value" ]] || die "ssh password env is empty: $sshpass_env_name"
  if ! SSHPASS="$sshpass_value" sshpass -e ssh \
    -M -N -f \
    -o ControlMaster=yes \
    -o ControlPersist=3600 \
    -S "$control_path" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$host"; then
    printf '\n'
    return 0
  fi
  printf '%s\n' "$control_path"
}

SEQUENCER_CONTROL_PATH=$(open_master_connection "$SEQUENCER_SSH_HOST" "$SEQUENCER_SSHPASS_ENV")
STORAGE_CONTROL_PATH=$(open_master_connection "$STORAGE_SSH_HOST" "$STORAGE_SSHPASS_ENV")

ssh_run() {
  local host=$1
  local control_path=$2
  shift 2
  local ssh_args=()
  if [[ -n "$control_path" && -S "$control_path" ]] \
    && ssh -S "$control_path" -O check "$host" >/dev/null 2>&1; then
    ssh_args+=(-S "$control_path")
  else
    ssh_args+=(
      -o ControlMaster=no
      -o ControlPath=none
      -o PreferredAuthentications=password
      -o PubkeyAuthentication=no
      -o NumberOfPasswordPrompts=1
      -o ConnectTimeout=10
      -o ServerAliveInterval=15
      -o ServerAliveCountMax=2
    )
    if [[ "$host" == "$SEQUENCER_SSH_HOST" ]]; then
      local sequencer_sshpass=${!SEQUENCER_SSHPASS_ENV:-}
      [[ -n "$sequencer_sshpass" ]] || die "ssh password env is empty: $SEQUENCER_SSHPASS_ENV"
      SSHPASS="$sequencer_sshpass" sshpass -e ssh "${ssh_args[@]}" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "$host" \
        "$@"
      return $?
    fi
    if [[ "$host" == "$STORAGE_SSH_HOST" ]]; then
      local storage_sshpass=${!STORAGE_SSHPASS_ENV:-}
      [[ -n "$storage_sshpass" ]] || die "ssh password env is empty: $STORAGE_SSHPASS_ENV"
      SSHPASS="$storage_sshpass" sshpass -e ssh "${ssh_args[@]}" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "$host" \
        "$@"
      return $?
    fi
  fi
  ssh "${ssh_args[@]}" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$host" \
    "$@"
}

json_sequencer_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
    and (.readiness.status // null) == "ready"
    and (((.readiness.failed_gates // []) | length) == 0)
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.last_execution_height // 0) > 0)
    and (((.consensus.last_execution_block_hash // "") | tostring | length) > 0)
    and (((.consensus.last_execution_state_root // "") | tostring | length) > 0)
    and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
    and ((.world_resource.readiness_status // null) == "ready")
    and (((.world_resource.failed_gates // []) | length) == 0)
  ' "$path" >/dev/null 2>&1
}

json_liveness_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
  ' "$path" >/dev/null 2>&1
}

json_storage_ok() {
  local path=$1
  jq -e '
    .running == true
    and (.last_error == null or .last_error == "null")
    and (.readiness.status // null) == "ready"
    and (((.readiness.failed_gates // []) | length) == 0)
    and (.consensus.storage_challenge_network_degraded_height // null) == null
    and ((.observability.storage_challenge_network_degraded // false) | not)
    and (.replication.connected_peers | length) >= 1
    and ((.consensus.committed_height // 0) > 0)
    and ((.consensus.last_execution_height // 0) > 0)
    and (((.consensus.last_execution_block_hash // "") | tostring | length) > 0)
    and (((.consensus.last_execution_state_root // "") | tostring | length) > 0)
    and ((.consensus.network_head.height // 0) >= (.consensus.committed_height // 0))
    and ((.world_resource.readiness_status // null) == "ready")
    and (((.world_resource.failed_gates // []) | length) == 0)
  ' "$path" >/dev/null 2>&1
}

poll_status_with_check() {
  local url=$1
  local out_path=$2
  local label=$3
  local check_fn=$4
  local attempt=1
  while (( attempt <= POLL_ATTEMPTS )); do
    if curl -fsSL "$url" -o "$out_path.tmp"; then
      if "$check_fn" "$out_path.tmp"; then
        mv "$out_path.tmp" "$out_path"
        return 0
      fi
    fi
    attempt=$((attempt + 1))
    sleep "$POLL_SLEEP_SECONDS"
  done
  if [[ -f "$out_path.tmp" ]]; then
    mv "$out_path.tmp" "$out_path"
  fi
  return 1
}

write_staggered_recovery_receipt() {
  local failed_role=$1
  local phase=$2
  local message=$3
  local preserved_role=$4
  local cleanup_status=${5:-completed}
  jq -n \
    --arg mode "staggered" \
    --arg failed_role "$failed_role" \
    --arg phase "$phase" \
    --arg message "$message" \
    --arg preserved_role "$preserved_role" \
    --arg cleanup_status "$cleanup_status" \
    --arg out_dir "$OUT_DIR" \
    --argjson max_stopped 1 \
    ' {
        schema_version: "oasis7.validator_pair_staggered_recovery.v1",
        mode: $mode,
        failed_role: $failed_role,
        failed_phase: $phase,
        failure: $message,
        preserved_live_peer: $preserved_role,
        max_simultaneously_stopped_validators: $max_stopped,
        rollback: {
          action: "target_only_cleanup_and_preserve_live_peer",
          cleanup_status: $cleanup_status,
          restore_deleted_chain_state: false,
          restore_old_node_state: false,
          requires_fresh_clean_stage: true,
          validator_47_no_start_unchanged: true
        },
        recovery_artifact_dir: $out_dir
      }' >"$OUT_DIR/staggered-recovery.json"
}

staggered_role_host() {
  case "$1" in
    sequencer) printf '%s\n' "$SEQUENCER_SSH_HOST" ;;
    storage) printf '%s\n' "$STORAGE_SSH_HOST" ;;
    *) die "unknown staggered role: $1" ;;
  esac
}

staggered_role_control_path() {
  case "$1" in
    sequencer) printf '%s\n' "$SEQUENCER_CONTROL_PATH" ;;
    storage) printf '%s\n' "$STORAGE_CONTROL_PATH" ;;
    *) die "unknown staggered role: $1" ;;
  esac
}

staggered_role_service() {
  case "$1" in
    sequencer) printf '%s\n' "$SEQUENCER_SERVICE" ;;
    storage) printf '%s\n' "$STORAGE_SERVICE" ;;
    *) die "unknown staggered role: $1" ;;
  esac
}

staggered_role_status_url() {
  case "$1" in
    sequencer) printf '%s\n' "$SEQUENCER_STATUS_URL" ;;
    storage) printf '%s\n' "$STORAGE_STATUS_URL" ;;
    *) die "unknown staggered role: $1" ;;
  esac
}

staggered_peer_role() {
  case "$1" in
    sequencer) printf '%s\n' storage ;;
    storage) printf '%s\n' sequencer ;;
    *) die "unknown staggered role: $1" ;;
  esac
}

staggered_readback_peer() {
  local role=$1
  local peer
  peer=$(staggered_peer_role "$role")
  local peer_url
  peer_url=$(staggered_role_status_url "$peer")
  local peer_path="$OUT_DIR/staggered-${peer}-before-${role}.json"
  poll_status_with_check "$peer_url" "$peer_path" \
    "${peer} live peer readback before ${role} cutover" json_liveness_ok
}

staggered_sequencer_ready() {
  json_sequencer_ok "$1"
}

staggered_storage_ready() {
  json_storage_ok "$1"
}

staggered_abort() {
  local role=$1
  local phase=$2
  local message=$3
  local cleanup_target=${4:-1}
  local peer
  peer=$(staggered_peer_role "$role")
  local cleanup_status="not_required"
  if [[ "$cleanup_target" == 1 ]]; then
    local host control_path service
    host=$(staggered_role_host "$role")
    control_path=$(staggered_role_control_path "$role")
    service=$(staggered_role_service "$role")
    if cleanup_host_processes "$host" "$control_path" "$service"; then
      cleanup_status="completed"
    else
      cleanup_status="failed"
    fi
  fi
  write_staggered_recovery_receipt "$role" "$phase" "$message" "$peer" "$cleanup_status"
  if [[ "$cleanup_status" == failed ]]; then
    die "$message; target-only cleanup failed; see $OUT_DIR/staggered-recovery.json"
  fi
  die "$message; ${peer} was preserved live; see $OUT_DIR/staggered-recovery.json"
}

repair_rebuild_log_value() {
  local log_path=$1
  local key=$2
  awk -F= -v key="$key" '$1 == key { value=$2 } END { if (value != "") print value }' "$log_path"
}

validate_repair_rebuild_log() {
  local log_path=$1
  local label=$2
  local world_time
  local journal_events
  local tick_consensus_records
  world_time=$(repair_rebuild_log_value "$log_path" "world_time")
  journal_events=$(repair_rebuild_log_value "$log_path" "journal_events")
  tick_consensus_records=$(repair_rebuild_log_value "$log_path" "tick_consensus_records")

  if [[ "$world_time" != "0" ]]; then
    printf 'error: %s repair rebuild produced world_time=%s, expected 0\n' "$label" "$world_time" >&2
    return 1
  fi
  if [[ "$journal_events" != "0" ]]; then
    printf 'error: %s repair rebuild produced journal_events=%s, expected 0\n' "$label" "$journal_events" >&2
    return 1
  fi
  if [[ "$tick_consensus_records" != "0" ]]; then
    printf 'error: %s repair rebuild produced tick_consensus_records=%s, expected 0\n' "$label" "$tick_consensus_records" >&2
    return 1
  fi
}

run_repair_rebuild_host() {
  local host=$1
  local control_path=$2
  local label=$3
  local repair_log="$OUT_DIR/$label-repair-rebuild.log"
  local remote_log="$STACK_ROOT/config/doc/testing/evidence/public-testnet-repair-rebuild-$label.log"

  if ! ssh_run "$host" "$control_path" \
    "'$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' --generated-world-dir '$STACK_ROOT/staged-world' --output-world-dir '$STACK_ROOT/data/execution-world' --world-id '$WORLD_RESOURCE_WORLD_ID' --chain-id '$WORLD_RESOURCE_CHAIN_ID' --resource-commit-height 0 --resource-commit-hash genesis" \
    >"$repair_log" 2>&1; then
    cat "$repair_log" >&2 || true
    printf 'error: %s repair rebuild failed\n' "$label" >&2
    return 1
  fi
  cat "$repair_log" >&2
  if ! validate_repair_rebuild_log "$repair_log" "$label"; then
    return 1
  fi
  ssh_run "$host" "$control_path" "cat > '$remote_log'" <"$repair_log"
}

preflight_host() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "command -v python3 >/dev/null && command -v tar >/dev/null && command -v systemctl >/dev/null && command -v ps >/dev/null && test -x '$STACK_ROOT/current/bin/oasis7_chain_runtime' && test -x '$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' && test -x '$STACK_ROOT/current/bin/oasis7_governance_registry_import' && '$STACK_ROOT/current/bin/oasis7_world_repair_rebuild' --help 2>&1 | grep -F -- '--generated-world-dir' >/dev/null"
}

stage_host() {
  local host=$1
  local control_path=$2
  local label=$3
  ssh_run "$host" "$control_path" \
    "mkdir -p '$STACK_ROOT/config/doc/testing/evidence' '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world'"

  local file
  for file in "${config_files[@]}"; do
    require_file "$file"
    local base
    base=$(basename "$file")
    ssh_run "$host" "$control_path" "cat > '$STACK_ROOT/config/$base'" <"$file"
    ssh_run "$host" "$control_path" "cp '$STACK_ROOT/config/$base' '$STACK_ROOT/config/doc/testing/evidence/$base'"
  done
  if ((${#evidence_files[@]} > 0)); then
    for file in "${evidence_files[@]}"; do
      require_file "$file"
      local base
      base=$(basename "$file")
      ssh_run "$host" "$control_path" "cat > '$STACK_ROOT/config/doc/testing/evidence/$base'" <"$file"
    done
  fi

  ssh_run "$host" "$control_path" \
    "rm -rf '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world' && mkdir -p '$STACK_ROOT/staged-world' '$STACK_ROOT/data/execution-world'"
  COPYFILE_DISABLE=1 tar -C "$WORLD_DIR" -cf - . \
    | ssh_run "$host" "$control_path" "tar -C '$STACK_ROOT/staged-world' -xf -"
  ssh_run "$host" "$control_path" \
    "find '$STACK_ROOT/staged-world' \\( -name '._*' -o -name '.DS_Store' \\) -delete"
  if ! run_repair_rebuild_host "$host" "$control_path" "$label"; then
    return 1
  fi
  ssh_run "$host" "$control_path" \
    "cp -R '$STACK_ROOT/staged-world/generated-scenario-world' '$STACK_ROOT/data/execution-world/generated-scenario-world' && cp '$STACK_ROOT/staged-world/world-generation-provenance.json' '$STACK_ROOT/data/execution-world/world-generation-provenance.json'"

  sync_staged_deployment_truth "$host" "$control_path" >&2
  import_staged_governance_registry "$host" "$control_path" >&2
}

sync_staged_deployment_truth() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

stack_root = Path(os.environ['STACK_ROOT'])
config_dir = stack_root / 'config'
env_path = config_dir / 'node.env'
runtime_path = stack_root / 'current' / 'bin' / 'oasis7_chain_runtime'
generated_world_root = stack_root / 'data' / 'execution-world'
generated_world_sidecar_path = generated_world_root / 'generated-scenario-world'
world_generation_provenance_path = generated_world_root / 'world-generation-provenance.json'

if not env_path.is_file():
    raise SystemExit(f'missing node env: {env_path}')
if not runtime_path.is_file():
    raise SystemExit(f'missing installed runtime: {runtime_path}')
if not generated_world_sidecar_path.is_dir():
    raise SystemExit(f'missing staged generated world sidecar: {generated_world_sidecar_path}')
if not world_generation_provenance_path.is_file():
    raise SystemExit(f'missing staged world generation provenance: {world_generation_provenance_path}')

env_values = {}
env_values['STACK_ROOT'] = str(stack_root)
for raw in env_path.read_text(encoding='utf-8').splitlines():
    stripped = raw.strip()
    if not stripped or stripped.startswith('#') or '=' not in stripped:
        continue
    key, value = stripped.split('=', 1)
    if (
        len(value) >= 2
        and value[0] == value[-1]
        and value[0] in (chr(34), chr(39))
    ):
        value = value[1:-1]
    env_values[key] = value

def expand_env_value(value: str) -> str:
    previous = value
    for _ in range(8):
        expanded = previous
        for name, replacement in sorted(env_values.items(), key=lambda item: len(item[0]), reverse=True):
            expanded = expanded.replace(chr(36) + '{' + name + '}', replacement)
            expanded = expanded.replace(chr(36) + name, replacement)
        if chr(36) in expanded:
            raise SystemExit(f'unsupported variable in GENESIS_VALIDATOR_REGISTRY_PATH: {expanded}')
        if expanded == previous:
            return expanded
        previous = expanded
    raise SystemExit('GENESIS_VALIDATOR_REGISTRY_PATH variable expansion did not converge')

registry_raw = env_values.get('GENESIS_VALIDATOR_REGISTRY_PATH')
if not registry_raw:
    raise SystemExit(f'missing GENESIS_VALIDATOR_REGISTRY_PATH in {env_path}')
registry_path = Path(expand_env_value(registry_raw))
if not registry_path.is_absolute():
    registry_path = stack_root / registry_path
try:
    registry_path.resolve(strict=False).relative_to(stack_root.resolve(strict=False))
except ValueError:
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH must stay under stack root: {registry_path}')
if not registry_path.is_file():
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH does not exist after staging: {registry_path}')

registry_data = json.loads(registry_path.read_text(encoding='utf-8'))
validators = registry_data.get('validators')
if not isinstance(validators, list) or not validators:
    raise SystemExit(f'GENESIS_VALIDATOR_REGISTRY_PATH has no validators: {registry_path}')

registry_sha256 = hashlib.sha256(registry_path.read_bytes()).hexdigest()
registry_canonical = {
    'signer_bindings': {
        'governance.finality.v1.{}'.format(item['node_id']): str(item['finality_signer_public_key']).lower()
        for item in validators
    },
    'slot_id': registry_data.get('slot_id'),
    'threshold': registry_data.get('threshold'),
    'threshold_bps': registry_data.get('threshold_bps'),
    'validator_stakes': {
        'governance.finality.v1.{}'.format(item['node_id']): item['stake']
        for item in validators
    },
}
registry_semantic_sha256 = hashlib.sha256(
    json.dumps(registry_canonical, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
).hexdigest()

# A pair rebuild can be the staging boundary for the exact governed triad.
# In that mode every existing managed node must receive the same triad
# inventory and digest.  Never borrow validator-47's node.env: the inventory
# is an authority artifact, while each existing host keeps its own role/env.
triad_inventory_path = config_dir / 'public-testnet-validator-triad-inventory.v1.json'
triad_rollout = len(validators) == 3
inventory_sha256 = ''
if triad_rollout:
    if triad_inventory_path.is_symlink() or not triad_inventory_path.is_file():
        raise SystemExit(f'missing regular triad deployment inventory: {triad_inventory_path}')
    inventory_sha256 = hashlib.sha256(triad_inventory_path.read_bytes()).hexdigest()
    if inventory_sha256 != 'b983bd9df4f29bf7a0e32dd9d1d56323d85d16d4cf572c10bc8c567908565739':
        raise SystemExit('triad deployment inventory is not the canonical governed authority')
    inventory_data = json.loads(triad_inventory_path.read_text(encoding='utf-8'))
    if inventory_data.get('schema_version') != 'oasis7.public_testnet_validator_triad_inventory.v1':
        raise SystemExit('triad deployment inventory schema mismatch')
    if inventory_data.get('network_tier') != 'public_testnet' or inventory_data.get('topology') != 'three_equal_validator':
        raise SystemExit('triad deployment inventory network/topology mismatch')
    inventory_authority = inventory_data.get('authority')
    inventory_nodes = inventory_data.get('nodes')
    if not isinstance(inventory_authority, dict) or not isinstance(inventory_nodes, dict):
        raise SystemExit('triad deployment inventory authority/nodes are missing')
    if inventory_authority.get('generated_registry_sha256') != registry_sha256:
        raise SystemExit('triad deployment inventory generated registry digest mismatch')
    if inventory_authority.get('generated_registry_semantic_sha256') != registry_semantic_sha256:
        raise SystemExit('triad deployment inventory generated registry semantic digest mismatch')
    registry_node_ids = {item.get('node_id') for item in validators}
    inventory_node_ids = {
        item.get('node_id') for item in inventory_nodes.values()
        if isinstance(item, dict) and isinstance(item.get('node_id'), str)
    }
    if inventory_node_ids != registry_node_ids:
        raise SystemExit('triad deployment inventory validator identities mismatch')

signer_pairs = []
for validator in validators:
    node_id = validator.get('node_id')
    public_key = validator.get('finality_signer_public_key')
    if not node_id or not public_key:
        raise SystemExit(f'validator registry entry is missing node_id/finality_signer_public_key in {registry_path}')
    signer_pairs.append(f'{node_id}:{public_key}')
signers_csv = ','.join(signer_pairs)

lines = env_path.read_text(encoding='utf-8').splitlines()
rewrote_signers = False
rewrote_adaptive_tick_scheduler = False
rendered = []
for line in lines:
    if line.startswith('NODE_VALIDATOR_SIGNERS_CSV='):
        rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
        rewrote_signers = True
    elif line.startswith('POS_ADAPTIVE_TICK_SCHEDULER='):
        rendered.append('POS_ADAPTIVE_TICK_SCHEDULER=1')
        rewrote_adaptive_tick_scheduler = True
    elif line.startswith('DEPLOYMENT_INVENTORY_PATH=') or line.startswith('DEPLOYMENT_INVENTORY_SHA256='):
        # Pair-era env files may contain stale inventory claims.  Remove them
        # unless this exact three-validator handoff has been validated below.
        continue
    elif line.startswith('GENESIS_VALIDATOR_REGISTRY_SHA256=') or line.startswith('GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256='):
        # Replace stale pair-era registry claims only after triad validation.
        if triad_rollout:
            continue
        rendered.append(line)
    elif line.startswith('P2P_NODE_ROLE=') and triad_rollout:
        continue
    else:
        rendered.append(line)
if not rewrote_signers:
    rendered.append(f'NODE_VALIDATOR_SIGNERS_CSV={signers_csv}')
if not rewrote_adaptive_tick_scheduler:
    rendered.append('POS_ADAPTIVE_TICK_SCHEDULER=1')
if triad_rollout:
    expected_registry_path = config_dir / 'public-testnet-governed-bootstrap-validator-registry-2026-06-06.json'
    if registry_path.is_symlink() or registry_path.resolve() != expected_registry_path.resolve():
        raise SystemExit(
            'triad pair rebuild must use the staged governed validator registry path; '
            f'got {registry_path}'
        )
    node_id = env_values.get('NODE_ID')
    expected_node_ids = {
        'triad-testnet-sequencer',
        'triad-testnet-storage',
    }
    if node_id not in expected_node_ids:
        raise SystemExit(
            'triad pair rebuild must update only sequencer/storage node.env; '
            f'got NODE_ID={node_id!r}'
        )
    expected_role = 'storage' if node_id == 'triad-testnet-storage' else 'sequencer'
    expected_p2p_role = 'full_storage' if expected_role == 'storage' else 'sequencer'
    if env_values.get('NODE_ROLE') != expected_role:
        raise SystemExit(
            'triad pair rebuild must preserve the existing node role contract; '
            f'expected NODE_ROLE={expected_role!r}, got {env_values.get("NODE_ROLE")!r}'
        )
    if env_values.get('P2P_NODE_ROLE') not in (None, expected_p2p_role):
        raise SystemExit(
            'triad pair rebuild found an incompatible P2P_NODE_ROLE; '
            f'expected {expected_p2p_role!r}, got {env_values.get("P2P_NODE_ROLE")!r}'
        )
    expected_inventory_node = (
        inventory_nodes.get('sequencer-204')
        if node_id == 'triad-testnet-sequencer'
        else inventory_nodes.get('storage-205')
    )
    if not isinstance(expected_inventory_node, dict) or expected_inventory_node.get('node_id') != node_id:
        raise SystemExit(f'triad deployment inventory does not bind {node_id}')
    rendered.append(f'GENESIS_VALIDATOR_REGISTRY_SHA256={registry_sha256}')
    rendered.append(f'GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256={registry_semantic_sha256}')
    rendered.append(f'P2P_NODE_ROLE={expected_p2p_role}')
    rendered.append('DEPLOYMENT_INVENTORY_PATH=config/public-testnet-validator-triad-inventory.v1.json')
    rendered.append(f'DEPLOYMENT_INVENTORY_SHA256={inventory_sha256}')
env_path.write_text('\\n'.join(rendered) + '\\n', encoding='utf-8')

digest = hashlib.sha256()
with runtime_path.open('rb') as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b''):
        digest.update(chunk)
runtime_sha = digest.hexdigest()
runtime_size = runtime_path.stat().st_size
provenance_digest = hashlib.sha256()
with world_generation_provenance_path.open('rb') as fh:
    for chunk in iter(lambda: fh.read(1024 * 1024), b''):
        provenance_digest.update(chunk)
world_generation_provenance_sha = provenance_digest.hexdigest()
world_generation_provenance_size = world_generation_provenance_path.stat().st_size

buildinfo = {}
buildinfo_path = stack_root / 'DEPLOYED_BUILDINFO'
if buildinfo_path.is_file():
    for raw in buildinfo_path.read_text(encoding='utf-8').splitlines():
        key, sep, value = raw.partition('=')
        if sep:
            buildinfo[key] = value

bundle_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-bundle-2026-06-06.json'))
if not bundle_paths:
    raise SystemExit(f'no governed bootstrap bundle found under {config_dir}')

genesis_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-genesis-2026-06-06.json'))
if not genesis_paths:
    raise SystemExit(f'no governed bootstrap genesis found under {config_dir}')

updated_by = 'p2p-public-testnet-rebuild-validators staged deployment truth sync'
if buildinfo.get('package_version') or buildinfo.get('run_id'):
    updated_by += f\" package={buildinfo.get('package_version', 'unknown')} run={buildinfo.get('run_id', 'unknown')}\"

for bundle_path in bundle_paths:
    data = json.loads(bundle_path.read_text(encoding='utf-8'))
    runtime = data.setdefault('runtime_build', {})
    runtime['kind'] = 'file'
    runtime['path'] = str(runtime_path)
    runtime['resolved_path'] = str(runtime_path)
    runtime['sha256'] = runtime_sha
    runtime['size_bytes'] = runtime_size
    runtime['updated_by'] = updated_by
    if buildinfo.get('commit'):
        runtime['git_commit'] = buildinfo['commit']
        data['git_commit'] = buildinfo['commit']
    if buildinfo.get('package_version'):
        runtime['package_version'] = buildinfo['package_version']
    if buildinfo.get('run_id'):
        runtime['run_id'] = buildinfo['run_id']
    if isinstance(data.get('generated_world_sidecar'), dict):
        sidecar = data['generated_world_sidecar']
        sidecar['kind'] = 'directory'
        sidecar['path'] = str(generated_world_sidecar_path)
        sidecar['resolved_path'] = str(generated_world_sidecar_path)
    if isinstance(data.get('world_generation_provenance'), dict):
        provenance = data['world_generation_provenance']
        provenance['kind'] = 'file'
        provenance['path'] = str(world_generation_provenance_path)
        provenance['resolved_path'] = str(world_generation_provenance_path)
        provenance['sha256'] = world_generation_provenance_sha
        provenance['size_bytes'] = world_generation_provenance_size
    data['updated_by'] = updated_by
    bundle_path.write_text(
        json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + '\\n',
        encoding='utf-8',
    )

for genesis_path in genesis_paths:
    data = json.loads(genesis_path.read_text(encoding='utf-8'))
    refs = data.get('governance_bootstrap_refs')
    if isinstance(refs, dict):
        for key, value in list(refs.items()):
            if not isinstance(value, str) or not value.strip():
                continue
            refs[key] = str(config_dir / 'doc' / 'testing' / 'evidence' / Path(value).name)
        genesis_path.write_text(
            json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + '\\n',
            encoding='utf-8',
        )

print(f'synced_validator_signer_count={len(signer_pairs)}')
print(f'synced_runtime_sha256={runtime_sha}')
print(f'synced_generated_world_sidecar={generated_world_sidecar_path}')
print(f'synced_world_generation_provenance_sha256={world_generation_provenance_sha}')
print(f'synced_bundle_count={len(bundle_paths)}')
print(f'synced_genesis_count={len(genesis_paths)}')
PY"
}

import_staged_governance_registry() {
  local host=$1
  local control_path=$2
  ssh_run "$host" "$control_path" \
    "STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

stack_root = Path(os.environ['STACK_ROOT'])
config_dir = stack_root / 'config'
world_dir = stack_root / 'data' / 'execution-world'
import_bin = stack_root / 'current' / 'bin' / 'oasis7_governance_registry_import'

if not import_bin.is_file():
    raise SystemExit(f'missing governance registry import binary: {import_bin}')
if not world_dir.is_dir():
    raise SystemExit(f'missing execution world dir before governance import: {world_dir}')

genesis_paths = sorted(config_dir.rglob('public-testnet-governed-bootstrap-genesis-2026-06-06.json'))
if not genesis_paths:
    raise SystemExit(f'no governed bootstrap genesis found under {config_dir}')

governance_public_manifest = None
for genesis_path in genesis_paths:
    data = json.loads(genesis_path.read_text(encoding='utf-8'))
    refs = data.get('governance_bootstrap_refs')
    if not isinstance(refs, dict):
        continue
    raw = refs.get('governance_public_manifest_ref')
    if isinstance(raw, str) and raw.strip():
        candidate = Path(raw)
        if not candidate.is_absolute():
            candidate = genesis_path.parent / candidate
        if candidate.is_file():
            governance_public_manifest = candidate
            break

if governance_public_manifest is None:
    raise SystemExit(f'no readable governance_public_manifest_ref found under {config_dir}')

command = [
    str(import_bin),
    '--world-dir',
    str(world_dir),
    '--public-manifest',
    str(governance_public_manifest),
]
subprocess.run(command, check=True)
print(f'imported_governance_public_manifest={governance_public_manifest}')
PY"
}

cleanup_host_processes() {
  local host=$1
  local control_path=$2
  local service=$3
  ssh_run "$host" "$control_path" \
    "SERVICE_NAME='$service' STACK_ROOT='$STACK_ROOT' python3 - <<'PY'
import os
import signal
import shlex
import subprocess
import sys
import time

service_name = os.environ['SERVICE_NAME']
stack_root = os.environ['STACK_ROOT']
needles = (
    f'{stack_root}/current/bin/oasis7_chain_runtime',
    f'{stack_root}/bin/start-node.sh',
    f'{stack_root}/releases/',
)

def is_stack_path(value):
    value = (value or '').strip()
    return value == stack_root or value.startswith(f'{stack_root}/')

def unit_metadata_matches_stack_root(metadata):
    for line in metadata.splitlines():
        key, _, value = line.partition('=')
        if key == 'WorkingDirectory':
            if is_stack_path(value):
                return True
        elif key == 'ExecStart':
            for token in shlex.split(value):
                if is_stack_path(token):
                    return True
    return False

def discover_stack_services():
    candidates = {service_name}
    for command in (
        ['systemctl', 'list-unit-files', '--type=service', '--no-legend', '--no-pager'],
        ['systemctl', 'list-units', '--all', '--type=service', '--no-legend', '--no-pager'],
    ):
        out = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
        if out.returncode != 0:
            continue
        for line in out.stdout.splitlines():
            name = line.strip().split(None, 1)[0] if line.strip() else ''
            if name.endswith('.service'):
                candidates.add(name)
    owners = set()
    for candidate in candidates:
        show = subprocess.run(
            ['systemctl', 'show', candidate, '-p', 'FragmentPath', '-p', 'ExecStart', '-p', 'WorkingDirectory'],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if candidate == service_name or (show.returncode == 0 and unit_metadata_matches_stack_root(show.stdout)):
            owners.add(candidate)
    return [service_name] + sorted(owner for owner in owners if owner != service_name)

service_names = discover_stack_services()

def quiesce_systemd():
    for owner in service_names:
        mask = subprocess.run(
            ['systemctl', 'mask', '--runtime', owner],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if mask.returncode != 0:
            details = (mask.stderr or mask.stdout or '').strip()
            suffix = f': {details}' if details else f' (exit {mask.returncode})'
            print(f'cleanup failed: systemctl runtime mask failed for {owner}{suffix}', file=sys.stderr)
            raise SystemExit(1)
        subprocess.run(['systemctl', 'stop', owner], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        subprocess.run(
            ['systemctl', 'kill', '--kill-who=all', '--signal=SIGKILL', owner],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        subprocess.run(['systemctl', 'reset-failed', owner], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)

def matching_pids():
    current = os.getpid()
    parent = os.getppid()
    out = subprocess.run(['ps', '-eo', 'pid=,args='], text=True, stdout=subprocess.PIPE, check=False).stdout
    pids = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        raw_pid, _, args = line.partition(' ')
        try:
            pid = int(raw_pid)
        except ValueError:
            continue
        if pid in (current, parent):
            continue
        if any(needle in args for needle in needles):
            pids.append(pid)
    return pids

cleanup_deadline_seconds = float(os.environ.get('CLEANUP_DEADLINE_SECONDS', '10'))
cleanup_quiet_seconds = float(os.environ.get('CLEANUP_QUIET_SECONDS', '2'))
deadline = time.monotonic() + cleanup_deadline_seconds
quiet_since = None
quiet_window_observed = False
while time.monotonic() < deadline:
    quiesce_systemd()
    pids = matching_pids()
    if pids:
        quiet_since = None
        for sig in (signal.SIGTERM, signal.SIGKILL):
            for pid in matching_pids():
                try:
                    os.kill(pid, sig)
                except ProcessLookupError:
                    pass
            time.sleep(0.25)
    else:
        if quiet_since is None:
            quiet_since = time.monotonic()
        if time.monotonic() - quiet_since >= cleanup_quiet_seconds:
            quiet_window_observed = True
            break
        time.sleep(0.25)

quiesce_systemd()
remaining = matching_pids()
if remaining:
    print(f'cleanup failed: stack-root processes remain after SIGKILL: {remaining}', file=sys.stderr)
    raise SystemExit(1)
if not quiet_window_observed:
    print('cleanup failed: stable quiet window was not observed before deadline', file=sys.stderr)
    raise SystemExit(1)
PY
"
}

reset_host() {
  local host=$1
  local control_path=$2
  local service=$3
  cleanup_host_processes "$host" "$control_path" "$service"
  ssh_run "$host" "$control_path" \
    "rm -rf '$STACK_ROOT/data/execution-records' '$STACK_ROOT/data/execution-world' '$STACK_ROOT/data/execution-world-simulator-mirror' '$STACK_ROOT/data/storage' '$STACK_ROOT/data/runtime-root' '$STACK_ROOT/data/replication-root' '$STACK_ROOT/output/chain-runtime' '$STACK_ROOT/output/node-distfs'; mkdir -p '$STACK_ROOT/data/execution-records' '$STACK_ROOT/data/execution-world' '$STACK_ROOT/data/storage' '$STACK_ROOT/data/runtime-root' '$STACK_ROOT/data/replication-root' '$STACK_ROOT/output/chain-runtime' '$STACK_ROOT/output/node-distfs'"
}

start_host() {
  local host=$1
  local control_path=$2
  local service=$3
  ssh_run "$host" "$control_path" "systemctl unmask '$service' || true; systemctl reset-failed '$service' || true; systemctl start '$service'"
}

preflight_host "$SEQUENCER_SSH_HOST" "$SEQUENCER_CONTROL_PATH"
preflight_host "$STORAGE_SSH_HOST" "$STORAGE_CONTROL_PATH"

# Capture a live baseline before the first destructive action. This hard gate
# prevents an already-stopped peer from being taken down as well: every
# staggered transaction starts with both existing validators observable as live.
if ! poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/staggered-preflight-sequencer.json" \
  "sequencer preflight liveness" json_liveness_ok; then
  write_staggered_recovery_receipt "sequencer" "preflight_liveness" \
    "sequencer preflight liveness readback failed" "storage" "not_required"
  die "sequencer preflight liveness failed; no validator was mutated; see $OUT_DIR/staggered-recovery.json"
fi
if ! poll_status_with_check "$STORAGE_STATUS_URL" "$OUT_DIR/staggered-preflight-storage.json" \
  "storage preflight liveness" json_liveness_ok; then
  write_staggered_recovery_receipt "storage" "preflight_liveness" \
    "storage preflight liveness readback failed" "sequencer" "not_required"
  die "storage preflight liveness failed; no validator was mutated; see $OUT_DIR/staggered-recovery.json"
fi

# Mutate one member at a time. storage-205 goes first so the current
# sequencer remains the live producer while storage is rebuilt. Before each
# reset, the other member is read back again; after each start, the rebuilt
# member is read back before the next member may be touched.
for staggered_role in storage sequencer; do
  if ! staggered_readback_peer "$staggered_role"; then
    staggered_abort "$staggered_role" "peer_readback_before_reset" \
      "$(staggered_peer_role "$staggered_role") was not live before ${staggered_role} reset" 0
  fi

  staggered_host=$(staggered_role_host "$staggered_role")
  staggered_control_path=$(staggered_role_control_path "$staggered_role")
  staggered_service=$(staggered_role_service "$staggered_role")
  if ! reset_host "$staggered_host" "$staggered_control_path" "$staggered_service"; then
    staggered_abort "$staggered_role" "reset" \
      "${staggered_role} reset failed" 1
  fi
  if ! stage_host "$staggered_host" "$staggered_control_path" "$staggered_role"; then
    staggered_abort "$staggered_role" "stage" \
      "${staggered_role} stage failed" 1
  fi

  if ! start_host "$staggered_host" "$staggered_control_path" "$staggered_service"; then
    staggered_abort "$staggered_role" "start" \
      "${staggered_role} start failed" 1
  fi
  staggered_status_url=$(staggered_role_status_url "$staggered_role")
  staggered_readiness_check="staggered_${staggered_role}_ready"
  if ! poll_status_with_check "$staggered_status_url" "$OUT_DIR/staggered-${staggered_role}-liveness.json" \
    "${staggered_role} readiness readback" "$staggered_readiness_check"; then
    staggered_abort "$staggered_role" "readiness_readback" \
      "${staggered_role} readiness readback failed" 1
  fi
done

poll_status_with_check "$SEQUENCER_STATUS_URL" "$OUT_DIR/sequencer-status.json" "sequencer readiness" json_sequencer_ok \
  || staggered_abort "sequencer" "final_sequencer_readiness" \
    "sequencer readiness failed after staggered cutover" 1
poll_status_with_check "$STORAGE_STATUS_URL" "$OUT_DIR/storage-status.json" "storage readiness" json_storage_ok \
  || staggered_abort "storage" "final_storage_readiness" \
    "storage readiness failed after staggered cutover" 1

jq -n \
  --arg config_dir "$CONFIG_DIR" \
  --arg world_dir "$WORLD_DIR" \
  --arg stack_root "$STACK_ROOT" \
  --arg sequencer_status_url "$SEQUENCER_STATUS_URL" \
  --arg storage_status_url "$STORAGE_STATUS_URL" \
  --arg sequencer_repair_rebuild_log "$OUT_DIR/sequencer-repair-rebuild.log" \
  --arg storage_repair_rebuild_log "$OUT_DIR/storage-repair-rebuild.log" \
  --argjson staggered_member_order '["storage", "sequencer"]' \
  --slurpfile sequencer "$OUT_DIR/sequencer-status.json" \
  --slurpfile storage "$OUT_DIR/storage-status.json" \
  '
    {
      execution_mode: "staggered",
      staggered_member_order: $staggered_member_order,
      pair_preservation: {
        max_simultaneously_stopped_validators: 1,
        live_peer_readback_before_each_reset: true,
        rebuilt_member_readback_before_next_reset: true
      },
      rollback_boundary: {
        restore_deleted_chain_state: false,
        restore_old_node_state: false,
        failure_action: "target_only_cleanup_and_preserve_live_peer",
        requires_fresh_clean_stage: true,
        validator_47_no_start_unchanged: true
      },
      config_dir: $config_dir,
      world_dir: $world_dir,
      stack_root: $stack_root,
      sequencer_status_url: $sequencer_status_url,
      storage_status_url: $storage_status_url,
      sequencer_repair_rebuild_log: $sequencer_repair_rebuild_log,
      storage_repair_rebuild_log: $storage_repair_rebuild_log,
      sequencer_status: $sequencer[0],
      storage_status: $storage[0]
    }
  ' >"$OUT_DIR/rebuild-summary.json"

jq -n \
  --slurpfile sequencer "$OUT_DIR/sequencer-status.json" \
  --slurpfile storage "$OUT_DIR/storage-status.json" \
  '{
    sequencer: {
      running: $sequencer[0].running,
      last_error: $sequencer[0].last_error,
      committed_height: $sequencer[0].consensus.committed_height,
      last_execution_height: $sequencer[0].consensus.last_execution_height,
      local_peer_id: $sequencer[0].replication.local_peer_id,
      connected_peers: $sequencer[0].replication.connected_peers
    },
    storage: {
      running: $storage[0].running,
      last_error: $storage[0].last_error,
      committed_height: $storage[0].consensus.committed_height,
      last_execution_height: $storage[0].consensus.last_execution_height,
      local_peer_id: $storage[0].replication.local_peer_id,
      connected_peers: $storage[0].replication.connected_peers
    }
  }'
