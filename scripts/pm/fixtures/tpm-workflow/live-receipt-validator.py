#!/usr/bin/env python3
"""Test-only live receipt validator spy.

The production driver must invoke this adapter; merely copying its name into a
receipt is not evidence.  The invocation is captured for the contract test.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def main() -> int:
    log = Path(os.environ["TPM_LIVE_VALIDATOR_LOG"])
    payload = {"argv": sys.argv[1:], "stdin": sys.stdin.read()}
    log.write_text(json.dumps(payload, sort_keys=True))
    response = json.loads(os.environ.get(
        "TPM_LIVE_VALIDATOR_RESPONSE", '{"ok":true,"live_readback":true}'
    ))
    print(json.dumps(response, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
