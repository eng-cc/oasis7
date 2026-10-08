#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for legacy hooks. Ordinary local commits are not a
# verification gate; use ci-tests.sh for checks and actual CI for PR validation.
exit 0
