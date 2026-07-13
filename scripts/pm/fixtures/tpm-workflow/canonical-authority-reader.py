#!/usr/bin/env python3
"""Test-only canonical task authority reader, separate from the main adapter."""

import json
import os
import sys

if os.environ.get("TPM_AUTHORITY_READER_FAIL") == "1":
    print(json.dumps({"ok": False, "status": 503}))
    raise SystemExit(75)

print(os.environ["TPM_AUTHORITY_READER_RESPONSE"])
