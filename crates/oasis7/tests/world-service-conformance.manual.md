# PRE2 World Service conformance execution

Task: `task_490d0cf6273b423eb9510723170eef9a`, Issue #4254. QA authoring slice; this document does not constitute independent review.

Current evidence status: the new QA cases are **not_run**. Topology A and B are **not_evaluated**, C is **not_run**, and production is **not_evaluated**. Earlier client/authority/outage unit results do not establish A/B closure.

After the coordinator releases the exclusive Cargo slot, run from the canonical worktree:

```bash
rtk proxy ./crates/oasis7/scripts/world-service-conformance.sh /tmp/pre2-world-service-conformance
```

The runner uses `--no-default-features --features node-libp2p,test_tier_required`, matching the current runtime admission bundle, and the existing shared Cargo cache and records HEAD, the working candidate patch, full output, exit status and SHA-256 artifact digests. It requires Darwin `sandbox-exec`; unsupported platforms return 77 with `not_run`. It fails if the expected tests are absent, even when Cargo exits successfully. Read the complete log and inspect failure signatures before reporting a pass.

The server fixture uses the real `NodeRuntimeExecutionDriver` as the `NodeRuntime` execution hook, persisted canonical owner claim, Agent identity, signed WorldIdentity and genuine execution-record/CAS pin. The application receives only endpoint, configured service key, world identity, caller signing key and a signed intent. A child of the same compiled test executable runs in a separate application working directory under an OS sandbox denying reads and writes beneath the node root. It first proves that an existing identity file is denied, then exercises Describe, Submit, Lookup, View and Changes for direct Agent Cognition and actual signed CollectData, then the actual configured provider enqueue path through canonical Reserve/Prefix, Cognition receipt, lease settlement and receipt-gated memory. The parent emits the executable BLAKE3 digest. No Noop/capturing execution hook is used.

Controlled Cognition and the isolated application case use a test-only forwarding hook that gates automatic consensus commits, then calls the real driver with explicit canonical commit contexts. This proves the execution/service boundary and actual CAS publication; automatic consensus-worker scheduling is exercised by the other cases. It does not replace full provider execution.

Focused cases cover direct authenticated Cognition receipt/lineage/feedback, owner-signed delegation receipt/idempotency, minimum committed read and changed permission generation, forged delegation signature, conflicting request key, canonical delegation revocation and unrelated protected-scope reader, invalid cursor stream, cross-era resync rejection and protected snapshot-to-changes delivery in bounded one-item batches with stable cursor replay, tampering of an actual dispatcher-produced signed Describe response, actual live-world writer exclusion while service remains readable, rejection of tampered record roots and missing CAS during durable service bootstrap/view, authenticated CollectData with a deliberately lost TCP response and stable Lookup, and execution-driver rehydration of the exact canonical nonce/result. The separate process-restart case launches the same compiled artifact as a genuine server/node process, submits authenticated CollectData, kills that process, relaunches from verified durable service bootstrap, and requires the exact original receipt and minimum committed View. Driver rehydration alone remains a narrower case.

Still required before A/B acceptance: full provider wake/feedback closeout assertion beyond the authored enqueue/settlement/memory case; every frozen D0 live consumer negative; real same application artifact under the complete required A/B configurations. Passing the focused runner must leave these obligations open.

The genuine wake case runs a separate sandboxed application against a scheduler-enabled canonical fixture. It uses the production Wait checkpoint getter, signed gameplay steps, canonical wake selection, ResumeWake, and Act closure; it requires a signed Completed continuation and no remaining wake. Execution remains pending until the current-source runner succeeds.

Acceptance mapping after attempt 3 (2026-10-07): this is focused evidence, not a complete TA or topology verdict.

| Plan obligation | Actual evidence / remaining scope |
| --- | --- |
| TA03, TA24 | Same test executable is configured for isolated children and process restart, but two service endpoints with the same world identity and configuration-only switching have not been executed. Open. |
| TA04 | Sandbox denial probe and five operations are authored; both child cases stop at delegation rejection before launch. Open, no no-mount application pass. |
| TA05–07 | No complete D0 consumer matrix for diagnostic-path isolation, world/trust mismatch, or pending UI/Agent effects. Open. |
| TA08 | Actual TCP lost response + original signed gameplay Lookup and driver rehydration passed. Viewer Unknown identity/mutex test independently passed. Agent lost-response paths remain open. |
| TA09–10 | Changed-key/content test stops at delegation rejection; nonce-used with absent result index not covered. Open. |
| TA11 | Driver rehydration passed; full node/server child fails restart bootstrap before readiness. Cache eviction/index rebuild not covered. Open. |
| TA12–13 | Forged delegation rejection passed; revoked delegate and minimum-commit positive/negative cases stop at delegation rejection. Open. |
| TA14–15 | Missing CAS and tampered durable record rejected; same-generation publication races and application clock separation not fully verified. Open. |
| TA16–17 | Bounded cursor replay/resync authored in isolated child but not reached. Forged cursor rejection passed. Open. |
| TA18–20 | Complete slow-client, batch/size/backoff, outage/recovery and cross-user protected enumeration matrix absent. Open. |
| TA21 | Actual delayed TCP submission releases Viewer mutex and preserves original Unknown identity: one library test passed. Model waits, presence fairness and remaining callers open. |
| TA22–23 | Complete live fallback/legacy compatibility matrix absent. Open. |
| TA27–28 | Topology-independent identity derivation and unsupported guarantee demands not comprehensively exercised. Open. |
| TA25–26 | C not_run; production not_evaluated. |

Attempt 3: 6 passed, 6 failed, 2 ignored. Passing focused cases: lost-response gameplay/driver rehydration; forged delegation/cursor; tampered service signature; live writer lock; missing CAS bootstrap; tampered record bootstrap. Failed delegation cases report canonical runtime_world_id_mismatch; full server process restart fails before readiness. See immutable `/tmp/pre2-world-service-conformance-qa-attempt3/report.json` and `conformance.log`. A/B remain not_evaluated.

D3 configuration checks (not yet executed at this checkpoint):

```sh
rtk proxy ./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_viewer_live --no-default-features --features node-libp2p,test_tier_required world_service_bootstrap -- --nocapture
rtk proxy ./scripts/cargo-dev.sh test -p oasis7 --lib --no-default-features --features node-libp2p,test_tier_required agent_signer_requires_explicit_complete_valid_delegation_configuration -- --nocapture
```

These parser/assembly checks do not replace actual executable no-mount operation through live handlers. The shipped process reads explicit `OASIS7_WORLD_SERVICE_*` connection metadata and a distinct Agent signer/delegation configuration; no private values belong in reports.

Attempt 4 (immutable `/tmp/pre2-world-service-conformance-qa-attempt4/report.json`): 12 passed, 2 failed, 2 ignored. Real full process restart and endpoint-only switching passed with the same executable; the old request resolved to the exact receipt and the new endpoint returned a min-commit verified view. This provides focused TA03/TA24 transport evidence, not C topology evidence. Missing/tampered admitted setup boundary startup negatives passed. Signed Cognition receipt/feedback, deduplication/conflict/revocation and minimum-commit cases passed. Both sandboxed application cases remained incomplete: the regular child asserted the unsigned event variant instead of canonical DataCollectedAuthenticated; the genuine Wait helper lacked required provider configuration metadata. The QA event assertion is corrected but unrun. A/B remain not_evaluated pending all required consumers and application closure.

Focused application attempt 6 (`/tmp/pre2-focused-application-children-attempt6.log`): two actual parents selected, one passed and one failed. The regular application uses only service configuration, without legacy chain-status configuration. Actual sandbox denial, all five operations, signed gameplay, signed Cognition, configured provider enqueue, canonical receipt/feedback, lease settlement and nonzero receipt-gated memory passed. The parent independently verified the terminal feedback against the real canonical outbox and receipt lineage. The genuine Wait case reached canonical admission, settlement and selected wake; ResumeWake remained `Received { durability: Volatile }` until its finite deadline. Completed wake is not proved. The memory fixture now uses the existing permitted `session_private` lane; no memory policy was relaxed.

The runner now builds the actual `oasis7_viewer_live` executable with the same feature bundle, records its digest, and passes its path as `PRE2_VIEWER_BINARY`. Five newly authored shipped-process cases cover initial/periodic projection, exact recovery root/height/cursor and reconnect, Step/Play canonical non-advancement, signed CollectData handler, wrong trust/world/scope rejection, lost-ACK original Lookup, and actual TCP transport interruption/recovery. Each process receives connection configuration, runs in an isolated directory under the node-root-denying Darwin profile, and records an independent existing-file denial probe under that exact profile. These five cases are **unrun** at this checkpoint. The outage mechanism closes accepted service sockets before dispatch; this is transport outage evidence, distinct from the separately covered server process restart. The runner requires their evidence markers as well as all fourteen existing parent cases; a zero-selected filter is never acceptance evidence.

D3 parser/assembly checks above were executed independently: two binary tests and one Agent signer library test passed. Normalized receipt binding regressions (two), pure trace overflow regression (one), and admitted Wait cleanup-checkpoint retention regression (one) passed. These results do not establish the remaining live-handler matrix. A/B remain not_evaluated, C not_run, production not_evaluated.

Shipped protocol attempt 1 selected three cases: two positive snapshot timeouts and one nominal negative pass. This run omitted RequestSnapshot after Subscribe; the nominal negative pass is therefore vacuous and excluded from acceptance evidence. The corrected five cases require actual RequestSnapshot, verified recovery roots/heights/cursors, original Lookup digest observation, and a transport attempt during outage. They remain unrun until the current source is compiled and executed.


Current focused evidence (2026-10-07; historical rows above describe earlier checkpoints):

| Plan obligation | Actual evidence and remaining scope |
| --- | --- |
| TA03/24 | Attempt4 same-artifact endpoint switch and exact original receipt/min-commit view passed. No C claim. |
| TA04–08/15–17/21 | Service-only normal child passed genuine OS denial, five operations, signed gameplay/Cognition, provider enqueue, canonical receipt, settled lease and nonzero memory. Current shipped protocol attempt3 passed all five actual executable cases, including initial/periodic projection, exact recovery root/height/cursor, controls without canonical advance, genuine SessionRegister plus CollectData, lost ACK original Lookup, transport interruption/recovery and trust/world/scope refusal. Remaining D0 callers are assessed individually; these five tests do not prove every caller. |
| TA09/12–13 | Attempt4 conflict/revocation, signed Cognition and minimum-commit cases passed. TA10 absent-index/consumed-nonce behavior remains open. |
| TA11/14 | Genuine full process restart, missing CAS and tampered record/setup-boundary rejection passed. Disposable-cache eviction and concurrent publication-pin races remain open. |
| TA18 | Current client cooldown tests 2/2 and server admission tests 3/3 passed. Agent recreated scheduler cooldown test selected one and failed before HTTP at invalid canonical observation digest. Real production listener pressure remains unrun; serial QA dispatcher bypasses its admission loop. See [admission limits and rationale](world-service-admission.md). |
| TA19–20/22–23/27–28 | Focused outage evidence is above; protected enumeration, complete legacy/fallback compatibility, topology identity and unsupported guarantees still need mapped acceptance evidence. |
| Genuine Agent wake | Attempt10 selected three parents: normal passed; two wake cases failed. Positive genuine empty driver commit preserved non-time WorldState and all four registered continuation context axes matched; canonical Resume rejected with continuation_proposal_invalid. Resource-changing negative produced cognition_context_mismatch with baseline/precondition mismatch; its QA terminal-text guard was corrected afterward and awaits rerun. No completed wake claim. |
| TA25–26 | C not_run; production not_evaluated. A/B remain not_evaluated. |

Immutable latest logs: `/tmp/pre2-focused-application-children-attempt10.log` (1 passed, 2 failed), `/tmp/pre2-shipped-viewer-protocol-attempt3.log` (5 passed), `/tmp/pre2-client-cooldown-attempt1.log` (2 passed), `/tmp/pre2-status-admission-attempt1.log` (3 passed), `/tmp/pre2-agent-scheduler-cooldown-attempt1.log` (1 failed). Earlier shipped attempt2 was 3 passed/2 failed due missing SessionRegister; attempt3 adds the real signed registration and resolves both handler failures. Launcher service assembly 2/2, canonical chat refusal 1/1, isolated offline provider 1/1, exact newer-mirror preservation 1/1, six admission scenarios in one selected test, and mutex/Unknown identity 1/1 passed independently. Counts are selected test functions, not scenario totals.

The runner currently lists twenty exact parents, including separate genuine wake and protected-resource-drift negative. No full twenty-case run has occurred. Marker completeness, source/artifact identity and actual outcomes are required; nominal selection count alone cannot establish A/B acceptance.


Application attempt13 selected three parents: normal service-only application and resource-drift rejection passed; genuine positive failed after canonical Resume accepted (rejection none), because the repeated production helper required a selected wake that Resume had consumed. The fresh schedule witness was next_wake_tick None, execution/derived tick 9, valid-until 21, unexpired. This does not prove completed wake/Act. Fresh Resume schedule regression 1/1 and corrected bounded Agent recreated-query cooldown regression 1/1 subsequently passed. The earlier cooldown attempt2 hung at an unbounded test listener and was terminated by exact verified test PID; it has no terminal behavioral result and is excluded.

The production-listener pressure parent is authored and compiled, unrun. It calls the actual status-server constructor, exercises four slow same-IP streams, 429 with Retry-After, total deadline release, same-peer recovery and 64KiB/413, while asserting unchanged canonical height. Independent-source signed Describe is attempted with a real 127.0.0.2 bind; unsupported alias transport is explicitly not_run. Actual global saturation is not_run (pure global bound unit passed). The runner now requires twenty-one exact parents and pressure evidence; no full twenty-one-case run has occurred.


Two further bounded publication tests are authored, unrun: disposable `world/snapshot.json` eviction preserves canonical Lookup and exact view/cursor using retained identity/CAS; sixteen signed fixed-commit reads overlap four actual successor commits and retain the original generation, while the current projection advances consistently. These target TA11 and TA14; no result-index authority is removed or rebuilt by the fixture. They will be registered in the runner after actual selection/results. TA10 consumed legacy nonce with absent service index, TA20 protected enumeration, TA23 legacy routes, and TA27/28 identity/unsupported guarantees remain open.
