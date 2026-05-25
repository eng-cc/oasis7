# shared_devnet shared access check (2026-05-23)

## Meta
- owner_role: `qa_engineer`
- track: `shared_devnet`
- window_id: `shared-devnet-live-reset-20260523`
- candidate_id: `shared-devnet-live-reset-20260523-01`

## Shared Endpoint
- `viewer_url`:
  - `http://39.104.204.172:4173/software_safe.html?ws=ws://39.104.204.172:5011&test_api=1`
- `live_addr`:
  - `39.104.204.172:5023`
- `operator_contact_ref`:
  - `.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md`
- `independent_operator_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-shared-access-2026-05-23.md`

## What changed
- The old local-only player entry blocker has been removed.
- Shared player entry has been moved onto the cloud sequencer host with public IPv4:
  - host: `39.104.204.172`
  - viewer HTTP: `http://39.104.204.172:4173/software_safe.html?ws=ws://39.104.204.172:5011&test_api=1`
  - viewer WebSocket bridge: `39.104.204.172:5011`
  - live TCP: `39.104.204.172:5023`
  - chain status / health: `39.104.204.172:5631`

## Deployment facts
- Cloud sequencer env now pins:
  - `STATUS_BIND=0.0.0.0:5631`
  - `PLAYER_ENTRY_ENABLE=1`
  - `PLAYER_ENTRY_VIEWER_BIND=0.0.0.0:5023`
  - `PLAYER_ENTRY_WEB_BIND=0.0.0.0:5011`
  - `PLAYER_ENTRY_HTTP_BIND=0.0.0.0`
  - `PLAYER_ENTRY_HTTP_PORT=4173`
  - `PLAYER_ENTRY_LLM_MODE=llm`
- The cloud `start-node.sh` hotfix now exports `OASIS7_LLM_*` into `oasis7_viewer_live`.
- The cloud release `d104864026bb-triad-full-game-nodes-20260516-213138` now has:
  - `bin/oasis7_viewer_live`
  - `web/`

## Access Validation
- `access_mode`:
  - `shared_multi_operator`
- `validated_by`:
  - `qa_engineer`
- `validated_at`:
  - `2026-05-23 22:03:07 CST`
- `validation_steps`:
  - independent workstation opened the cloud viewer endpoint
  - independent workstation reached the live WebSocket-backed software-safe page
  - independent cloud storage host reached the sequencer player-entry HTTP endpoint and live health endpoint over the shared network
  - candidate truth matched current bundle / gate
- `candidate_bundle_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- `candidate_gate_summary_ref`:
  - `doc/testing/evidence/generated-shared-network-gates/shared_devnet-20260523-214249/summary.md`
- `evidence_ref`:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-access-proof/cloud-shared-access-20260523.png`
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-access-proof/cloud-shared-access-state-20260523.json`

## Verification
- Public HTTP probe from this workstation:
  - `curl -I -fsS 'http://39.104.204.172:4173/software_safe.html?ws=ws://39.104.204.172:5011&test_api=1'`
  - result: `HTTP/1.0 200 OK`
- Browser proof from this workstation:
  - screenshot: `.tmp/shared-devnet-live-reset-20260523-01/shared-access-proof/cloud-shared-access-20260523.png`
  - state dump: `.tmp/shared-devnet-live-reset-20260523-01/shared-access-proof/cloud-shared-access-state-20260523.json`
  - key facts:
    - `connectionStatus=connected`
    - `controlProfile=live`
    - `worldId=live-runtime-llm_bootstrap`
    - `wsUrl=ws://39.104.204.172:5011`
- Local health probe on the cloud sequencer:
  - `curl -fsS http://127.0.0.1:5631/healthz`
  - result: `{"ok":true}`
- Local chain status probe on the cloud sequencer:
  - `curl -fsS http://127.0.0.1:5631/v1/chain/status`
  - result sample: `committed_height=829`, `network_committed_height=829`, `last_error=null`
- Cloud sequencer listening surface after restart:
  - `0.0.0.0:4173`
  - `0.0.0.0:5011`
  - `0.0.0.0:5023`
  - `0.0.0.0:5631`
- `oasis7_viewer_live` environment sample on the cloud sequencer confirms:
  - `OASIS7_LLM_MODEL=gpt-5.4-mini`
  - `OASIS7_LLM_BASE_URL=https://api.letai.run/v1`
  - `OASIS7_LLM_API_KEY=...`
- Independent cloud storage host probe:
  - `curl -I -fsS http://172.26.53.91:4173/`
  - result: `HTTP/1.0 200 OK`
  - `curl -fsS http://172.26.53.91:5631/healthz`
  - result: `{"ok":true}`

## Conclusion
## Verdict
- `lane_result`:
  - `pass`
- `reason`:
  - the endpoint is no longer local-only
  - a real cloud shared endpoint exists on the sequencer host
  - operator/handoff truth is pinned in the task execution log and this candidate-window evidence
  - independent access evidence is now pinned from both an external workstation and the cloud storage host path in the same candidate window
