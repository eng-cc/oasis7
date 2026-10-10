#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Optional real compiler proof: requires installed sccache and cached itoa.
if [[ "${1:-}" == "--real" ]]; then
  python3 - "$ROOT_DIR" <<'PYTHON'
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile

root = Path(sys.argv[1])
with tempfile.TemporaryDirectory(prefix="oasis7-sccache-proof-") as directory:
    fixture = Path(directory)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    env = dict(os.environ, CI="", OASIS7_CARGO_SCCACHE="1",
               SCCACHE_DIR=str(fixture / "cache"), SCCACHE_SERVER_PORT=str(port))
    try:
        previous_hits = 0
        for name in ("a", "b"):
            repo = fixture / name
            (repo / "src").mkdir(parents=True)
            (repo / "scripts").mkdir()
            (repo / "Cargo.toml").write_text(
                '[package]\nname="cache_proof"\nversion="0.1.0"\nedition="2021"\n'
                '[dependencies]\nitoa="1"\n')
            (repo / "src/lib.rs").write_text(
                'pub fn format(n: u64) -> String { itoa::Buffer::new().format(n).to_owned() }\n')
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            for filename in ("cargo-dev.sh", "find-python-with-module.sh"):
                shutil.copy2(root / "scripts" / filename, repo / "scripts" / filename)
            subprocess.run([str(repo / "scripts/cargo-dev.sh"), "build", "--offline"],
                           cwd=repo, env=env, check=True)
            stats = json.loads(subprocess.check_output(
                ["sccache", "--show-stats", "--stats-format=json"], env=env))
            hits = sum(stats["stats"]["cache_hits"]["counts"].values())
            if name == "b" and hits <= previous_hits:
                raise RuntimeError("second source/target path did not reuse a Rust compilation")
            previous_hits = hits
        print(f"cargo-dev-sccache real proof: {hits} cache hit(s)")
    finally:
        subprocess.run(["sccache", "--stop-server"], env=env, check=False,
                       stdout=subprocess.DEVNULL)
PYTHON
  exit 0
fi
FIXTURE="$(mktemp -d)"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/repo/scripts" "$FIXTURE/bin"
cp "$ROOT_DIR/scripts/cargo-dev.sh" "$ROOT_DIR/scripts/find-python-with-module.sh" "$FIXTURE/repo/scripts/"
git -C "$FIXTURE/repo" init -q
cat >"$FIXTURE/bin/sccache" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
cat >"$FIXTURE/bin/cargo" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "wrapper=${RUSTC_WRAPPER:-unset}" "incremental=${CARGO_INCREMENTAL:-unset}" "target=${CARGO_TARGET_DIR:-unset}" "$@"
EOF
chmod +x "$FIXTURE/bin/"*
cd "$FIXTURE/repo"
export PATH="$FIXTURE/bin:$PATH"
export CI= OASIS7_CARGO_SCCACHE=1
export RUSTC_WRAPPER=hostile-wrapper CARGO_TARGET_DIR=hostile-target
result="$(./scripts/cargo-dev.sh run --bin example -- --target-dir application-argument)"
[[ "$result" == *"wrapper=$FIXTURE/bin/sccache"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
[[ "$result" == *"incremental=0"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
[[ "$result" == *"target=unset"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
[[ "$result" == *$'run\n--target-dir\n'* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
[[ "$result" == *$'--\n--target-dir\napplication-argument' ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
for mode in check clippy; do
  result="$(./scripts/cargo-dev.sh "$mode")"
  [[ "$result" == *"wrapper=unset"* && "$result" == *"incremental=unset"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
done
for setting in 'CI=1' 'CI=true' 'OASIS7_CARGO_SCCACHE=0'; do
  result="$(env "$setting" ./scripts/cargo-dev.sh build)"
  [[ "$result" == *"wrapper=unset"* && "$result" != *"target=unset"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
done
# Simulate a missing installation even when the host has sccache.
command() {
  if [[ "${1:-}" == "-v" && "${2:-}" == "sccache" ]]; then return 1; fi
  builtin command "$@"
}
export -f command
result="$(./scripts/cargo-dev.sh build)"
[[ "$result" == *"wrapper=unset"* && "$result" != *"target=unset"* ]] || { echo "unexpected cargo environment or arguments: $result" >&2; exit 1; }
unset -f command
echo "cargo-dev-sccache.test: OK"
