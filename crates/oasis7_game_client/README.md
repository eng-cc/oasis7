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
