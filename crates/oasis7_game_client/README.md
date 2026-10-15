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

## Bounded recovery of original operations

`Client` sends each public intent once and queries its immutable original after
uncertain delivery. Queries retry only transport failures, HTTP 503 and an already
identified shared cooldown, within 40 attempts and an eight-second window.
They never resubmit, re-sign, or allocate another operation ID. Rejected, expired,
configuration and assurance failures remain explicit; Unknown remains unresolved.

Private context preparation retries only the same durable signed prepare proof on
transport failures, HTTP 503 or an identified cooldown. This is the idempotent
private prepare protocol, not a public action submission retry. The service keeps
the immutable prepared winner and checks the original fence and lifetime again.

Factory/recipe quotes refresh the same prepared context only for the two fixed,
route-specific base/delivery drift codes. Context reads retry the exact same signed
GET proof only for their fixed delivery drift code. Both allow at most three
requests within eight seconds; an already in-flight bounded HTTP request may
finish after the window, but no further request starts. Generic conflicts, expiry,
permission and verification errors are not retried. Quote refresh preserves the
complete prepared input, context identity, operation, nonce, lifetime and business
selection; existing Action/Outcome records take priority over fresh quotes.

These checks do not eliminate the race between receiving an observation and
canonical action admission. A stale or otherwise rejected original remains rejected;
callers must retain its evidence and choose an explicit recovery policy. This SDK
slice does not implement a built-in model runner or claim autonomous play success.
