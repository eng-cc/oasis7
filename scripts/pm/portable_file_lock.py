"""Small cross-platform advisory file-lock compatibility layer."""
from __future__ import annotations

import contextlib
import errno
import os
import pathlib
import time

try:
    import fcntl as fcntl
except ImportError:  # pragma: no cover - exercised on Windows
    import msvcrt

    class _WindowsFcntl:
        LOCK_EX = 0
        LOCK_UN = 1

        @staticmethod
        def try_flock(fd: int) -> bool:
            import os

            os.lseek(fd, 0, os.SEEK_SET)
            try:
                msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
            except OSError as exc:
                if (getattr(exc, "winerror", None) in (33, 36, 158)
                        or getattr(exc, "errno", None) in (errno.EACCES, errno.EAGAIN, 36)):
                    return False
                raise
            return True

        @staticmethod
        def flock(fd: int, operation: int) -> None:
            import os

            os.lseek(fd, 0, os.SEEK_SET)
            if operation == _WindowsFcntl.LOCK_UN:
                msvcrt.locking(fd, msvcrt.LK_UNLCK, 1)
            else:
                # LK_LOCK can report ERROR_POSSIBLE_DEADLOCK when another
                # Python process owns the byte. Poll the non-blocking form so
                # contention behaves like POSIX flock instead of failing.
                while True:
                    try:
                        msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
                        break
                    except OSError as exc:
                        if (getattr(exc, "winerror", None) not in (33, 36, 158)
                                and getattr(exc, "errno", None) not in (13, 36)):
                            raise
                        time.sleep(0.01)

    fcntl = _WindowsFcntl()


def ensure_lock_byte(handle) -> None:
    """Ensure the byte range used by Windows locking exists."""
    if handle.tell() == 0:
        handle.write(b"0")
        handle.flush()


def try_lock(handle) -> bool:
    """Acquire the lock without waiting; return False only for contention."""
    if hasattr(fcntl, "try_flock"):
        return fcntl.try_flock(handle.fileno())
    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    except OSError as exc:
        if exc.errno in (errno.EACCES, errno.EAGAIN):
            return False
        raise
    return True


@contextlib.contextmanager
def locked_file(path: os.PathLike[str] | str, *, timeout: float = 2.0,
                poll_interval: float = 0.01):
    """Open a persistent private lock inode and acquire it within a bound.

    Lock files must not be unlinked while a process may hold them: doing so
    would permit a second process to lock a new inode at the same path.
    """
    lock_path = pathlib.Path(path)
    lock_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if os.name != "nt":
        try:
            os.chmod(lock_path.parent, 0o700)
        except OSError:
            pass
    descriptor = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o600)
    if os.name != "nt":
        try:
            os.fchmod(descriptor, 0o600)
        except OSError:
            pass
    handle = os.fdopen(descriptor, "r+b", buffering=0)
    acquired = False
    try:
        handle.seek(0, os.SEEK_END)
        ensure_lock_byte(handle)
        deadline = time.monotonic() + max(0.0, timeout)
        while not (acquired := try_lock(handle)):
            if time.monotonic() >= deadline:
                raise TimeoutError(f"timed out acquiring lock {lock_path.name}")
            time.sleep(max(0.001, poll_interval))
        yield handle
    finally:
        if acquired:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
        handle.close()
