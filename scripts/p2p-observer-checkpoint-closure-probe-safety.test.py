#!/usr/bin/env python3
"""Negative archive-safety regression checks for the clean-room probe."""
import importlib.util, io, tarfile, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("probe", HERE / "p2p-observer-checkpoint-closure-probe.py")
probe = importlib.util.module_from_spec(spec); assert spec.loader; spec.loader.exec_module(probe)
for kind, name in (("path", "../../escape"), ("symlink", "inside-link"), ("hardlink", "inside-hard"), ("char", "device")):
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw); package = root / "package"; package.mkdir(); archive = package / "oasis7-linux-x64-bundle.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            member = tarfile.TarInfo(name)
            if kind == "path": member.size = 1; tar.addfile(member, io.BytesIO(b"x"))
            elif kind == "symlink": member.type = tarfile.SYMTYPE; member.linkname = "/tmp/escape"; tar.addfile(member)
            elif kind == "hardlink": member.type = tarfile.LNKTYPE; member.linkname = "/tmp/escape"; tar.addfile(member)
            else: member.type = tarfile.CHRTYPE; tar.addfile(member)
        try: probe.package_runtime(package, root / "app")
        except SystemExit: pass
        else: raise AssertionError(f"unsafe {kind} archive was accepted")
print("ok: checkpoint closure probe rejects archive escapes and special members")
