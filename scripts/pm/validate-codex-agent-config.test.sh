#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-codex-agent-config-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

if ! TOML_PYTHON="$($ROOT_DIR/scripts/pm/find-python-with-module.sh tomllib)"; then
  echo "validate-codex-agent-config.test: no Python 3.11+ stdlib tomllib interpreter available" >&2
  exit 1
fi

mkdir -p "$TMP_DIR/future-python-bin"
ln -s "$TOML_PYTHON" "$TMP_DIR/future-python-bin/python42"
future_python="$(PATH="$TMP_DIR/future-python-bin:/usr/bin:/bin" \
  "$ROOT_DIR/scripts/pm/find-python-with-module.sh" tomllib)"
if [[ "$(basename "$future_python")" != "python42" ]]; then
  echo "validate-codex-agent-config.test: generic future Python discovery failed: $future_python" >&2
  exit 1
fi
printf 'interpreter discovery passed: %s\n' "$future_python"

new_fixture() {
  FIXTURE="$TMP_DIR/$1"
  mkdir -p "$FIXTURE/.codex" "$FIXTURE/.agents/roles" "$FIXTURE/doc/engineering/workflow"
  cp -R "$ROOT_DIR/.codex/agents" "$FIXTURE/.codex/agents"
  cp "$ROOT_DIR/.codex/config.toml" "$FIXTURE/.codex/config.toml"
  cp "$ROOT_DIR/doc/engineering/workflow/source-of-truth.md" \
    "$FIXTURE/doc/engineering/workflow/source-of-truth.md"
  cp "$ROOT_DIR"/.agents/roles/*.md "$FIXTURE/.agents/roles/"
}

rewrite() {
  local path="$1"
  local old="$2"
  local new="$3"
  "$TOML_PYTHON" - "$path" "$old" "$new" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
old, new = sys.argv[2], sys.argv[3]
if old not in text:
    raise SystemExit(f"fixture marker not found: {old}")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
PY
}

expect_fail() {
  local name="$1"
  local fixture="$2"
  local expected_message="${3:-}"
  if "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" --skip-native-probe >"$TMP_DIR/$name.out" 2>"$TMP_DIR/$name.err"; then
    echo "validate-codex-agent-config.test: expected failure for $name" >&2
    exit 1
  fi
  if [[ -n "$expected_message" ]] && ! grep -Fq "$expected_message" "$TMP_DIR/$name.err"; then
    echo "validate-codex-agent-config.test: $name did not report expected diagnostic: $expected_message" >&2
    cat "$TMP_DIR/$name.err" >&2
    exit 1
  fi
  printf 'negative case passed: %s\n' "$name"
}

new_fixture baseline
fixture="$FIXTURE"
"$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
  --root "$fixture" --skip-native-probe >/dev/null
printf 'positive case passed: baseline\n'

# The executable validator is an operator-facing entrypoint.  It must recover
# when PATH's conventional python3 is too old for tomllib, provided the
# repository's generic interpreter finder can discover a compatible runtime.
# Keep python42 after /usr/bin so the shebang reaches the known old python3,
# while the finder can still enumerate the compatible fixture interpreter.
if ! PATH="/usr/bin:/bin:$TMP_DIR/future-python-bin" \
  "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" --skip-native-probe >"$TMP_DIR/direct-entrypoint.out" \
    2>"$TMP_DIR/direct-entrypoint.err"; then
  echo "validate-codex-agent-config.test: direct validator did not discover a compatible tomllib interpreter" >&2
  cat "$TMP_DIR/direct-entrypoint.err" >&2
  exit 1
fi
printf 'positive case passed: direct entrypoint interpreter discovery\n'

new_fixture wrong-adapter-model
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/runtime_engineer.toml" \
  'model = "gpt-5.6-terra"' \
  'model = "gpt-5.5"'
expect_fail wrong_adapter_model "$fixture"

new_fixture wrong-adapter-reasoning
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/runtime_engineer.toml" \
  'model_reasoning_effort = "medium"' \
  'model_reasoning_effort = "high"'
expect_fail wrong_adapter_reasoning "$fixture"

new_fixture unexpected-adapter-key
fixture="$FIXTURE"
printf 'unexpected = true\n' >> "$fixture/.codex/agents/runtime_engineer.toml"
expect_fail unexpected_adapter_key "$fixture"

new_fixture invalid-toml
fixture="$FIXTURE"
printf 'invalid = [\n' >> "$fixture/.codex/agents/runtime_engineer.toml"
expect_fail invalid_trailing_toml "$fixture"

new_fixture symlinked-adapter
fixture="$FIXTURE"
mv "$fixture/.codex/agents/runtime_engineer.toml" "$fixture/runtime-engineer.toml"
ln -s "$fixture/runtime-engineer.toml" "$fixture/.codex/agents/runtime_engineer.toml"
expect_fail symlinked_adapter "$fixture" \
  "must be a regular file and not a symlink"

new_fixture swapped-path
fixture="$FIXTURE"
rewrite "$fixture/.codex/config.toml" \
  'config_file = "agents/gameplay_designer.toml"' \
  'config_file = "agents/runtime_engineer.toml"'
expect_fail swapped_role_path "$fixture"

new_fixture unexpected-registered-role
fixture="$FIXTURE"
cat >> "$fixture/.codex/config.toml" <<'TOML'

[agents.unapproved_specialist]
description = "Unexpected specialist role fixture."
config_file = "agents/unapproved_specialist.toml"
TOML
expect_fail unexpected_registered_role "$fixture" \
  ".codex/config.toml must register exactly the eleven specialist roles and not tpm"

new_fixture wrong-role
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/gameplay_designer.toml" \
  'oasis7 gameplay_designer bounded specialist subagent' \
  'oasis7 runtime_engineer bounded specialist subagent'
expect_fail wrong_role_instructions "$fixture"

new_fixture semantic-swap
fixture="$FIXTURE"
cp "$fixture/.codex/agents/runtime_engineer.toml" \
  "$fixture/.codex/agents/gameplay_designer.toml"
rewrite "$fixture/.codex/agents/gameplay_designer.toml" \
  'oasis7 runtime_engineer bounded specialist subagent' \
  'oasis7 gameplay_designer bounded specialist subagent'
rewrite "$fixture/.codex/agents/gameplay_designer.toml" \
  '.agents/roles/runtime_engineer.md' \
  '.agents/roles/gameplay_designer.md'
expect_fail semantic_role_body_swap "$fixture"

new_fixture broadened-agent
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/agent_engineer.toml" \
  'Own in-world Agent perception, memory retrieval, planning, execution,' \
  'Own provider configuration. Own in-world Agent perception, memory retrieval, planning, execution,'
expect_fail agent_role_broadening "$fixture"

new_fixture broadened-producer
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/producer_system_designer.toml" \
  'Own product goals, world rules, economy and governance semantics,' \
  'Own runtime implementation. Own product goals, world rules, economy and governance semantics,'
expect_fail producer_role_broadening "$fixture"

new_fixture broadened-repository-health
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/repository_health_engineer.toml" \
  'Own repository-governance alignment,' \
  'Own live subagent dispatch. Own repository-governance alignment,'
expect_fail repository_health_role_broadening "$fixture"

new_fixture gameplay-role-card-projection-drift
fixture="$FIXTURE"
rewrite "$fixture/.agents/roles/gameplay_designer.md" \
  'Own gameplay loops, player verbs, progression, rewards, failure recovery, balance risks, abuse paths, and playability acceptance.' \
  'Own runtime implementation and generic tooling.'
expect_fail gameplay_role_card_projection_drift "$fixture"

new_fixture fake-paired-hash-bypass
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/gameplay_designer.toml" \
  'Own gameplay loops, player verbs, progression, rewards, failure recovery, balance risks, abuse paths, and playability acceptance.' \
  'Own runtime and server implementation.'
rewrite "$fixture/.agents/roles/gameplay_designer.md" \
  '## Codex Adapter Projection' \
  $'## Codex Adapter Binding\n- Adapter Instructions SHA256: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`\n\n## Codex Adapter Projection'
expect_fail fake_paired_hash_cannot_authorize_body_swap "$fixture"

new_fixture registry-description-drift
fixture="$FIXTURE"
rewrite "$fixture/.codex/config.toml" \
  'description = "In-world Agent perception, memory, planning, execution, feedback, behavior stability, evaluation, and inference cost."' \
  'description = "Generic Codex agent tooling and providers."'
expect_fail registry_description_role_mismatch "$fixture"

new_fixture missing-marker
fixture="$FIXTURE"
rewrite "$fixture/.codex/agents/qa_engineer.toml" \
  'Treat third_party as read-only.' \
  'Third-party code is outside this slice.'
expect_fail missing_mandatory_marker "$fixture"

new_fixture missing-role-card
fixture="$FIXTURE"
rm -f "$fixture/.agents/roles/runtime_engineer.md"
expect_fail missing_role_card "$fixture" \
  'cannot read role card'

new_fixture root-pin
fixture="$FIXTURE"
rewrite "$fixture/.codex/config.toml" \
  'sandbox_mode = "danger-full-access"' \
  $'sandbox_mode = "danger-full-access"\nmodel = "gpt-5.5"'
expect_fail unintended_root_model_pin "$fixture"

new_fixture missing-capability-boundary
fixture="$FIXTURE"
rewrite "$fixture/doc/engineering/workflow/source-of-truth.md" \
  'Adapter registration is not proof of adapter activation.' \
  'Adapter registration is evidence of adapter activation.'
expect_fail missing_source_of_truth_capability_boundary "$fixture"

FAKE_BIN="$TMP_DIR/fake-bin"
FAKE_PID_FILE="$TMP_DIR/fake-codex.pid"
mkdir -p "$FAKE_BIN"
cat >"$FAKE_BIN/codex" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$$" >"$FAKE_CODEX_PID_FILE"
case "${FAKE_CODEX_MODE:-exit}" in
  hang) sleep 60 ;;
  hold_stdout)
    sleep 60 &
    printf '%s\n' "$!" >"$FAKE_CODEX_CHILD_PID_FILE"
    exit 7
    ;;
  delayed_response)
    exec 3<&0
    (
      read -r _ <&3 || true
      sleep "${FAKE_CODEX_RESPONSE_DELAY_SECONDS:-0.25}"
      cat "$FAKE_CODEX_DELAYED_RESPONSE_FILE"
      read -r _ <&3 || true
      read -r _ <&3 || true
    ) &
    printf '%s\n' "$!" >"$FAKE_CODEX_CHILD_PID_FILE"
    exit 7
    ;;
  exit) echo "fake early exit" >&2; exit 7 ;;
  *) exit 9 ;;
esac
SH
chmod +x "$FAKE_BIN/codex"

assert_pid_not_live() {
  local pid_file="$1"
  local label="$2"
  [[ -f "$pid_file" ]] || return 0
  local pid
  pid="$(cat "$pid_file")"
  for _ in {1..50}; do
    if ! kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
    sleep 0.02
  done
  echo "validate-codex-agent-config.test: leaked fake Codex PID $pid after $label" >&2
  exit 1
}

terminate_and_assert_pid_not_live() {
  local pid_file="$1"
  local label="$2"
  [[ -f "$pid_file" ]] || {
    echo "validate-codex-agent-config.test: missing fake Codex child PID for $label" >&2
    exit 1
  }
  local pid
  pid="$(cat "$pid_file")"
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
  fi
  assert_pid_not_live "$pid_file" "$label"
}

new_fixture native-lifecycle
fixture="$FIXTURE"
for invalid_timeout in invalid 0 301; do
  rm -f "$FAKE_PID_FILE"
  if PATH="$FAKE_BIN:$PATH" FAKE_CODEX_PID_FILE="$FAKE_PID_FILE" \
    CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS="$invalid_timeout" \
    "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
      --root "$fixture" >"$TMP_DIR/timeout-$invalid_timeout.out" \
      2>"$TMP_DIR/timeout-$invalid_timeout.err"; then
    echo "validate-codex-agent-config.test: expected timeout validation failure: $invalid_timeout" >&2
    exit 1
  fi
  if [[ -f "$FAKE_PID_FILE" ]]; then
    echo "validate-codex-agent-config.test: invalid timeout spawned Codex: $invalid_timeout" >&2
    exit 1
  fi
done
printf 'native timeout validation occurs before spawn\n'

rm -f "$FAKE_PID_FILE"
if PATH="$FAKE_BIN:$PATH" FAKE_CODEX_PID_FILE="$FAKE_PID_FILE" \
  FAKE_CODEX_MODE=hang CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS=1 \
  "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" >"$TMP_DIR/native-timeout.out" 2>"$TMP_DIR/native-timeout.err"; then
  echo "validate-codex-agent-config.test: expected native response timeout" >&2
  exit 1
fi
grep -F "timed out after 1s" "$TMP_DIR/native-timeout.err" >/dev/null
assert_pid_not_live "$FAKE_PID_FILE" response-timeout
printf 'native response-timeout process cleanup passed\n'

rm -f "$FAKE_PID_FILE"
if PATH="$FAKE_BIN:$PATH" FAKE_CODEX_PID_FILE="$FAKE_PID_FILE" \
  FAKE_CODEX_MODE=exit CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS=5 \
  "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" >"$TMP_DIR/native-exit.out" 2>"$TMP_DIR/native-exit.err"; then
  echo "validate-codex-agent-config.test: expected native early exit" >&2
  exit 1
fi
grep -F "exited before response 1" "$TMP_DIR/native-exit.err" >/dev/null
assert_pid_not_live "$FAKE_PID_FILE" early-exit
printf 'native early-exit process cleanup passed\n'

FAKE_CODEX_CHILD_PID_FILE="$TMP_DIR/fake-codex-child.pid"
rm -f "$FAKE_PID_FILE" "$FAKE_CODEX_CHILD_PID_FILE"
if PATH="$FAKE_BIN:$PATH" FAKE_CODEX_PID_FILE="$FAKE_PID_FILE" \
  FAKE_CODEX_CHILD_PID_FILE="$FAKE_CODEX_CHILD_PID_FILE" FAKE_CODEX_MODE=hold_stdout \
  CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS=1 \
  "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" >"$TMP_DIR/native-held-stdout.out" \
    2>"$TMP_DIR/native-held-stdout.err"; then
  terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" held-stdout
  echo "validate-codex-agent-config.test: expected native held-stdout early exit" >&2
  exit 1
fi
if ! grep -F "exited before response 1" "$TMP_DIR/native-held-stdout.err" >/dev/null; then
  terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" held-stdout
  echo "validate-codex-agent-config.test: held-stdout parent exit was not classified as early exit" >&2
  cat "$TMP_DIR/native-held-stdout.err" >&2
  exit 1
fi
terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" held-stdout
printf 'native held-stdout early-exit process cleanup passed\n'

FAKE_CODEX_DELAYED_RESPONSE_FILE="$TMP_DIR/fake-codex-delayed-response.jsonl"
FAKE_CODEX_CHILD_PID_FILE="$TMP_DIR/fake-codex-delayed-response-child.pid"
printf '%s\n' '{"id":1,"result":{}}' >"$FAKE_CODEX_DELAYED_RESPONSE_FILE"
rm -f "$FAKE_PID_FILE" "$FAKE_CODEX_CHILD_PID_FILE"
if PATH="$FAKE_BIN:$PATH" FAKE_CODEX_PID_FILE="$FAKE_PID_FILE" \
  FAKE_CODEX_CHILD_PID_FILE="$FAKE_CODEX_CHILD_PID_FILE" \
  FAKE_CODEX_DELAYED_RESPONSE_FILE="$FAKE_CODEX_DELAYED_RESPONSE_FILE" \
  FAKE_CODEX_MODE=delayed_response CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS=1 \
  "$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
    --root "$fixture" >"$TMP_DIR/native-delayed-response.out" \
    2>"$TMP_DIR/native-delayed-response.err"; then
  terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" delayed-response
  echo "validate-codex-agent-config.test: expected delayed initialize response to leave config/read unanswered" >&2
  exit 1
else
  if ! grep -F "exited before response 2" "$TMP_DIR/native-delayed-response.err" >/dev/null; then
    terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" delayed-response
    echo "validate-codex-agent-config.test: delayed initialize response was lost after parent exit" >&2
    cat "$TMP_DIR/native-delayed-response.err" >&2
    exit 1
  fi
  terminate_and_assert_pid_not_live "$FAKE_CODEX_CHILD_PID_FILE" delayed-response
  printf 'native delayed initialize response reached config/read boundary\n'
fi

printf 'validate-codex-agent-config.test: PASS\n'
