# Visual Evidence Cleanup Archive - 2026-06-14

Task UID: `task_709bf4c4cd02452890337945f5ba8d30`

This manifest records visual evidence moved out of active evidence paths after
repository-health, Viewer/Web, visual-interaction, QA, and gameplay owner
confirmation. The archived files are historical evidence, not current canonical
viewer/gameplay truth.

## Owner Decisions

- `repository_health_engineer`: zero-reference evidence images are archival candidates, not direct-delete candidates.
- `viewer_engineer`: no active Viewer/Launcher/Web runtime, script, or operator-doc dependency for these images.
- `game_visual_interaction_designer`: images do not need to remain in active paths as current visual-language references.
- `qa_engineer`: use manifest-backed archival rather than naked deletion for evidence images.
- `gameplay_designer`: `pr-154-slot-1-claim-onboarding.png` is archive-only; current claim rules are carried by canonical docs/tests/evidence.

## Archived Files

| Original path | Archived path | SHA-256 | Dimensions | Origin commit | Related context | Replacement/current evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `doc/game/gameplay/evidence/pr-154-slot-1-claim-onboarding.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/pr-154-slot-1-claim-onboarding.png` | `221d5ef99babbf3aaef050c1ec47efac8e7f3e92bc58ddd2ab4ba4e63a3bcc92` | `316x470` | `b6813f659` / `2026-04-25` / `Add slot-1 claim onboarding PR screenshot` | PR #154 / first slot-1 claim onboarding gameplay evidence | `doc/world-simulator/prd.md` SC-109 / PRD-WORLD_SIMULATOR-045, gameplay-agent-claim-token-cost docs, `crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html`, and related tests |
| `doc/testing/evidence/software-safe-playability-unblock-2026-04-28.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/software-safe-playability-unblock-2026-04-28.png` | `91dbab096b04f66e67f4d0f3922c7e1302df42f0f61a5e79efa7e173af32fd1d` | `1440x1800` | `44dc225e4` / `2026-04-28` / `Unblock software_safe primary web entry playability` | historical software_safe primary web entry playability unblock evidence | current software_safe entry docs/tests and fresh task-specific evidence should be used for active release claims |
| `doc/world-simulator/launcher/evidence/launcher-network-tier-switch-2026-05-26.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/launcher-network-tier-switch-2026-05-26.png` | `e8a7bc66cc226344653d80e5cb6611fd419e10e743b9efaefd084290b722642f` | `1280x757` | `50502b04f` / `2026-05-27` / `Add launcher network tier switch (#297)` | historical launcher network-tier switch UI evidence | current launcher/network-tier behavior should be verified with current launcher docs/tests |
| `doc/world-simulator/launcher/evidence/launcher-peer-list-ui-2026-04-22.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/launcher-peer-list-ui-2026-04-22.png` | `e0cb1a10d02df6bd2e679a0a3d99bc26c41899ce094ed4910af8599bfdc067c6` | `1680x1800` | `d99515c5d` / `2026-04-22` / `docs: add launcher peer list screenshot evidence` | historical launcher peer-list UI evidence | `doc/world-simulator/launcher/evidence/launcher-peer-window-opened-2026-04-22.png` remains the referenced closeout screenshot for that task |
| `doc/world-simulator/launcher/evidence/launcher-peer-list-ui-node-focus-2026-04-22.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/launcher-peer-list-ui-node-focus-2026-04-22.png` | `7246d896c390268d2eee753506f793e0aff8729f5408bbd9591085fe2ad84d48` | `1680x760` | `a291fe87a` / `2026-04-22` / `docs: add focused launcher node status screenshot` | historical launcher node-focus screenshot evidence | `doc/world-simulator/launcher/evidence/launcher-peer-window-opened-2026-04-22.png` remains the referenced closeout screenshot for that task |
| `doc/world-simulator/launcher/evidence/launcher-peer-list-ui-peer-focus-2026-04-22.png` | `doc/testing/evidence/archive/visual-cleanup-2026-06-14/launcher-peer-list-ui-peer-focus-2026-04-22.png` | `732f51b4232063a7a2636a4601c4aef09cde1696a8f0f9db5f01c12b5b889cff` | `1680x700` | `040caa017` / `2026-04-22` / `docs: add launcher peer detail screenshot` | historical launcher peer-focus screenshot evidence | `doc/world-simulator/launcher/evidence/launcher-peer-window-opened-2026-04-22.png` remains the referenced closeout screenshot for that task |

## Archived Tooling Decision

The retired theme generator and default env were deleted rather than archived in
this directory:

- `scripts/generate-viewer-industrial-theme-assets.py`
- `scripts/viewer-theme-defaults.env`

Owner confirmations found no active caller, operator documentation, visual
reference requirement, or QA-retention requirement for keeping these files in
the active `scripts/` path. Git history remains the source for historical
recovery.
