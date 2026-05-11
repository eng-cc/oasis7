---
name: gpt-image-2
displayName: "🪞 GPT Image 2 — Image Generation via Your ChatGPT Subscription"
description: >
  Generate images with GPT Image 2 (ChatGPT Images 2.0) inside Claude Code,
  using your existing ChatGPT Plus or Pro subscription — no separate OpenAI
  access, no per-image billing. Supports text-to-image, image-to-image
  editing, style transfer, and multi-reference composition. Prefer the
  current session's built-in image generation path; fall back to the local
  Codex CLI wrapper only when direct image output is unavailable. Triggers on
  "gpt image 2", "gpt-image-2", "ChatGPT Images 2.0", "image 2", or any
  explicit ask to generate or edit an image through the user's ChatGPT plan.
emoji: "🪞"
homepage: https://agentspace.so
license: MIT
---

# 🪞 GPT Image 2 — Image Generation via Your ChatGPT Subscription

[agentspace.so](https://agentspace.so/?utm_source=skills.sh&utm_medium=skill&utm_campaign=gpt-image-2) · [GitHub](https://github.com/agentspace-so/agent-skills/tree/main/gpt-image-2)

Generate images with **GPT Image 2** (ChatGPT Images 2.0) inside your agent, using your existing ChatGPT Plus or Pro subscription — **no separate OpenAI access, no Fal or Replicate tokens, no per-image billing.**

Text-to-image, image-to-image editing, style transfer, and multi-reference composition. Prefer the current Codex session's built-in image output path. Use the bundled local `codex` CLI wrapper only as a fallback when the current session cannot emit images directly.

> **Heads up — this skill requires a ChatGPT Plus or Pro subscription _plus_ the Codex CLI installed locally.** If you have neither, you can use GPT Image 2 in the browser via RunComfy instead — hosted, no ChatGPT subscription or local install needed (RunComfy account required):
>
> - **Text-to-image:** [runcomfy.com/models/openai/gpt-image-2/text-to-image](https://www.runcomfy.com/models/openai/gpt-image-2/text-to-image)
> - **Image edit (i2i):** [runcomfy.com/models/openai/gpt-image-2/edit](https://www.runcomfy.com/models/openai/gpt-image-2/edit)
>
> The rest of this document covers the local Codex CLI flow for agents whose user has a ChatGPT subscription.

![GPT Image 2 example — flat-color lobster repainted as a 1950s ukiyo-e woodblock print](https://raw.githubusercontent.com/agentspace-so/agent-skills/main/gpt-image-2/gallery/d-ukiyoe.png)

*Example output: a plain flat-color icon repainted via `--ref` in ukiyo-e style — composition preserved, rendering swapped, period-appropriate red seal added by the model unprompted.*

## When to trigger

Trigger when the user explicitly asks for GPT Image 2 via their ChatGPT subscription, for example:

- "use GPT Image 2" / "use gpt-image-2" / "use ChatGPT Images 2.0"
- "use Image 2" / "image 2 this"
- attached a reference image and asked to remix / edit / restyle it

Do **not** auto-trigger for a plain "generate an image" request if the user didn't specify this route. If they did specify it, do not silently fall back to HTML mockups, screenshots, or a different image model.

## How to invoke

This skill has two execution paths.

### Default path: current session direct image output

Use the current session's built-in image generation capability first. Do not jump straight to `scripts/gen.sh` when the current session can already generate images.

Process:

1. Generate the image directly in the current session.
2. If the user named a destination path or the output is meant for the current project, move or copy the selected image into the workspace.
3. Report the saved path and keep the prompt faithful to the user's request.

Use this path for ordinary "generate/edit an image with GPT Image 2" requests.

### Fallback path: local `codex` CLI wrapper

Use `scripts/gen.sh` only when one of these is true:

- the current session does not expose direct image output capability
- the current session explicitly refuses direct image output
- the user explicitly wants a local shell workflow / terminal command / repeatable script entrypoint

Fallback text-to-image:

```bash
bash scripts/gen.sh \
  --prompt "<user's raw prompt>" \
  --out <absolute/path/to/output.png>
```

Fallback image-to-image (reference flag is repeatable for multi-reference composition):

```bash
bash scripts/gen.sh \
  --prompt "<user's raw prompt, e.g. 'repaint in watercolor'>" \
  --ref /absolute/path/to/reference.png \
  --out <absolute/path/to/output.png>
```

Optional: `--timeout-sec 300` (default 300).

## Default behavior

- **Pass the user's prompt through raw.** Do not translate, polish, or add style modifiers unless the user asked for it.
- **Prefer direct image output in the current session.** Treat `scripts/gen.sh` as fallback infrastructure, not the primary route.
- **Choose the output path.** If using the fallback script and the user did not specify a path, default to `./image-<YYYYMMDD-HHMMSS>.png` in the current working directory.
- **Deliver the image.** After the script succeeds, display / attach the output file. Do not stop at "done, see path X".
- **Text-heavy layouts are fine.** Image 2 handles infographics and timeline prompts well. Do not preemptively warn just because a prompt has a lot of text.

## Hard constraints

- Do not switch routes without permission. If the user said "use GPT Image 2", do not substitute DALL·E, Midjourney, an HTML mockup, or a manual screenshot workflow.
- Do not force a nested `codex exec` when the current session already has direct image output capability.
- Do not rewrite the prompt unless asked.
- Do not imply the fallback script works without a local `codex` login and a valid ChatGPT subscription with image-generation entitlement.
- Do not imply the fallback script can manufacture image capability when the nested session itself reports that direct image output is unavailable.

## Prerequisites

### Default path

- A Codex session that exposes built-in direct image output capability.

### Fallback path

1. `codex` CLI installed — `brew install codex` or see [openai/codex](https://github.com/openai/codex).
2. Logged in with a ChatGPT plan that includes Image 2 — `codex login`.
3. `python3` on PATH (ships with macOS; `apt install python3` on Linux).

This skill does **not** grant image-generation capability on its own. Both paths depend on capability the user already has; the fallback script only repackages that capability through a nested `codex exec`.

## Fallback exit codes

| code | meaning |
|------|---------|
| 0    | success — output path printed on stdout |
| 2    | bad args |
| 3    | `codex` or `python3` CLI missing |
| 4    | `--ref` file does not exist |
| 5    | `codex exec` failed (auth? network? model?) |
| 6    | no new session file detected |
| 7    | imagegen did not produce an image payload (feature not enabled, quota, or capability refused) |

On fallback failure, name the layer in one sentence instead of dumping the full stderr at the user.

## Fallback internals

The fallback wrapper uses a nested `codex exec`. It is no longer the preferred path because nested sessions can fail to inherit direct image output capability even when another session succeeded earlier.

When fallback is required, the script:

1. snapshots `~/.codex/sessions/` before the run
2. runs `codex exec --enable image_generation ...` from an isolated temporary directory (with `-i <file>` for each reference image)
3. diffs the sessions directory, then invokes `scripts/extract_image.py` to scan every new rollout JSONL for image payloads
4. accepts both classic base64 blobs and inline `data:image/...` payloads, then writes the largest matching result to `--out`
5. if no image payload exists, surfaces the nested session's final failure sentence so the caller can distinguish "no capability" from "parse failure"

Important fallback behavior:

- `--enable image_generation` is **required**; the feature is still under-development and off by default.
- `--ephemeral` **must not** be used — ephemeral sessions aren't persisted, so the image payload has nowhere to live.
- A nested success requires a real image-generation event in the nested rollout. If the nested session only answers with text like `Direct image output is unavailable in this session.`, no script-side extraction change can turn that run into a successful image.

## Data handling

The script is narrowly scoped on purpose:

- It reads **only** session rollout files created by its own `codex exec` invocation. The sessions directory is snapshotted before the call and diffed after, so any prior `~/.codex/sessions/*` files (which may contain unrelated Codex conversations) are never touched, read, or transmitted.
- It writes only two kinds of file: the output PNG at the caller's `--out` path, and short-lived `mktemp` logs that are auto-deleted on exit via a trap.
- No environment variables are read. No credentials are requested. No other paths under `~/.codex/` are accessed.
- No network calls leave this skill. The only outbound traffic is the one made by the `codex` CLI itself (to OpenAI, using the user's existing ChatGPT login) — this skill does not add endpoints, telemetry, or callbacks.

## What this skill is not

Not a direct OpenAI API client. Not a capability grant — it depends on the user's existing direct or nested Codex image capability. Not a guarantee that nested fallback will always inherit the same entitlement as the current session. Not a multi-tenant service (one call per invocation; concurrent calls are serialized by the filesystem-snapshot diff).
