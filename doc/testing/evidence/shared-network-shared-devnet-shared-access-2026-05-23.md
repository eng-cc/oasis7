# shared_devnet shared access check (2026-05-23)

历史运行边界：本文只保留所列 candidate/window 的原始观测、失败、修复尝试和操作记录；“当前”、后续安排、部署/恢复命令不构成今日环境真值或执行授权。退役环境的总体归属见[legacy shared-devnet provenance](legacy-shared-devnet-provenance-2026-07-26.md)，当前操作使用[正式网络 runbook](../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md)并重新采集同窗证据；不得据此恢复退役网络或改写旧判定。

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
  - 固定历史定位：[原始 execution log](https://github.com/nil-studio/oasis7/blob/0450e84dce0ae037be3a7fe8041f7f10f154e721/.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md)；原路径仅为历史身份，独立人员交接的证明限制见本文末尾补注。
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
  - `doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md`
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

## 历史引用与 handoff 证明边界补注

上述 verdict 和 `shared_multi_operator` 为原窗口的历史记录，保持原值；其中 “operator/handoff truth is pinned” 不应扩读为已验证独立人员交接。可取回的 [原日志](https://github.com/nil-studio/oasis7/blob/0450e84dce0ae037be3a7fe8041f7f10f154e721/.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md) 记录访问与 lane 处置；本页的 workstation/cloud storage host 证明来自不同访问位置，未给出可关联的独立人员接手记录，`independent_operator_ref` 的自引用也不能补足该证明。

[May 24 QA/producer 决定](https://github.com/nil-studio/oasis7/blob/0450e84dce0ae037be3a7fe8041f7f10f154e721/.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md#L198-L220) 确实存在，但针对 mixed-topology catch-up 和 uplift，并非 shared-access 独立人 handoff 验收。保留原始 `pass` 不等于新增这项证明，不表示已补做交接或当前 readiness。
