# Playability L4 Review Packet Template

## Purpose
- Use this packet as the canonical input bundle for one complete `L4` review run.
- Keep `L4A synthetic` and `L4B human` evidence in the same packet, but do not collapse their verdicts.

## Required Fields
- `change_scope`:
- `target_experience_claim`:
- `formal_surfaces`:
- `automated_evidence`:
- `playability_evidence`:
- `known_blockers`:
- `requested_roles`:
- `selected_personas`:
- `questions_to_probe`:
- `target_l4_lane`: `L4A_only` / `L4A_then_L4B`

## Optional Fields
- `artifact_paths`:
- `telemetry_hypothesis`:
- `limited_preview_context`:
- `outside_scope`:
- `persona_panel_request`:
- `open_questions`:

## Packet Notes
- Explicitly distinguish `player leverage` evidence from `world activity only` evidence.
- If a formal surface is blocked, say so here before asking roles or personas to infer higher-level conclusions.
- If the run only reached `L4A`, mark `L4B` as missing instead of leaving it implicit.
