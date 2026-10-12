# Independent prepared-world task playtest

Candidate: b59d010556e39beaf1928eb1e95aa9aad099e180. Profile: prepared_world attached_codex_subagent. Isolation: instruction_only/source_visible; not sourceblind. Read only delivered formal Skill, public help, and authorized CLI responses. No source, logs, world files, signer/storage, other sessions, or witness inspection.

13 CLI calls including help, two effective movement decisions, within 240 seconds. Local signed CLI used by this attached Codex executor; no paid Runtime or GUI.

Revision 1: selected native.move_agent to (200,0,0) cm from observed (0,0,0). Context retry-player-b59d0105-round1. Correlation task-request-2681614d3f78741c47efd77e944c81c6fc5ebcd8658f2b9cf1f03af651de0695. action:3 committed at position 45 with verified finality; receipt blake3:02b9f4bf3e5774de0b725ba8b547655ab1b1bbebba0b0eddc5969e8a649b7fdf. Original action lookup and exact unchanged move retry returned identical correlation, action, position and receipt. Fresh observation confirmed (200,0,0) and one AgentMoved event.

Task reread detected owner-published goal revision 2, task revision 4, executor epoch 1 unchanged. Fresh context retry-player-b59d0105-round2 delivered complete current goal and first movement result. Selected native.move_agent to (-200,0,0) cm. Correlation task-request-378a95d0901ff4d8b5229761d0c839cddc747f72f28f58634e33538090cffdb2. action:4 committed at position 53 with verified finality; receipt blake3:b2c4cb573c7578cdf8449424ec649f69b5c7d4d037393739afd4438e487cfe98. Fresh final observation at capture position 56 confirmed (-200,0,0), last_active 53, and second AgentMoved from (200,0,0).

Final observation context retired with fixed executor_aborted outcome, committed at position 58. Its failed=true receipt describes the deliberately unused observation request, not either successful movement. No third effective movement decision.

No transport failures, rejected actions, unknown or pending action outcomes occurred. Lookup/retry recovered the exact original action without replacement. No goal or executor binding changed by this player.

Fees unknown: no fee settlement fields were exposed, resources.amounts remained empty; not evidence of zero fees.

Gaps: cold entry, OpenClaw integration, external paid Runtime, GUI, full Game HTTP catalog, transport-loss recovery, grant expiry/revocation and old-context rejection after goal publication not tested.
