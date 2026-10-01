#!/usr/bin/python3 -I
"""Fixed release installation CLI. No fixture backend selector."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import sys
import types


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
    return result


def approved_modules(expected_manifest_sha256):
    if not sys.flags.isolated or Path(sys.executable) != Path("/usr/bin/python3"):
        raise ValueError("fixed isolated /usr/bin/python3 required")
    root = Path(__file__).absolute().parent
    if os.geteuid() == 0 and (root.parent != Path("/private/var/db/oasis7-local-signer-approved") or not root.name):
        raise ValueError("root execution requires approved fixed release directory")
    current = Path("/")
    for part in root.parts[1:]:
        current /= part
        info = current.lstat()
        if stat.S_ISLNK(info.st_mode) or (os.geteuid() == 0 and (info.st_uid != 0 or info.st_mode & 0o022)):
            raise ValueError("unsafe approved module ancestor")
    fd = os.open(root / "manifest.json", os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or not 0 < info.st_size <= 65536 or (os.geteuid() == 0 and (info.st_uid != 0 or info.st_mode & 0o022)):
            raise ValueError("unsafe approved manifest")
        raw = os.read(fd, 65537)
    finally:
        os.close(fd)
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
    entries = {entry["name"]: entry for entry in manifest["files"]}
    if len(entries) != len(manifest["files"]):
        raise ValueError("duplicate release member")
    if os.geteuid() == 0 and root.name != manifest["release_id"]:
        raise ValueError("staging release identity mismatch")
    for name in ("installer", "macos_host"):
        fd = os.open(root / (name + ".py"), os.O_RDONLY | os.O_NOFOLLOW)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > 1024 * 1024 or (os.geteuid() == 0 and (info.st_uid != 0 or info.st_mode & 0o022)):
                raise ValueError("unsafe approved module")
            raw = os.read(fd, 1024 * 1024 + 1)
        finally:
            os.close(fd)
        entry = entries[name + ".py"]
        if len(raw) != entry["size_bytes"] or hashlib.sha256(raw).hexdigest() != entry["sha256"]:
            raise ValueError("approved module byte mismatch before imports")
        module = types.ModuleType(name)
        module.__file__ = str(root / (name + ".py"))
        sys.modules[name] = module
        exec(compile(raw, module.__file__, "exec"), module.__dict__)
    return sys.modules["installer"], sys.modules["macos_host"]


def main():
    args = parser().parse_args()
    try:
        api, host_module = approved_modules(args.expected_manifest_sha256)
        target = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}.get(platform.machine(), "unsupported")
        release = api.validate_release(args.release_dir, args.expected_manifest_sha256, target)
        host = host_module.MacOSHost()
        if args.command == "plan":
            request = {name: getattr(args, name) for name in ("store_dir", "work_dir", "caller_user", "signer_user", "installation_id", "deployment_id")}
            plan = api.plan_installation(request, release, host)
            api.write_new(Path(args.plan_out), api.canonical_bytes(plan), 0o600)
            output = {"status": "PLANNED", "plan_sha256": api.digest(api.canonical_bytes(plan)), "host_mutated": False, "signing_enabled": False}
        else:
            plan = api.parse_json(api.read_file(args.plan, 1024 * 1024))
            output = api.apply_installation(release, plan, args.expected_plan_sha256, host)
        print(json.dumps(output, sort_keys=True, separators=(",", ":")))
        return 10 if output["status"] == "RECOVERY_REQUIRED" else 0
    except Exception as error:
        print(json.dumps({"status": "BLOCKED", "code": getattr(error, "code", "INSTALLATION_DRIFT"), "host_mutated": False, "signing_enabled": False}, sort_keys=True))
        return 9


if __name__ == "__main__":
    sys.exit(main())
