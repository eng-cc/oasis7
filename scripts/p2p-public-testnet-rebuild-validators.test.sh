#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-rebuild-validators-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/bin" "$TMP_DIR/config" "$TMP_DIR/world" "$TMP_DIR/remote" "$TMP_DIR/status"

cat >"$TMP_DIR/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{"ok":true}
EOF

cat >"$TMP_DIR/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" <<'EOF'
{"manifest":true}
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
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/.*rm -rf '\([^']*\)\/staged-world'.*/\1/p")
    rm -rf "$root$stack_root/staged-world" "$root$stack_root/data/execution-world"
    mkdir -p "$root$stack_root/staged-world" "$root$stack_root/data/execution-world"
    ;;
  tar\ -C*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/tar -C '\([^']*\)\/staged-world'.*/\1/p")
    tar -C "$root$stack_root/staged-world" -xf -
    ;;
  cp\ -R*)
    stack_root=$(printf '%s\n' "$cmd" | sed -n "s/cp -R '\([^']*\)\/staged-world\/.' '\([^']*\)\/data\/execution-world\/'.*/\1/p")
    cp -R "$root$stack_root/staged-world/." "$root$stack_root/data/execution-world/"
    ;;
  systemctl\ stop*)
    ;;
  systemctl\ start*)
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
export SEQ_PASS="sequencer-pass"
export STO_PASS="storage-pass"

json=$("$ROOT_DIR/scripts/p2p-public-testnet-rebuild-validators.sh" \
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
