#!/usr/bin/env python3
"""Negative archive and rollout trust-order regression checks."""
import hashlib, importlib.util, io, json, sys, tarfile, tempfile
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

rollout_spec = importlib.util.spec_from_file_location(
    "rollout", HERE / "p2p-public-testnet-package-rollout.py"
)
rollout = importlib.util.module_from_spec(rollout_spec); assert rollout_spec.loader
rollout_spec.loader.exec_module(rollout)

with tempfile.TemporaryDirectory() as temp:
    root = Path(temp)
    package = root / "package"; macos = package / "macos"; macos.mkdir(parents=True)
    linux_bundle = package / "oasis7-linux-x64-bundle.tar.gz"
    linux_bundle.write_bytes(b"trusted-linux-bundle")
    linux_info = package / "linux-x64-BUILDINFO"; linux_info.write_text(
        "commit=fixture\npackage_version=fixture\nrun_id=fixture\n", encoding="utf-8"
    )
    mac_dmg = macos / "oasis7-macos-arm64.dmg"; mac_dmg.write_bytes(b"trusted-macos-dmg")
    mac_info = macos / "macos-arm64-BUILDINFO"; mac_info.write_text(
        "commit=fixture\npackage_version=fixture\nrun_id=fixture\nplatform=macos-arm64\ntarget_triple=aarch64-apple-darwin\n",
        encoding="utf-8",
    )
    def sums(directory, names, out):
        out.write_text("".join(
            f"{hashlib.sha256((directory / name).read_bytes()).hexdigest()}  {name}\n" for name in names
        ), encoding="utf-8")
    sums(package, [linux_bundle.name, linux_info.name], package / "linux-x64-SHA256SUMS")
    sums(macos, [mac_dmg.name, mac_info.name], macos / "macos-arm64-SHA256SUMS")
    linux_bundle.write_bytes(b"tampered-linux-bundle")
    manifest = root / "manifest.json"; manifest.write_text(json.dumps({"nodes": [
        {"name": "sequencer", "platform": "macos-arm64", "status_url": "http://127.0.0.1:1/v1/chain/status"},
        {"name": "storage", "platform": "macos-arm64", "status_url": "http://127.0.0.1:2/v1/chain/status"},
        {"name": "macos-observer", "platform": "macos-arm64", "status_url": "http://127.0.0.1:3/v1/chain/status"},
    ]}), encoding="utf-8")
    probe_started = False
    def forbidden_probe(*_args):
        nonlocal_probe[0] = True
        raise AssertionError("tampered package reached probe")
    nonlocal_probe = [False]
    original_probe, original_argv = rollout.run_checkpoint_closure_probe, sys.argv
    rollout.run_checkpoint_closure_probe = forbidden_probe
    sys.argv = ["rollout", "--manifest", str(manifest), "--package-dir", str(package), "--out-dir", str(root / "out")]
    try:
        try: rollout.main()
        except SystemExit as error: assert error.code == 1
        else: raise AssertionError("tampered non-Linux plan unexpectedly succeeded")
    finally:
        rollout.run_checkpoint_closure_probe, sys.argv = original_probe, original_argv
    assert not nonlocal_probe[0], "tampered Linux probe bundle must fail before probe/runtime start"

print("ok: non-Linux observer plan verifies the Linux probe bundle before probe start")
