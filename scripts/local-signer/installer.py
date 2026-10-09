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
