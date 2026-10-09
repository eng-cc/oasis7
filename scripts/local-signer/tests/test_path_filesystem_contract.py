import sys
from pathlib import Path
import unittest
import tempfile
import stat
from types import SimpleNamespace
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import macos_host as host

class FilesystemContract(unittest.TestCase):
    def test_arbitrary_jobs_path_rejected_before_observation(self):
        backend = host.MacOSHost(runtime_identity={"fixture": "identity"})
        caller = SimpleNamespace(pw_name="caller", pw_uid=504, pw_gid=20)
        with patch.object(backend, "inspect_path") as inspect:
            with self.assertRaises(host.InstallError):
                backend.validate_jobs(Path("/Users/caller/Documents/keys/jobs"), caller)
            inspect.assert_not_called()

    def test_jobs_parent_exact_owner_group_mode(self):
        backend = host.MacOSHost(runtime_identity={"fixture": "identity"})
        caller = SimpleNamespace(pw_name="caller", pw_uid=504, pw_gid=20)
        work = host.JOBS_ROOT / "caller" / "oasis7-local-signer"
        def metadata(path, *, uid=504, gid=20, mode=0o700):
            if path == host.JOBS_ROOT:
                return SimpleNamespace(st_mode=stat.S_IFDIR | 0o711, st_uid=0, st_gid=0)
            return SimpleNamespace(st_mode=stat.S_IFDIR | mode, st_uid=uid, st_gid=gid)
        for uid, gid, mode in ((504, 20, 0o700), (505, 20, 0o700), (504, 21, 0o700), (504, 20, 0o755)):
            with patch.object(backend, "inspect_path"), patch.object(Path, "lstat", autospec=True, side_effect=lambda p: metadata(p, uid=uid, gid=gid, mode=mode)), patch.object(host.os.path, "lexists", return_value=False):
                if (uid, gid, mode) == (504, 20, 0o700):
                    backend.validate_jobs(work, caller)
                else:
                    with self.assertRaises(host.InstallError):
                        backend.validate_jobs(work, caller)

    def test_jobs_root_mode_rejected(self):
        backend = host.MacOSHost(runtime_identity={"fixture": "identity"})
        caller = SimpleNamespace(pw_name="caller", pw_uid=504, pw_gid=20)
        with patch.object(backend, "inspect_path"), patch.object(Path, "lstat", return_value=SimpleNamespace(st_mode=stat.S_IFDIR | 0o777, st_uid=0, st_gid=0)):
            with self.assertRaises(host.InstallError):
                backend.validate_jobs(host.JOBS_ROOT / "caller" / "oasis7-local-signer", caller)

    def inspect(self, path, callback=None, filesystem=None):
        backend = host.MacOSHost(runtime_identity={"fixture": "identity"})
        def run(argv):
            if callback:
                callback(argv[-1])
            return "drwx------ 1 fixture staff 0 fixture\n"
        with patch.object(backend, "run", side_effect=run), patch.object(host, "descriptor_filesystem", side_effect=filesystem or (lambda fd: {"type": "apfs", "flags": 0x1000, "fsid": [1, 2]})):
            return backend.inspect_path(path, protected=False)

    def test_existing_ancestors_and_absent_leaf_are_proved(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root).resolve() / "absent" / "leaf"
            facts = self.inspect(path)
            self.assertTrue(facts[-1]["absent"])
            self.assertTrue(all("filesystem" in value for value in facts[:-1]))
            self.assertEqual(facts[0]["path"], "/")

    def test_symlink_component_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root).resolve()
            (root / "link").symlink_to(root, target_is_directory=True)
            with self.assertRaises(host.InstallError):
                self.inspect(root / "link" / "leaf")

    def test_path_replacement_after_open_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root).resolve()
            child = root / "child"
            child.mkdir()
            def replace(path):
                if path == str(child):
                    child.rename(root / "old")
                    child.mkdir()
            with self.assertRaises(host.InstallError):
                self.inspect(child, callback=replace)

    def test_filesystem_change_after_open_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            calls = [0]
            def changed(fd):
                calls[0] += 1
                return {"type": "apfs", "flags": 0x1000, "fsid": [calls[0], 2]}
            with self.assertRaises(host.InstallError):
                self.inspect(Path(root).resolve(), filesystem=changed)

    def test_local_apfs_and_hfs(self):
        for name in ("apfs", "hfs"):
            host.validate_filesystem({"type": name, "flags": 0x1000, "fsid": [1, 2]})

    def test_remote_unknown_and_noowners_rejected(self):
        for facts in ({"type": "apfs", "flags": 0}, {"type": "nfs", "flags": 0x1000}, {"type": "autofs", "flags": 0x1000}, {"type": "apfs", "flags": 0x201000}):
            with self.assertRaises(host.InstallError):
                host.validate_filesystem(facts)

    def test_installed_darwin_descriptor(self):
        if sys.platform != "darwin":
            self.skipTest("actual Darwin ABI requires macOS")
        import os
        fd = os.open("/private/var/db", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            facts = host.descriptor_filesystem(fd)
            self.assertEqual(facts["type"], "apfs")
            host.validate_filesystem(facts)
        finally:
            os.close(fd)

if __name__ == "__main__":
    unittest.main()
