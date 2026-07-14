#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for legacy hooks. Ordinary local commits are not a
# verification gate; CI required and frozen-head Pre-PR Ready own validation.
exit 0
