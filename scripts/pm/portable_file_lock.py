"""Small cross-platform advisory file-lock compatibility layer."""
from __future__ import annotations

try:
    import fcntl as fcntl
except ImportError:  # pragma: no cover - exercised on Windows
    import msvcrt

    class _WindowsFcntl:
        LOCK_EX = 0
        LOCK_UN = 1

        @staticmethod
        def flock(fd: int, operation: int) -> None:
            import os

            os.lseek(fd, 0, os.SEEK_SET)
            if operation == _WindowsFcntl.LOCK_UN:
                msvcrt.locking(fd, msvcrt.LK_UNLCK, 1)
            else:
                msvcrt.locking(fd, msvcrt.LK_LOCK, 1)

    fcntl = _WindowsFcntl()


def ensure_lock_byte(handle) -> None:
    """Ensure the byte range used by Windows locking exists."""
    if handle.tell() == 0:
        handle.write(b"0")
        handle.flush()
