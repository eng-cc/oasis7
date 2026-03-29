# Controlled Limited Preview Invite Pack（2026-03-22）

审计轮次: 1

## Meta
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Round ID: `LTP-20260322-R1`
- Channel Focus: `GitHub issue`（primary for round-1） + `GitHub Builder Channels`
- Candidate Reference: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- Claim Envelope: `limited playable technical preview`

## 1. Round Goal
- Run one controlled builder-facing callout without claim drift.
- Collect the first 3 valid signals for producer / QA review.
- Convert at least 1 signal into GitHub issue/PR or internal owner follow-up.

## 2. Controlled Builder-Facing Callout
### Main Copy
```text
oasis7 is currently a limited playable technical preview.

We are opening a small builder-facing round to inspect the candidate path, not announcing a public launch.

If you inspect a rough edge, send it back as a GitHub issue or PR.

Current focus:
- headed world path
- pure_api parity
- recovery after failure

Reply in this thread if you want the evidence link and builder route.
```

### Follow-up Comment
```text
Boundary stays explicit:
- limited playable technical preview
- controlled builder-facing access
- not a closed beta announcement
- not a public launch

If your interest is concrete bug/proof-boundary feedback, GitHub is still the cleanest return path.
```

### Safe Reply Snippets
- `This round is still a limited playable technical preview, not a public launch.`
- `We are collecting builder feedback, not opening broad player access.`
- `If you can reproduce the issue, the best next step is a GitHub issue or PR.`
- `Nothing in this thread changes the current stage or announces closed beta.`

## 3. Monitoring Windows
| Window | What To Check | Action |
| --- | --- | --- |
| `T+15m` | main copy visible, first comment intact, no immediate claim drift | correct formatting / wording issues immediately |
| `T+1h` | comments, mentions, DMs, obvious misunderstanding | reply to claim drift first, then classify valid feedback |
| `T+4h` | repeated confusion, concrete bug reports, builder intent | open escalation thread, assign owner, tag summary buckets |
| `T+24h` | aggregated signal quality, unresolved incidents, conversion to issue/PR | prepare producer summary and same-day / next-day devlog |

## 4. Signal Buckets
| Bucket | Meaning | Default Owner | Example |
| --- | --- | --- | --- |
| `Blocking` | candidate path bug, crash, unusable flow, or gate-relevant regression | `qa_engineer` | `pure_api cannot advance`, `viewer path crashes` |
| `Opportunity` | friction, docs gap, wording problem, low-risk UX issue | `liveops_community` or relevant engineer | `builder asks for clearer evidence link`, `CTA wording rough` |
| `Idea` | future direction, feature interest, platform question | `producer_system_designer` | `more world proof`, `identity/onchain interest` |

## 5. Claim Drift Rules
Mark `claim drift = yes` if the external statement implies any of the following:
- `closed beta`
- `play now`
- `live now`
- `public launch`
- `official integration announced`
- broad public playability instead of controlled builder-facing access

If `claim drift = yes`:
1. Correct the statement in the same monitoring window.
2. Log it in the feedback template.
3. If high-visibility or repeated, open an incident row and escalate to `producer_system_designer`.

## 6. Producer Summary Template
Use this at `T+24h`.

| Field | Value |
| --- | --- |
| Round ID | `LTP-20260322-R1` |
| Candidate ID | current candidate tag/date |
| Valid Signal Count | integer |
| Signal Mix | `Blocking=X / Opportunity=Y / Idea=Z` |
| Claim Drift Count | integer |
| Converted Actions | issue / PR / owner follow-up links |
| Highest Risk | one-line summary |
| Recommendation | `continue` / `hold` / `reassess` |

## 7. Stop Conditions
Pause the round and escalate immediately if:
- a high-visibility thread frames the round as `closed beta` or `public launch` and cannot be corrected promptly
- a `Blocking` issue proves a candidate path is not actually usable
- repeated claim drift suggests the current callout wording is still too loose

## 8. Completion Definition
- 1 controlled builder-facing callout is prepared and approved for use.
- Monitoring windows and correction rules are fixed before posting.
- Feedback / incident templates can capture every signal with `Round ID`, bucket, owner, and next action.
- Producer can read one summary row and decide `continue / hold / reassess`.
