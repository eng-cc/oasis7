# Video Recording

Capture browser automation as video for debugging, documentation, or verification.

All examples assume an owned named session exported through
`AGENT_BROWSER_SESSION` and scoped `EXIT INT TERM` cleanup. Stop recording
before closing that exact session; never use the default session or global
multi-session cleanup.

**Related**: [commands.md](commands.md) for full command reference, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [Basic Recording](#basic-recording)
- [Recording Commands](#recording-commands)
- [Use Cases](#use-cases)
- [Best Practices](#best-practices)
- [Output Format](#output-format)
- [Limitations](#limitations)

## Basic Recording

```bash
# Start recording
agent-browser record start ./demo.webm

# Perform actions
agent-browser open https://example.com
agent-browser snapshot -i
agent-browser click @e1
agent-browser fill @e2 "test input"

# Stop and save
agent-browser record stop
```

## Recording Commands

```bash
# Start recording to file
agent-browser record start ./output.webm

# Stop current recording
agent-browser record stop

# Restart with new file (stops current + starts new)
agent-browser record restart ./take2.webm
```

## Use Cases

### Debugging Failed Automation

```bash
#!/bin/bash
# Record automation for debugging

agent-browser record start ./debug-$(date +%Y%m%d-%H%M%S).webm

# Run your automation
agent-browser open https://app.example.com
agent-browser snapshot -i
agent-browser click @e1 || {
    echo "Click failed - check recording"
    agent-browser record stop
    exit 1
}

agent-browser record stop
```

### Documentation Generation

```bash
#!/bin/bash
# Record workflow for documentation

agent-browser record start ./docs/how-to-login.webm

agent-browser open https://app.example.com/login
agent-browser wait 1000  # Pause for visibility

agent-browser snapshot -i
agent-browser fill @e1 "demo@example.com"
agent-browser wait 500

agent-browser fill @e2 "password"
agent-browser wait 500

agent-browser click @e3
agent-browser wait --load networkidle
agent-browser wait 1000  # Show result

agent-browser record stop
```

### CI/CD Test Evidence

```bash
#!/bin/bash
set -uo pipefail
# Reuse only the session owned by this run. run_e2e_test must use the
# session ID passed as its first argument for every browser command.
: "${AB_SESSION:?Set AB_SESSION to the session owned by this run}"
mkdir -p ./test-recordings || exit $?
RECORDING_DIR="$(mktemp -d ./test-recordings/e2e.XXXXXX)" || exit $?
RECORDING_PATH="$RECORDING_DIR/e2e.webm"

if agent-browser --session "$AB_SESSION" record start "$RECORDING_PATH" >"$RECORDING_DIR/start.log" 2>&1; then
    :
else
    start_status=$?
    printf 'Recording start failed: %s; see %s/start.log\n' "$start_status" "$RECORDING_DIR" >&2
    exit "$start_status"
fi

if run_e2e_test "$AB_SESSION"; then
    test_status=0
else
    test_status=$?
fi

if agent-browser --session "$AB_SESSION" record stop >"$RECORDING_DIR/stop.log" 2>&1; then
    stop_status=0
    printf 'Recording stop succeeded; verify artifact at %s\n' "$RECORDING_PATH"
else
    stop_status=$?
    printf 'Recording cleanup blocker: %s; artifact not confirmed saved; see %s/stop.log\n' "$stop_status" "$RECORDING_DIR" >&2
fi
if ! printf 'test_status=%s\nstop_status=%s\n' "$test_status" "$stop_status" >"$RECORDING_DIR/result.status"; then
    printf 'Could not save result.status\n' >&2
fi
if (( test_status != 0 )); then
    exit "$test_status"
fi
exit "$stop_status"
```

The test status is captured immediately. A failed test keeps its original code even if recording cleanup succeeds or also fails. A successful test with failed `record stop` returns the stop failure. Keep the stop output and check the video artifact before claiming it was saved. This example stops recording only; the session owner still performs its own bounded session cleanup. It does not establish that a repository CI pipeline uses this sample.


## Best Practices

### 1. Add Pauses for Clarity

```bash
# Slow down for human viewing
agent-browser click @e1
agent-browser wait 500  # Let viewer see result
```

### 2. Use Descriptive Filenames

```bash
# Include context in filename
agent-browser record start ./recordings/login-flow-2024-01-15.webm
agent-browser record start ./recordings/checkout-test-run-42.webm
```

### 3. Handle Recording in Error Cases

Use the CI/CD example above to preserve the original test status and recording cleanup status separately. If adding an EXIT trap, save `$?` before any command, attempt `record stop` once for the owned session, and preserve its output. Keep a prior nonzero status; if the main action succeeded, a stop failure makes the run nonzero. Do not use `|| true` to report an unconfirmed recording or session cleanup as successful.

### 4. Combine with Screenshots

```bash
# Record video AND capture key frames
agent-browser record start ./flow.webm

agent-browser open https://example.com
agent-browser screenshot ./screenshots/step1-homepage.png

agent-browser click @e1
agent-browser screenshot ./screenshots/step2-after-click.png

agent-browser record stop
```

## Output Format

- Default format: WebM (VP8/VP9 codec)
- Compatible with all modern browsers and video players
- Compressed but high quality

## Limitations

- Recording adds slight overhead to automation
- Large recordings can consume significant disk space
- Some headless environments may have codec limitations
