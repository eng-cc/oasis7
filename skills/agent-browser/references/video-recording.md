# Video Recording

Capture browser automation as video for debugging, documentation, or verification.

Use the owned-session setup and cleanup from [session-management.md](session-management.md)
in the same shell before these examples (also inside any standalone script):

```bash
AB_SESSION="$(agent-browser session id --scope worktree --prefix video)" || exit $?
: "${AB_SESSION:?Session creation returned an empty id}"
ab() { agent-browser --session "$AB_SESSION" "$@"; }
```

Install the scoped `EXIT INT TERM` cleanup from that reference after setup.
Every `ab` invocation passes the owned id explicitly; an ambient
`AGENT_BROWSER_SESSION` alone is not sufficient. Never use the default session
or global multi-session cleanup. This follows the repository's 0.37.1 session
contract; if the installed CLI differs, read `agent-browser skills get core --full`
before adapting these examples.

Stop recording before closing that exact session, and retain stop failures as described below.

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
ab record start ./demo.webm

# Perform actions
ab open https://example.com
ab snapshot -i
ab click @e1
ab fill @e2 "test input"

# Stop and save
ab record stop
```

## Recording Commands

```bash
# Start recording to file
ab record start ./output.webm

# Stop current recording
ab record stop

# Restart with new file (stops current + starts new)
ab record restart ./take2.webm
```

## Use Cases

### Debugging Failed Automation

```bash
#!/bin/bash
# Record automation for debugging

ab record start ./debug-$(date +%Y%m%d-%H%M%S).webm

# Run your automation
ab open https://app.example.com
ab snapshot -i
ab click @e1 || {
    echo "Click failed - check recording"
    ab record stop
    exit 1
}

ab record stop
```

### Documentation Generation

```bash
#!/bin/bash
# Record workflow for documentation

ab record start ./docs/how-to-login.webm

ab open https://app.example.com/login
ab wait 1000  # Pause for visibility

ab snapshot -i
ab fill @e1 "demo@example.com"
ab wait 500

ab fill @e2 "password"
ab wait 500

ab click @e3
ab wait --load networkidle
ab wait 1000  # Show result

ab record stop
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
ab click @e1
ab wait 500  # Let viewer see result
```

### 2. Use Descriptive Filenames

```bash
# Include context in filename
ab record start ./recordings/login-flow-2024-01-15.webm
ab record start ./recordings/checkout-test-run-42.webm
```

### 3. Handle Recording in Error Cases

Use the CI/CD example above to preserve the original test status and recording cleanup status separately. If adding an EXIT trap, save `$?` before any command, attempt `record stop` once for the owned session, and preserve its output. Keep a prior nonzero status; if the main action succeeded, a stop failure makes the run nonzero. Do not use `|| true` to report an unconfirmed recording or session cleanup as successful.

### 4. Combine with Screenshots

```bash
# Record video AND capture key frames
ab record start ./flow.webm

ab open https://example.com
ab screenshot ./screenshots/step1-homepage.png

ab click @e1
ab screenshot ./screenshots/step2-after-click.png

ab record stop
```

## Output Format

- Default format: WebM (VP8/VP9 codec)
- Compatible with all modern browsers and video players
- Compressed but high quality

## Limitations

- Recording adds slight overhead to automation
- Large recordings can consume significant disk space
- Some headless environments may have codec limitations
