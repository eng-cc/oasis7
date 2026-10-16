# Profiling

Capture Chrome DevTools performance profiles during browser automation for performance analysis.

Use the owned-session setup and cleanup from [session-management.md](session-management.md)
in the same shell before these examples (also inside any standalone script):

```bash
AB_SESSION="$(agent-browser session id --scope worktree --prefix profile)" || exit $?
: "${AB_SESSION:?Session creation returned an empty id}"
ab() { agent-browser --session "$AB_SESSION" "$@"; }
```

Install the scoped `EXIT INT TERM` cleanup from that reference after setup.
Every `ab` invocation passes the owned id explicitly; an ambient
`AGENT_BROWSER_SESSION` alone is not sufficient. Never use the default session
or global multi-session cleanup. This follows the repository's 0.37.1 session
contract; if the installed CLI differs, read `agent-browser skills get core --full`
before adapting these examples.

**Related**: [commands.md](commands.md) for full command reference, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [Basic Profiling](#basic-profiling)
- [Profiler Commands](#profiler-commands)
- [Categories](#categories)
- [Use Cases](#use-cases)
- [Output Format](#output-format)
- [Viewing Profiles](#viewing-profiles)
- [Limitations](#limitations)

## Basic Profiling

```bash
# Start profiling
ab profiler start

# Perform actions
ab navigate https://example.com
ab click "#button"
ab wait 1000

# Stop and save
ab profiler stop ./trace.json
```

## Profiler Commands

```bash
# Start profiling with default categories
ab profiler start

# Start with custom trace categories
ab profiler start --categories "devtools.timeline,v8.execute,blink.user_timing"

# Stop profiling and save to file
ab profiler stop ./trace.json
```

## Categories

The `--categories` flag accepts a comma-separated list of Chrome trace categories. Default categories include:

- `devtools.timeline` -- standard DevTools performance traces
- `v8.execute` -- time spent running JavaScript
- `blink` -- renderer events
- `blink.user_timing` -- `performance.mark()` / `performance.measure()` calls
- `latencyInfo` -- input-to-latency tracking
- `renderer.scheduler` -- task scheduling and execution
- `toplevel` -- broad-spectrum basic events

Several `disabled-by-default-*` categories are also included for detailed timeline, call stack, and V8 CPU profiling data.

## Use Cases

### Diagnosing Slow Page Loads

```bash
ab profiler start
ab navigate https://app.example.com
ab wait --load networkidle
ab profiler stop ./page-load-profile.json
```

### Profiling User Interactions

```bash
ab navigate https://app.example.com
ab profiler start
ab click "#submit"
ab wait 2000
ab profiler stop ./interaction-profile.json
```

### CI Performance Regression Checks

```bash
#!/bin/bash
ab profiler start
ab navigate https://app.example.com
ab wait --load networkidle
ab profiler stop "./profiles/build-${BUILD_ID}.json"
```

## Output Format

The output is a JSON file in Chrome Trace Event format:

```json
{
  "traceEvents": [
    { "cat": "devtools.timeline", "name": "RunTask", "ph": "X", "ts": 12345, "dur": 100, ... },
    ...
  ],
  "metadata": {
    "clock-domain": "LINUX_CLOCK_MONOTONIC"
  }
}
```

The `metadata.clock-domain` field is set based on the host platform (Linux or macOS). On Windows it is omitted.

## Viewing Profiles

Load the output JSON file in any of these tools:

- **Chrome DevTools**: Performance panel > Load profile (Ctrl+Shift+I > Performance)
- **Perfetto UI**: https://ui.perfetto.dev/ -- drag and drop the JSON file
- **Trace Viewer**: `chrome://tracing` in any Chromium browser

## Limitations

- Only works with Chromium-based browsers (Chrome, Edge). Not supported on Firefox or WebKit.
- Trace data accumulates in memory while profiling is active (capped at 5 million events). Stop profiling promptly after the area of interest.
- Data collection on stop has a 30-second timeout. If the browser is unresponsive, the stop command may fail.
