---
name: oasis7
description: Public raw Markdown for the oasis7 Local Provider real-play workflow. Use it when you need the current repo-backed commands for downloading a release bundle, starting the local provider bridge, launching gameplay, or running parity smoke without relying on the deleted repo-local skill wrapper.
---

# Oasis7

## Overview

`oasis7` is the public raw workflow for running a real Local Provider-backed oasis7 session.
Use it for bundle-first试玩、repo-backed bridge/bootstrap、`player_parity` vs `headless_agent` execution lanes, and first-line debugging of the local `provider_loopback_http` path.

This public copy is intentionally self-contained.
It does not depend on any private repo-local skill bundle, and all commands below are repo-native commands that remain valid after that internal helper surface is removed.

## When To Use

Use this skill when the task involves any of these:

- Configure a real Local Provider gameplay run instead of mock provider tests
- Download a playable oasis7 release bundle
- Install or refresh the lightweight Local Provider runtime agent
- Start or debug `oasis7_provider_local_bridge`
- Launch `oasis7_game_launcher` in `provider_loopback_http` mode
- Run `P0-001` parity smoke and inspect Local Provider failures
- Explain current Local Provider execution-lane or chain-runtime safety boundaries

Do not use this skill for:

- Generic LLM provider work unrelated to Local Provider
- Editing third-party Local Provider source under `third_party/`
- Viewer-only styling work with no Local Provider runtime involvement

## Execution Lanes

Read `oasis7` with one product rule in mind: Local Provider real-play can run without a Viewer.

- `headless_agent`: default for smoke, CI, servers, low-spec machines, and no-UI regression
- `player_parity`: use when producer/QA wants a player-feel run

The Viewer is optional and is not the authority execution path.

## Core Workflow

### 1. Verify local prerequisites

Check these first:

- the provider CLI configured by `OASIS7_PROVIDER_CLI_BIN` is callable
- Local Provider Gateway is live on `127.0.0.1:18789`
- the oasis7 bridge is or can be made available on `127.0.0.1:5841`
- `cargo` is required for repo-backed bridge startup, source-tree launch, and direct smoke
- cargo commands use `env -u RUSTC_WRAPPER cargo ...`

Useful probes:

```bash
provider_cli_bin="${OASIS7_PROVIDER_CLI_BIN:-provider}"
"$provider_cli_bin" --version
curl -sS http://127.0.0.1:18789/health
```

### 2. Download a playable release bundle

Use the release bundle as the default operator entry:

```bash
release_tag="${OASIS7_RELEASE_TAG:-latest}"
platform="${OASIS7_RELEASE_PLATFORM:-linux-x64}"
download_dir="${OASIS7_RELEASE_DIR:-$HOME/.cache/oasis7/releases/$release_tag/$platform}"
mkdir -p "$download_dir"
echo "Download the ${platform} installer asset for tag ${release_tag} from:"
echo "https://github.com/eng-cc/oasis7/releases"
echo "Then extract/install it so you have a bundle directory containing run-game.sh."
echo "Recommended bundle dir: $download_dir/bundle"
```

Current public distribution truth:

- release assets come from `eng-cc/oasis7` GitHub Releases
- the usable bundle must contain `run-game.sh`
- bundle-first is the preferred operator path when you do not want repo-local bootstrap logic

### 3. Install the lightweight runtime agent

For real gameplay or parity, prefer the repo-owned lightweight agent instead of the user’s default Local Provider workspace.

```bash
bash scripts/setup-provider-oasis7-runtime.sh
```

Defaults:

- agent id: `oasis7_provider_agent`
- workspace: `tools/provider/oasis7_provider_workspace`
- model: `custom-right-codes/gpt-5.4`

### 4. Start the bridge

Run the local bridge that exposes world-simulator provider endpoints:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_provider_local_bridge -- --provider-agent oasis7_provider_agent
```

Expected local provider URL:

- `http://127.0.0.1:5841`

Health probes:

```bash
curl -sS http://127.0.0.1:5841/v1/provider/info | jq .
curl -sS http://127.0.0.1:5841/v1/provider/health | jq .
```

### 5. Launch a real gameplay run

You can launch from either the source tree or a downloaded release bundle.

Repo source path:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --agent-execution-lane player_parity
```

Release bundle path:

```bash
./run-game.sh \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --agent-execution-lane player_parity
```

Required real-play settings:

- `agent_provider_mode=provider_loopback_http`
- `agent_provider_url=http://127.0.0.1:5841`
- `agent_provider_connect_timeout_ms=15000`
- `agent_provider_profile=oasis7_p0_low_freq_npc`

### 5.1 Choose execution lane

Default no-UI regression lane:

```bash
bash scripts/provider-parity-p0.sh \
  --provider-only \
  --samples 1 \
  --ticks 4 \
  --timeout-ms 15000 \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --execution-mode headless_agent
```

Player-feel / producer / QA run from source tree:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --agent-execution-lane player_parity
```

### 5.2 Chain Key Safety

Current Local Provider real-play still uses the product default chain startup path unless you explicitly pass `--chain-disable`.
That means:

- node private key is sensitive and must never be written to git, issues, devlogs, screenshots, shared shell history, or CI logs
- node public key is not secret, but it is still environment identity material and should be labeled by environment
- local trial runs should prefer disposable `chain storage profile` state

### 6. Run parity smoke

Use this as the fastest real verification path:

```bash
bash scripts/provider-parity-p0.sh \
  --provider-only \
  --samples 1 \
  --ticks 4 \
  --timeout-ms 15000 \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --execution-mode headless_agent
```

Primary success target today:

- `P0-001`
- `status=passed`
- `goal_completed=true`
- `invalid_action_count=0`

## Debug Checklist

If the run fails, inspect in this order:

1. Gateway health: `http://127.0.0.1:18789/health`
2. Bridge health: `http://127.0.0.1:5841/v1/provider/health`
3. Wrong provider mode or missing profile
4. Bundle missing `run-game.sh` or wrong extracted directory
5. Runtime agent not installed with `bash scripts/setup-provider-oasis7-runtime.sh`
6. Parity artifacts under `output/provider_parity/*`

Current known reality:

- correctness is largely working for `P0-001`
- builtin/Local Provider parity is still not the final fully-passed public claim boundary
- `headless_agent` is the regression lane; `player_parity` is the player-feel lane
- `agent_chat` and `prompt_control` are still not end-to-end player-authority paths in current Local Provider mode

## Repo Anchors

Use these files as the source of truth:

- bridge entry: `crates/oasis7/src/bin/oasis7_provider_local_bridge.rs`
- launcher entry: `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- runtime workspace installer: `scripts/setup-provider-oasis7-runtime.sh`
- parity harness: `scripts/provider-parity-p0.sh`
- runtime workspace policy: `tools/provider/oasis7_provider_workspace/AGENTS.md`
- module tracker: `doc/world-simulator/project.md`
- dual-mode verdict: `doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`
- parity rollout verdict: `doc/testing/provider-agent-parity-p0-t4-closure-2026-03-17.md`

## Output Expectations

When using this skill:

- prefer exact commands over abstract advice
- state which process provides `127.0.0.1:18789` and which provides `127.0.0.1:5841`
- distinguish runtime agent workspace/profile from repo documentation surfaces
- distinguish downloaded release bundle from repo-backed bridge/smoke tooling
