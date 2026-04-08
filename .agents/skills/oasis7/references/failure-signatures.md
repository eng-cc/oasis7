# Failure Signatures

## `bundle_missing_or_incomplete`

Meaning:

- extracted release bundle is wrong, incomplete, or you pointed `--bundle-dir` at the wrong directory

Check:

1. `run-game.sh` exists under the chosen bundle root
2. the archive extracted into `oasis7-<platform>/` or an equivalent directory that contains `run-game.sh`
3. rerun `.agents/skills/oasis7/scripts/oasis7-run.sh download --force-download`

## `provider_unreachable`

Meaning:

- runtime could not complete a decision call against the local HTTP provider

Check:

1. `curl -sS http://127.0.0.1:5841/v1/provider/health`
2. bridge process is still running
3. `agent_provider_url` is loopback and correct

## `provider_gateway_unreachable`

Meaning:

- bridge could not get a valid response from `provider` / Gateway

Check:

1. `curl -sS http://127.0.0.1:18789/health`
2. `provider_cli_bin="$(.agents/skills/oasis7/scripts/oasis7-run.sh resolve-provider-cli)" && "$provider_cli_bin" --version`
3. bridge stderr / log file

## `bridge_model_output_invalid`

Meaning:

- Local Provider returned malformed or non-whitelisted JSON for the world action

Current behavior:

- bridge records structured diagnostics
- some `P0-001` patrol cases may be rerouted by guardrail instead of failing hard

## `unsupported_agent_profile`

Meaning:

- requested `agent_profile` is empty or not supported by the bridge path

Check:

- use `oasis7_p0_low_freq_npc`

## `wait-only` sample with `goal_completed=false`

Meaning:

- run is alive, but the agent is not producing progress

Check:

1. lightweight runtime agent is installed and used
2. bridge is started with `--provider-agent oasis7_provider_agent`
3. parity artifact raw trace under `output/provider_parity/*`
4. latency may still be too high even when correctness is fine

## `agent_chat` / `prompt_control` unsupported

Meaning:

- this is expected in current Local Provider mode

Current boundary:

- real NPC autoplay path is supported
- direct player-side hot control is not yet supported
- treat current software-safe prompt/chat surfaces as observer/debug-only for Local Provider real-play, not as a supported player-authority lane

## `repo_bootstrap_unavailable`

Meaning:

- bundle-first no-`cargo` play may still be fine, but repo-backed bridge/bootstrap cannot be auto-started because `cargo` or repo root is unavailable

Check:

1. if bridge is already running, use `--reuse-bridge --skip-agent-setup` with a valid `--bundle-dir`
2. otherwise install `cargo` and provide `--repo-root <path>` so `oasis7` can bootstrap the runtime agent and bridge
3. use `doctor --json` and compare `bundle-play` vs `repo-bootstrap` statuses

## `play_wrapper_orphan_subtree`

Meaning:

- stopping the wrapper used to leave launcher/runtime/viewer children alive in the background

Current behavior:

- `oasis7-run.sh play` now starts the launcher under a supervised process group when possible
- on `INT` / `TERM` / `HUP` / wrapper exit, it tears down the launcher subtree before returning

If you still suspect leftovers:

1. inspect `ps -ef | grep -E 'oasis7_game_launcher|oasis7_chain_runtime|oasis7_viewer_live'`
2. rerun `.agents/skills/oasis7/scripts/oasis7-run-shutdown-test.sh`
3. confirm no stale bridge or launcher ports remain occupied

## `doctor` mode

Use this first when the local Local Provider path is not obviously healthy:

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh doctor
.agents/skills/oasis7/scripts/oasis7-run.sh doctor --json
```

It reports:

- whether `provider` is available
- whether `cargo` is available for repo-backed bootstrap
- whether Gateway responds on `127.0.0.1:18789`
- whether the configured runtime agent exists
- whether the bridge/provider responds on `127.0.0.1:5841`
- whether `provider/info` is readable
- whether an optional `--bundle-dir` looks usable
- whether `bundle-play` is ready for no-`cargo` reuse-bridge play
- whether `repo-bootstrap` is available for auto setup

Use `--json` when another script or UI needs a machine-readable JSON summary.
