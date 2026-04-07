---
name: oasis7
description: OpenClaw real-play and parity workflow for oasis7. Use when the user wants to configure, start, validate, or debug a real local OpenClaw gameplay path, including downloading a GitHub Release game bundle, installing the lightweight runtime agent, starting the local bridge, launching `oasis7_game_launcher`, probing `provider_loopback_http`, or running parity smoke for OpenClaw NPC behavior.
---

# Oasis7

## Overview

`oasis7` is the repo-local workflow for running a real OpenClaw-backed oasis7 NPC.
Use it for “能不能真跑起来”, “怎么配 OpenClaw 试玩”, “起 bridge / launcher / parity”, and first-line debugging of the local `provider_loopback_http` path.

默认推荐 `bundle-first`：先下载 GitHub Release 的游戏包，再把 OpenClaw provider 配到该 bundle 的 `run-game.sh`，避免把试玩路径绑死在 repo 内的相对目录结构上。
当 bundle 已就绪且本地 bridge 已在运行时，`play --bundle-dir ... --reuse-bridge --skip-agent-setup` 是一条一等公民的无 `cargo` real-play 路径；`doctor` 也会把这条路径与 repo-backed bridge/bootstrap readiness 分开报告。
停止 `oasis7-run.sh play` 时，wrapper 现在会一并终止它启动的 launcher 子树，避免残留 `oasis7_game_launcher` / `oasis7_chain_runtime` / `oasis7_viewer_live`。
同时要注意：当前 `run-game.sh` / `oasis7_game_launcher` 默认会启动 `oasis7_chain_runtime`，因此所选 `chain storage profile` 下的 node private key 属于重要资产；`oasis7` 文档只描述管理规则，不会输出或托管真实私钥。

## When To Use

Use this skill when the task involves any of these:

- Configure a real OpenClaw gameplay run instead of mock provider tests
- Download a playable oasis7 bundle from GitHub Release
- Install or refresh the lightweight OpenClaw runtime agent
- Start or debug `oasis7_provider_local_bridge`
- Launch the product path with `oasis7_game_launcher` in `provider_loopback_http` mode
- Run `P0-001` parity smoke or inspect OpenClaw latency / wait-only failures
- Explain which OpenClaw settings are required for a real local试玩

Do not use this skill for:

- Generic LLM provider work unrelated to OpenClaw
- Editing OpenClaw third-party source under `third_party/`
- Viewer-only UI styling tasks with no OpenClaw runtime involvement

## Execution Lanes

Read `oasis7` with one product rule in mind: OpenClaw real-play can run without a Viewer.

- `headless_agent`: default for smoke, CI, servers, low-spec machines, and “does the agent still complete the loop” checks
- `player_parity`: use when a producer/QA/operator wants to judge “does this feel like a player-facing run”

The Viewer is optional and is not the authority execution path.
If you need `debug_viewer`, `software_safe`, or other UI/observer guidance, read `references/viewer-ui-lanes.md`.

## Core Workflow

### 1. Verify local prerequisites

Check these first:

- `openclaw` CLI exists in `PATH`
- OpenClaw Gateway is live on `127.0.0.1:18789`
- oasis7 bridge is or can be made available on `127.0.0.1:5841`
- `cargo` is only required for repo-backed runtime-agent bootstrap, auto bridge startup, source-tree launch, and smoke
- Cargo commands use `env -u RUSTC_WRAPPER cargo ...`

Useful probes:

```bash
openclaw --version
curl -sS http://127.0.0.1:18789/health
```

For exact field values and launch examples, read `references/real-play-config.md`.
For Viewer / `software_safe` / observer-only UI boundaries, read `references/viewer-ui-lanes.md`.

### 2. Download a playable game bundle

Use the release bundle as the default operator entry:

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
printf '%s\n' "$bundle_dir"
```

By default it downloads the latest asset from `eng-cc/oasis7` GitHub Releases, verifies `oasis7-checksums.txt` when available, extracts the archive, and returns a directory that contains `run-game.sh`.
Current-user `~` in `--download-dir` is expanded before use, and the returned `bundle_dir` is an absolute path.

Useful overrides:

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh download \
  --release-platform linux-x64 \
  --release-tag latest \
  --download-dir ~/.cache/oasis7/releases
```

### 3. Install the lightweight runtime agent

For real gameplay or parity, prefer the repo-owned lightweight agent instead of the user’s default OpenClaw workspace.

```bash
scripts/setup-provider-oasis7-runtime.sh
```

Defaults:

- agent id: `oasis7_provider_agent`
- workspace: `tools/provider/oasis7_provider_workspace`
- model: `custom-right-codes/gpt-5.4`

The runtime workspace is intentionally slim and is not meant for daily chat.

### 4. Start the bridge

Run the local bridge that exposes world-simulator provider endpoints:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_provider_local_bridge -- --openclaw-agent oasis7_provider_agent
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
  --openclaw-base-url http://127.0.0.1:5841 \
  --openclaw-connect-timeout-ms 15000 \
  --openclaw-agent-profile oasis7_p0_low_freq_npc
```

Release bundle path:

```bash
./run-game.sh \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_loopback_http \
  --openclaw-base-url http://127.0.0.1:5841 \
  --openclaw-connect-timeout-ms 15000 \
  --openclaw-agent-profile oasis7_p0_low_freq_npc
```

Required real-play settings:

- `agent_provider_mode=provider_loopback_http`
- `openclaw_base_url=http://127.0.0.1:5841`
- `openclaw_connect_timeout_ms=15000`
- `openclaw_agent_profile=oasis7_p0_low_freq_npc`

### 5.1 Choose execution lane

Default no-UI / regression lane:

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh smoke \
  --execution-mode headless_agent
```

Real play without depending on a browser:

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode headless_agent \
  --reuse-bridge \
  --skip-agent-setup \
  --no-open-browser
```

Player-feel / producer / QA run:

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --execution-mode player_parity \
  --reuse-bridge \
  --skip-agent-setup
```

UI is optional here.
If you need Viewer / `software_safe` behavior, fallback rules, or current observer-only boundaries, read `references/viewer-ui-lanes.md`.

### 5.2 Chain Key Safety

`oasis7` 的 OpenClaw real-play 只是替换 agent provider；当前产品默认启动链路仍会拉起 `oasis7_chain_runtime`，除非你显式传 `--chain-disable`。这意味着：

- node private key 是高敏资产，绝不能写进 git、issue、devlog、截图、共享 shell 历史或 CI 日志
- node public key 不是秘密，但仍属于节点身份资产，应按环境（local temp / persistent / release / soak）标注来源
- 本地临时试玩优先使用一次性/可丢弃的 `chain storage profile`，避免把持久节点身份混进录屏、直播或共享机器
- 若需要复用持久 `chain storage profile`，先确认操作者知道该 profile 下会继续使用同一 node key material
- `oasis7` / release bundle 不应导出、回显或要求粘贴真实 node private key；只允许说明如何保护它

### 6. Run parity smoke

Use this as the fastest real verification path:

```bash
bash scripts/provider-parity-p0.sh \
  --openclaw-only \
  --samples 1 \
  --ticks 4 \
  --timeout-ms 15000 \
  --openclaw-base-url http://127.0.0.1:5841 \
  --openclaw-connect-timeout-ms 15000 \
  --openclaw-agent-profile oasis7_p0_low_freq_npc
```

Primary success target today:

- `P0-001`
- `status=passed`
- `goal_completed=true`
- `invalid_action_count=0`

## One-Command Helpers

Use the bundled wrapper when you want the skill to do the repetitive setup for you.

### Download

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh download
```

### Real play from release bundle

```bash
bundle_dir="$(.agents/skills/oasis7/scripts/oasis7-run.sh download)"
.agents/skills/oasis7/scripts/oasis7-run.sh play \
  --bundle-dir "$bundle_dir" \
  --reuse-bridge \
  --skip-agent-setup \
  --no-open-browser
```

### Real play from source tree

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh play --repo-root /path/to/oasis7 --no-open-browser
```

### Smoke

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh smoke --repo-root /path/to/oasis7
```

### Doctor

```bash
.agents/skills/oasis7/scripts/oasis7-run.sh doctor
.agents/skills/oasis7/scripts/oasis7-run.sh doctor --json
```

What it does:

- `download`: downloads and extracts the GitHub Release bundle, then prints the usable bundle directory
- `doctor`: checks command availability, Gateway health, bridge health, provider info, runtime agent presence, and optional `--bundle-dir` validity; add `--json` for machine-readable output
- `play`: bootstrap `oasis7_provider_agent` unless you disable it, verify Gateway health, start the local bridge unless you pass `--reuse-bridge`, then run launcher from the bundle or source tree
- `smoke`: remains repo-backed because the parity harness lives under `scripts/provider-parity-p0.sh`

## Debug Checklist

If the run fails, inspect in this order:

1. Gateway health: `http://127.0.0.1:18789/health`
2. Bridge health: `http://127.0.0.1:5841/v1/provider/health`
3. Wrong provider mode or missing profile
4. Bundle missing `run-game.sh` or wrong extracted directory
5. Bridge not started with the lightweight agent
6. Parity artifacts under `artifacts/openclaw_parity_*`

For common failure strings and what to check next, read `references/failure-signatures.md`. Run `doctor` first when you need a fast local diagnosis summary.

Current known reality:

- Correctness is largely working for `P0-001`
- builtin/OpenClaw parity 的默认启用门槛仍未通过；当前正式口径是 `behavior_parity_pass / latency_class B / keep experimental`
- `headless_agent` is the default execution/regression lane; `player_parity` is the player-feel lane
- `software_safe` is the weak-graphics observer/debug fallback, not the main player-experience mode
- `agent_chat` and `prompt_control` are still unsupported as end-to-end player authority in current OpenClaw mode

## Repo Anchors

Use these files as the source of truth:

- Bridge entry: `crates/oasis7/src/bin/oasis7_provider_local_bridge.rs`
- Launcher entry: `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- Runtime workspace installer: `scripts/setup-provider-oasis7-runtime.sh`
- Runtime workspace policy: `tools/provider/oasis7_provider_workspace/AGENTS.md`
- Module tracker: `doc/world-simulator/project.md`
- Dual-mode verdict: `doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`
- Parity rollout verdict: `doc/testing/openclaw-agent-parity-p0-t4-closure-2026-03-17.md`

## Output Expectations

When using this skill:

- Prefer exact commands over abstract advice
- State which process provides `127.0.0.1:18789` and which provides `127.0.0.1:5841`
- Distinguish “runtime agent workspace/profile” from Codex repo skills
- Distinguish “downloaded release bundle” from “repo-backed bridge/smoke tooling”
- If you changed behavior or tooling, update `doc/world-simulator/project.md` and `doc/devlog/YYYY-MM-DD.md`
