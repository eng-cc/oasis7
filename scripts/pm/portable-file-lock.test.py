#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys
import tempfile
import types
import unittest
from unittest import mock


PATH = pathlib.Path(__file__).with_name("portable_file_lock.py")


def load_lock(name: str, *, windows_fallback: bool = False, fake_msvcrt=None):
    spec = importlib.util.spec_from_file_location(name, PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    if windows_fallback:
        with mock.patch.dict(sys.modules, {"fcntl": None, "msvcrt": fake_msvcrt}):
            spec.loader.exec_module(module)
    else:
        spec.loader.exec_module(module)
    return module


class PortableFileLockTests(unittest.TestCase):
    def test_timeout_is_bounded_and_process_crash_releases_lock(self):
        with tempfile.TemporaryDirectory() as temp:
            path = pathlib.Path(temp) / "locks" / "shared.lock"
            lock = load_lock("portable_lock_process_test")
            owner_code = r'''import importlib.util, pathlib, sys, time
spec = importlib.util.spec_from_file_location("owner_lock", pathlib.Path(sys.argv[1]))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
with module.locked_file(sys.argv[2], timeout=1):
    print("LOCKED", flush=True)
    time.sleep(30)
'''
            owner = subprocess.Popen([sys.executable, "-c", owner_code, str(PATH), str(path)],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertEqual(owner.stdout.readline().strip(), "LOCKED")
            with self.assertRaises(TimeoutError):
                with lock.locked_file(path, timeout=0.05, poll_interval=0.005):
                    self.fail("contended lock unexpectedly acquired")
            owner.terminate()
            owner.communicate(timeout=3)
            with lock.locked_file(path, timeout=0.2):
                self.assertTrue(path.exists())

    def test_windows_nonblocking_contention_translates_to_retry_and_bounded_timeout(self):
        class FakeMSVCRT(types.ModuleType):
            LK_NBLCK = 1
            LK_UNLCK = 2

            def __init__(self):
                super().__init__("msvcrt")
                self.busy_calls = 2
                self.calls = []

            def locking(self, fd, operation, count):
                self.calls.append((fd, operation, count))
                if operation == self.LK_NBLCK and self.busy_calls:
                    self.busy_calls -= 1
                    error = OSError("byte range locked")
                    error.winerror = 33
                    raise error

        fake = FakeMSVCRT()
        lock = load_lock("portable_lock_windows_test", windows_fallback=True, fake_msvcrt=fake)
        with tempfile.TemporaryDirectory() as temp:
            probe = pathlib.Path(temp) / "probe.lock"
            with probe.open("w+b") as handle:
                self.assertFalse(lock.fcntl.try_flock(handle.fileno()))
            path = pathlib.Path(temp) / "windows.lock"
            with lock.locked_file(path, timeout=0.2, poll_interval=0.001):
                self.assertTrue(path.exists())
            self.assertGreaterEqual(len(fake.calls), 4)

    def test_windows_busy_lock_timeout_does_not_wait_forever(self):
        class AlwaysBusy(types.ModuleType):
            LK_NBLCK = 1
            LK_UNLCK = 2

            def __init__(self):
                super().__init__("msvcrt")

            def locking(self, _fd, operation, _count):
                if operation == self.LK_NBLCK:
                    error = OSError("byte range locked")
                    error.winerror = 33
                    raise error

        lock = load_lock("portable_lock_windows_busy_test", windows_fallback=True,
                         fake_msvcrt=AlwaysBusy())
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaises(TimeoutError):
                with lock.locked_file(pathlib.Path(temp) / "busy.lock", timeout=0.04,
                                      poll_interval=0.002):
                    self.fail("busy Windows lock unexpectedly acquired")


if __name__ == "__main__":
    unittest.main(verbosity=2)
