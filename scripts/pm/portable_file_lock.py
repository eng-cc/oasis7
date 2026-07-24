"""Small cross-platform advisory file-lock compatibility layer."""
from __future__ import annotations

import time

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
