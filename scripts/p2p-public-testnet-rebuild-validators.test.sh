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

cat >"$TMP_DIR/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'systemctl\t%s\n' "$*" >>"${TEST_SYSTEMCTL_LOG:?}"
effective_host="${TEST_SSH_HOST:-${TEST_SYSTEMD_RESTART_LOOP_HOST:-}}"
case "${1:-}" in
  list-unit-files|list-units)
    IFS=',' read -r -a services <<< "${TEST_SYSTEMD_STACK_OWNER_SERVICES:-}"
    for service in "${services[@]-}"; do
      [[ -n "$service" ]] && printf '%s enabled\n' "$service"
    done
    exit 0
    ;;
  show)
    service="${2:-}"
    IFS=',' read -r -a services <<< "${TEST_SYSTEMD_STACK_OWNER_SERVICES:-}"
    for owner in "${services[@]-}"; do
      if [[ "$service" == "$owner" ]]; then
        stack_root="${TEST_SYSTEMD_RESTART_LOOP_STACK_ROOT:-/opt/oasis7/p2p-testnet}"
        printf 'FragmentPath=/etc/systemd/system/%s\n' "$service"
        printf 'ExecStart={ path=%s/bin/start-node.sh ; argv[]=%s/bin/start-node.sh ; }\n' "$stack_root" "$stack_root"
        printf 'WorkingDirectory=%s\n' "$stack_root"
        exit 0
      fi
    done
    printf 'FragmentPath=/etc/systemd/system/%s\n' "$service"
    printf 'ExecStart=\n'
    printf 'WorkingDirectory=\n'
    exit 0
    ;;
esac
if [[ "${TEST_FAIL_SYSTEMCTL_MASK_HOST:-}" == "$effective_host" && "${1:-}" == "mask" ]]; then
  echo "runtime mask denied" >&2
  exit 23
fi
if [[ "${TEST_SYSTEMD_RESTART_LOOP_HOST:-}" == "$effective_host" ]]; then
  service=${*: -1}
  state_dir="${TEST_REMOTE_ROOT:?}/systemd-${effective_host//[^A-Za-z0-9_.-]/_}-${service//[^A-Za-z0-9_.-]/_}"
  case "$1" in
    mask)
      mkdir -p "$state_dir"
      touch "$state_dir/masked"
      ;;
    unmask)
      mkdir -p "$state_dir"
      rm -f "$state_dir/masked"
      ;;
    start)
      mkdir -p "$state_dir"
      rm -f "$state_dir/killed"
      touch "$state_dir/armed"
      ;;
    stop|reset-failed)
      if [[ -f "$state_dir/armed" && ! -f "$state_dir/killed" && ! -f "$state_dir/masked" ]]; then
        stack_root="${TEST_SYSTEMD_RESTART_LOOP_STACK_ROOT:?}"
        mkdir -p "${TEST_REMOTE_ROOT:?}/${effective_host}${stack_root}/bin"
        printf 'spawned\t%s\t%s\n' "$effective_host" "$service" >>"${TEST_SYSTEMCTL_LOG:?}"
        (
          exec -a "${TEST_REMOTE_ROOT:?}/${effective_host}${stack_root}/bin/start-node.sh" sleep 30
        ) &
      fi
      ;;
    kill)
      mkdir -p "$state_dir"
      touch "$state_dir/killed"
      ;;
  esac
fi
exit 0
EOF
chmod +x "$TMP_DIR/bin/systemctl"

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
  SERVICE_NAME=*STACK_ROOT=*python3*|systemctl\ stop*\;*\ STACK_ROOT=*python3*|STACK_ROOT=*python3*)
    if [[ "${TEST_FAIL_CLEANUP_AFTER_START_HOST:-}" == "$host" ]] \
      && [[ -f "${TEST_REMOTE_ROOT:?}/started-${host//[^A-Za-z0-9_.-]/_}" ]]; then
      exit 43
    fi
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/STACK_ROOT='\([^']*\)'.*/\1/p")
    if [[ "$cmd" == SERVICE_NAME=* ]]; then
      cleanup_cmd="$cmd"
    else
      cleanup_cmd="STACK_ROOT=${cmd#*STACK_ROOT=}"
    fi
    mapped_cmd=${cleanup_cmd//STACK_ROOT=\'$stack_root\'/STACK_ROOT=\'$root$stack_root\'}
    if [[ "${TEST_SPAWN_DELAYED_CLEANUP_PROCESS_HOST:-}" == "$host" ]]; then
      mkdir -p "$root$stack_root/bin"
      (
        sleep 0.4
        exec -a "$root$stack_root/bin/start-node.sh" sleep 30
      ) &
    fi
    if [[ "${TEST_SPAWN_LATE_CLEANUP_PROCESS_HOST:-}" == "$host" ]]; then
      mapped_cmd="CLEANUP_DEADLINE_SECONDS=1 CLEANUP_QUIET_SECONDS=2 $mapped_cmd"
      mkdir -p "$root$stack_root/bin"
      (
        sleep 0.8
        exec -a "$root$stack_root/bin/start-node.sh" sleep 30
      ) &
    fi
    TEST_SSH_HOST="$host" bash -c "$mapped_cmd"
    ;;
  systemctl\ stop*)
    ;;
  systemctl\ unmask*\;*\ systemctl\ reset-failed*\;*\ systemctl\ start*|systemctl\ reset-failed*\;*\ systemctl\ start*|systemctl\ start*)
    touch "${TEST_REMOTE_ROOT:?}/started-${host//[^A-Za-z0-9_.-]/_}"
    if [[ "${TEST_SYSTEMD_RESTART_LOOP_HOST:-}" == "$host" ]]; then
      unmask_service=$(printf '%s\n' "$cmd" | sed -n "s/.*systemctl unmask '\([^']*\)'.*/\1/p")
      if [[ -n "$unmask_service" ]]; then
        TEST_SSH_HOST="$host" systemctl unmask "$unmask_service"
      fi
      service=$(printf '%s\n' "$cmd" | sed -n "s/.*systemctl start '\([^']*\)'.*/\1/p")
      if [[ -n "$service" ]]; then
        TEST_SSH_HOST="$host" systemctl start "$service"
      fi
    fi
    if [[ "${TEST_FAIL_START_HOST:-}" == "$host" ]]; then
      exit 42
    fi
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
    if [[ -n "${TEST_STORAGE_STATUS_OVERRIDE:-}" ]]; then
      cp "$TEST_STORAGE_STATUS_OVERRIDE" "$out"
    else
      cp "${TEST_STATUS_ROOT:?}/storage.json" "$out"
    fi
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
export TEST_SYSTEMCTL_LOG="$TMP_DIR/systemctl.log"
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
    if (
        "SERVICE_NAME='oasis7-triad-sequencer.service'" in command
        or "systemctl stop 'oasis7-triad-sequencer.service'" in command
    )
    and "STACK_ROOT='/opt/oasis7/p2p-testnet' python3" in command
    and "oasis7_chain_runtime" in command
    and "start-node.sh" in command
]
if not start_indexes:
    raise SystemExit("missing sequencer start command")
if not any(index > start_indexes[-1] for index in cleanup_indexes):
    raise SystemExit("missing post-start cleanup after failed sequencer readiness")
PY

: >"$TEST_SSH_LOG"
: >"$TEST_SYSTEMCTL_LOG"
rm -f "$TEST_REMOTE_ROOT"/started-*
if TEST_SYSTEMD_RESTART_LOOP_HOST=root@sequencer \
  TEST_SYSTEMD_RESTART_LOOP_STACK_ROOT=/opt/oasis7/p2p-testnet \
  "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-delayed-cleanup" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-delayed-cleanup.out 2>&1; then
  :
fi

if pgrep -f "$TEST_REMOTE_ROOT/root@sequencer/opt/oasis7/p2p-testnet/bin/start-node.sh" >/dev/null; then
  pkill -f "$TEST_REMOTE_ROOT/root@sequencer/opt/oasis7/p2p-testnet/bin/start-node.sh" || true
  echo "systemd restart-loop cleanup process survived stable quiet cleanup" >&2
  exit 1
fi
if [[ ! -f "$TEST_REMOTE_ROOT/systemd-root_sequencer-oasis7-triad-sequencer.service/masked" ]]; then
  echo "expected failed cleanup path to leave sequencer service runtime-masked" >&2
  exit 1
fi

python3 - "$TEST_SSH_LOG" <<'PY'
import pathlib
import sys

log_path = pathlib.Path(sys.argv[1])
lines = log_path.read_text(encoding="utf-8").splitlines()
sequencer_commands = [line.split("\t", 1)[1] for line in lines if line.startswith("root@sequencer\t")]
if not any(
    (
        "SERVICE_NAME='oasis7-triad-sequencer.service'" in command
        or "systemctl stop 'oasis7-triad-sequencer.service'" in command
    )
    and "STACK_ROOT='/opt/oasis7/p2p-testnet' python3" in command
    and "oasis7_chain_runtime" in command
    and "start-node.sh" in command
    for command in sequencer_commands
):
    raise SystemExit("missing sequencer cleanup command for delayed cleanup process test")
PY

python3 - "$TEST_SYSTEMCTL_LOG" <<'PY'
import pathlib
import sys

log_path = pathlib.Path(sys.argv[1])
lines = log_path.read_text(encoding="utf-8").splitlines()
unmask_count = sum(
    1
    for line in lines
    if line == "systemctl\tunmask oasis7-triad-sequencer.service"
)
mask_count = sum(
    1
    for line in lines
    if line == "systemctl\tmask --runtime oasis7-triad-sequencer.service"
)
stop_count = sum(
    1
    for line in lines
    if line == "systemctl\tstop oasis7-triad-sequencer.service"
)
kill_count = sum(
    1
    for line in lines
    if line == "systemctl\tkill --kill-who=all --signal=SIGKILL oasis7-triad-sequencer.service"
)
reset_count = sum(
    1
    for line in lines
    if line == "systemctl\treset-failed oasis7-triad-sequencer.service"
)
if stop_count < 2 or kill_count < 2 or reset_count < 2:
    raise SystemExit(
        "cleanup did not repeatedly quiesce systemd while waiting for a stable quiet window"
    )
if unmask_count < 1:
    raise SystemExit("start path did not unmask the sequencer service before systemctl start")
if mask_count < 2:
    raise SystemExit("cleanup did not repeatedly mask the sequencer service while waiting for quiet")
if any(line == "spawned\troot@sequencer\toasis7-triad-sequencer.service" for line in lines):
    raise SystemExit("fake systemd restart-loop spawned despite runtime service mask")
PY

: >"$TEST_SSH_LOG"
: >"$TEST_SYSTEMCTL_LOG"
rm -f "$TEST_REMOTE_ROOT"/started-*
legacy_state_dir="$TEST_REMOTE_ROOT/systemd-root_sequencer-oasis7-triad-sequencer.service"
mkdir -p "$legacy_state_dir"
touch "$legacy_state_dir/armed"
if TEST_SYSTEMD_RESTART_LOOP_HOST=root@sequencer \
  TEST_SYSTEMD_RESTART_LOOP_STACK_ROOT=/opt/oasis7/p2p-testnet \
  TEST_SYSTEMD_STACK_OWNER_SERVICES=oasis7-testnet-sequencer.service,oasis7-triad-sequencer.service \
  "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
  --config-dir "$TMP_DIR/config" \
  --world-dir "$TMP_DIR/world" \
  --sequencer-ssh-host root@sequencer \
  --sequencer-sshpass-env SEQ_PASS \
  --sequencer-service oasis7-testnet-sequencer.service \
  --sequencer-status-url http://sequencer/status \
  --storage-ssh-host root@storage \
  --storage-sshpass-env STO_PASS \
  --storage-service oasis7-testnet-storage.service \
  --storage-status-url http://storage/status \
  --stack-root /opt/oasis7/p2p-testnet \
  --out-dir "$TMP_DIR/out-stack-owner-services" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-stack-owner-services.out 2>&1; then
  :
fi

python3 - "$TEST_SYSTEMCTL_LOG" <<'PY'
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
required = {
    "systemctl\tmask --runtime oasis7-testnet-sequencer.service",
    "systemctl\tmask --runtime oasis7-triad-sequencer.service",
}
missing = sorted(required - set(lines))
if missing:
    raise SystemExit(f"missing stack-owner runtime mask calls: {missing}")
if "systemctl\tunmask oasis7-triad-sequencer.service" in lines:
    raise SystemExit("legacy stack-owner service was unmasked by explicit start path")
if any(line == "spawned\troot@sequencer\toasis7-triad-sequencer.service" for line in lines):
    raise SystemExit("legacy stack-owner service respawned during cleanup")
PY

: >"$TEST_SSH_LOG"
rm -f "$TEST_REMOTE_ROOT"/started-*
if TEST_SPAWN_LATE_CLEANUP_PROCESS_HOST=root@sequencer "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-late-cleanup" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-late-cleanup.out 2>&1; then
  echo "expected rebuild cleanup to fail when stable quiet window is not observed" >&2
  exit 1
fi
grep -q "cleanup failed: stable quiet window was not observed before deadline" \
  /tmp/oasis7-rebuild-validators-late-cleanup.out
pkill -f "$TEST_REMOTE_ROOT/root@sequencer/opt/oasis7/p2p-testnet/bin/start-node.sh" || true

: >"$TEST_SSH_LOG"
if TEST_FAIL_SYSTEMCTL_MASK_HOST=root@sequencer "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-mask-fail" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-mask-fail.out 2>&1; then
  echo "expected rebuild cleanup to fail when systemctl runtime mask fails" >&2
  exit 1
fi
grep -q "cleanup failed: systemctl runtime mask failed for oasis7-triad-sequencer.service: runtime mask denied" \
  /tmp/oasis7-rebuild-validators-mask-fail.out

: >"$TEST_SSH_LOG"
if TEST_FAIL_START_HOST=root@sequencer "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-start-fail" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-start-fail.out 2>&1; then
  echo "expected rebuild to fail when sequencer systemctl start fails" >&2
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
    if (
        "SERVICE_NAME='oasis7-triad-sequencer.service'" in command
        or "systemctl stop 'oasis7-triad-sequencer.service'" in command
    )
    and "STACK_ROOT='/opt/oasis7/p2p-testnet' python3" in command
    and "oasis7_chain_runtime" in command
    and "start-node.sh" in command
]
if not start_indexes:
    raise SystemExit("missing sequencer start command for start-failure path")
if not any(index > start_indexes[-1] for index in cleanup_indexes):
    raise SystemExit("missing post-start cleanup after failed sequencer systemctl start")
PY

: >"$TEST_SSH_LOG"
rm -f "$TEST_REMOTE_ROOT"/started-*
if TEST_FAIL_CLEANUP_AFTER_START_HOST=root@sequencer "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-cleanup-fail" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-cleanup-fail.out 2>&1; then
  echo "expected rebuild to fail when post-start cleanup command fails" >&2
  exit 1
fi

grep -q "sequencer readiness failed checks after restart and cleanup failed" \
  /tmp/oasis7-rebuild-validators-cleanup-fail.out

cat >"$TMP_DIR/status/storage-not-ready.json" <<'JSON'
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
    "storage_challenge_network_degraded_height": null,
    "network_head": {
      "height": null
    }
  },
  "replication": {
    "connected_peers": []
  }
}
JSON

: >"$TEST_SSH_LOG"
rm -f "$TEST_REMOTE_ROOT"/started-*
if TEST_STORAGE_STATUS_OVERRIDE="$TMP_DIR/status/storage-not-ready.json" "$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
  --out-dir "$TMP_DIR/out-storage-fail" \
  --poll-attempts 1 \
  --poll-sleep-seconds 0 >/tmp/oasis7-rebuild-validators-storage-fail.out 2>&1; then
  echo "expected rebuild to fail when storage readiness stays not_ready" >&2
  exit 1
fi

python3 - "$TEST_SSH_LOG" <<'PY'
import pathlib
import sys

log_path = pathlib.Path(sys.argv[1])
lines = log_path.read_text(encoding="utf-8").splitlines()

def commands_for(host: str) -> list[str]:
    prefix = f"{host}\t"
    return [line.split("\t", 1)[1] for line in lines if line.startswith(prefix)]

for host, service in (
    ("root@sequencer", "oasis7-triad-sequencer.service"),
    ("root@storage", "oasis7-triad-storage.service"),
):
    commands = commands_for(host)
    start_indexes = [
        index
        for index, command in enumerate(commands)
        if f"systemctl start '{service}'" in command
    ]
    cleanup_indexes = [
        index
        for index, command in enumerate(commands)
        if (
            f"SERVICE_NAME='{service}'" in command
            or f"systemctl stop '{service}'" in command
        )
        and "STACK_ROOT='/opt/oasis7/p2p-testnet' python3" in command
        and "oasis7_chain_runtime" in command
        and "start-node.sh" in command
    ]
    if not start_indexes:
        raise SystemExit(f"missing {host} start command for storage-failure path")
    if not any(index > start_indexes[-1] for index in cleanup_indexes):
        raise SystemExit(f"missing post-start cleanup for {host} after storage readiness failure")
PY
