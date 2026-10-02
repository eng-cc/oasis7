"""Production-only macOS backend; never selectable as a fixture via CLI/environment."""
from installer import InstallError, canonical_bytes, digest, read_file, write_new, parse_json, ACTIONS, build_installation_config
from contextlib import contextmanager
import fcntl
import os
from pathlib import Path
import platform
import pwd
import re
import stat
import subprocess
import sys
from collections.abc import Mapping

CONFIG = Path("/private/etc/oasis7/local-signer-installation.json")
RELEASE_ROOT = Path("/usr/local/libexec/oasis7-local-signer")
JOURNAL = Path("/private/var/db/oasis7-local-signer-install.json")
SUDO = Path("/private/etc/sudoers.d/oasis7-local-signer")
ENV = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "C", "LC_ALL": "C"}
RUNTIME_PATH = Path("/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9")


class MacOSHost:
    def __init__(self, *, runtime_identity):
        if (not isinstance(runtime_identity, Mapping) or not runtime_identity
                or any(not isinstance(key, str) or not isinstance(value, str) for key, value in runtime_identity.items())):
            raise InstallError("UNSUPPORTED_PLATFORM_OR_FS", "trusted runtime identity is missing")
        self.runtime_identity = dict(runtime_identity)
        self.config = None
        self.receipt = None

    def observe(self, request, release):
        target = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}.get(platform.machine(), "unsupported")
        facts = dict(platform=sys.platform, target=target, root=os.geteuid() == 0, safe=False, acl_safe=False, sudo_safe=False, identity_available=False, runtime_identity=dict(self.runtime_identity))
        if sys.platform != "darwin":
            return facts
        self.validate_interpreter(self.runtime_identity)
        caller = pwd.getpwnam(request["caller_user"])
        if caller.pw_uid == 0 or request["signer_user"] != "_oasis7_signer":
            return facts
        users = parse_ids(self.run(["/usr/bin/dscl", ".", "-list", "/Users", "UniqueID"]))
        groups = parse_ids(self.run(["/usr/bin/dscl", ".", "-list", "/Groups", "PrimaryGroupID"]))
        existing = request["signer_user"] in users or request["signer_user"] in groups
        previous = self.completed_installation()
        if existing and not previous:
            return facts
        available = [value for value in range(400, 500) if value not in users.values() and value not in groups.values() and value != caller.pw_uid]
        if not available and not existing:
            return facts
        uid = users.get(request["signer_user"], available[0] if available else 0)
        gid = groups.get(request["signer_user"], available[0] if available else 0)
        facts.update(caller_uid=caller.pw_uid, caller_gid=caller.pw_gid, signer_uid=uid, signer_gid=gid, identity_available=not existing)
        paths = [Path(request["store_dir"]), CONFIG, RELEASE_ROOT / release["manifest"]["release_id"], SUDO, JOURNAL]
        identities = []
        for path in paths:
            identities.extend(self.inspect_path(path, protected=True))
        work = Path(request["work_dir"])
        identities.extend(self.inspect_path(work, protected=False))
        if work.exists():
            info = work.lstat()
            if info.st_uid != caller.pw_uid or stat.S_IMODE(info.st_mode) != 0o700 or not stat.S_ISDIR(info.st_mode):
                return facts
        elif not work.parent.is_dir() or work.parent.lstat().st_uid != caller.pw_uid:
            return facts
        mounts = self.run(["/sbin/mount"])
        if not local_mounts_only(mounts):
            return facts
        policy = self.run(["/usr/bin/sudo", "-n", "-l", "-U", request["caller_user"]], allow_failure=True)
        self.run(["/usr/sbin/visudo", "-c"])
        worker = str(RELEASE_ROOT / release["manifest"]["release_id"] / "oasis7_local_signer_worker")
        facts.update(safe=True, acl_safe=True, sudo_safe=sudo_policy_safe(policy, uid, gid, worker), identities=identities, sudo_policy_sha256=digest(policy.encode()), mounts_sha256=digest(mounts.encode()))
        if not previous and any(path.exists() for path in (Path(request["store_dir"]), CONFIG, RELEASE_ROOT / release["manifest"]["release_id"], SUDO)):
            facts["safe"] = False
        return facts

    @staticmethod
    def run(argv, *, allow_failure=False):
        if not argv or argv[0] not in {"/usr/bin/dscl", "/usr/bin/dscacheutil", "/usr/bin/sudo", "/usr/sbin/visudo", "/bin/ls", "/sbin/mount"}:
            raise InstallError("INSTALLATION_DRIFT", "unapproved host executable")
        try:
            result = subprocess.run(argv, env=ENV, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=30, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise InstallError("INSTALLATION_DRIFT", "host tool unavailable or timed out") from error
        if len(result.stdout) > 1024 * 1024 or len(result.stderr) > 65536 or (result.returncode and not allow_failure):
            raise InstallError("INSTALLATION_DRIFT", "host tool rejected operation")
        if result.returncode and allow_failure:
            return "UNOBSERVABLE"
        return result.stdout

    @staticmethod
    def validate_interpreter(runtime_identity):
        executable = Path(sys.executable)
        if (executable != RUNTIME_PATH or tuple(sys.version_info[:2]) != (3, 9)
                or not sys.flags.isolated or not sys.flags.no_site or not sys.flags.dont_write_bytecode):
            raise InstallError("UNSUPPORTED_PLATFORM_OR_FS", "fixed isolated CLT Python 3.9 runtime required")
        info = executable.lstat()
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022
                or str(info.st_dev) != runtime_identity.get("runtime_dev")
                or str(info.st_ino) != runtime_identity.get("runtime_ino")
                or str(info.st_mode) != runtime_identity.get("runtime_mode")
                or runtime_identity.get("runtime_cdhash") != "77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf"):
            raise InstallError("UNSUPPORTED_PLATFORM_OR_FS", "unprotected interpreter")

    def inspect_path(self, path, *, protected):
        observations = []
        current = Path("/")
        for part in path.parts[1:]:
            current /= part
            if not current.exists() and not current.is_symlink():
                observations.append(dict(path=str(current), absent=True))
                break
            info = current.lstat()
            if stat.S_ISLNK(info.st_mode) or (protected and (info.st_uid != 0 or info.st_mode & 0o022)):
                raise InstallError("INSTALLATION_DRIFT", "unsafe path ancestor")
            acl = self.run(["/bin/ls", "-lde", str(current)])
            if not acl_safe(acl):
                raise InstallError("INSTALLATION_DRIFT", "ACL present or unobservable")
            observations.append(dict(path=str(current), dev=info.st_dev, ino=info.st_ino, uid=info.st_uid, gid=info.st_gid, mode=stat.S_IMODE(info.st_mode), acl_sha256=digest(acl.encode())))
        return observations

    @contextmanager
    def lock(self):
        path = Path("/private/var/run/oasis7-local-signer-install.lock")
        self.inspect_path(path.parent, protected=True)
        fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        try:
            info = os.fstat(fd)
            if info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o600 or info.st_nlink != 1:
                raise InstallError("INSTALLATION_DRIFT", "unsafe installer lock")
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            yield
        finally:
            os.close(fd)

    def completed_installation(self):
        if not JOURNAL.exists():
            return None
        self.inspect_path(JOURNAL, protected=True)
        self.receipt = parse_json(read_file(JOURNAL, 1024 * 1024))
        return self.receipt

    def journal(self, record):
        if self.config is not None:
            record = dict(record, installation_config=self.config)
        self.atomic_root_file(JOURNAL, canonical_bytes(record), 0o600, replace=True)

    def create_identity(self, value):
        identity = value["plan"]["signer"]
        # Recheck absence immediately before creation; a partial account is never reused.
        users = parse_ids(self.run(["/usr/bin/dscl", ".", "-list", "/Users", "UniqueID"]))
        groups = parse_ids(self.run(["/usr/bin/dscl", ".", "-list", "/Groups", "PrimaryGroupID"]))
        if identity["name"] in users or identity["name"] in groups or identity["uid"] in users.values() or identity["gid"] in groups.values():
            raise InstallError("INSTALLATION_DRIFT", "identity allocation changed")
        for argv in identity_commands(identity):
            self.run(argv)
        self.run(["/usr/bin/dscacheutil", "-flushcache"])
        self.verify_identity(identity)

    def verify_identity(self, identity):
        for record, key, expected in (("Users", "UniqueID", str(identity["uid"])), ("Users", "PrimaryGroupID", str(identity["gid"])), ("Users", "UserShell", "/usr/bin/false"), ("Users", "NFSHomeDirectory", "/var/empty"), ("Groups", "PrimaryGroupID", str(identity["gid"]))):
            output = self.run(["/usr/bin/dscl", ".", "-read", "/" + record + "/" + identity["name"], key]).strip()
            if output != key + ": " + expected:
                raise InstallError("INSTALLATION_DRIFT", "signer identity drift")

    def make_dir(self, path, uid, gid, mode):
        path = Path(path)
        if not path.parent.exists():
            self.make_dir(path.parent, 0, 0, 0o755)
        self.inspect_path(path.parent, protected=False)
        info = path.parent.lstat()
        if info.st_uid not in (0, uid) or info.st_mode & 0o022:
            raise InstallError("INSTALLATION_DRIFT", "unsafe layout parent")
        parent = self.open_dir(path.parent)
        try:
            os.mkdir(path.name, mode=mode, dir_fd=parent)
            fd = os.open(path.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
            try:
                os.fchown(fd, uid, gid)
                os.fchmod(fd, mode)
                os.fsync(fd)
                os.fsync(parent)
            finally:
                os.close(fd)
        finally:
            os.close(parent)

    @staticmethod
    def open_dir(path):
        fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY)
        try:
            for part in Path(path).parts[1:]:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
                os.close(fd)
                fd = child
            return fd
        except BaseException:
            os.close(fd)
            raise

    def create_layout(self, value):
        plan = value["plan"]
        for path, uid, gid, mode in layout(plan):
            self.make_dir(path, uid, gid, mode)
        work = Path(plan["caller"]["work_dir"])
        if not work.exists():
            parent = self.open_dir(work.parent)
            try:
                os.mkdir(work.name, 0o700, dir_fd=parent)
                fd = os.open(work.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
                try:
                    os.fchown(fd, plan["caller"]["uid"], plan["observations"][0]["facts"]["caller_gid"])
                    os.fchmod(fd, 0o700)
                    os.fsync(fd)
                    os.fsync(parent)
                finally:
                    os.close(fd)
            finally:
                os.close(parent)

    def publish_release(self, value):
        path = RELEASE_ROOT / value["plan"]["release_id"]
        self.make_dir(path, 0, 0, 0o755)
        for name, raw in value["release"]["verified_bytes"].items():
            self.atomic_root_file(path / name, raw, 0o444 if name.endswith(".py") else 0o555)
        self.atomic_root_file(path / "manifest.json", value["release"]["manifest_bytes"], 0o444)

    def publish_binding(self, value):
        plan, release = value["plan"], value["release"]
        store_fd, work_fd = self.open_dir(plan["store_dir"]), self.open_dir(plan["caller"]["work_dir"])
        try:
            store, work = os.fstat(store_fd), os.fstat(work_fd)
            self.config = build_installation_config(plan, release, store, work)
        finally:
            os.close(store_fd)
            os.close(work_fd)
        self.atomic_root_file(CONFIG, canonical_bytes(self.config), 0o644)

    def validate_installation(self, value):
        if not self.validate_installed(value["plan"], value["release"], check_sudo=False):
            raise InstallError("INSTALLATION_DRIFT", "installed readback mismatch")

    def validate_installed(self, plan, release, check_sudo=True):
        try:
            self.verify_identity(plan["signer"])
            self.inspect_path(CONFIG, protected=True)
            config = parse_json(read_file(CONFIG, 65536))
            if self.receipt and self.receipt.get("stage") == "complete" and self.receipt.get("installation_config") != config:
                return False
            if config["installation_id"] != plan["installation_id"] or config["deployment_id"] != plan["deployment_id"] or config["release_id"] != plan["release_id"] or config["signer_uid"] != plan["signer"]["uid"] or config["signer_gid"] != plan["signer"]["gid"]:
                return False
            for path, uid, gid, mode in layout(plan):
                info = Path(path).lstat()
                if not stat.S_ISDIR(info.st_mode) or (info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode)) != (uid, gid, mode) or not acl_safe(self.run(["/bin/ls", "-lde", str(path)])):
                    return False
            store, work = Path(plan["store_dir"]).lstat(), Path(plan["caller"]["work_dir"]).lstat()
            if (store.st_dev, store.st_ino) != (config["store_device_id"], config["store_inode"]) or config["callers"] != [dict(uid=plan["caller"]["uid"], work_dir=plan["caller"]["work_dir"], work_device_id=work.st_dev, work_inode=work.st_ino)] or work.st_uid != plan["caller"]["uid"] or stat.S_IMODE(work.st_mode) != 0o700:
                return False
            expected = build_installation_config(plan, release, store, work)
            if config != expected or stat.S_IMODE(CONFIG.lstat().st_mode) != 0o644 or CONFIG.lstat().st_nlink != 1 or not stat.S_ISDIR(work.st_mode) or not acl_safe(self.run(["/bin/ls", "-lde", str(plan["caller"]["work_dir"])])):
                return False
            for name in ("keys", "state/records", "control/grants", "control/revoked-grants", "work", "backup-staging"):
                if os.listdir(Path(plan["store_dir"]) / name):
                    return False
            if (Path(plan["store_dir"]) / "control/policy.json").exists():
                return False
            path = RELEASE_ROOT / plan["release_id"]
            for name, expected in release["verified_bytes"].items():
                self.inspect_path(path / name, protected=True)
                if read_file(path / name, len(expected)) != expected:
                    return False
                if stat.S_IMODE((path / name).lstat().st_mode) != (0o444 if name.endswith(".py") else 0o555):
                    return False
            if check_sudo:
                self.inspect_path(SUDO, protected=True)
                if (SUDO.lstat().st_uid, SUDO.lstat().st_gid, stat.S_IMODE(SUDO.lstat().st_mode), SUDO.lstat().st_nlink) != (0, 0, 0o440, 1):
                    return False
                if read_file(SUDO, 65536) != sudo_rule(plan).encode():
                    return False
                self.run(["/usr/sbin/visudo", "-c"])
                if not sudo_policy_safe(self.run(["/usr/bin/sudo", "-n", "-l", "-U", plan["caller"]["name"]]), plan["signer"]["uid"], plan["signer"]["gid"], str(path / "oasis7_local_signer_worker"), require_worker=True):
                    return False
            return True
        except (OSError, KeyError, InstallError):
            return False

    def publish_sudo(self, value):
        candidate = SUDO.with_name(".oasis7-local-signer-approved-" + str(os.getpid()))
        raw = sudo_rule(value["plan"]).encode()
        self.atomic_root_file(candidate, raw, 0o440)
        self.run(["/usr/sbin/visudo", "-c", "-f", str(candidate)])
        self.atomic_root_file(SUDO, raw, 0o440)
        parent = self.open_dir(candidate.parent)
        try:
            os.unlink(candidate.name, dir_fd=parent)
            os.fsync(parent)
        finally:
            os.close(parent)
        self.run(["/usr/sbin/visudo", "-c"])

    def report(self, value):
        if not self.validate_installed(value["plan"], value["release"]):
            raise InstallError("INSTALLATION_DRIFT", "final installation verification failed")

    def atomic_root_file(self, path, raw, mode, replace=False):
        path = Path(path)
        if not path.parent.exists():
            self.make_dir(path.parent, 0, 0, 0o755)
        self.inspect_path(path.parent, protected=True)
        parent = self.open_dir(path.parent)
        temporary = "." + path.name + ".installer-" + str(os.getpid())
        try:
            if not replace:
                try:
                    os.stat(path.name, dir_fd=parent, follow_symlinks=False)
                    raise InstallError("INSTALLATION_DRIFT", "existing destination")
                except FileNotFoundError:
                    pass
            fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode, dir_fd=parent)
            try:
                os.fchown(fd, 0, 0)
                os.fchmod(fd, mode)
                with os.fdopen(fd, "wb", closefd=False) as stream:
                    stream.write(raw)
                    stream.flush()
                    os.fsync(fd)
            finally:
                os.close(fd)
            if replace:
                os.rename(temporary, path.name, src_dir_fd=parent, dst_dir_fd=parent)
            else:
                os.link(temporary, path.name, src_dir_fd=parent, dst_dir_fd=parent, follow_symlinks=False)
                os.unlink(temporary, dir_fd=parent)
            os.fsync(parent)
        finally:
            os.close(parent)


def parse_ids(output):
    result = {}
    for line in output.splitlines():
        words = line.split()
        if len(words) != 2 or not words[1].isdigit() or words[0] in result:
            raise InstallError("INSTALLATION_DRIFT", "unobservable directory identities")
        result[words[0]] = int(words[1])
    return result


def acl_safe(output):
    lines = output.splitlines()
    return bool(lines) and len(lines) == 1 and not lines[0].split()[0].endswith("+")


def local_mounts_only(output):
    # Conservative host-wide gate: any network volume requires a separate supported contract.
    return bool(output.strip()) and all("(apfs," in line or "(hfs," in line or "(devfs," in line for line in output.splitlines())


def sudo_policy_safe(output, uid, gid, worker, require_worker=False):
    if not output.strip() or any(token in output for token in ("UNOBSERVABLE", "!authenticate", "exempt_group", "!env_reset", "env_keep", "setenv")) or "SETENV:" in output.replace("NOSETENV:", ""):
        return False
    observed_worker = False
    for line in output.splitlines():
        if "NOPASSWD:" in line:
            match = re.fullmatch(r"\(\s*(#[0-9]+|_oasis7_signer)\s*:\s*(#[0-9]+|_oasis7_signer)\s*\)\s*(NOPASSWD: NOSETENV:|NOSETENV: NOPASSWD:)\s*(.+)", line.strip())
            if not match or match.group(1) not in (f"#{uid}", "_oasis7_signer") or match.group(2) not in (f"#{gid}", "_oasis7_signer") or match.group(4) != worker + ' ""':
                return False
            observed_worker = True
    return "may run the following commands" in output and (observed_worker or not require_worker)


def sudo_rule(plan):
    worker = RELEASE_ROOT / plan["release_id"] / "oasis7_local_signer_worker"
    return f"#{plan['caller']['uid']} ALL=(#{plan['signer']['uid']}:#{plan['signer']['gid']}) NOPASSWD: NOSETENV: {worker} \"\"\n"


def identity_commands(identity):
    user, group = "/Users/" + identity["name"], "/Groups/" + identity["name"]
    return [["/usr/bin/dscl", ".", "-create", group], ["/usr/bin/dscl", ".", "-create", group, "PrimaryGroupID", str(identity["gid"])]] + [["/usr/bin/dscl", ".", "-create", user, key, value] for key, value in (("UniqueID", str(identity["uid"])), ("PrimaryGroupID", str(identity["gid"])), ("UserShell", "/usr/bin/false"), ("NFSHomeDirectory", "/var/empty"), ("Password", "*"), ("IsHidden", "1"))]


def layout(plan):
    root = Path(plan["store_dir"])
    uid, gid = plan["signer"]["uid"], plan["signer"]["gid"]
    return [(root / name if name else root, owner, group, mode) for name, owner, group, mode in (("", 0, 0, 0o711), ("control", 0, gid, 0o750), ("control/grants", 0, gid, 0o750), ("control/revoked-grants", 0, gid, 0o750), ("keys", uid, gid, 0o700), ("state", uid, gid, 0o700), ("state/records", uid, gid, 0o700), ("work", 0, 0, 0o711), ("backup-staging", uid, gid, 0o700))]
