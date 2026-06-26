#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d)
launcher_manifest="tools/wasm_build_suite/Cargo.toml"
launcher_backup="$tmp_dir/wasm_build_suite.Cargo.toml"
root_manifest="Cargo.toml"
root_backup="$tmp_dir/root.Cargo.toml"
cp "$launcher_manifest" "$launcher_backup"
cp "$root_manifest" "$root_backup"
cleanup() {
  cp "$launcher_backup" "$launcher_manifest" 2>/dev/null || true
  cp "$root_backup" "$root_manifest" 2>/dev/null || true
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

direct_baseline="$tmp_dir/direct-dependency-baseline.json"
cp scripts/rustsec-ignore-direct-dependency-baseline.json "$direct_baseline"

run_check() {
  OASIS7_RUSTSEC_DENY_TOML="$1" \
    OASIS7_RUSTSEC_DIRECT_DEP_BASELINE="${2:-$direct_baseline}" \
    OASIS7_RUSTSEC_TODAY="2026-06-24" \
    ./scripts/check-rustsec-ignore-baseline.sh
}

valid_out="$tmp_dir/valid.out"
run_check deny.toml >"$valid_out"
if ! grep -q "ok: RustSec ignore baseline" "$valid_out"; then
  echo "expected valid baseline to pass" >&2
  cat "$valid_out" >&2
  exit 1
fi

missing_metadata="$tmp_dir/missing-metadata.toml"
python3 - deny.toml "$missing_metadata" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out = []
skip_previous = False
for line in source:
    if '"RUSTSEC-2024-0436"' in line and out and "rustsec-ignore:" in out[-1]:
        out.pop()
    out.append(line)
Path(sys.argv[2]).write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check "$missing_metadata" >"$tmp_dir/missing.out" 2>&1; then
  echo "expected missing metadata case to fail" >&2
  exit 1
fi
grep -q "missing rustsec-ignore metadata" "$tmp_dir/missing.out"

commented_id="$tmp_dir/commented-id.toml"
python3 - deny.toml "$commented_id" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out = []
for line in source:
    if '"RUSTSEC-2024-0436"' in line and not line.lstrip().startswith("#"):
        out.append("  # " + line.strip() + " # test-only commented debt note")
    else:
        out.append(line)
Path(sys.argv[2]).write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check "$commented_id" >"$tmp_dir/commented.out" 2>&1; then
  echo "expected commented advisory id case to fail" >&2
  exit 1
fi
grep -q "approved RustSec ignore id(s) missing" "$tmp_dir/commented.out"

extra_id="$tmp_dir/extra-id.toml"
python3 - deny.toml "$extra_id" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out = []
for line in source:
    out.append(line)
    if '"RUSTSEC-2024-0436"' in line:
        out.append("  # rustsec-ignore: owner=repository_health_engineer; scope=test; reason=test; expiry=2026-08-31; validation=test")
        out.append('  "RUSTSEC-2099-0001", # test-only unapproved advisory')
Path(sys.argv[2]).write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check "$extra_id" >"$tmp_dir/extra.out" 2>&1; then
  echo "expected unapproved advisory case to fail" >&2
  exit 1
fi
grep -q "unapproved RustSec ignore id" "$tmp_dir/extra.out"

expired="$tmp_dir/expired.toml"
python3 - deny.toml "$expired" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
Path(sys.argv[2]).write_text(text.replace("expiry=2026-09-30", "expiry=2026-01-01"), encoding="utf-8")
PY
if run_check "$expired" >"$tmp_dir/expired.out" 2>&1; then
  echo "expected expired metadata case to fail" >&2
  exit 1
fi
grep -q "metadata expired" "$tmp_dir/expired.out"

bad_validation="$tmp_dir/bad-validation.toml"
python3 - deny.toml "$bad_validation" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
Path(sys.argv[2]).write_text(
    text.replace("validation=cargo tree --all-features -i serde_cbor", "validation=echo not-a-scope-check"),
    encoding="utf-8",
)
PY
if run_check "$bad_validation" >"$tmp_dir/bad-validation.out" 2>&1; then
  echo "expected unsupported validation command case to fail" >&2
  exit 1
fi
grep -q 'validation must be a `cargo tree -i ...` command' "$tmp_dir/bad-validation.out"

extra_local_crate="$tmp_dir/extra-local-crate.toml"
python3 - deny.toml "$extra_local_crate" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
Path(sys.argv[2]).write_text(
    text.replace(
        "local_crates=oasis7,oasis7_client_launcher,oasis7_consensus,oasis7_distfs,oasis7_net,oasis7_node,oasis7_proto,oasis7_wasm_executor,oasis7_wasm_sdk",
        "local_crates=oasis7_client_launcher,oasis7_consensus,oasis7_distfs,oasis7_net,oasis7_node,oasis7_proto,oasis7_wasm_executor,oasis7_wasm_sdk",
    ),
    encoding="utf-8",
)
PY
if run_check "$extra_local_crate" >"$tmp_dir/extra-local-crate.out" 2>&1; then
  echo "expected unapproved local crate scope case to fail" >&2
  exit 1
fi
grep -q "appears in unapproved local crate scope" "$tmp_dir/extra-local-crate.out"

missing_local_crate="$tmp_dir/missing-local-crate.toml"
python3 - deny.toml "$missing_local_crate" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
Path(sys.argv[2]).write_text(
    text.replace(
        "local_crates=oasis7,oasis7_client_launcher,oasis7_consensus,oasis7_distfs,oasis7_net,oasis7_node,oasis7_proto,oasis7_wasm_executor,oasis7_wasm_sdk",
        "local_crates=oasis7,oasis7_client_launcher,oasis7_consensus,oasis7_distfs,oasis7_net,oasis7_node,oasis7_proto,oasis7_wasm_executor,oasis7_wasm_sdk,oasis7_fake_scope",
    ),
    encoding="utf-8",
)
PY
if run_check "$missing_local_crate" >"$tmp_dir/missing-local-crate.out" 2>&1; then
  echo "expected missing approved local crate scope case to fail" >&2
  exit 1
fi
grep -q "approved local crate scope(s) missing from validation" "$tmp_dir/missing-local-crate.out"

direct_growth="$tmp_dir/direct-growth.json"
python3 - "$direct_baseline" "$direct_growth" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
paths = payload["ignored_direct_manifests"]["serde_cbor"]
paths.remove("crates/oasis7/Cargo.toml")
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if run_check deny.toml "$direct_growth" >"$tmp_dir/direct-growth.out" 2>&1; then
  echo "expected direct manifest growth case to fail" >&2
  exit 1
fi
grep -q "direct dependency manifest scope grew beyond RustSec baseline" "$tmp_dir/direct-growth.out"

direct_stale="$tmp_dir/direct-stale.json"
python3 - "$direct_baseline" "$direct_stale" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["ignored_direct_manifests"]["serde_cbor"].append("crates/oasis7_fake_scope/Cargo.toml")
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if run_check deny.toml "$direct_stale" >"$tmp_dir/direct-stale.out" 2>&1; then
  echo "expected stale direct manifest baseline case to fail" >&2
  exit 1
fi
grep -q "direct dependency manifest baseline is stale" "$tmp_dir/direct-stale.out"

python3 - "$launcher_manifest" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = path.read_text(encoding="utf-8").splitlines()
out = []
inserted = False
for line in lines:
    out.append(line)
    if line.strip() == "[dependencies]" and not inserted:
        out.append('paste = "1"')
        inserted = True
if not inserted:
    raise SystemExit("test manifest missing [dependencies]")
path.write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check deny.toml >"$tmp_dir/paste-direct.out" 2>&1; then
  echo "expected direct paste manifest case to fail" >&2
  exit 1
fi
grep -q '`paste` direct dependency manifest scope grew beyond RustSec baseline' "$tmp_dir/paste-direct.out"
cp "$launcher_backup" "$launcher_manifest"

python3 - "$root_manifest" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = path.read_text(encoding="utf-8").splitlines()
out = []
inserted = False
for line in lines:
    out.append(line)
    if line.strip().startswith("resolver = ") and not inserted:
        out.append("[workspace.dependencies]")
        out.append('paste = "1"')
        inserted = True
if not inserted:
    raise SystemExit("root manifest missing workspace resolver")
path.write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check deny.toml >"$tmp_dir/paste-root-direct.out" 2>&1; then
  echo "expected root workspace direct paste manifest case to fail" >&2
  exit 1
fi
grep -q '`paste` direct dependency manifest scope grew beyond RustSec baseline: Cargo.toml' "$tmp_dir/paste-root-direct.out"
cp "$root_backup" "$root_manifest"

echo "check-rustsec-ignore-baseline.test: OK"
