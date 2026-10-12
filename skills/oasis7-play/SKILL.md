---
name: oasis7-play
description: Execute an authorized Oasis7 native task through the signed game client, read the complete private goal and captured context, choose a supported action, and verify its canonical result. Use for the prepared-world task execution profile with an operator-provided device configuration.
---

# Oasis7 native task play

This profile uses the ordinary `oasis7_game_client` executable and an operator-provided device configuration. It supports task discovery, complete private goal/context capture, `native.move_agent`, original result lookup and terminal provider outcomes. It does not provide cold world entry, token pairing, social operations or the full Game HTTP write catalog.

Use only the supplied executable and device configuration. Do not inspect owner keys, server storage, fixture source or other players' state. Never print signer files or copy private goal content into public action reasons. Keep full goals and observations inside the authorized model context.

## Discover and capture

Run the supplied executable with `--help`, then:

```sh
<client> --config <device.json> describe
<client> --config <device.json> task
<client> --config <device.json> capabilities
<client> --config <device.json> context --operation <unique-stable-context-id>
```

Choose a unique operation ID once per decision context. Retain it across transport errors and restarts. The client durably records the original signed preparation and public prefix before sending. Do not substitute another nonce or operation after an unknown result.

If context returns `usable_context=false`, inspect its original canonical outcome. Pending or unknown outcomes require original lookup/retry. A settled rejection or expiry cannot become a usable context by repeating it: preserve that result, reread the task, and capture a new decision context with a new operation ID while still authorized. A changing world may invalidate a captured base between preparation and submission.

A context is usable only after its prefix is canonically committed. Read the returned complete goal, soft constraints, current task/executor fence, observation schema and actual capability catalog. `world-agent-state/v1` is raw Agent state, not the legacy ProviderObservation format. Derive your decision from this goal and observation. Contexts have a bounded tick lease, at most 16 ticks and never beyond the capability grant expiry; refresh promptly before making a new decision.

## Decide and act

The supported CLI proposal is movement of the configured Agent to three signed i64 centimeter coordinates:

```sh
<client> --config <device.json> move --context <context-id> --to <x_cm,y_cm,z_cm>
<client> --config <device.json> lookup --context <context-id> --kind action
```

Choose coordinates yourself to satisfy the actual goal. The runtime still checks authority, capability admission, current task/goal/executor fences, base binding and available resources. A submitted or pending request is not a successful world effect. Inspect the canonical outcome and action receipt, and read a new context to check the resulting position when another decision is needed.

Each context has one original action. Retry the same coordinates and context to recover that exact result. A changed action conflicts and leaves the original intact. After a committed action, capture a new context with a new operation ID before another decision.

If the original action is canonically rejected or expired, retain that result and use a fresh context for a new decision. Never treat rejection as an effect or create a replacement operation while the original outcome is unknown.

## Recover and finish

For a lost response, use `lookup --kind prefix|action|outcome` with the original context ID. Preserve rejected and expired results as evidence; do not report them as successful actions. For a current usable context that cannot produce an action, terminate with a fixed public reason:

```sh
<client> --config <device.json> outcome --context <context-id> --reason executor_aborted
```

The other supported reasons are `provider_failed` and `transport_exhausted`. Do not place private explanation text in public reasons. Expired contexts cannot be decrypted through this CLI; a fresh context retires expired pending work when current grants permit it. Grant expiry requires owner renewal or Stop, not unauthorized reuse.

After goal publication, executor replacement, Stop or delegation revocation, old contexts cannot authorize new effects. Query original settled results, reread `task`, and capture a fresh context only while still authorized. On Stop or revoked authority, stop acting and report the boundary. Do not rebind yourself or alter the owner goal.

Report the goal revision, chosen action, canonical result and observed effect concisely. Distinguish a prepared-world attached-agent run from cold entry or an OpenClaw integration test. If independent isolation is instruction-only, say so.
