#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-rebuild-validators-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/bin" "$TMP_DIR/config" "$TMP_DIR/world" "$TMP_DIR/remote" "$TMP_DIR/status"

cat >"$TMP_DIR/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "ok": true,
  "runtime_build": {
    "sha256": "old-runtime-sha",
    "size_bytes": 1
  }
}
EOF

cat >"$TMP_DIR/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" <<'EOF'
{"manifest":true}
EOF

cat >"$TMP_DIR/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'EOF'
{
  "validators": [
    {
      "node_id": "triad-testnet-sequencer",
      "stake": 100,
      "finality_signer_public_key": "new-sequencer-signer"
    },
    {
      "node_id": "triad-testnet-storage",
      "stake": 100,
      "finality_signer_public_key": "new-storage-signer"
    }
  ]
}
EOF

cat >"$TMP_DIR/config/000-old-validator-registry.json" <<'EOF'
{
  "validators": [
    {
      "node_id": "triad-testnet-sequencer",
      "stake": 100,
      "finality_signer_public_key": "wrong-sorted-first-sequencer-signer"
    },
    {
      "node_id": "triad-testnet-storage",
      "stake": 100,
      "finality_signer_public_key": "wrong-sorted-first-storage-signer"
    }
  ]
}
EOF

cat >"$TMP_DIR/world/snapshot.json" <<'EOF'
{"world":true}
EOF

cat >"$TMP_DIR/world/journal.json" <<'EOF'
[]
EOF

cat >"$TMP_DIR/status/sequencer.json" <<'JSON'
{
  "running": true,
  "last_error": null,
  "readiness": {
    "status": "ready"
  },
  "observability": {
    "storage_challenge_network_degraded": false
  },
  "consensus": {
    "committed_height": 1,
    "last_execution_height": 1,
    "storage_challenge_network_degraded_height": null
  },
  "replication": {
    "local_peer_id": "12D3KooWSequencer",
    "connected_peers": ["12D3KooWStorage"]
  }
}
JSON

cat >"$TMP_DIR/status/storage.json" <<'JSON'
{
  "running": true,
  "last_error": null,
  "readiness": {
    "status": "ready"
  },
  "observability": {
    "storage_challenge_network_degraded": false
  },
  "consensus": {
    "committed_height": 1,
    "last_execution_height": 0,
    "storage_challenge_network_degraded_height": null,
    "network_head": {
      "height": 1
    }
  },
  "replication": {
    "local_peer_id": "12D3KooWStorage",
    "connected_peers": ["12D3KooWSequencer"]
  }
}
JSON

cat >"$TMP_DIR/bin/sshpass" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" != "-e" ]]; then
  echo "expected -e" >&2
  exit 1
fi
shift
exec "$@"
EOF
chmod +x "$TMP_DIR/bin/sshpass"

cat >"$TMP_DIR/bin/ssh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
while [[ $# -gt 0 && "$1" == -* ]]; do
  case "$1" in
    -o|-S|-O)
      shift 2
      ;;
    -M|-N|-f)
      shift
      ;;
    *)
      shift
      ;;
  esac
done
host=$1
shift
if [[ $# -eq 0 ]]; then
  exit 0
fi
cmd="$*"
root="${TEST_REMOTE_ROOT:?}/$host"
mkdir -p "$root"
if [[ -n "${TEST_SSH_LOG:-}" ]]; then
  logged_cmd=${cmd//$'\n'/\\n}
  printf '%s\t%s\n' "$host" "$logged_cmd" >>"$TEST_SSH_LOG"
fi
case "$cmd" in
  mkdir\ -p*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/.*mkdir -p '\([^']*\)\/config\/doc\/testing\/evidence'.*/\1/p")
    mkdir -p "$root$stack_root/config/doc/testing/evidence" "$root$stack_root/staged-world" "$root$stack_root/data/execution-world"
    ;;
  cat\ \>*)
    target=$(printf '%s\n' "$cmd" | sed -n "s/cat > '\([^']*\)'/\1/p")
    mkdir -p "$(dirname "$root$target")"
    cat >"$root$target"
    ;;
  cp\ \'*)
    src=$(printf '%s\n' "$cmd" | sed -n "s/cp '\([^']*\)' '\([^']*\)'/\1/p")
    dest=$(printf '%s\n' "$cmd" | sed -n "s/cp '\([^']*\)' '\([^']*\)'/\2/p")
    mkdir -p "$(dirname "$root$dest")"
    cp "$root$src" "$root$dest"
    ;;
  rm\ -rf*)
    if [[ "$cmd" == *"/staged-world"* ]]; then
      stack_root=$(printf '%s\n' "$cmd" | sed -n "s/.*rm -rf '\([^']*\)\/staged-world'.*/\1/p")
      rm -rf "$root$stack_root/staged-world" "$root$stack_root/data/execution-world"
      mkdir -p "$root$stack_root/staged-world" "$root$stack_root/data/execution-world"
    else
      stack_root=$(printf '%s\n' "$cmd" | sed -n "s/.*rm -rf '\([^']*\)\/data\/execution-records'.*/\1/p")
      rm -rf "$root$stack_root/data/execution-records" \
        "$root$stack_root/data/storage" \
        "$root$stack_root/data/runtime-root" \
        "$root$stack_root/data/replication-root" \
        "$root$stack_root/output/chain-runtime" \
        "$root$stack_root/output/node-distfs"
      mkdir -p "$root$stack_root/data/execution-records" \
        "$root$stack_root/data/storage" \
        "$root$stack_root/data/runtime-root" \
        "$root$stack_root/data/replication-root" \
        "$root$stack_root/output/chain-runtime" \
        "$root$stack_root/output/node-distfs"
    fi
    ;;
  tar\ -C*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/tar -C '\([^']*\)\/staged-world'.*/\1/p")
    tar -C "$root$stack_root/staged-world" -xf -
    ;;
  cp\ -R*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/cp -R '\([^']*\)\/staged-world\/.' '\([^']*\)\/data\/execution-world\/'.*/\1/p")
    cp -R "$root$stack_root/staged-world/." "$root$stack_root/data/execution-world/"
    ;;
  STACK_ROOT=*python3*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/STACK_ROOT='\([^']*\)'.*/\1/p")
    mapped_cmd=${cmd//STACK_ROOT=\'$stack_root\'/STACK_ROOT=\'$root$stack_root\'}
    bash -c "$mapped_cmd"
    ;;
  systemctl\ stop*)
    ;;
  systemctl\ reset-failed*\;*\ systemctl\ start*|systemctl\ start*)
    ;;
  *)
    echo "unhandled ssh command: $cmd" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$TMP_DIR/bin/ssh"

cat >"$TMP_DIR/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=""
url=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o)
      out=$2
      shift 2
      ;;
    -f|-s|-S|-L|-fsSL|-fsS|-fs)
      shift
      ;;
    *)
      url=$1
      shift
      ;;
  esac
done
case "$url" in
  http://sequencer/status)
    cp "${TEST_STATUS_ROOT:?}/sequencer.json" "$out"
    ;;
  http://storage/status)
    cp "${TEST_STATUS_ROOT:?}/storage.json" "$out"
    ;;
  *)
    echo "unexpected url: $url" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$TMP_DIR/bin/curl"

export PATH="$TMP_DIR/bin:$PATH"
export TEST_REMOTE_ROOT="$TMP_DIR/remote"
export TEST_STATUS_ROOT="$TMP_DIR/status"
export TEST_SSH_LOG="$TMP_DIR/ssh.log"
export SEQ_PASS="sequencer-pass"
export STO_PASS="storage-pass"

for host in root@sequencer root@storage; do
  host_root="$TMP_DIR/remote/$host/opt/oasis7/p2p-testnet"
  mkdir -p "$host_root/config" "$host_root/current/bin"
  cat >"$host_root/config/node.env" <<'EOF'
NODE_ID=triad-testnet-sequencer
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:100
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:old-sequencer-signer,triad-testnet-storage:old-storage-signer
GENESIS_VALIDATOR_REGISTRY_PATH=${STACK_ROOT}/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
EOF
  printf 'runtime-v2' >"$host_root/current/bin/oasis7_chain_runtime"
  cat >"$host_root/DEPLOYED_BUILDINFO" <<'EOF'
commit=test-commit
package_version=0.0.0+testnet.test
run_id=test-run
EOF
done

json=$("$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
  --config-dir "$TMP_DIR/config" \
  --world-dir "$TMP_DIR/world" \
  --sequencer-ssh-host root@sequencer \
  --sequencer-sshpass-env SEQ_PASS \
  --sequencer-service oasis7-triad-sequencer.service \
  --sequencer-status-url http://sequencer/status \
  --storage-ssh-host root@storage \
  --storage-sshpass-env STO_PASS \
  --storage-service oasis7-triad-storage.service \
  --storage-status-url http://storage/status \
  --stack-root /opt/oasis7/p2p-testnet \
  --out-dir "$TMP_DIR/out" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0)

jq -e '
  .sequencer.running == true
  and .sequencer.committed_height == 1
  and .storage.running == true
  and .storage.last_execution_height == 0
' <<<"$json" >/dev/null

test -f "$TMP_DIR/out/rebuild-summary.json"
test -f "$TMP_DIR/remote/root@sequencer/opt/oasis7/p2p-testnet/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
test -f "$TMP_DIR/remote/root@storage/opt/oasis7/p2p-testnet/data/execution-world/snapshot.json"
grep -q '^NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:new-sequencer-signer,triad-testnet-storage:new-storage-signer$' \
  "$TMP_DIR/remote/root@sequencer/opt/oasis7/p2p-testnet/config/node.env"
grep -q '^NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:new-sequencer-signer,triad-testnet-storage:new-storage-signer$' \
  "$TMP_DIR/remote/root@storage/opt/oasis7/p2p-testnet/config/node.env"
expected_runtime_sha=$(printf 'runtime-v2' | shasum -a 256 | awk '{print $1}')
for host in root@sequencer root@storage; do
  for bundle in \
    "$TMP_DIR/remote/$host/opt/oasis7/p2p-testnet/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
    "$TMP_DIR/remote/$host/opt/oasis7/p2p-testnet/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"; do
    jq -e --arg expected "$expected_runtime_sha" '
      .runtime_build.sha256 == $expected
      and .runtime_build.size_bytes == 10
      and .runtime_build.git_commit == "test-commit"
      and .runtime_build.package_version == "0.0.0+testnet.test"
      and .runtime_build.run_id == "test-run"
    ' "$bundle" >/dev/null
  done
done
test -d "$TMP_DIR/remote/root@sequencer/opt/oasis7/p2p-testnet/data/runtime-root"
test -d "$TMP_DIR/remote/root@sequencer/opt/oasis7/p2p-testnet/data/replication-root"
test -d "$TMP_DIR/remote/root@storage/opt/oasis7/p2p-testnet/data/runtime-root"
test -d "$TMP_DIR/remote/root@storage/opt/oasis7/p2p-testnet/data/replication-root"

cat >"$TMP_DIR/status/sequencer.json" <<'JSON'
{
  "running": true,
  "last_error": null,
  "readiness": {
    "status": "not_ready"
  },
  "observability": {
    "storage_challenge_network_degraded": false
  },
  "consensus": {
    "committed_height": 0,
    "last_execution_height": 0,
    "storage_challenge_network_degraded_height": null
  },
  "replication": {
    "connected_peers": []
  }
}
JSON

: >"$TEST_SSH_LOG"
if "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
  --config-dir "$TMP_DIR/config" \
  --world-dir "$TMP_DIR/world" \
  --sequencer-ssh-host root@sequencer \
  --sequencer-sshpass-env SEQ_PASS \
  --sequencer-service oasis7-triad-sequencer.service \
  --sequencer-status-url http://sequencer/status \
  --storage-ssh-host root@storage \
  --storage-sshpass-env STO_PASS \
  --storage-service oasis7-triad-storage.service \
  --storage-status-url http://storage/status \
  --stack-root /opt/oasis7/p2p-testnet \
  --out-dir "$TMP_DIR/out-fail" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-fail.out 2>&1; then
  echo "expected rebuild to fail when sequencer readiness stays not_ready" >&2
  exit 1
fi

python3 - "$TEST_SSH_LOG" <<'PY'
import pathlib
import sys

log_path = pathlib.Path(sys.argv[1])
lines = log_path.read_text(encoding="utf-8").splitlines()
sequencer_commands = [line.split("\t", 1)[1] for line in lines if line.startswith("root@sequencer\t")]
start_indexes = [
    index
    for index, command in enumerate(sequencer_commands)
    if "systemctl start 'oasis7-triad-sequencer.service'" in command
]
cleanup_indexes = [
    index
    for index, command in enumerate(sequencer_commands)
    if "systemctl stop 'oasis7-triad-sequencer.service'" in command
    and "STACK_ROOT='/opt/oasis7/p2p-testnet' python3" in command
    and "oasis7_chain_runtime" in command
    and "start-node.sh" in command
]
if not start_indexes:
    raise SystemExit("missing sequencer start command")
if not any(index > start_indexes[-1] for index in cleanup_indexes):
    raise SystemExit("missing post-start cleanup after failed sequencer readiness")
PY
