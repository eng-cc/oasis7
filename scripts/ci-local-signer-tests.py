#!/usr/bin/env python3
"""Run all local-signer contracts and enforce native platform coverage."""
import argparse
from pathlib import Path
import sys
import unittest

NATIVE_TEST = "test_path_filesystem_contract.FilesystemContract.test_installed_darwin_descriptor"


class CoverageResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.successful_ids = set()

    def getDescription(self, test):
        return test.id()

    def addSuccess(self, test):
        self.successful_ids.add(test.id())
        super().addSuccess(test)


def test_ids(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from test_ids(test)
        else:
            yield test.id()


def run_suite(suite, *, require_darwin=False, platform=None, stream=None):
    platform = sys.platform if platform is None else platform
    stream = sys.stderr if stream is None else stream
    ids = list(test_ids(suite))
    if require_darwin and platform != "darwin":
        print("local-signer coverage: --require-darwin requires Darwin", file=stream)
        return 1
    if not ids or NATIVE_TEST not in ids:
        print("local-signer coverage: empty suite or missing native descriptor test", file=stream)
        return 1
    result = unittest.TextTestRunner(stream=stream, verbosity=2,
                                     resultclass=CoverageResult).run(suite)
    allowed_skips = {NATIVE_TEST} if platform.startswith("linux") and not require_darwin else set()
    unexpected_skips = [test.id() for test, _ in result.skipped if test.id() not in allowed_skips]
    native_missing = platform == "darwin" and NATIVE_TEST not in result.successful_ids
    if unexpected_skips or native_missing:
        print(f"local-signer coverage: unexpected skips={unexpected_skips}; "
              f"native success missing={native_missing}", file=stream)
    return int(not result.wasSuccessful() or bool(unexpected_skips) or native_missing)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-darwin", action="store_true")
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent / "local-signer" / "tests"
    suite = unittest.TestLoader().discover(str(directory), pattern="test_*.py")
    return run_suite(suite, require_darwin=args.require_darwin)


if __name__ == "__main__":
    sys.exit(main())
