# Native game client

The SDK and `oasis7_game_client` executable share authenticated World Service reads,
private task Context and quotes, signing, immutable operation recovery, and bounded HTTP.
They depend on `oasis7_client_api`, `oasis7_agent_api` and serialization/crypto/transport
libraries. They do not link the Oasis7 application, World, Viewer, or a model SDK.

```sh
env -u RUSTC_WRAPPER cargo build -p oasis7_game_client --bin oasis7_game_client
./target/debug/oasis7_game_client --help
```

The executable keeps the existing device configuration and commands documented by
[the player Skill](../../skills/oasis7-play/SKILL.md). The signer file and encrypted
operation cache retain their existing formats. Secure local file storage currently
requires Unix; unsupported platform guarantees fail closed.

Rust callers use `device_client::Client::open` for the same durable operation flow as
the CLI, or `WorldServiceTaskClient` for typed native task protocol calls. The lower
level `WorldServiceTransport` verifies request-bound service envelopes; callers of
its generic methods must also validate world, scope, evidence and business correlation.
A service signature is not an independent consensus or durable receipt proof.

`lookup_typed` retains error categories for recovery decisions. After one
submission, the high-level client retries only the original result query for
transport failure, HTTP 503 or bounded Retry-After advice. It keeps forty attempts
within an eight-second query window; an already in-flight read retains its
configured timeout. Assurance/configuration failures stop immediately. Neither
query retry nor an unknown result creates a replacement action or signature.
Native Harness model-call reservations are separately encrypted and locked in
the local operation store; they do not become world task authority.

Public JSON observations explicitly use `/v1/world/view-json`: the server applies
the same pinning and authorization as `/v1/world/view`, normalizes the projection to
JSON, then signs it in the distinct path domain. `VerifiedTaskWorldView` retains the
full authenticated projection and validates the consumed scope/runtime/task fields.
The old typed-CBOR signature must never be interpreted as a JSON-profile signature.
Servers without this profile fail explicitly; the client does not silently fall back.

The native task wire profile rejects legacy provider/browser/lease/continuation
request variants. It does not expand the CLI's supported gameplay or migrate the
legacy built-in executor. See [the API scope](../../doc/world-simulator/llm/external-agent-game-api.design.md)
and [implementation scope](https://github.com/eng-cc/oasis7/issues/4726).

The feedback foundation exposes bounded, service-authenticated scoped history.
`record_caller_persisted_feedback_acceptance` resolves a record inside a
`VerifiedTaskWorldView`; call it only after the caller durably commits its private
feedback effects and acceptance digest. The SDK records that signed caller
statement, not proof of private-memory correctness. It does not run or restore
Harness cognition, reorder private effects, or turn server delivery state into a
local acceptance marker. Only committed service feedback supports the existing
ACK protocol; other feedback remains readable without gaining ACK authority.

Acceptance and the original signed ACK share the existing encrypted operation
store. The view and journal bind the exact configured read authority; changing
that trust configuration cannot authorize acceptance or recovery of the old ACK. Immutable feedback-ID and session/sequence indexes must agree before an
ACK can be recovered or sent. A repeated acceptance must retain the same digest,
generation and feedback identity; recovery uses `recover_caller_persisted_feedback_ack`
without applying private effects again. `acknowledge_caller_persisted_feedback`
submits that exact cached payload and uses bounded original-result polling.
Unknown/Pending remains unresolved, and no authoritative ACK-completion marker is
persisted locally. A missing or conflicting index blocks sending; an interrupted
initial write can be retried with the exact original acceptance. This foundation
alone does not complete the legacy feedback/memory consumer migration.
