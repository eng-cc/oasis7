#!/usr/bin/env python3
"""Test-only durable scheduler delivery adapter spy."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

payload = json.loads(sys.stdin.read() or "{}")
Path(os.environ["TPM_DELIVERY_ADAPTER_LOG"]).write_text(json.dumps(payload, sort_keys=True))
print(json.dumps({"ok": True, "status": "delivered",
                  "delivery_id": payload.get("delivery_id")}, sort_keys=True))
