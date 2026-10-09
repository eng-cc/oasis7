#!/usr/bin/env python3
"""Extract an operator tar with the shared bounded authority policy."""
import importlib.util
from pathlib import Path
import shutil
import sys
import tempfile


def main():
    if len(sys.argv) != 3:
        raise ValueError("usage: p2p-safe-extract-tar.py <archive> <destination>")
    spec = importlib.util.spec_from_file_location("safe_git_archive", Path(__file__).resolve().with_name("safe_git_archive.py"))
    helper = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = helper
    spec.loader.exec_module(helper)
    destination = Path(sys.argv[2])
    destination.mkdir(parents=True, exist_ok=True)
    created = []
    with tempfile.TemporaryDirectory(dir=destination.parent, prefix="oasis7-extract-") as temporary:
        tree = Path(temporary) / "tree"
        helper.extract_archive(sys.argv[1], tree)
        children = list(tree.iterdir())
        if any((destination / item.name).exists() or (destination / item.name).is_symlink() for item in children):
            raise ValueError("tar extraction destination already contains top-level path")
        try:
            for item in children:
                target = destination / item.name
                item.rename(target)
                created.append(target)
        except BaseException:
            for target in created:
                if target.is_dir(): shutil.rmtree(target)
                else: target.unlink()
            raise
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError) as error:
        print("error: " + str(error), file=sys.stderr)
        raise SystemExit(1)
