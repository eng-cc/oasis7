# Agent harness

The existing portable actor, TurnEngine, continuation and provider primitives stay
available to their consumers. Native builds additionally expose
`NativeTaskRunner` and `SdkNativeTaskHost` for the explicit
`native_task_industry_v1` execution profile. They consume the same signed HTTP SDK
as the ordinary CLI; neither the Harness nor SDK links the World or Viewer.

The native runner reads the full authorized task goal, constraints, observation
and capability catalog. It accepts strict observation, quote, move, transfer,
built-in build/schedule, bounded world-progress wait and abort tools. Unknown tools and authority arguments
fail explicitly. Module execution, legacy chat, private memory and continuation
are still served by their existing consumers and are not advertised by this
profile. It is not full built-in capability parity.

Each run has an immutable ID/configuration/task-fence binding and explicit model
call limit. Encrypted, locked budget reservations commit before each physical
model call and survive process restart. A failed, unknown or interrupted call
still consumes its reservation. Increasing the same run's limit or changing its
configuration fails closed; intentionally starting another run needs a new ID
and a new operator-authorized budget. Local storage is not an anti-rollback
hardware boundary.

A bounded worker runs model inference while the runner independently polls
authenticated task control. Stop, changed goal/epoch, expiry and timeout discard
late model output. This does not claim upstream cancellation or refunded usage.
Original operation IDs, signatures and cached results are recovered before new
contexts; unknown originals stop progress instead of acquiring replacement IDs.
World progress and authoritative admission remain with the Runtime.

The chain node composes this worker after its existing World Service starts:

```sh
export OASIS7_AGENT_EXECUTION_PROFILE=native_task_industry_v1
export OASIS7_AGENT_DEVICE_CONFIG=/absolute/path/to/executor-device.json
export OASIS7_AGENT_RUN_ID=operator-approved-run-1
export OASIS7_AGENT_MAX_MODEL_CALLS=8
export OASIS7_AGENT_MAX_TURNS=8
```

Then use the existing node/launcher startup command. The device must target that
node's literal-loopback HTTP endpoint and service identity, and already possess
ordinary task-executor authorization. This entry does not create a goal, claim an
Agent or grant itself authority. Model configuration uses the extracted
[`oasis7_model_provider`](../oasis7_model_provider/README.md) source precedence.
Do not enable it without an approved inference account and bounded budget.
Absent selection preserves the explicit legacy profile; unknown profile names
fail without fallback. Never run another worker for the same Agent.

`GET /v1/world/executor-status` reports the selected profile, node-process
location, execution phase and bounded run outcome. Turn-limit exhaustion does
not mean goal completion. Closing the Viewer does not stop the node-owned
worker; stopping the node does. No cloud hosting is implied. This diagnostic
endpoint is not an authority or completion receipt.

The deterministic fake-host/provider tests and real HTTP conformance fixture
exercise protocol behavior. They do not replace authorized real-model play,
full legacy capability migration, performance comparison or deployment evidence
tracked by [#4726](https://github.com/eng-cc/oasis7/issues/4726).
