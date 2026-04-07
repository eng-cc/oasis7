#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../../.." && pwd)"
script_path="$script_dir/oasis7-run.sh"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

fake_bin="$tmp_dir/bin"
bundle_dir="$tmp_dir/bundle"
mkdir -p "$fake_bin" "$bundle_dir"

cat > "$bundle_dir/run-game.sh" <<'RUN'
#!/usr/bin/env bash
exit 0
RUN
chmod +x "$bundle_dir/run-game.sh"

cat > "$fake_bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
url="${@: -1}"
case "$url" in
  http://127.0.0.1:18789/health)
    printf '{"status":"ok"}\n'
    ;;
  http://127.0.0.1:5841/v1/provider/health)
    printf '{"status":"ok"}\n'
    ;;
  http://127.0.0.1:5841/v1/provider/info)
    printf '{"provider_id":"provider_loopback_http","provider_version":"test","protocol_version":"v1"}\n'
    ;;
  *)
    echo "unexpected curl url: $url" >&2
    exit 22
    ;;
esac
CURL
chmod +x "$fake_bin/curl"

cat > "$fake_bin/openclaw" <<'OPENCLAW'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$#" -ge 3 && "$1" == "agents" && "$2" == "list" && "$3" == "--json" ]]; then
  printf '[{"id":"oasis7_provider_agent","workspace":"fake-workspace","model":"fake-model"}]\n'
  exit 0
fi
echo "unexpected openclaw invocation: $*" >&2
exit 1
OPENCLAW
chmod +x "$fake_bin/openclaw"

sanitized_path="$fake_bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

doctor_json="$(cd "$repo_root" && PATH="$sanitized_path" bash "$script_path" doctor --json --bundle-dir "$bundle_dir" --reuse-bridge --skip-agent-setup)"
DOCTOR_JSON="$doctor_json" python3 - <<'PY'
import json, os
payload = json.loads(os.environ['DOCTOR_JSON'])
assert payload['ok'] is True, payload
checks = {(item['label'], item['level']): item['detail'] for item in payload['checks']}
assert ('command', 'WARN') in checks and 'cargo not found' in checks[('command', 'WARN')], payload
assert ('repo-bootstrap', 'WARN') in checks, payload
assert ('bundle-play', 'OK') in checks and '--reuse-bridge --skip-agent-setup' in checks[('bundle-play', 'OK')], payload
PY

play_stderr="$tmp_dir/play.stderr"
if (cd "$repo_root" && PATH="$sanitized_path" bash "$script_path" play --bundle-dir "$bundle_dir" --skip-agent-setup --no-open-browser > /dev/null 2>"$play_stderr"); then
  echo "expected play command without cargo to fail" >&2
  exit 1
fi

if ! grep -Fq "bundle is valid at $bundle_dir, but repo-backed bridge/bootstrap for 'play' requires cargo" "$play_stderr"; then
  echo "missing actionable cargo error" >&2
  cat "$play_stderr" >&2
  exit 1
fi
if ! grep -Fq -- "--reuse-bridge --skip-agent-setup" "$play_stderr"; then
  echo "missing no-cargo reuse bridge hint" >&2
  cat "$play_stderr" >&2
  exit 1
fi

echo "oasis7-run bootstrap tests passed"
