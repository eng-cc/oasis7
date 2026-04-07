# Real-Play Config

## Required Settings

Use these exact values for a real local Local Provider gameplay run:

- `agent_provider_mode=provider_loopback_http`
- `agent_provider_url=http://127.0.0.1:5841`
- `agent_provider_connect_timeout_ms=15000`
- `agent_provider_profile=oasis7_p0_low_freq_npc`

Compatibility note:

- `provider_loopback_http` remains accepted as a legacy alias for `agent_provider_mode`

## Process Ownership

- `127.0.0.1:18789`: Local Provider Gateway
- `127.0.0.1:5841`: oasis7 local bridge

## Execution Modes And Viewer Contract

Read the real-play path in two layers:

- Local Provider execution layer: `headless_agent` or `player_parity`
- optional observation layer: Viewer / `debug_viewer`

Operator rule:

- Local Provider real-play does not require a Viewer to keep running
- `headless_agent` is the default regression/server lane
- `player_parity` is the lane to use when a human is judging player feel
- detailed Viewer / `software_safe` / observer-only UI guidance lives in `references/viewer-ui-lanes.md`

Recommended commands:

### Headless smoke / regression

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh smoke \
  --execution-mode headless_agent
```

### Real play without depending on a browser

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode headless_agent \
  --reuse-bridge \
  --skip-agent-setup \
  --no-open-browser
```

### Player-feel / producer / QA run

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode player_parity \
  --reuse-bridge \
  --skip-agent-setup
```

## Chain Default and Node Key Safety

Current default: the `run-game.sh` / `oasis7_game_launcher` product path still starts `oasis7_chain_runtime` unless you explicitly pass `--chain-disable`. So even in Local Provider mode, the selected chain profile may load node key material.

Operator rules:

- `node private key`: secret / asset-bearing / never paste into docs, git, issues, chat logs, screenshots, or CI output
- `node public key`: shareable when needed, but still treat it as environment-specific node identity metadata
- local real-play / debugging should prefer disposable `chain storage profile` data when you do not need a persistent on-chain identity
- if you intentionally reuse a persistent profile, treat it like a wallet/validator identity and confirm backup + rotation ownership before recording or screen sharing
- if you only want Local Provider NPC behavior and do not need chain state, add `--chain-disable` when launching outside the current `oasis7-run.sh play` helper

## Bundle-First Entry

For a real试玩，先下载 GitHub Release 的游戏包，再把 Local Provider provider 配到 bundle：

- latest Linux bundle: `https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-linux-x64.tar.gz`
- latest macOS bundle: `https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-macos-x64.tar.gz`
- latest Windows bundle: `https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-windows-x64.zip`
- checksums: `https://github.com/eng-cc/oasis7/releases/latest/download/oasis7-checksums.txt`

One-command download via `oasis7`:

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
printf '%s\n' "$bundle_dir"
```

The returned directory must contain `run-game.sh`.
If you use `--download-dir ~/.cache/oasis7/releases`, the helper expands the current user's `~` and returns an absolute bundle path.

## Preferred Runtime Agent

Prefer the repo-owned lightweight runtime agent:

- agent id: `oasis7_provider_agent`
- installer: `scripts/setup-provider-oasis7-runtime.sh`
- workspace: `tools/provider/oasis7_provider_workspace`

## Product Launch Command

### Release bundle path

```bash
./run-game.sh \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc
```

### Repo source path

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc
```

### Bundle + wrapper path

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --reuse-bridge \
  --skip-agent-setup \
  --no-open-browser
```

## Repo-Backed Components

Current boundary:

- runtime agent installer is repo-backed: `scripts/setup-provider-oasis7-runtime.sh`
- local bridge is repo-backed: `oasis7_provider_local_bridge`
- parity smoke is repo-backed: `scripts/provider-parity-p0.sh`

So a downloaded game bundle is enough for real play. If bridge and runtime agent are already running, prefer `--reuse-bridge --skip-agent-setup` as the no-`cargo` path; only auto bridge bootstrap, runtime-agent install, source-tree launch, and `smoke` still need repo access + `cargo`.

## Fast Smoke Command

```bash
bash scripts/provider-parity-p0.sh \
  --provider-only \
  --samples 1 \
  --ticks 4 \
  --timeout-ms 15000 \
  --agent-provider-url http://127.0.0.1:5841 \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc
```

## Doctor Contract

`oasis7-run.sh doctor` now reports two operator-facing readiness tracks separately:

- `bundle-play`: whether a valid bundle plus reachable bridge can support no-`cargo` real play via `--reuse-bridge --skip-agent-setup`
- `repo-bootstrap`: whether repo root + `cargo` are available for auto runtime-agent/bootstrap work

## Shutdown Contract

Killing `oasis7-run.sh play` now performs best-effort teardown of the launched play subtree. In the bundle-backed path this includes the wrapper-started launcher stack instead of leaving residual `oasis7_game_launcher` / `oasis7_chain_runtime` / `oasis7_viewer_live` behind.
