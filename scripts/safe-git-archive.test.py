#!/usr/bin/env python3
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import replace
spec = importlib.util.spec_from_file_location("archive_helper", Path(__file__).with_name("safe_git_archive.py"))
helper = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = helper
spec.loader.exec_module(helper)

with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary)
    def archive(entries):
        path = root / "fixture.tar"
        with tarfile.open(path, "w") as output:
            for name, kind, value in entries:
                member = tarfile.TarInfo(name)
                member.mode = 0o755
                if kind == "file":
                    member.size = len(value)
                    output.addfile(member, io.BytesIO(value))
                else:
                    member.type = kind
                    member.linkname = value
                    output.addfile(member)
        return path
    def rejects(entries, **kwargs):
        path = archive(entries)
        try: helper.extract_archive(path, root / "out", **kwargs)
        except (ValueError, tarfile.TarError): pass
        else: raise AssertionError(entries)
        assert not (root / "out").exists()
    for path in ("../escape", "/escape", "C:/escape", "a/../../escape", "a\\escape"):
        rejects([(path, "file", b"x")])
    for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.CHRTYPE, tarfile.FIFOTYPE):
        rejects([("special", kind, "../escape")])
    rejects([("a", tarfile.SYMTYPE, "b"), ("b", tarfile.SYMTYPE, "a")], policy=helper.ArchivePolicy.CARGO_SNAPSHOT)
    rejects([("a", tarfile.SYMTYPE, "../outside")], policy=helper.ArchivePolicy.CARGO_SNAPSHOT)
    rejects([("a", "file", b"1"), ("a/b", "file", b"1")])
    rejects([("a", "file", b"1"), ("a", "file", b"1")])
    rejects([("a", "file", b"12")], limits=replace(helper.ArchiveLimits(), member_bytes=1))
    rejects([("a", "file", b"1"), ("b", "file", b"1")], limits=replace(helper.ArchiveLimits(), members=1))
    rejects([("a", "file", b"1"), ("b", "file", b"1")], limits=replace(helper.ArchiveLimits(), expanded_bytes=1))
    for kind in (tarfile.XHDTYPE, tarfile.GNUTYPE_LONGNAME):
        header = tarfile.TarInfo("metadata")
        header.type = kind
        header.size = 65537
        malicious = root / "metadata.tar.gz"
        import gzip
        with gzip.open(malicious, "wb") as output:
            output.write(header.tobuf())
            output.write(bytes(66048 + 1024))
        try: helper.extract_archive(malicious, root / "out")
        except ValueError as error: assert "extended header" in str(error)
        else: raise AssertionError("unbounded metadata accepted")
        assert not (root / "out").exists()
    bomb = root / "bomb.tar.gz"
    with gzip.open(bomb, "wb") as output: output.write(bytes(100000))
    try: helper.extract_archive(bomb, root / "out", limits=replace(helper.ArchiveLimits(), members=1, expanded_bytes=1))
    except ValueError as error: assert "decompressed" in str(error)
    else: raise AssertionError("inflation budget bypass")
    truncated = archive([("a", "file", b"payload")])
    truncated.write_bytes(truncated.read_bytes()[:514])
    try: helper.extract_archive(truncated, root / "out")
    except ValueError: pass
    else: raise AssertionError("truncated member accepted")
    assert not (root / "out").exists()
    exact=archive([("a", "file", b"x")])
    helper.extract_archive(exact, root / "out", limits=replace(helper.ArchiveLimits(), member_bytes=1, members=1, expanded_bytes=1))
    import shutil
    shutil.rmtree(root / "out")
    path=archive([("a", "file", b"x"), ("link", tarfile.SYMTYPE, "a")])
    helper.extract_archive(path, root / "out", policy=helper.ArchivePolicy.CARGO_SNAPSHOT)
    assert (root / "out/link").read_bytes() == b"x"
    assert (root / "out/a").stat().st_mode & 0o111
    import shutil
    shutil.rmtree(root / "out")
    subprocess.run(["git", "init", "-q", str(root / "repo")], check=True)
    repo=root / "repo"
    (repo / "file").write_text("bounded")
    subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
    subprocess.run(["git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.test", "commit", "-qm", "fixture"],check=True)
    oid=subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"],text=True).strip()
    with helper.extracted_git_tree(repo, oid) as tree:
        captured=tree.root
        assert (captured / "file").read_text() == "bounded"
    assert not captured.exists()
    try:
        with helper.extracted_git_tree(repo, oid, limits=replace(helper.ArchiveLimits(), archive_bytes=1)): pass
    except ValueError: pass
    else: raise AssertionError("archive limit bypass")
    original_popen = helper.subprocess.Popen
    for code in ("import sys;sys.stderr.write('e'*100000);sys.exit(2)", "import time;time.sleep(10)", "import sys;sys.stdout.buffer.write(b'not a tar')"):
        def fake_git(arguments, *args, **kwargs):
            if "archive" in arguments:
                return original_popen([sys.executable, "-c", code], *args, **kwargs)
            return original_popen(arguments, *args, **kwargs)
        helper.subprocess.Popen = fake_git
        try:
            try:
                with helper.extracted_git_tree(repo, oid, limits=replace(helper.ArchiveLimits(), seconds=0.5)):
                    raise AssertionError("failed Git capture exposed a tree")
            except ValueError: pass
        finally:
            helper.subprocess.Popen = original_popen
print("ok: bounded archive policies, symlinks, executable mode, cleanup, Git capture")
