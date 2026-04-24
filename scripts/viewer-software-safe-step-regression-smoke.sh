#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/agent-browser-lib.sh"

ensure_file_contains() {
  local file=$1
  local pattern=$2
  if ! rg -F -q -- "$pattern" "$file"; then
    echo "error: pattern not found: $pattern" >&2
    echo "  file=$file" >&2
    exit 1
  fi
}

wait_for_file() {
  local path=$1
  local timeout_secs=${2:-10}
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if [[ -s "$path" ]]; then
      return 0
    fi
    sleep 1
  done
  return 1
}

require_cmd python3
require_cmd rg
ab_require

tmpdir=$(mktemp -d)
fixture_root="$tmpdir/fixture"
out_root="$tmpdir/output"
http_port_file="$tmpdir/http-port"
ws_port_file="$tmpdir/ws-port"
http_pid=""
ws_pid=""

cleanup() {
  local exit_code=$?
  set +e
  if [[ -n "$http_pid" ]]; then
    kill "$http_pid" >/dev/null 2>&1 || true
    wait "$http_pid" >/dev/null 2>&1 || true
  fi
  if [[ -n "$ws_pid" ]]; then
    kill "$ws_pid" >/dev/null 2>&1 || true
    wait "$ws_pid" >/dev/null 2>&1 || true
  fi
  rm -rf "$tmpdir"
  exit "$exit_code"
}
trap cleanup EXIT

mkdir -p "$fixture_root"

cat >"$fixture_root/software_safe.html" <<'EOF'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>software_safe smoke fixture</title>
    <style>
      body {
        font-family: sans-serif;
        margin: 24px;
        line-height: 1.5;
      }
      .card {
        border: 1px solid #d0d7de;
        border-radius: 8px;
        margin-top: 12px;
        padding: 12px;
      }
      .selected {
        background: #eef6ff;
      }
    </style>
  </head>
  <body>
    <h1>Formal Gameplay Summary</h1>
    <div class="card">Recent Events</div>
    <div id="connection" class="card"></div>
    <div id="metrics" class="card"></div>
    <div id="selection-root" class="card"></div>
    <div id="blocker-root" class="card"></div>
    <script>
      (() => {
        const state = {
          connectionStatus: "connecting",
          renderMode: "software_safe",
          lastError: "",
          selectedId: null,
          selectedKind: null,
          logicalTime: 0,
          eventSeq: 0,
          gameplaySummary: {
            stageStatus: "running",
            blockerKind: null,
            blockerDetail: null,
          },
        };

        function cloneState() {
          return JSON.parse(JSON.stringify(state));
        }

        function render() {
          document.getElementById("connection").textContent =
            `connection=${state.connectionStatus}`;
          document.getElementById("metrics").textContent =
            `logicalTime=${state.logicalTime} eventSeq=${state.eventSeq}`;
          const selectionRoot = document.getElementById("selection-root");
          const blockerRoot = document.getElementById("blocker-root");
          if (state.selectedId) {
            selectionRoot.innerHTML = `
              <div
                class="selected"
                data-select-kind="${state.selectedKind}"
                data-select-id="${state.selectedId}"
                data-selected="true"
              >
                Selected agent ${state.selectedId}
              </div>
            `;
          } else {
            selectionRoot.textContent = "No agent selected yet.";
          }
          if (state.gameplaySummary.stageStatus === "blocked") {
            blockerRoot.textContent =
              `blocked ${state.gameplaySummary.blockerKind} ${state.gameplaySummary.blockerDetail}`;
          } else {
            blockerRoot.textContent = "No blocker active.";
          }
        }

        window.__AW_TEST__ = {
          getState() {
            return cloneState();
          },
          select(target) {
            const [kind, id] = String(target || "").split(":");
            state.selectedKind = kind || null;
            state.selectedId = id || null;
            state.gameplaySummary.stageStatus = "blocked";
            state.gameplaySummary.blockerKind = "fixture_blocker";
            state.gameplaySummary.blockerDetail = "fixture smoke blocker";
            render();
            return cloneState();
          },
        };

        render();
        window.setTimeout(() => {
          state.connectionStatus = "connected";
          render();
        }, 120);
      })();
    </script>
  </body>
</html>
EOF

cat >"$tmpdir/http_server.py" <<'PY'
from __future__ import annotations

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
port_file = Path(sys.argv[2]).resolve()


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, format: str, *args: object) -> None:
        return


server = ThreadingHTTPServer(
    ("127.0.0.1", 0),
    partial(QuietHandler, directory=str(root)),
)
port_file.write_text(str(server.server_address[1]), encoding="utf-8")
server.serve_forever()
PY

cat >"$tmpdir/ws_listener.py" <<'PY'
from __future__ import annotations

from pathlib import Path
import signal
import socket
import sys

port_file = Path(sys.argv[1]).resolve()
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("127.0.0.1", 0))
sock.listen()
port_file.write_text(str(sock.getsockname()[1]), encoding="utf-8")


def shutdown(*_args: object) -> None:
    try:
        sock.close()
    finally:
        raise SystemExit(0)


signal.signal(signal.SIGTERM, shutdown)
signal.signal(signal.SIGINT, shutdown)

while True:
  try:
    conn, _addr = sock.accept()
  except OSError:
    break
  conn.close()
PY

python3 "$tmpdir/http_server.py" "$fixture_root" "$http_port_file" >/dev/null 2>&1 &
http_pid=$!
python3 "$tmpdir/ws_listener.py" "$ws_port_file" >/dev/null 2>&1 &
ws_pid=$!

wait_for_file "$http_port_file" 10 || { echo "error: fixture http server did not report a port" >&2; exit 1; }
wait_for_file "$ws_port_file" 10 || { echo "error: fake websocket listener did not report a port" >&2; exit 1; }

http_port=$(tr -d '\r\n' <"$http_port_file")
ws_port=$(tr -d '\r\n' <"$ws_port_file")
[[ "$http_port" =~ ^[0-9]+$ ]] || { echo "error: invalid fixture http port: $http_port" >&2; exit 1; }
[[ "$ws_port" =~ ^[0-9]+$ ]] || { echo "error: invalid fake websocket port: $ws_port" >&2; exit 1; }

fixture_url="http://127.0.0.1:${http_port}/software_safe.html?ws=ws://127.0.0.1:${ws_port}"

./scripts/viewer-software-safe-step-regression.sh \
  --url "$fixture_url" \
  --out-dir "$out_root" \
  --progress-timeout-ms 5000

summary_json=$(find "$out_root" -type f -name 'software-safe-step-summary.json' | sort | tail -n 1)
after_select_json=$(find "$out_root" -type f -name 'after_select_state.json' | sort | tail -n 1)
after_progress_json=$(find "$out_root" -type f -name 'after_progress_state.json' | sort | tail -n 1)

[[ -n "$summary_json" && -f "$summary_json" ]] || { echo "error: missing software-safe-step-summary.json" >&2; exit 1; }
[[ -n "$after_select_json" && -f "$after_select_json" ]] || { echo "error: missing after_select_state.json" >&2; exit 1; }
[[ -n "$after_progress_json" && -f "$after_progress_json" ]] || { echo "error: missing after_progress_state.json" >&2; exit 1; }

python3 - "$summary_json" "$after_select_json" "$after_progress_json" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

summary = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
after_select = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
after_progress = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))

assert summary["ok"] is True, summary
assert summary["renderMode"] == "software_safe", summary
assert summary["autoProgressObserved"] is False, summary
assert summary["logicalTimeAdvanced"] is False, summary
assert summary["eventSeqAdvanced"] is False, summary
assert summary["blockerDomVisible"] is True, summary
assert summary["stageStatus"] == "blocked", summary
assert summary["blockerKind"] == "fixture_blocker", summary
assert summary["selectedAgentVisible"] is True, summary
assert summary["playbackControlsVisible"] is False, summary
assert after_select["selectedId"] == "agent-0", after_select
assert after_select["selectedKind"] == "agent", after_select
assert after_progress["connectionStatus"] == "connected", after_progress
assert after_progress["gameplaySummary"]["stageStatus"] == "blocked", after_progress
assert after_progress["gameplaySummary"]["blockerKind"] == "fixture_blocker", after_progress
PY

ensure_file_contains "$summary_json" '"failCategory": null'

echo "viewer software_safe step regression smoke checks passed"
