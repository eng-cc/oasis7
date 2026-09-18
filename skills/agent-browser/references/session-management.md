# Session Management

This reference follows the `agent-browser 0.37.1` session API. When the CLI version differs, first load its bundled guide with `agent-browser skills get core --full`.

## Ownership contract

Every browser run creates an isolated, worktree-scoped named session. The creator owns its lifecycle and is the only actor allowed to close it.

```bash
AB_SESSION="$(agent-browser session id --scope worktree --prefix web-check)"
export AB_SESSION
ab() { agent-browser --session "$AB_SESSION" "$@"; }
ab session info --json
```

Pass the session explicitly on every command. The ambient session environment is useful for the wrapper, but it is not a substitute for recording the generated id. Use a distinct prefix for each parallel actor or run.

## Inspect active sessions

```bash
agent-browser session list --json
agent-browser --session "$AB_SESSION" session info --json
```

`session list` is an inspection operation. Do not guess ownership from a name, reuse another actor's id, or terminate sessions listed by another actor.

## Start and wait

```bash
ab --headed open "https://app.example.com"
ab wait --load domcontentloaded
ab wait --fn "window.appReady === true"
ab snapshot -i
```

Use `wait --text` when visible text is the contract. For long-lived applications, use an application signal after `domcontentloaded`; network-idle is not a sufficient universal readiness condition.

## Explicit cleanup

Headed sessions are not covered by ordinary idle cleanup. Each script must install a scoped trap:

```bash
cleanup() {
  local status=$?
  trap - EXIT INT TERM
  ab close >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT INT TERM
```

The trap must be installed after `$AB_SESSION` and `ab()` are defined. Never call a global close operation: it can destroy sessions belonging to unrelated worktrees or agents.

## Persistent login with restore

One-shot checks should use a fresh session. For deliberate persistence, use the restore flags with the same named session and a post-restore assertion:

```bash
ab --restore --restore-save auto \
  --restore-check-url '**/dashboard' \
  open "https://app.example.com/dashboard"
ab wait --load domcontentloaded
ab wait --text "Dashboard"
```

Other supported checks are `--restore-check-text` and `--restore-check-fn`. Keep credentials in Auth Vault when possible. Do not introduce JSON cookie/storage files as a new primary workflow; inspect an old file only as sensitive, compatibility evidence outside the repository.

## Parallel sessions and CDP tabs

```bash
FIRST="$(agent-browser session id --scope worktree --prefix actor-a)"
SECOND="$(agent-browser session id --scope worktree --prefix actor-b)"
agent-browser --session "$FIRST" open https://site-a.example
agent-browser --session "$SECOND" open https://site-b.example
agent-browser session list --json
```

When a run intentionally shares an existing CDP tab, select it once with `--pin-tab` and retain the same owned session. Do not rely on tab fallback to cross session boundaries.

## Failure handling

For daemon or IPC errors, preserve `session info --json` and command output, inspect the active list, and rebuild only the owned session. Do not replay a non-idempotent click or submission after an uncertain result. Re-run `agent-browser doctor --offline --quick --json` after repair; use `--fix` only when destructive local repair is explicitly in scope.

## Prohibited patterns

- Do not omit `--session` from a live command.
- Do not use an unnamed or ambient default browser session.
- Do not use the legacy session-name flag; generate ids with `session id` instead.
- Do not invoke the close command with its global all-sessions option.
- Do not use unversioned `npx agent-browser` as a fallback for the repo-owned CLI.
