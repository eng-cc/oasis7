---
name: oasis7-play
description: Execute an authorized Oasis7 native task through the signed game client, read the complete private goal and captured context, choose a supported action, and verify its canonical result. Use for the prepared-world task execution profile with an operator-provided device configuration.
---

# Oasis7 native task play

This profile uses the ordinary `oasis7_game_client` executable and an operator-provided device configuration. It supports task discovery, complete private goal/context capture, `native.move_agent`, `native.transfer_material`, built-in `native.build_factory` and `native.schedule_recipe` with signed quotes, authenticated industry observation, original result lookup and terminal provider outcomes. It does not provide cold world entry, token pairing, social operations or the full Game HTTP write catalog.

Use only the supplied executable and device configuration. Do not inspect owner keys, server storage, fixture source or other players' state. Never print signer files or copy private goal content into public action reasons. Keep full goals and observations inside the authorized model context.

## Executor selection

The operator may instead explicitly select the built-in `native_task_industry_v1`
executor described in the [Harness startup instructions](../../crates/oasis7_agent_harness/README.md).
It uses this profile's ordinary device authorization and the same signed SDK,
with strict native tools, bounded world-progress wait and a persistent physical
model-call budget. Its worker belongs to the node process; closing the Viewer
does not stop that worker. Missing status or exhausted budget does not establish
goal completion. The profile does not support generic modules, legacy chat,
private memory or continuation.

This is an operator choice made before play, not a tool for claiming another
executor's task. Do not run CLI effects concurrently with a selected built-in
worker for the same Agent. Switching executors requires the owner-controlled
fence and recovery of original operations; never replace an unknown request or
increase the same run's immutable budget. The CLI instructions below apply when
you are the authorized executor, with no competing worker. They do not authorize
paid inference or establish complete built-in/Runtime parity.

## Discover and capture

Run the supplied executable with `--help`, then:

```sh
<client> --config <device.json> describe
<client> --config <device.json> task
<client> --config <device.json> capabilities
<client> --config <device.json> industry
<client> --config <device.json> context --operation <unique-stable-context-id>
```

Choose a unique operation ID once per decision context. Retain it across transport errors and restarts. The client durably records the original signed preparation and public prefix before sending. Do not substitute another nonce or operation after an unknown result.

If context returns `usable_context=false`, inspect its original canonical outcome. Pending or unknown outcomes require original lookup/retry. A settled rejection or expiry cannot become a usable context by repeating it: preserve that result, reread the task, and capture a new decision context with a new operation ID while still authorized. A changing world may invalidate a captured base between preparation and submission.

A context is usable only after its prefix is canonically committed. Read the returned complete goal, soft constraints, current task/executor fence, observation schema and actual capability catalog. `world-agent-state/v1` is raw Agent state, not the legacy ProviderObservation format. Derive your decision from this goal and observation. Contexts have a bounded tick lease, at most 16 ticks and never beyond the capability grant expiry; refresh promptly before making a new decision.

## Decide and act

The movement CLI proposal is movement of the configured Agent to three signed i64 centimeter coordinates:

```sh
<client> --config <device.json> move --context <context-id> --to <x_cm,y_cm,z_cm>
<client> --config <device.json> lookup --context <context-id> --kind action
```

Choose coordinates yourself to satisfy the actual goal. The runtime still checks authority, capability admission, current task/goal/executor fences, base binding and available resources. A submitted or pending request is not a successful world effect. Inspect the canonical outcome and action receipt, and read a new context to check the resulting position when another decision is needed.

For a material-delivery goal, read `industry` before choosing source/destination ledgers, material, route path and amount. The authenticated read includes actual material ledgers, route availability/capacity, factories, pending transits and recipe jobs; each collection is capped at 128 entries and reports truncation. Do not treat omitted entries as absent. This command is an observation, not a signed cost quote.

```sh
<client> --config <device.json> transfer --context <context-id> --from <ledger-id> --to <ledger-id> --material <kind> --amount <positive-i64> --distance-km <nonnegative-i64> --priority <standard|urgent> --routes <route-id,...|direct>
```

Ledger IDs are `world`, `agent:<id>`, `site:<id>` or `factory:<id>`. Routes are an ordered path of at most 16 distinct IDs; `direct` requests a transfer without a registered route path. Automatic rerouting is disabled. The requester is always the configured Agent. Choose arguments from the actual goal and industry state; the runtime checks ledger ownership, route continuity/availability, inventory, power and native grants. A committed admission may only start a transit: reread industry after world progress to verify destination delivery.

`capabilities` separates local CLI proposal forms, the module command catalog and native action/object electricity grants. Inspect grant status, action kinds, object scope and remaining units. A module catalog entry or a current native grant alone does not guarantee action admission. Agent signer delegation is a separate identity boundary. Generic module proposals and custom recipe plans are not supported by this CLI.

For a factory-construction goal, read `industry` and choose a ready site authorized for the configured Agent at its current authorized location. Inspect the site's location anchor, construction electricity profile, Agent material ledger and power. Collection truncation means omitted entries are unknown. Only `factory.smelter.mk1` and `factory.assembler.mk1` are supported factory kinds; the builder is always the configured Agent.

```text
<client> --config <device.json> quote-build --context <context-id> --site <site-id> --factory <new-factory-id> --kind <factory.smelter.mk1|factory.assembler.mk1>
<client> --config <device.json> build --context <context-id> --site <site-id> --factory <new-factory-id> --kind <factory.smelter.mk1|factory.assembler.mk1>
```

Read the signed quote's rejection or construction preview, material ledger/costs, power requirements and duration. A quote checks current admission without reserving inventory, power or delegation budget. `build` obtains a verified quote on its first submission and uses the authoritative factory spec and same returned public context. Do not inject a spec, costs, power obligation or grant. Committed admission or a pending construction job does not mean the factory is ready: after world progress, reread `industry` and verify the completed `FactoryBuilt` result and actual factory before transfer or scheduling. Capture a new context for that next decision.

Both build commands recover a cached original Action before any fresh context or quote: the same site, factory ID and canonical kind must match. Unknown/pending results freeze that exact signed operation, including after a process restart. Recover with `lookup --context <context-id> --kind action`; do not replace it with a new build or recipe. This profile assumes an already claimed and configured Agent with an authorized site. Factory construction does not establish cold entry, claiming, token pairing, bootstrap or full AC-EXT-010 coverage.

For a production goal, select a factory owned by the configured Agent and an exact built-in recipe ID from the current goal and industry state. The eleven IDs are `recipe.smelter.iron_ingot`, `recipe.smelter.copper_wire`, `recipe.smelter.polymer_resin`, `recipe.smelter.alloy_plate`, `recipe.assembler.gear`, `recipe.assembler.control_chip`, `recipe.assembler.motor_mk1`, `recipe.assembler.logistics_drone`, `recipe.assembler.sensor_pack`, `recipe.assembler.module_rack` and `recipe.assembler.factory_core`.

```sh
<client> --config <device.json> quote-schedule --context <context-id> --factory <factory-id> --recipe <canonical-recipe-id> --batches <positive-u32>
<client> --config <device.json> schedule --context <context-id> --factory <factory-id> --recipe <canonical-recipe-id> --batches <positive-u32>
<client> --config <device.json> lookup --context <context-id> --kind action
```

Read the signed quote's rejection or start preview, input/output ledgers, materials, byproducts, electricity and duration. A quote checks current canonical production and grant admission without reserving materials, power or budget. It is conditional on its signed read binding; it does not guarantee later admission. `schedule` obtains a verified quote on the first submission and uses its authoritative plan and context. Do not supply a raw plan, costs or grant. A started recipe is not completed production: reread `industry` after world progress to verify finished output. Quotes require a usable captured context; if a quote reports changed state, preserve any original action and use the ordinary context recovery rules.

If `schedule` or `quote-schedule` finds an original cached action, it checks the same factory, recipe and batches and queries that original result before reading another context or requesting another quote. Unknown or pending results keep the original operation; they do not create a replacement action. This lookup also works after a process restart. Use `lookup --kind action` to recover a lost submission response.

Each context has one original action shared by movement, transfer, construction and production. Retry the exact same action arguments and context to recover that exact result. Changing the action kind, route order, ledgers, amount or other arguments conflicts and leaves the original intact. After a committed action, capture a new context with a new operation ID before another decision.

If the original action is canonically rejected or expired, retain that result. Rejection does not necessarily release its active context: obtain a committed terminal outcome or wait for the context's bounded lease to expire before capturing a new decision context. Never treat rejection as an effect or create a replacement operation while the original outcome is unknown.

## Recover and finish

For a lost response, use `lookup --kind prefix|action|outcome` with the original context ID. Preserve rejected and expired results as evidence; do not report them as successful actions. For a current usable context that cannot produce an action, terminate with a fixed public reason:

```sh
<client> --config <device.json> outcome --context <context-id> --reason executor_aborted
```

The other supported reasons are `provider_failed` and `transport_exhausted`. Do not place private explanation text in public reasons. An outcome can itself be rejected when the world advances beyond its captured base. This CLI preserves that immutable original outcome; it does not automatically re-sign it with a fresh base. Query its original result and, after confirmed rejection, wait for the context lease to expire if it remains active. Expired contexts cannot be decrypted through this CLI; a fresh context retires expired pending work when current grants permit it. Grant expiry requires owner renewal or Stop, not unauthorized reuse.

After goal publication, executor replacement, Stop or delegation revocation, old contexts cannot authorize new effects. Query original settled results, reread `task`, and capture a fresh context only while still authorized. On Stop or revoked authority, stop acting and report the boundary. Do not rebind yourself or alter the owner goal.

Report the goal revision, chosen action, canonical result and observed effect concisely. Distinguish a prepared-world attached-agent run from cold entry or an OpenClaw integration test. If independent isolation is instruction-only, say so.
