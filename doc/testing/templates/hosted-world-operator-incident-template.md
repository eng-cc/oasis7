# Hosted Access Operator Incident Template

审计轮次: 1

## Meta
- `incident_id`:
- `reported_at`:
- `reported_by`:
- `world_id`:
- `deployment_mode`: `hosted_public_join`
- `owner`: `liveops_community`

## Exposure
- Note: this template covers hosted access into the unified persistent world. It must not describe a separate player world.
- `shared_url`:
- `intended_join_url`:
- `exposed_surface`: `operator_url` / `private_control_plane` / `claims_boundary` / `other`
- `publicly_visible_duration`:
- `evidence_ref`:

## Immediate Action
- `sharing_stopped`: `yes` / `no`
- `claims_frozen`: `yes` / `no`
- `wrong_url_retracted`: `yes` / `no`
- `corrected_join_url_posted`: `yes` / `no`
- `runtime_restart_required`: `yes` / `no`
- `session_revoke_required`: `yes` / `no`

## User Impact
- `player_impact`: `none` / `limited` / `unknown`
- `operator_impact`:
- `sensitive_state_exposed`: `yes` / `no` / `unknown`

## Communication Boundary
- `public_message_state`:
- `preview_claim_used`: `yes` / `no`
- `overclaim_detected`: `yes` / `no`
- `correction_message_ref`:

## Follow-up
- `runtime_owner_action`:
- `qa_owner_action`:
- `liveops_owner_action`:
- `producer_review_required`: `yes` / `no`
- `next_check_at`:
