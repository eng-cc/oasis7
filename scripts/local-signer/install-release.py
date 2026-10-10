#!/usr/bin/python3 -I
"""Fixed release installation CLI. No fixture backend selector."""
import argparse
from collections.abc import Mapping
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import sys
import types

RUNTIME_PATH = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"

TRUSTED_BOOTSTRAP = r"""import ctypes
import ctypes.util
import errno
import fcntl
import hashlib
import json
import os
import platform
import re
import stat
import sys
from types import MappingProxyType

FILES = (
    "oasis7_local_signer",
    "oasis7_local_signer_worker",
    "oasis7_local_signer_admin",
    "install-release.py",
    "installer.py",
    "macos_host.py",
)
MANIFEST_NAME = "manifest.json"
LIMITS = {name: (1024 * 1024 if name.endswith(".py") else 128 * 1024 * 1024) for name in FILES}
LIMITS[MANIFEST_NAME] = 65536
MANIFEST_FIELDS = {
    "schema_version", "release_id", "target", "source_revision",
    "installation_schema_version", "control_schema_version", "files",
}
SEAM_NAMES = ("_bootstrap_acl_query", "_bootstrap_metadata", "_bootstrap_execute", "_bootstrap_target")
RUNTIME_PATH = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"
FRAMEWORK_PATH = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9"
RUNTIME_ATTESTATION_FIELDS = (
    "apple_anchor", "bootstrap_sha256", "dependency_policy_id", "framework_cdhash",
    "framework_dev", "framework_ino", "framework_mode", "framework_path",
    "framework_resource_seal", "framework_uid", "os_build", "policy_id",
    "requirement_id", "runtime_arch", "runtime_cdhash", "runtime_dev",
    "runtime_ino", "runtime_mode", "runtime_path", "runtime_uid",
    "runtime_version", "schema_version", "system_volume_trust",
)
RUNTIME_FIXED = {
    "apple_anchor": "apple",
    "dependency_policy_id": "clt-python39-apple-dyld-v1",
    "framework_cdhash": "a43551195b8d2eefd9356d81c1098ffc9e0a8b47",
    "framework_path": FRAMEWORK_PATH,
    "framework_resource_seal": "valid",
    "policy_id": "clt-python3.9-v1",
    "requirement_id": "com.apple.python3",
    "runtime_cdhash": "77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf",
    "runtime_path": RUNTIME_PATH,
    "runtime_version": "3.9",
    "schema_version": "oasis7.local-signer.runtime-attestation.v1",
    "system_volume_trust": "current-booted-apple-os",
}


def pairs(items):
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate JSON field")
        result[key] = value
    return result


def option(name):
    found = [index for index, value in enumerate(sys.argv[:-1]) if value == name]
    if len(found) != 1:
        raise ValueError("missing or duplicate operator argument")
    return sys.argv[found[0] + 1]


def valid_hash(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def read_runtime_attestation(seams):
    if seams:
        provider = globals().get("_bootstrap_runtime_attestation")
        if not callable(provider):
            raise ValueError("native runtime attestation is missing")
        raw = provider()
        if type(raw) is not bytes:
            raise ValueError("native runtime attestation test evidence is malformed")
    else:
        descriptor = 3
        try:
            info = os.fstat(descriptor)
            flags = fcntl.fcntl(descriptor, fcntl.F_GETFL)
            if (not stat.S_ISFIFO(info.st_mode) or info.st_uid != os.geteuid()
                    or flags & os.O_ACCMODE != os.O_RDONLY):
                raise ValueError("native runtime attestation descriptor is unsafe")
            chunks = []
            size = 0
            while size <= 4096:
                chunk = os.read(descriptor, min(4097 - size, 1024))
                if not chunk:
                    break
                chunks.append(chunk)
                size += len(chunk)
            raw = b"".join(chunks)
            if size > 4096:
                raise ValueError("native runtime attestation exceeds its bound")
        finally:
            os.close(descriptor)
    if not raw or len(raw) > 4096 or not raw.endswith(b"\n"):
        raise ValueError("native runtime attestation framing is invalid")
    try:
        value = json.loads(raw.decode("ascii"), object_pairs_hook=pairs,
                           parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
    except (UnicodeError, ValueError) as error:
        raise ValueError("native runtime attestation JSON is invalid") from error
    if (type(value) is not dict or tuple(sorted(value)) != RUNTIME_ATTESTATION_FIELDS
            or set(value) != set(RUNTIME_ATTESTATION_FIELDS)
            or any(type(item) is not str for item in value.values())):
        raise ValueError("native runtime attestation schema is invalid")
    canonical = (json.dumps(value, sort_keys=True, separators=(",", ":"),
                            ensure_ascii=True, allow_nan=False) + "\n").encode("ascii")
    if raw != canonical:
        raise ValueError("native runtime attestation is not canonical")
    for name, expected in RUNTIME_FIXED.items():
        if value[name] != expected:
            raise ValueError("native runtime attestation fixed identity mismatch")
    for name in ("bootstrap_sha256",):
        if not re.fullmatch(r"[0-9a-f]{64}", value[name]):
            raise ValueError("native runtime attestation digest is malformed")
    for name in ("runtime_cdhash", "framework_cdhash"):
        if not re.fullmatch(r"[0-9a-f]{40}", value[name]):
            raise ValueError("native runtime attestation code identity is malformed")
    if value["runtime_arch"] not in ("arm64", "x86_64"):
        raise ValueError("native runtime attestation architecture is unsupported")
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,128}", value["os_build"]):
        raise ValueError("native runtime attestation OS build is malformed")
    for prefix, expected_path, directory in (("runtime", RUNTIME_PATH, False), ("framework", FRAMEWORK_PATH, True)):
        if value[prefix + "_path"] != expected_path or value[prefix + "_uid"] != "0":
            raise ValueError("native runtime attestation path or owner mismatch")
        for suffix in ("dev", "ino", "mode"):
            number = value[prefix + "_" + suffix]
            if not re.fullmatch(r"(?:0|[1-9][0-9]{0,19})", number):
                raise ValueError("native runtime attestation metadata is noncanonical")
        if value[prefix + "_dev"] == "0" or value[prefix + "_ino"] == "0":
            raise ValueError("native runtime attestation metadata is empty")
        mode = int(value[prefix + "_mode"])
        if mode < 0 or mode & (stat.S_ISUID | stat.S_ISGID | stat.S_ISVTX) or mode & 0o022:
            raise ValueError("native runtime attestation mode is unsafe")
        if directory:
            if not stat.S_ISDIR(mode) or mode & 0o500 != 0o500:
                raise ValueError("native framework attestation is not a directory")
        elif not stat.S_ISREG(mode) or not mode & 0o100:
            raise ValueError("native runtime attestation is not an executable regular file")
    if value["framework_mode"] == "0" or value["runtime_mode"] == "0":
        raise ValueError("native runtime attestation mode is empty")
    if not seams:
        if (sys.platform != "darwin" or sys.executable != RUNTIME_PATH
                or tuple(sys.version_info[:2]) != (3, 9)
                or not sys.flags.isolated or not sys.flags.no_site
                or not sys.flags.dont_write_bytecode):
            raise ValueError("approved isolated CLT Python runtime is required")
        _verify_runtime_names(value)
        for entry in sys.path:
            if not entry or not os.path.isabs(entry):
                raise ValueError("runtime import path is not absolute")
            resolved = os.path.realpath(entry)
            if resolved != FRAMEWORK_PATH and not resolved.startswith(FRAMEWORK_PATH + os.sep):
                raise ValueError("runtime import path escapes the approved framework")
    return MappingProxyType(value)


def _verify_runtime_names(value):
    flags = os.O_RDONLY | os.O_NOFOLLOW
    if hasattr(os, "O_DIRECTORY"):
        framework_fd = os.open(FRAMEWORK_PATH, flags | os.O_DIRECTORY)
    else:
        raise ValueError("native runtime directory descriptor check is unavailable")
    try:
        runtime_fd = os.open(RUNTIME_PATH, flags)
        try:
            for prefix, fd in (("framework", framework_fd), ("runtime", runtime_fd)):
                info = os.fstat(fd)
                mode = str(info.st_mode)
                if (str(info.st_dev) != value[prefix + "_dev"]
                        or str(info.st_ino) != value[prefix + "_ino"]
                        or str(info.st_uid) != value[prefix + "_uid"]
                        or mode != value[prefix + "_mode"]):
                    raise ValueError("native runtime descriptor identity changed")
        finally:
            os.close(runtime_fd)
    finally:
        os.close(framework_fd)


def acl_empty(fd, seams):
    if seams:
        observed = _bootstrap_acl_query(fd)
        if type(observed) not in (tuple, list) or observed:
            raise ValueError("ACL evidence is malformed or contains entries")
        return
    if sys.platform != "darwin":
        raise ValueError("native Darwin descriptor ACL query is unavailable")
    library = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
    class AttrList(ctypes.Structure):
        _fields_ = (
            ("bitmapcount", ctypes.c_uint16),
            ("reserved", ctypes.c_uint16),
            ("commonattr", ctypes.c_uint32),
            ("volattr", ctypes.c_uint32),
            ("dirattr", ctypes.c_uint32),
            ("fileattr", ctypes.c_uint32),
            ("forkattr", ctypes.c_uint32),
        )

    get_attrs = getattr(library, "fgetattrlist", None)
    if get_attrs is None:
        raise ValueError("native same-descriptor extended-security query is unavailable")
    get_attrs.argtypes = (
        ctypes.c_int,
        ctypes.POINTER(AttrList),
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_ulong,
    )
    get_attrs.restype = ctypes.c_int

    def query(request, capacity, description):
        output = ctypes.create_string_buffer(capacity)
        ctypes.set_errno(0)
        result = get_attrs(fd, ctypes.byref(request), output, capacity, 0x00000004)  # FSOPT_REPORT_FULLSIZE
        if result != 0:
            code = ctypes.get_errno() or errno.EIO
            raise OSError(code, description)
        raw = output.raw
        if len(raw) < 4:
            raise ValueError("native attribute response is truncated")
        size = int.from_bytes(raw[:4], sys.byteorder)
        if size < 4 or size > capacity:
            raise ValueError("native attribute response length is invalid")
        return raw[:size]

    # An omitted per-file ACL is meaningful only when this volume reports
    # extended-security support in both the valid and capability masks.
    volume_request = AttrList(5, 0, 0, 0x80020000, 0, 0, 0)  # volattr: ATTR_VOL_INFO | ATTR_VOL_CAPABILITIES
    volume = query(volume_request, 64, "cannot read descriptor volume capabilities")
    if len(volume) != 36:
        raise ValueError("volume capability response has an unexpected size")
    capabilities = tuple(
        int.from_bytes(volume[4 + index * 4:8 + index * 4], sys.byteorder)
        for index in range(4)
    )
    valid = tuple(
        int.from_bytes(volume[20 + index * 4:24 + index * 4], sys.byteorder)
        for index in range(4)
    )
    extended_security = 0x00000400  # VOL_CAP_INT_EXTENDED_SECURITY
    if not valid[1] & extended_security or not capabilities[1] & extended_security:
        raise ValueError("descriptor volume does not prove extended-security support")

    # RETURNED_ATTRS precedes the requested attrreference.  XNU's zero-filled
    # fixed record for an absent ACL is exactly 32 bytes: length, attribute_set,
    # and an all-zero eight-byte attrreference.
    security_request = AttrList(5, 0, 0x80400000, 0, 0, 0, 0)  # RETURNED_ATTRS | EXTENDED_SECURITY
    security = query(security_request, 8192, "cannot read descriptor extended security")
    if len(security) < 32:
        raise ValueError("extended-security response is truncated")
    returned = tuple(
        int.from_bytes(security[4 + index * 4:8 + index * 4], sys.byteorder)
        for index in range(5)
    )
    returned_common, returned_volume, returned_directory, returned_file, returned_fork = returned
    returned_attrs = 0x80000000  # ATTR_CMN_RETURNED_ATTRS
    extended_security_attr = 0x00400000  # ATTR_CMN_EXTENDED_SECURITY
    if not returned_common & returned_attrs or any(returned[1:]):
        raise ValueError("extended-security returned-attribute bitmap is malformed")
    allowed_common = returned_attrs | extended_security_attr
    if returned_common & ~allowed_common:
        raise ValueError("extended-security returned unexpected attributes")
    reference = security[24:32]
    present = bool(returned_common & extended_security_attr)
    if not present:
        if returned_common != returned_attrs or len(security) != 32 or reference != b"\0" * 8:
            raise ValueError("omitted ACL response has malformed reference framing")
        return

    data_offset = int.from_bytes(reference[:4], sys.byteorder, signed=True)
    data_length = int.from_bytes(reference[4:], sys.byteorder)
    data_start = 24 + data_offset
    if data_offset != 8 or data_length < 44 or data_start != 32:
        raise ValueError("extended-security attribute reference is malformed")
    if data_length > len(security) - data_start or data_start + data_length != len(security):
        raise ValueError("extended-security filesec bounds are invalid")
    filesec = security[data_start:]
    magic = int.from_bytes(filesec[:4], sys.byteorder)
    entry_count = int.from_bytes(filesec[36:40], sys.byteorder)
    acl_flags = int.from_bytes(filesec[40:44], sys.byteorder)
    if magic != 0x012CC16D or entry_count != 0xFFFFFFFF or data_length != 44 or acl_flags != 0:
        raise ValueError("extended-security filesec is malformed or contains an ACL")


def metadata(fd, seams):
    return _bootstrap_metadata(fd) if seams else os.fstat(fd)


def signature(info):
    return tuple(getattr(info, name) for name in (
        "st_dev", "st_ino", "st_uid", "st_gid", "st_mode", "st_nlink",
        "st_size", "st_mtime_ns", "st_ctime_ns",
    ))


def safe_directory(fd, seams):
    info = metadata(fd, seams)
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
        raise ValueError("unsafe release ancestor metadata")
    acl_empty(fd, seams)
    return info


def directory_chain(path, seams):
    if not isinstance(path, str) or not path.startswith("/") or "\x00" in path:
        raise ValueError("release path must be absolute")
    parts = path.split("/")[1:]
    if not parts or any(part in ("", ".", "..") for part in parts):
        raise ValueError("release path is not canonical")
    if "/" + "/".join(parts) != path:
        raise ValueError("release path is not canonical")
    if not seams and path.rsplit("/", 1)[0] != "/private/var/db/oasis7-local-signer-approved":
        raise ValueError("root release must be in the protected approved staging parent")
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    root_fd = os.open("/", flags)
    opened = [root_fd]
    records = [(None, None, root_fd, safe_directory(root_fd, seams))]
    parent = root_fd
    try:
        for part in parts:
            child = os.open(part, flags, dir_fd=parent)
            opened.append(child)
            info = safe_directory(child, seams)
            named = os.stat(part, dir_fd=parent, follow_symlinks=False)
            if (named.st_dev, named.st_ino) != (info.st_dev, info.st_ino):
                raise ValueError("release ancestor changed during admission")
            records.append((parent, part, child, info))
            parent = child
        return parent, opened, records
    except BaseException:
        for descriptor in reversed(opened):
            os.close(descriptor)
        raise


def recheck_directories(records, seams):
    for parent, name, fd, before in records:
        after = metadata(fd, seams)
        if signature(before) != signature(after):
            raise ValueError("release ancestor metadata changed during capture")
        acl_empty(fd, seams)
        if parent is not None:
            named = os.stat(name, dir_fd=parent, follow_symlinks=False)
            if (named.st_dev, named.st_ino) != (after.st_dev, after.st_ino):
                raise ValueError("release ancestor name changed during capture")


def capture_member(directory_fd, name, maximum, seams, expected_size=None):
    fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory_fd)
    try:
        before = metadata(fd, seams)
        if (not stat.S_ISREG(before.st_mode) or before.st_uid != 0 or before.st_nlink != 1
                or before.st_mode & 0o022 or not 0 < before.st_size <= maximum
                or expected_size is not None and before.st_size != expected_size):
            raise ValueError("unsafe release member metadata")
        acl_empty(fd, seams)
        chunks = []
        remaining = min(maximum, before.st_size) + 1
        while remaining:
            chunk = os.read(fd, min(65536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
        after = metadata(fd, seams)
        acl_empty(fd, seams)
        if signature(before) != signature(after) or len(data) != before.st_size:
            raise ValueError("release member changed during descriptor capture")
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (named.st_dev, named.st_ino) != (after.st_dev, after.st_ino):
            raise ValueError("release member name changed during descriptor capture")
        return data
    finally:
        os.close(fd)


def validate_manifest(raw, expected_digest, release_id, target):
    if not valid_hash(expected_digest) or hashlib.sha256(raw).hexdigest() != expected_digest:
        raise ValueError("independent manifest approval mismatch")
    manifest = json.loads(raw.decode("utf-8"), object_pairs_hook=pairs,
                          parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
    if not isinstance(manifest, dict) or set(manifest) != MANIFEST_FIELDS:
        raise ValueError("invalid closed manifest schema")
    if (manifest.get("schema_version") != "oasis7.local_signer_release.v1"
            or manifest.get("target") != target
            or target not in ("aarch64-apple-darwin", "x86_64-apple-darwin")
            or manifest.get("installation_schema_version") != "oasis7.local_signer_installation.v3"
            or manifest.get("control_schema_version") != "oasis7.local_signer_control.v1"
            or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,127}", str(manifest.get("release_id", "")))
            or manifest.get("release_id") != release_id
            or not re.fullmatch(r"[0-9a-f]{40}", str(manifest.get("source_revision", "")))):
        raise ValueError("manifest identity or target mismatch")
    entries = manifest.get("files")
    if not isinstance(entries, list) or len(entries) != len(FILES):
        raise ValueError("manifest file inventory is incomplete")
    expected_names = sorted(FILES)
    names = []
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {"name", "size_bytes", "sha256"}:
            raise ValueError("invalid manifest file entry")
        name, size, file_hash = entry["name"], entry["size_bytes"], entry["sha256"]
        if (not isinstance(name, str) or type(size) is not int or not 0 < size <= LIMITS.get(name, 0)
                or not valid_hash(file_hash)):
            raise ValueError("invalid manifest file identity")
        names.append(name)
    if names != expected_names:
        raise ValueError("manifest inventory is not the closed sorted package")
    return manifest, {entry["name"]: entry for entry in entries}


def invoke():
    seam_flags = tuple(name in globals() for name in SEAM_NAMES)
    if any(seam_flags) and not all(seam_flags):
        raise ValueError("partial bootstrap test seam")
    seams = all(seam_flags)
    if seams:
        if (not callable(_bootstrap_acl_query) or not callable(_bootstrap_metadata)
                or not callable(_bootstrap_execute) or _bootstrap_target not in ("aarch64-apple-darwin", "x86_64-apple-darwin")):
            raise ValueError("invalid importer-only bootstrap test seam")
    runtime_identity = read_runtime_attestation(seams)
    if len(sys.argv) < 2 or sys.argv[1] not in ("plan", "apply"):
        raise ValueError("unsupported installer operation")
    operation = sys.argv[1]
    if not seams and operation == "apply" and os.geteuid() != 0:
        raise ValueError("root apply required")
    stage_path = option("--release-dir")
    expected_manifest = option("--expected-manifest-sha256")
    release_id = stage_path.rsplit("/", 1)[-1]
    if not release_id:
        raise ValueError("release identity is missing")
    target = _bootstrap_target if seams else {
        "arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin",
    }.get(platform.machine(), "unsupported")
    directory_fd, opened, records = directory_chain(stage_path, seams)
    try:
        allowed = set(FILES) | {MANIFEST_NAME}
        if set(os.listdir(directory_fd)) != allowed:
            raise ValueError("approved stage has unknown or missing members")
        manifest_bytes = capture_member(directory_fd, MANIFEST_NAME, LIMITS[MANIFEST_NAME], seams)
        manifest, entries = validate_manifest(manifest_bytes, expected_manifest, release_id, target)
        captured = {MANIFEST_NAME: manifest_bytes}
        for name in FILES:
            entry = entries[name]
            captured[name] = capture_member(directory_fd, name, LIMITS[name], seams, entry["size_bytes"])
            if hashlib.sha256(captured[name]).hexdigest() != entry["sha256"]:
                raise ValueError("release member digest mismatch")
        recheck_directories(records, seams)
        if set(os.listdir(directory_fd)) != allowed:
            raise ValueError("approved stage inventory changed during capture")
        capsule = MappingProxyType(captured)
        after_capture = globals().get("_bootstrap_after_capture")
        if seams and after_capture is not None:
            if not callable(after_capture):
                raise ValueError("invalid post-capture test seam")
            after_capture()
        if seams:
            _bootstrap_execute(capsule["install-release.py"], capsule)
        else:
            namespace = {
                "__name__": "__main__",
                "__file__": "<approved-captured-install-release.py>",
                "_CAPTURED_RELEASE_FILES": capsule,
                "_RUNTIME_ATTESTATION": runtime_identity,
            }
            exec(compile(capsule["install-release.py"], namespace["__file__"], "exec"), namespace)
    finally:
        for descriptor in reversed(opened):
            os.close(descriptor)


try:
    invoke()
except SystemExit:
    raise
except Exception as error:
    if all(name in globals() for name in SEAM_NAMES):
        raise
    print(json.dumps({"status": "BLOCKED", "code": "TRUSTED_BOOTSTRAP_REQUIRED",
                      "host_mutated": False, "signing_enabled": False},
                     sort_keys=True, separators=(",", ":")))
    raise SystemExit(9) from error"""


def parser():
    result = argparse.ArgumentParser()
    commands = result.add_subparsers(dest="command", required=True)
    plan = commands.add_parser("plan")
    apply = commands.add_parser("apply")
    for command in (plan, apply):
        command.add_argument("--release-dir", required=True)
        command.add_argument("--expected-manifest-sha256", required=True)
    for name in ("store-dir", "work-dir", "caller-user", "signer-user", "installation-id", "deployment-id", "plan-out"):
        plan.add_argument("--" + name, required=True)
    apply.add_argument("--plan", required=True)
    apply.add_argument("--expected-plan-sha256", required=True)
    recovery = apply.add_mutually_exclusive_group()
    recovery.add_argument("--completion-only", action="store_true")
    recovery.add_argument("--completion-check", action="store_true")
    recovery.add_argument("--binding-repair-plan-out")
    recovery.add_argument("--binding-repair-apply")
    recovery.add_argument("--binding-repair-check")
    recovery.add_argument("--binding-repair-quarantine")
    apply.add_argument("--expected-repair-receipt-sha256")
    apply.add_argument("--expected-repair-plan-sha256")
    apply.add_argument("--installed-release-dir")
    apply.add_argument("--expected-installed-manifest-sha256")
    apply.add_argument("--expected-journal-sha256")
    return result


def approved_modules(expected_manifest_sha256, captured_files=None):
    if (not sys.flags.isolated or not sys.flags.no_site or not sys.flags.dont_write_bytecode
            or Path(sys.executable) != Path(RUNTIME_PATH) or tuple(sys.version_info[:2]) != (3, 9)):
        raise ValueError("fixed isolated CLT Python 3.9 runtime required")
    if captured_files is None:
        root = Path(__file__).absolute().parent
        if os.geteuid() == 0:
            raise ValueError("TRUSTED_BOOTSTRAP_REQUIRED")
        current = Path("/")
        for part in root.parts[1:]:
            current /= part
            info = current.lstat()
            if stat.S_ISLNK(info.st_mode):
                raise ValueError("unsafe approved module ancestor")
        fd = os.open(root / "manifest.json", os.O_RDONLY | os.O_NOFOLLOW)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or not 0 < info.st_size <= 65536:
                raise ValueError("unsafe approved manifest")
            raw = os.read(fd, 65537)
        finally:
            os.close(fd)
    else:
        if not isinstance(captured_files, Mapping) or set(captured_files) != set(("manifest.json", "install-release.py", "installer.py", "macos_host.py", "oasis7_local_signer", "oasis7_local_signer_worker", "oasis7_local_signer_admin")) or any(type(data) is not bytes for data in captured_files.values()):
            raise ValueError("invalid immutable captured release")
        raw = captured_files["manifest.json"]
    if hashlib.sha256(raw).hexdigest() != expected_manifest_sha256:
        raise ValueError("independent manifest approval mismatch before imports")
    def unique_pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate manifest field")
            result[key] = value
        return result
    manifest = json.loads(raw, object_pairs_hook=unique_pairs)
    if not isinstance(manifest, dict) or set(manifest) != {"schema_version", "release_id", "target", "source_revision", "installation_schema_version", "control_schema_version", "files"} or manifest.get("schema_version") != "oasis7.local_signer_release.v1":
        raise ValueError("invalid closed release manifest before imports")
    entries = {entry["name"]: entry for entry in manifest["files"]}
    if len(entries) != len(manifest["files"]) or set(entries) != {"oasis7_local_signer", "oasis7_local_signer_worker", "oasis7_local_signer_admin", "install-release.py", "installer.py", "macos_host.py"}:
        raise ValueError("duplicate release member")
    if captured_files is None and root.name != manifest["release_id"]:
        raise ValueError("staging release identity mismatch")
    for name in ("installer", "macos_host"):
        if captured_files is None:
            fd = os.open(root / (name + ".py"), os.O_RDONLY | os.O_NOFOLLOW)
            try:
                info = os.fstat(fd)
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > 1024 * 1024:
                    raise ValueError("unsafe approved module")
                module_raw = os.read(fd, 1024 * 1024 + 1)
            finally:
                os.close(fd)
        else:
            module_raw = captured_files[name + ".py"]
        entry = entries[name + ".py"]
        if (not isinstance(entry, dict) or set(entry) != {"name", "size_bytes", "sha256"}
                or len(module_raw) != entry["size_bytes"] or hashlib.sha256(module_raw).hexdigest() != entry["sha256"]):
            raise ValueError("approved module byte mismatch before imports")
        module = types.ModuleType(name)
        module.__file__ = str(root / (name + ".py")) if captured_files is None else "<captured-" + name + ".py>"
        sys.modules[name] = module
        exec(compile(module_raw, module.__file__, "exec"), module.__dict__)
    return sys.modules["installer"], sys.modules["macos_host"]


def original_repair_inputs(args, api, host_module, host, plan, target):
    if os.geteuid() != 0 or not args.installed_release_dir or not args.expected_installed_manifest_sha256:
        raise api.InstallError("INSTALLATION_DRIFT", "explicit root repair inputs required")
    original_root = host_module.RELEASE_ROOT / plan["release_id"]
    if Path(args.installed_release_dir) != original_root:
        raise api.InstallError("INSTALLATION_DRIFT", "fixed original installed release path required")
    protected_plan(Path(args.plan), host, api)
    for name in (*api.FILES, "manifest.json"):
        host.inspect_path(original_root / name, protected=True)
        info = (original_root / name).lstat()
        mode = 0o444 if name.endswith(".py") or name == "manifest.json" else 0o555
        if (info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode), info.st_nlink) != (0, 0, mode, 1):
            raise api.InstallError("INSTALLATION_DRIFT", "unsafe original installed release")
    return api.validate_release(original_root, args.expected_installed_manifest_sha256, target)


def protected_plan(path, host, api):
    host.inspect_path(path, protected=True)
    info = path.lstat()
    if (info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode), info.st_nlink) != (0, 0, 0o600, 1):
        raise api.InstallError("INSTALLATION_DRIFT", "unsafe protected plan")


def main():
    captured_files = globals().get("_CAPTURED_RELEASE_FILES")
    runtime_identity = globals().get("_RUNTIME_ATTESTATION")
    if os.geteuid() == 0 and not isinstance(captured_files, types.MappingProxyType):
        print(json.dumps({"status": "BLOCKED", "code": "TRUSTED_BOOTSTRAP_REQUIRED", "host_mutated": False, "signing_enabled": False}, sort_keys=True))
        return 9
    if not isinstance(runtime_identity, Mapping):
        print(json.dumps({"status": "BLOCKED", "code": "TRUSTED_BOOTSTRAP_REQUIRED", "host_mutated": False, "signing_enabled": False}, sort_keys=True))
        return 9
    args = parser().parse_args()
    try:
        api, host_module = approved_modules(args.expected_manifest_sha256, captured_files)
        target = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}.get(platform.machine(), "unsupported")
        release = api.validate_release(args.release_dir, args.expected_manifest_sha256, target, captured_files=captured_files)
        host = host_module.MacOSHost(runtime_identity=runtime_identity)
        if args.command == "plan":
            request = {name: getattr(args, name) for name in ("store_dir", "work_dir", "caller_user", "signer_user", "installation_id", "deployment_id")}
            plan = api.plan_installation(request, release, host)
            api.write_new(Path(args.plan_out), api.canonical_bytes(plan), 0o600)
            output = {"status": "PLANNED", "plan_sha256": api.digest(api.canonical_bytes(plan)), "host_mutated": False, "signing_enabled": False}
        else:
            plan = api.parse_json(api.read_file(args.plan, 1024 * 1024))
            if args.binding_repair_plan_out or args.binding_repair_apply or args.binding_repair_check or args.binding_repair_quarantine:
                if args.expected_journal_sha256:
                    raise api.InstallError("INSTALLATION_DRIFT", "completion input in binding repair")
                original = original_repair_inputs(args, api, host_module, host, plan, target)
                if args.binding_repair_plan_out:
                    if args.expected_repair_plan_sha256 or args.expected_repair_receipt_sha256:
                        raise api.InstallError("INSTALLATION_DRIFT", "repair apply digest in planning")
                    repair = api.plan_binding_repair(original, plan, args.expected_plan_sha256, release, host)
                    destination = Path(args.binding_repair_plan_out)
                    if destination.parent != Path(args.plan).parent:
                        raise api.InstallError("INSTALLATION_DRIFT", "repair plan must use protected original plan directory")
                    host.inspect_path(destination.parent, protected=True)
                    api.write_new(destination, api.canonical_bytes(repair), 0o600)
                    output = dict(status="BINDING_REPAIR_PLANNED", code="OK", host_mutated=False,
                                  signing_enabled=False, repair_plan_sha256=api.digest(api.canonical_bytes(repair)),
                                  manifest_sha256=release["manifest_sha256"], original_manifest_sha256=original["manifest_sha256"])
                else:
                    path = Path(args.binding_repair_apply or args.binding_repair_check or args.binding_repair_quarantine)
                    protected_plan(path, host, api)
                    repair = api.parse_json(api.read_file(path, 1024 * 1024))
                    if args.binding_repair_quarantine:
                        output = api.quarantine_binding_repair(plan, repair, args.expected_repair_plan_sha256, args.expected_repair_receipt_sha256, host)
                    elif args.binding_repair_check:
                        if args.expected_repair_receipt_sha256:
                            raise api.InstallError("INSTALLATION_DRIFT", "quarantine digest in readonly verification")
                        output = api.verify_binding_repair(release, repair, args.expected_repair_plan_sha256, host)
                    else:
                        if args.expected_repair_receipt_sha256:
                            raise api.InstallError("INSTALLATION_DRIFT", "quarantine digest in repair apply")
                        output = api.apply_binding_repair(original, plan, release, repair, args.expected_repair_plan_sha256, host)
            elif args.completion_only or args.completion_check:
                if args.expected_repair_plan_sha256 or args.expected_repair_receipt_sha256:
                    raise api.InstallError("INSTALLATION_DRIFT", "binding repair input in completion mode")
                if os.geteuid() != 0 or not args.installed_release_dir or not args.expected_installed_manifest_sha256:
                    raise api.InstallError("INSTALLATION_DRIFT", "explicit root recovery inputs required")
                original_root = host_module.RELEASE_ROOT / plan["release_id"]
                if Path(args.installed_release_dir) != original_root:
                    raise api.InstallError("INSTALLATION_DRIFT", "fixed installed release path required")
                host.inspect_path(Path(args.plan), protected=True)
                info = Path(args.plan).lstat()
                if (info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode), info.st_nlink) != (0, 0, 0o600, 1):
                    raise api.InstallError("INSTALLATION_DRIFT", "unsafe original recovery plan")
                for name in (*api.FILES, "manifest.json"):
                    host.inspect_path(original_root / name, protected=True)
                    info = (original_root / name).lstat()
                    mode = 0o444 if name.endswith(".py") or name == "manifest.json" else 0o555
                    if (info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode), info.st_nlink) != (0, 0, mode, 1):
                        raise api.InstallError("INSTALLATION_DRIFT", "unsafe original installed release")
                original = api.validate_release(original_root, args.expected_installed_manifest_sha256, target)
                output = api.recover_completion(original, plan, args.expected_plan_sha256, host,
                    expected_journal_sha256=args.expected_journal_sha256, check_only=args.completion_check)
                output["recovery_manifest_sha256"] = release["manifest_sha256"]
            else:
                if args.installed_release_dir or args.expected_installed_manifest_sha256 or args.expected_journal_sha256 or args.expected_repair_plan_sha256 or args.expected_repair_receipt_sha256:
                    raise api.InstallError("INSTALLATION_DRIFT", "recovery inputs require explicit recovery mode")
                output = api.apply_installation(release, plan, args.expected_plan_sha256, host)
        print(json.dumps(output, sort_keys=True, separators=(",", ":")))
        return 10 if output["status"] == "RECOVERY_REQUIRED" else 0
    except Exception as error:
        print(json.dumps({"status": "BLOCKED", "code": getattr(error, "code", "INSTALLATION_DRIFT"), "host_mutated": False, "signing_enabled": False}, sort_keys=True))
        return 9


if __name__ == "__main__":
    sys.exit(main())
