# Full Guidance: agent-browser 0.37.1

This is the repo-owned operational reference for the installed `agent-browser 0.37.1` baseline. The CLI-bundled guide is the authority for command details and always matches the installed CLI:

```bash
agent-browser --version
agent-browser skills get core --full
```

If the reported version is different, load that version's bundled guide before using a flag. Do not infer behavior from an old copy of this file.

## Bootstrap an owned run

Run the environment checks before any UI action, then derive a stable session for this worktree. A session id is an ownership boundary, not a label to share between actors.

```bash
set -euo pipefail
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json
AB_SESSION="$(agent-browser session id --scope worktree --prefix browser-run)"
export AB_SESSION

ab() { agent-browser --session "$AB_SESSION" "$@"; }

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  ab close >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT INT TERM
```

Every command in the run must use the `ab` wrapper or pass `--session "$AB_SESSION"` explicitly. Record diagnostics before and after a long or important run:

```bash
ab session info --json | tee output/browser-session-info.json
agent-browser session list --json
```

The list operation is read-only. It identifies sessions owned by other actors; it is never a reason to close them.

## Navigate, wait, snapshot, act

Use headed mode for player-facing or visual acceptance. Wait for a browser load state and then an application-specific signal; a long-lived WebSocket page should not use network-idle as its only readiness condition.

```bash
ab --headed open "http://127.0.0.1:4173/?test_api=1"
ab wait --load domcontentloaded
ab wait --fn "typeof window.__AW_TEST__ === 'object'"
ab snapshot -i
ab click @e1
ab snapshot -i
ab screenshot output/browser.png
ab console | tee output/browser-console.log
```

Accessibility refs are valid only for the snapshot that produced them. Re-snapshot after navigation or a DOM-changing action. Use `wait --text` or another `wait --fn` expression when the app exposes a better readiness or completion signal.

For extraction and assertions, pair screenshots with observable state:

```bash
ab get url
ab get text body > output/browser-text.txt
ab eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)" > output/browser-state.json
```

## Persistence and authentication

For a one-shot check, use a fresh owned session and do not restore prior state. For an intentional persistent login, use the current restore flags keyed by the same named session:

```bash
ab --restore --restore-save auto \
  --restore-check-url '**/dashboard' \
  open "https://app.example.com/dashboard"
```

Use one of `--restore-check-url`, `--restore-check-text`, or `--restore-check-fn` whenever a reliable post-restore assertion is available. Auth Vault is preferred for credentials because the browser receives the saved profile without exposing a password to the agent:

```bash
printf '%s\n' "$APP_PASSWORD" |
  agent-browser auth save app --url "https://app.example.com/login" \
    --username "$APP_USERNAME" --password-stdin
ab auth login app
```

Do not make JSON cookie/storage state files the primary persistence workflow. If a legacy artifact must be inspected, treat it as sensitive, keep it outside the repository, and return to named-session restore for new runs.

## Parallel actors and tabs

Each parallel actor derives a different prefix and session id. Never use an ambient environment value as the only ownership proof and never omit the session flag from a command.

```bash
FIRST_SESSION="$(agent-browser session id --scope worktree --prefix first-actor)"
SECOND_SESSION="$(agent-browser session id --scope worktree --prefix second-actor)"
agent-browser --session "$FIRST_SESSION" open https://site-a.example
agent-browser --session "$SECOND_SESSION" open https://site-b.example
agent-browser session list --json
```

When intentionally attaching to an existing CDP tab, pin the chosen tab with `--pin-tab` and keep the same owned session for the rest of the run. Do not use tab fallback to cross an ownership boundary.

## Capture, recording, and comparison

The current CLI supports screenshots, PDF, video, console, trace, profiler, accessibility, and diff commands. Use only the command forms in the version-matched core guide when a specialized option is needed.

```bash
ab screenshot --full output/page-full.png
ab pdf output/page.pdf
ab record start output/run.webm
ab record stop
ab diff snapshot --baseline output/before.txt
ab diff screenshot --baseline output/before.png
```

Recording may require `ffmpeg`; a missing optional recorder is not a reason to terminate unrelated sessions. For oasis7 visual evidence, retain state, console, screenshot, and the browser environment diagnostic together.

## Cleanup and failure handling

The EXIT/INT/TERM trap must close only the session created by the current script. Explicit cleanup is required for headed runs because they are not covered by ordinary idle cleanup. Never invoke a global cleanup operation.

For daemon or IPC errors such as `Resource temporarily unavailable`:

1. Preserve the session-info JSON and command logs.
2. Confirm the owned session with `agent-browser session list --json` and `ab session info --json`.
3. Rebuild only the owned session if needed.
4. Do not replay non-idempotent UI actions after an uncertain result.

Run `agent-browser doctor --offline --quick --json` again after repair. Use `--fix` only when the repair is explicitly in scope; it can change local browser state.

## Oasis7 evidence boundary

For oasis7 Web/UI work, read `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` and `testing-manual.md` S6. Viewer page actions may use this CLI; launcher product actions remain GUI Agent first, with the page used for state and field verification. Record commands and observable results in the GitHub-backed task evidence sink when the browser run is part of task evidence. This reference does not replace `AGENTS.md` or `doc/engineering/workflow/source-of-truth.md`.
