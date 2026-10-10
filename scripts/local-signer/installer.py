"""Approved release installer API; behavioral implementation follows RED acceptance."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat
from collections.abc import Mapping
from types import MappingProxyType

RELEASE_SCHEMA = "oasis7.local_signer_release.v1"
PLAN_SCHEMA = "oasis7.local_signer_install_plan.v1"
FILES = ("oasis7_local_signer", "oasis7_local_signer_worker", "oasis7_local_signer_admin", "install-release.py", "installer.py", "macos_host.py")
ACTIONS = ("identity", "layout", "release", "binding", "validated", "sudo", "complete")


class InstallError(Exception):
    def __init__(self, code, message, *, completed_actions=()):
        super().__init__(message)
        self.code = code
        self.completed_actions = tuple(completed_actions)


def canonical_bytes(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def package_release(binary_dir, installer_dir, output_dir, *, release_id, target, source_revision):
    check_id(release_id)
    if target not in TARGETS or not re.fullmatch(r"[0-9a-f]{40}", source_revision):
        raise InstallError("INVALID_RELEASE", "invalid release identity")
    output = Path(output_dir)
    if output.exists() or output.is_symlink():
        raise InstallError("INVALID_RELEASE", "output must be new")
    data = {name: read_file(Path(installer_dir if name.endswith('.py') else binary_dir) / name, limit(name)) for name in FILES}
    manifest = dict(schema_version=RELEASE_SCHEMA, release_id=release_id, target=target, source_revision=source_revision, installation_schema_version="oasis7.local_signer_installation.v3", control_schema_version="oasis7.local_signer_control.v1", files=[dict(name=name, size_bytes=len(data[name]), sha256=digest(data[name])) for name in sorted(FILES)])
    output.mkdir(mode=0o755)
    for name, raw in data.items():
        write_new(output / name, raw, 0o555 if not name.endswith('.py') else 0o444)
    raw = canonical_bytes(manifest)
    write_new(output / "manifest.json", raw, 0o444)
    return {"manifest_sha256": digest(raw), "release_dir": str(output)}


def validate_release(release_dir, expected_manifest_sha256, target, *, captured_files=None):
    root = Path(release_dir)
    if captured_files is not None:
        if not isinstance(captured_files, Mapping) or set(captured_files) != set(FILES) | {"manifest.json"} or any(type(data) is not bytes for data in captured_files.values()):
            raise InstallError("INVALID_RELEASE", "invalid closed captured release")
        raw = captured_files["manifest.json"]
    else:
        raw = read_file(root / "manifest.json", 65536)
    if not valid_hash(expected_manifest_sha256) or digest(raw) != expected_manifest_sha256:
        raise InstallError("INVALID_RELEASE", "manifest approval digest mismatch")
    manifest = parse_json(raw)
    fields = {"schema_version", "release_id", "target", "source_revision", "installation_schema_version", "control_schema_version", "files"}
    if set(manifest) != fields or manifest["schema_version"] != RELEASE_SCHEMA or manifest["target"] != target or target not in TARGETS or manifest["installation_schema_version"] != "oasis7.local_signer_installation.v3" or manifest["control_schema_version"] != "oasis7.local_signer_control.v1" or not re.fullmatch(r"[0-9a-f]{40}", str(manifest["source_revision"])):
        raise InstallError("INVALID_RELEASE", "release schema or target mismatch")
    check_id(manifest["release_id"])
    if not isinstance(manifest["files"], list) or len(manifest["files"]) != len(FILES):
        raise InstallError("INVALID_RELEASE", "invalid file inventory")
    verified = {}
    for entry in manifest["files"]:
        if not isinstance(entry, dict) or set(entry) != {"name", "size_bytes", "sha256"}:
            raise InstallError("INVALID_RELEASE", "invalid file entry")
        name = entry["name"]
        if name not in FILES or name in verified or type(entry["size_bytes"]) is not int or not 0 < entry["size_bytes"] <= limit(name) or not valid_hash(entry["sha256"]):
            raise InstallError("INVALID_RELEASE", "invalid file identity or size")
        data = captured_files[name] if captured_files is not None else read_file(root / name, limit(name))
        if len(data) != entry["size_bytes"] or digest(data) != entry["sha256"]:
            raise InstallError("INVALID_RELEASE", "release file digest mismatch")
        verified[name] = data
    if captured_files is None and set(os.listdir(root)) != set(FILES) | {"manifest.json"}:
        raise InstallError("INVALID_RELEASE", "unexpected release members")
    return {"manifest": manifest, "manifest_sha256": expected_manifest_sha256, "verified_bytes": MappingProxyType(verified), "manifest_bytes": raw}


def plan_installation(request, release, backend):
    if set(request) != {"installation_id", "deployment_id", "store_dir", "work_dir", "caller_user", "signer_user"}:
        raise InstallError("INSTALLATION_DRIFT", "unknown plan input")
    for name in ("installation_id", "deployment_id"):
        check_id(request[name], "INSTALLATION_DRIFT")
    for name in ("caller_user", "signer_user"):
        if not isinstance(request[name], str) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_-]{0,31}", request[name]):
            raise InstallError("INSTALLATION_DRIFT", "unsafe account name")
    store, work = clean_path(request["store_dir"]), clean_path(request["work_dir"])
    if store == work or store in work.parents or work in store.parents:
        raise InstallError("INSTALLATION_DRIFT", "store and jobs overlap")
    facts = dict(backend.observe(request, release))
    if facts.get("platform") != "darwin":
        raise InstallError("UNSUPPORTED_PLATFORM_OR_FS", "only macOS apply supported")
    runtime_identity = facts.get("runtime_identity")
    if (not isinstance(runtime_identity, Mapping) or not runtime_identity
            or any(not isinstance(key, str) or not isinstance(value, str) for key, value in runtime_identity.items())):
        raise InstallError("INSTALLATION_DRIFT", "trusted runtime identity is missing")
    facts["runtime_identity"] = dict(runtime_identity)
    if facts.get("target") != release["manifest"]["target"] or not all(facts.get(key) is True for key in ("safe", "acl_safe", "sudo_safe", "identity_available")):
        raise InstallError("INSTALLATION_DRIFT", "host isolation preflight blocked")
    uid, gid, caller = facts.get("signer_uid"), facts.get("signer_gid"), facts.get("caller_uid")
    if not all(type(value) is int and 0 < value < 2**32 for value in (uid, gid, caller)) or caller == uid:
        raise InstallError("INSTALLATION_DRIFT", "invalid identity allocation")
    caller_gid = facts.get("caller_gid")
    if type(caller_gid) is not int or not 0 <= caller_gid < 2**32:
        raise InstallError("INSTALLATION_DRIFT", "missing caller group observation")
    return dict(schema_version=PLAN_SCHEMA, release_id=release["manifest"]["release_id"], manifest_sha256=release["manifest_sha256"], platform="darwin", installation_id=request["installation_id"], deployment_id=request["deployment_id"], store_dir=str(store), caller=dict(name=request["caller_user"], uid=caller, work_dir=str(work)), signer=dict(name=request["signer_user"], uid=uid, gid=gid), observations=[dict(subject="host", facts=facts)], actions=[dict(kind=kind, target=str(store)) for kind in ACTIONS], signing_enabled=False)


def apply_installation(release, plan, expected_plan_sha256, backend):
    if not valid_hash(expected_plan_sha256) or digest(canonical_bytes(plan)) != expected_plan_sha256:
        raise InstallError("INSTALLATION_DRIFT", "plan approval mismatch")
    if plan.get("schema_version") != PLAN_SCHEMA or plan.get("manifest_sha256") != release["manifest_sha256"] or plan.get("release_id") != release["manifest"]["release_id"] or plan.get("signing_enabled") is not False:
        raise InstallError("INSTALLATION_DRIFT", "invalid plan binding")
    request = dict(installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], store_dir=plan["store_dir"], work_dir=plan["caller"]["work_dir"], caller_user=plan["caller"]["name"], signer_user=plan["signer"]["name"])
    facts = dict(backend.observe(request, release))
    try:
        planned_runtime = plan["observations"][0]["facts"]["runtime_identity"]
        current_runtime = facts["runtime_identity"]
        runtime_unchanged = (isinstance(planned_runtime, Mapping) and isinstance(current_runtime, Mapping)
                             and canonical_bytes(dict(planned_runtime)) == canonical_bytes(dict(current_runtime)))
    except (IndexError, KeyError, TypeError, ValueError):
        runtime_unchanged = False
    if not runtime_unchanged:
        raise InstallError("INSTALLATION_DRIFT", "trusted runtime identity changed")
    if facts.get("platform") != "darwin" or facts.get("root") is not True:
        raise InstallError("UNSUPPORTED_PLATFORM_OR_FS", "macOS root apply required")
    completed = []
    with backend.lock():
        previous = backend.completed_installation()
        if previous:
            if previous.get("stage") != "complete":
                return report(plan, expected_plan_sha256, "RECOVERY_REQUIRED", "RECOVERY_REQUIRED", previous.get("completed_actions", []), True)
            if previous.get("plan_sha256") != expected_plan_sha256 or previous.get("manifest_sha256") != release["manifest_sha256"] or not backend.validate_installed(plan, release):
                raise InstallError("INSTALLATION_DRIFT", "completed installation readback drift")
            return report(plan, expected_plan_sha256, "VERIFIED_UNCHANGED", "OK", list(ACTIONS), False)
        if canonical_bytes(plan_installation(request, release, backend)) != canonical_bytes(plan):
            raise InstallError("INSTALLATION_DRIFT", "live plan observations changed")
        record = dict(stage="intent", plan_sha256=expected_plan_sha256, manifest_sha256=release["manifest_sha256"], installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], completed_actions=[])
        try:
            backend.journal(record)
            value = {"plan": plan, "release": release}
            for kind, method in (("identity", "create_identity"), ("layout", "create_layout"), ("release", "publish_release"), ("binding", "publish_binding"), ("validated", "validate_installation"), ("sudo", "publish_sudo"), ("complete", "report")):
                getattr(backend, method)(value)
                completed.append(kind)
                record = dict(record, stage=kind, completed_actions=list(completed))
                backend.journal(record)
            return report(plan, expected_plan_sha256, "INSTALLED_UNREADY", "OK", completed, True)
        except Exception:
            # A failing final fsync must never leave an apparently complete journal.
            try:
                backend.journal(dict(record, stage="recovery", completed_actions=list(completed)))
            except Exception:
                pass
            return report(plan, expected_plan_sha256, "RECOVERY_REQUIRED", "RECOVERY_REQUIRED", completed, True)


def recover_completion(release, plan, expected_plan_sha256, backend, *, expected_journal_sha256=None, check_only=False):
    """Explicit terminal recovery only: never replay installation actions."""
    if (not valid_hash(expected_plan_sha256) or digest(canonical_bytes(plan)) != expected_plan_sha256
            or plan.get("schema_version") != PLAN_SCHEMA or plan.get("signing_enabled") is not False
            or plan.get("manifest_sha256") != release["manifest_sha256"]
            or plan.get("release_id") != release["manifest"]["release_id"]):
        raise InstallError("INSTALLATION_DRIFT", "original recovery plan binding mismatch")
    request = {"installation_id": plan["installation_id"], "deployment_id": plan["deployment_id"],
               "store_dir": plan["store_dir"], "work_dir": plan["caller"]["work_dir"],
               "caller_user": plan["caller"]["name"], "signer_user": plan["signer"]["name"]}
    facts = backend.observe(request, release)
    if (facts.get("platform") != "darwin" or facts.get("root") is not True
            or canonical_bytes(facts.get("runtime_identity")) != canonical_bytes(plan["observations"][0]["facts"]["runtime_identity"])):
        raise InstallError("INSTALLATION_DRIFT", "recovery runtime identity mismatch")
    with backend.lock():
        previous, journal_sha256 = backend.recovery_snapshot()
        if not check_only and (not valid_hash(expected_journal_sha256) or journal_sha256 != expected_journal_sha256):
            raise InstallError("INSTALLATION_DRIFT", "recovery journal preimage mismatch")
        required = {"stage", "completed_actions", "plan_sha256", "manifest_sha256", "installation_id", "deployment_id", "installation_config"}
        if (set(previous) != required or previous["stage"] != "recovery"
                or previous["completed_actions"] != list(ACTIONS[:-1])
                or previous["plan_sha256"] != expected_plan_sha256
                or previous["manifest_sha256"] != release["manifest_sha256"]
                or previous["installation_id"] != plan["installation_id"]
                or previous["deployment_id"] != plan["deployment_id"]):
            raise InstallError("INSTALLATION_DRIFT", "recovery journal scope mismatch")
        if not backend.validate_installed(plan, release) or previous["installation_config"] != backend.installed_config():
            raise InstallError("INSTALLATION_DRIFT", "recovery installed state verification failed")
        if check_only:
            value = report(plan, expected_plan_sha256, "RECOVERY_VERIFIED", "OK", list(ACTIONS[:-1]), False)
            value["journal_sha256"] = journal_sha256
            return value
        # Pin the verified configuration so journal cannot omit its binding.
        backend.config = previous["installation_config"]
        try:
            backend.journal(dict(previous, stage="complete", completed_actions=list(ACTIONS)))
        except Exception:
            # Never report success after an ambiguous fsync; restore recovery if possible.
            try:
                backend.journal(previous)
            except Exception:
                pass
            return report(plan, expected_plan_sha256, "RECOVERY_REQUIRED", "RECOVERY_REQUIRED", list(ACTIONS[:-1]), True)
        return report(plan, expected_plan_sha256, "INSTALLED_UNREADY", "OK", list(ACTIONS), True)


BINDING_REPAIR_SCHEMA = "oasis7.local_signer_binding_repair_plan.v1"
BINDING_REPAIR_REASON = "custody-search-only-directory-open"
BINDING_REPAIR_ACTIONS = ("intent", "sudo_disabled", "release", "binding", "validated", "sudo", "doctor", "complete")


BINDING_REPAIR_PLAN_FIELDS = {"schema_version", "reason", "installation_id", "deployment_id", "original_plan_sha256",
        "original_manifest_sha256", "original_journal_sha256", "original_journal", "original_config_sha256",
        "original_sudo_sha256", "original_sudo_policy_sha256", "runtime_identity", "new_release_id",
        "new_manifest_sha256", "new_installation_config", "new_installation_plan", "actions", "signing_enabled"}

def _repair_baseline(old_release, old_plan, old_sha, new_release, backend):
    """Read and bind the completed original installation; caller holds lock."""
    if (not valid_hash(old_sha) or digest(canonical_bytes(old_plan)) != old_sha
            or old_plan.get("schema_version") != PLAN_SCHEMA or old_plan.get("signing_enabled") is not False
            or old_plan.get("manifest_sha256") != old_release["manifest_sha256"]
            or old_plan.get("release_id") != old_release["manifest"]["release_id"]):
        raise InstallError("INSTALLATION_DRIFT", "original installation approval mismatch")
    new_id = new_release["manifest"]["release_id"]
    check_id(new_id, "INSTALLATION_DRIFT")
    if new_id == old_plan["release_id"] or new_release["manifest"]["target"] != old_release["manifest"]["target"]:
        raise InstallError("INSTALLATION_DRIFT", "repair release must be fresh and target compatible")
    request = dict(installation_id=old_plan["installation_id"], deployment_id=old_plan["deployment_id"],
        store_dir=old_plan["store_dir"], work_dir=old_plan["caller"]["work_dir"],
        caller_user=old_plan["caller"]["name"], signer_user=old_plan["signer"]["name"])
    facts = backend.observe(request, old_release)
    runtime = old_plan["observations"][0]["facts"]["runtime_identity"]
    if (facts.get("platform") != "darwin" or facts.get("root") is not True
            or canonical_bytes(facts.get("runtime_identity")) != canonical_bytes(runtime)):
        raise InstallError("INSTALLATION_DRIFT", "repair runtime changed")
    record, journal_sha = backend.recovery_snapshot()
    fields = {"stage", "completed_actions", "plan_sha256", "manifest_sha256", "installation_id", "deployment_id", "installation_config"}
    if (set(record) != fields or record["stage"] != "complete" or record["completed_actions"] != list(ACTIONS)
            or record["plan_sha256"] != old_sha or record["manifest_sha256"] != old_release["manifest_sha256"]
            or record["installation_id"] != old_plan["installation_id"] or record["deployment_id"] != old_plan["deployment_id"]
            or not valid_hash(journal_sha) or not backend.validate_installed(old_plan, old_release)
            or record["installation_config"] != backend.installed_config()):
        raise InstallError("INSTALLATION_DRIFT", "original completed installation drift")
    preflight = backend.repair_preflight(old_plan, old_release, new_release)
    required = {"safe", "new_release_absent", "repair_absent", "config_sha256", "sudo_sha256", "sudo_policy_sha256", "runtime_identity"}
    if (set(preflight) != required or any(preflight[key] is not True for key in ("safe", "new_release_absent", "repair_absent"))
            or not valid_hash(preflight["config_sha256"]) or not valid_hash(preflight["sudo_sha256"])
            or not valid_hash(preflight["sudo_policy_sha256"]) or preflight["runtime_identity"] != runtime):
        raise InstallError("INSTALLATION_DRIFT", "repair preflight blocked")
    old_config = record["installation_config"]
    if digest(canonical_bytes(old_config)) != preflight["config_sha256"]:
        raise InstallError("INSTALLATION_DRIFT", "original configuration digest mismatch")
    new_config = dict(old_config, release_id=new_id,
        worker_executable="/usr/local/libexec/oasis7-local-signer/" + new_id + "/oasis7_local_signer_worker",
        worker_sha256=digest(new_release["verified_bytes"]["oasis7_local_signer_worker"]))
    new_plan = dict(old_plan, release_id=new_id, manifest_sha256=new_release["manifest_sha256"])
    return dict(schema_version=BINDING_REPAIR_SCHEMA, reason=BINDING_REPAIR_REASON,
        installation_id=old_plan["installation_id"], deployment_id=old_plan["deployment_id"],
        original_plan_sha256=old_sha, original_manifest_sha256=old_release["manifest_sha256"],
        original_journal_sha256=journal_sha, original_journal=record,
        original_config_sha256=preflight["config_sha256"], original_sudo_sha256=preflight["sudo_sha256"], original_sudo_policy_sha256=preflight["sudo_policy_sha256"],
        runtime_identity=dict(runtime), new_release_id=new_id, new_manifest_sha256=new_release["manifest_sha256"],
        new_installation_config=new_config, new_installation_plan=new_plan,
        actions=list(BINDING_REPAIR_ACTIONS), signing_enabled=False)


def plan_binding_repair(old_release, old_plan, old_plan_sha256, new_release, backend):
    """Plan only; the original journal and all installed effects are immutable."""
    with backend.lock():
        return _repair_baseline(old_release, old_plan, old_plan_sha256, new_release, backend)


def apply_binding_repair(old_release, old_plan, new_release, repair_plan, expected_repair_plan_sha256, backend):
    """Single attempt replacement: revoke first; preserve original journal forever."""
    if (not valid_hash(expected_repair_plan_sha256)
            or digest(canonical_bytes(repair_plan)) != expected_repair_plan_sha256):
        raise InstallError("INSTALLATION_DRIFT", "repair approval mismatch")
    with backend.lock():
        baseline = _repair_baseline(old_release, old_plan, digest(canonical_bytes(old_plan)), new_release, backend)
        if canonical_bytes(baseline) != canonical_bytes(repair_plan):
            raise InstallError("INSTALLATION_DRIFT", "repair approved observations changed")
        completed = []
        receipt = dict(schema_version="oasis7.local_signer_binding_repair_receipt.v1",
            repair_plan_sha256=expected_repair_plan_sha256, original_journal_sha256=baseline["original_journal_sha256"],
            original_manifest_sha256=baseline["original_manifest_sha256"],
            new_manifest_sha256=baseline["new_manifest_sha256"], installation_id=baseline["installation_id"],
            deployment_id=baseline["deployment_id"], stage="intent", completed_actions=[])
        new_plan = baseline["new_installation_plan"]
        value = {"plan": new_plan, "release": new_release, "repair": True}
        try:
            backend.repair_receipt(receipt)
            completed.append("intent")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            backend.disable_sudo(old_plan)
            completed.append("sudo_disabled")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            backend.publish_release(value)
            completed.append("release")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            backend.repair_binding(new_plan, new_release, baseline["new_installation_config"])
            completed.append("binding")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            if not backend.validate_repaired(new_plan, new_release, baseline["original_journal"]["installation_config"], check_sudo=False):
                raise InstallError("INSTALLATION_DRIFT", "repair disabled-state verification failed")
            completed.append("validated")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            backend.repair_publish_sudo(value)
            completed.append("sudo")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            if not backend.validate_repaired(new_plan, new_release, baseline["original_journal"]["installation_config"], check_sudo=True):
                raise InstallError("INSTALLATION_DRIFT", "repair installed-state verification failed")
            doctor_result = backend.verify_caller_doctor(new_plan)
            if not _valid_doctor_result(doctor_result):
                raise InstallError("INSTALLATION_DRIFT", "repair caller denial verification failed")
            receipt["doctor_result"] = dict(doctor_result)
            completed.append("doctor")
            backend.repair_receipt(dict(receipt, completed_actions=list(completed)))
            backend.repair_receipt(dict(receipt, stage="complete", completed_actions=list(BINDING_REPAIR_ACTIONS)))
            completed.append("complete")
        except Exception:
            # Even an ambiguous intent/complete fsync must revoke rather than retry.
            try:
                backend.disable_sudo(old_plan)
            except Exception:
                pass
            try:
                backend.repair_receipt(dict(receipt, stage="recovery", completed_actions=list(completed)))
            except Exception:
                pass
            return _repair_report(baseline, expected_repair_plan_sha256, completed, "RECOVERY_REQUIRED")
        return _repair_report(baseline, expected_repair_plan_sha256, completed, "BOUND_REPAIRED_UNREADY")


def _valid_doctor_result(value):
    return (isinstance(value, dict) and set(value) == {"exit_code", "stdout_sha256", "stderr_sha256"}
            and type(value["exit_code"]) is int and value["exit_code"] == 3
            and valid_hash(value["stdout_sha256"]) and valid_hash(value["stderr_sha256"]))


def quarantine_binding_repair(old_plan, repair_plan, expected_repair_plan_sha256, expected_receipt_sha256, backend):
    """Explicit no-replay containment of interrupted repair; never roll forward."""

    if (not isinstance(repair_plan, dict) or set(repair_plan) != BINDING_REPAIR_PLAN_FIELDS
            or not valid_hash(expected_repair_plan_sha256) or digest(canonical_bytes(repair_plan)) != expected_repair_plan_sha256
            or not valid_hash(expected_receipt_sha256) or repair_plan["schema_version"] != BINDING_REPAIR_SCHEMA
            or repair_plan["reason"] != BINDING_REPAIR_REASON or repair_plan["signing_enabled"] is not False
            or repair_plan["actions"] != list(BINDING_REPAIR_ACTIONS)
            or repair_plan["original_plan_sha256"] != digest(canonical_bytes(old_plan))):
        raise InstallError("INSTALLATION_DRIFT", "quarantine approval binding mismatch")
    with backend.lock():
        _repair_environment(repair_plan, backend)
        record, receipt_sha256 = backend.repair_snapshot()
        fields = {"schema_version", "repair_plan_sha256", "original_journal_sha256", "original_manifest_sha256",
            "new_manifest_sha256", "installation_id", "deployment_id", "stage", "completed_actions"}
        completed = record.get("completed_actions") if isinstance(record, dict) else None
        has_doctor = isinstance(completed, list) and "doctor" in completed
        if has_doctor:
            fields.add("doctor_result")
        if (receipt_sha256 != expected_receipt_sha256 or not isinstance(record, dict) or set(record) != fields
                or record["schema_version"] != "oasis7.local_signer_binding_repair_receipt.v1"
                or record["stage"] not in ("intent", "recovery") or not isinstance(completed, list)
                or len(completed) >= len(BINDING_REPAIR_ACTIONS)
                or completed != list(BINDING_REPAIR_ACTIONS[:len(completed)])
                or has_doctor and not _valid_doctor_result(record["doctor_result"])
                or record["repair_plan_sha256"] != expected_repair_plan_sha256
                or any(record[name] != repair_plan[name] for name in ("original_journal_sha256", "original_manifest_sha256", "new_manifest_sha256", "installation_id", "deployment_id"))):
            raise InstallError("INSTALLATION_DRIFT", "quarantine receipt scope mismatch")
        original, journal_sha256 = backend.recovery_snapshot()
        if journal_sha256 != repair_plan["original_journal_sha256"] or original != repair_plan["original_journal"]:
            raise InstallError("INSTALLATION_DRIFT", "quarantine original journal changed")
        if backend.quarantine_preflight(repair_plan) is not True:
            raise InstallError("INSTALLATION_DRIFT", "quarantine host containment preflight blocked")
        mutated = False
        try:
            # Attempted revocation can mutate even if its durable readback fails.
            mutated = True
            backend.disable_sudo(old_plan)
            backend.repair_receipt(dict(record, stage="quarantined"))
        except Exception:
            result = _repair_report(repair_plan, expected_repair_plan_sha256, completed, "RECOVERY_REQUIRED")
            result["host_mutated"] = mutated
            return result
        result = _repair_report(repair_plan, expected_repair_plan_sha256, completed, "QUARANTINED_UNREADY")
        result["code"] = "OK"
        result["remaining_actions"] = []
        result["repair_receipt_sha256"] = digest(canonical_bytes(dict(record, stage="quarantined")))
        return result


def _repair_environment(plan, backend):
    facts = backend.repair_environment()
    if (facts.get("platform") != "darwin" or facts.get("root") is not True
            or facts.get("runtime_identity") != plan["runtime_identity"]):
        raise InstallError("INSTALLATION_DRIFT", "repair verification runtime mismatch")


def verify_binding_repair(new_release, repair_plan, expected_repair_plan_sha256, backend):
    """Read-only consumer of completed repair evidence, never historical apply."""
    if (not isinstance(repair_plan, dict) or set(repair_plan) != BINDING_REPAIR_PLAN_FIELDS
            or not valid_hash(expected_repair_plan_sha256) or digest(canonical_bytes(repair_plan)) != expected_repair_plan_sha256
            or repair_plan.get("schema_version") != BINDING_REPAIR_SCHEMA
            or repair_plan.get("reason") != BINDING_REPAIR_REASON or repair_plan.get("signing_enabled") is not False
            or repair_plan.get("new_manifest_sha256") != new_release["manifest_sha256"]
            or repair_plan.get("new_release_id") != new_release["manifest"]["release_id"]):
        raise InstallError("INSTALLATION_DRIFT", "repaired verification approval mismatch")
    with backend.lock():
        _repair_environment(repair_plan, backend)
        record, receipt_sha256 = backend.repair_snapshot()
        fields = {"schema_version", "repair_plan_sha256", "original_journal_sha256", "original_manifest_sha256",
            "new_manifest_sha256", "installation_id", "deployment_id", "stage", "completed_actions", "doctor_result"}
        if (not isinstance(record, dict) or set(record) != fields or not valid_hash(receipt_sha256)
                or record["schema_version"] != "oasis7.local_signer_binding_repair_receipt.v1"
                or record["stage"] != "complete" or record["completed_actions"] != list(BINDING_REPAIR_ACTIONS)
                or record["repair_plan_sha256"] != expected_repair_plan_sha256
                or not _valid_doctor_result(record["doctor_result"])
                or any(record[name] != repair_plan[name] for name in ("original_journal_sha256", "original_manifest_sha256", "new_manifest_sha256", "installation_id", "deployment_id"))):
            raise InstallError("INSTALLATION_DRIFT", "repaired completion receipt scope mismatch")
        original, journal_sha256 = backend.recovery_snapshot()
        if journal_sha256 != repair_plan["original_journal_sha256"] or original != repair_plan["original_journal"]:
            raise InstallError("INSTALLATION_DRIFT", "repaired original journal changed")
        new_plan = repair_plan["new_installation_plan"]
        if not backend.validate_repaired(new_plan, new_release, original["installation_config"], check_sudo=True):
            raise InstallError("INSTALLATION_DRIFT", "repaired installed state drift")
        doctor_result = backend.verify_caller_doctor(new_plan)
        if not _valid_doctor_result(doctor_result) or doctor_result != record["doctor_result"]:
            raise InstallError("INSTALLATION_DRIFT", "repaired caller denial evidence drift")
        result = _repair_report(repair_plan, expected_repair_plan_sha256, list(BINDING_REPAIR_ACTIONS), "REPAIRED_VERIFIED_UNREADY")
        result.update(code="OK", host_mutated=False, repair_receipt_sha256=receipt_sha256)
        return result


def _repair_report(plan, sha, completed, status):
    return dict(schema_version="oasis7.local_signer_binding_repair_report.v1", status=status,
        code="OK" if status == "BOUND_REPAIRED_UNREADY" else "RECOVERY_REQUIRED",
        installation_id=plan["installation_id"], deployment_id=plan["deployment_id"],
        release_id=plan["new_release_id"], manifest_sha256=plan["new_manifest_sha256"],
        repair_plan_sha256=sha, original_journal_sha256=plan["original_journal_sha256"],
        host_mutated=True, signing_enabled=False, completed_actions=list(completed),
        remaining_actions=[kind for kind in BINDING_REPAIR_ACTIONS if kind not in completed])


TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")


def valid_hash(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def check_id(value, code="INVALID_RELEASE"):
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,127}", value):
        raise InstallError(code, "unsafe identifier")


def clean_path(value):
    if not isinstance(value, str) or not value.startswith("/") or any(part in (".", "..", "") for part in value.split("/")[1:]) or "\x00" in value:
        raise InstallError("INSTALLATION_DRIFT", "noncanonical absolute path")
    return Path(value)


def parse_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON field")
            result[key] = value
        return result
    try:
        value = json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
        if not isinstance(value, dict):
            raise ValueError("object required")
        return value
    except (ValueError, UnicodeError) as error:
        raise InstallError("INVALID_RELEASE", "invalid JSON") from error


def limit(name):
    return 1024 * 1024 if name.endswith(".py") else 128 * 1024 * 1024


def read_file(path, maximum):
    """Retain parent FDs: hash the same regular-file bytes whose metadata was checked."""
    path = Path(path).absolute()
    fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY)
    try:
        for component in path.parts[1:-1]:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        file_fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=fd)
        try:
            before = os.fstat(file_fd)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or not 0 < before.st_size <= maximum:
                raise InstallError("INVALID_RELEASE", "unsafe or oversized regular file")
            chunks, size = [], 0
            while True:
                chunk = os.read(file_fd, min(65536, maximum + 1 - size))
                if not chunk:
                    break
                chunks.append(chunk)
                size += len(chunk)
                if size > maximum:
                    raise InstallError("INVALID_RELEASE", "oversized file")
            after = os.fstat(file_fd)
            if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
                raise InstallError("INVALID_RELEASE", "file changed during read")
            return b"".join(chunks)
        finally:
            os.close(file_fd)
    except OSError as error:
        raise InstallError("INVALID_RELEASE", "unsafe or unreadable release file") from error
    finally:
        os.close(fd)


def write_new(path, data, mode):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    try:
        with os.fdopen(fd, "wb", closefd=False) as stream:
            stream.write(data)
            stream.flush()
            os.fsync(fd)
    finally:
        os.close(fd)
    parent = os.open(Path(path).parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(parent)
    finally:
        os.close(parent)


def report(plan, sha, status, code, completed, mutated):
    return dict(schema_version="oasis7.local_signer_install_report.v1", status=status, code=code, installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], release_id=plan["release_id"], manifest_sha256=plan["manifest_sha256"], plan_sha256=sha, host_mutated=mutated, signing_enabled=False, completed_actions=list(completed), remaining_actions=[kind for kind in ACTIONS if kind not in completed])


def build_installation_config(plan, release, store, work):
    return dict(schema_version="oasis7.local_signer_installation.v3", installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], store_dir=plan["store_dir"], store_device_id=store.st_dev, store_inode=store.st_ino, signer_uid=plan["signer"]["uid"], signer_gid=plan["signer"]["gid"], callers=[dict(uid=plan["caller"]["uid"], work_dir=plan["caller"]["work_dir"], work_device_id=work.st_dev, work_inode=work.st_ino)], release_id=plan["release_id"], worker_executable="/usr/local/libexec/oasis7-local-signer/" + plan["release_id"] + "/oasis7_local_signer_worker", worker_sha256=digest(release["verified_bytes"]["oasis7_local_signer_worker"]), control_schema_version="oasis7.local_signer_control.v1")
