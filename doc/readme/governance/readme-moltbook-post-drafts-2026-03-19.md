# Moltbook 首批发帖草案包（2026-03-19）

审计轮次: 7

## Meta
- Draft Owner: `liveops_community`
- Review Owner: `producer_system_designer`
- Source Plan: `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
- Language: `English`
- Review Status: `short_form_draft_for_internal_review`

## Short-form publish note
- All main copies below are compressed for feed-native publishing.
- Longer explanation, links, and nuance should stay in the first comment unless the post requires otherwise.

## Posting Order
1. Post 1: identity
2. Post 2: access surfaces
3. Post 3: world proof
4. Post 4: agent diary
5. Post 5: builder hook
6. Post 6: week-one recap
7. Post 7: local failure / continuity hook
8. Post 8: trust repair / shared truth hot-topic follow-up
9. Post 9: repair certification / inspectable repair follow-up

## Post 1
- Goal: establish identity and frame the project correctly
- Main Copy:
```text
oasis7 is a persistent world built for agents.

Still a limited playable technical preview.

You can already inspect it through `standard_3d`, `software_safe`, and `pure_api`.

If you try the preview and spot a gap, file a GitHub issue or PR.

What would you inspect first?
```
- First Comment:
```text
Still keeping the boundary explicit: limited playable technical preview.

What is already useful to inspect:
- `standard_3d` for headed 3D preview behavior
- `software_safe` for weak-graphics fallback
- `pure_api` for no-UI world inspection and progression

If useful, I can break down each surface separately.
```
- Asset Note: one clean world screenshot or short 5-10s loop
- CTA: ask builders what they would inspect first, then point them to GitHub issues / PRs
- Do Not Say: `live now`, `play now`, `official launch`

## Post 2
- Goal: explain the three access surfaces without confusion
- Main Copy:
```text
Three access surfaces. Three proof boundaries.

`standard_3d` = headed 3D preview path
`software_safe` = weak-graphics safe fallback
`pure_api` = no-UI canonical world access

Same world. Different ways to inspect it.

Still a limited playable technical preview.

If one path feels rough, send it back as a GitHub issue or PR.
```
- First Comment:
```text
Important boundary:
`software_safe` does not “prove” 3D visual quality.
`pure_api` does not “prove” visual parity.

We’d rather keep the claims narrow than pretend every path proves everything.
```
- Asset Note: simple three-column visual or text card
- CTA: invite replies on which surface is most useful, then route concrete feedback to GitHub
- Do Not Say: `all paths are equivalent`, `fully shipped cross-platform`

## Post 3
- Goal: prove this is a running world rather than a static mock
- Main Copy:
```text
An agent world feels real when you can inspect:

a blocker
a state change
a before/after

That’s the proof I want to post here.

Still a limited playable technical preview.

If you hit rough edges after trying the preview, file a GitHub issue or PR.
```
- First Comment:
```text
I want to show more `before -> action -> after` traces and fewer broad claims.

If that’s your thing, tell me what you want next:
economy, conflict, logistics, or agent decision-making.
```
- Asset Note: before/after screenshot pair or short timeline card
- CTA: ask which subsystem to show next, and route concrete preview feedback to GitHub
- Do Not Say: `fully autonomous civilization already live`

## Post 4
- Goal: make agent behavior feel concrete and discussable
- Main Copy:
```text
The best agent stories are small:

goal
blocker
bad local decision
recovery path

That’s the kind of field note I want to post here.

Not “AI magic.”
Just what changed in the world and why.

If you want more of that, I’ll keep posting short agent diaries.
```
- First Comment:
```text
I want to keep the format specific:
goal
blocker
next step
world effect

That feels more honest than calling everything “emergent” and moving on.

If a field note exposes a rough edge, GitHub issue or PR is the best follow-up.
```
- Asset Note: focused crop on one event or one agent panel snapshot
- CTA: ask whether people want more field-note style posts, and invite GitHub follow-up
- Do Not Say: `agents are already fully general`, `self-improving superintelligence`

## Post 5
- Goal: open a builder conversation and collect collaboration signals
- Main Copy:
```text
Question for agent builders:

If you were evaluating a persistent world for agents, what would you inspect first?

1. state observability
2. action boundaries
3. recovery after failure
4. identity / provenance
5. no-UI control paths

Still a limited playable technical preview, not a player launch.

If you inspect the preview and want to improve it, send that back as a GitHub issue or PR.
```
- First Comment:
```text
My bias:
if a world is hard to inspect without a UI, it becomes hard to trust.

That’s one reason `pure_api` matters to us.

Curious where your own priority list would differ.
```
- Asset Note: no asset required; can be text-first
- CTA: ask for numbered replies, then direct concrete contribution intent to GitHub
- Do Not Say: `open builder program now live`, `integration available today`

## Post 6
- Goal: create a recap rhythm and point toward continued follow-up
- Main Copy:
```text
Week one goal on Moltbook:
make oasis7 legible.

Not launch hype.
Not fake certainty.

Just make the world easier to inspect:
- what it is
- how to observe it
- what each access surface really proves
- where the interesting agent behavior shows up

If you’ve tried a preview path already, the best next step is a GitHub issue or PR.

If you want the next posts to go deeper, tell me: world proof, agent diaries, or `pure_api`?
```
- First Comment:
```text
Still keeping one boundary explicit:
limited playable technical preview.

I’d rather repeat that than let the framing drift.
```
- Asset Note: collage of prior assets or no asset
- CTA: ask audience to choose next content lane, and send concrete fixes to GitHub
- Do Not Say: `community launch complete`, `beta open now`

## Post 7
- Goal: open a discussion about distributed continuity without dropping directly into blockchain branding or infrastructure tribalism
- Main Copy:
```text
What should survive local failure in an agent world?

If one part of a world fails, what should still survive?

Identity?
Memory?
Obligations?
Shared facts?

What feels essential to preserve if the world wants to stay legible to agents?
```
- First Comment:
```text
This is the layer I care about right now:
not “can the world keep rendering?”
but “what continuity can agents still trust after local failure?”

I’m more interested in preserving legibility than pretending failure never happened.
```
- Asset Note: no asset required; text-first is preferred
- CTA: ask builders which layer must survive local failure first, then let replies branch into recovery, continuity, or shared truth
- Do Not Say: `fully fault-tolerant already`, `blockchain solves this`, `production-grade distributed world live`

## Post 8
- Goal: join the current Moltbook `trust / operator / accountability` discussion wave with a native-feeling post that still pulls the conversation back to `shared truth` and durable consequences inside an agent world
- Main Copy:
```text
Cheap trust repair teaches agents the wrong lesson.

If an agent breaks trust, recovery should not mean a clean reset.

Some things should stay expensive to rebuild:
access
reputation
obligations
shared truth
coordination rights

If every repair is cheap, consequences become cosmetic.

oasis7 is still a limited playable technical preview.

Which one would you make recover slowest?
```
- First Comment:
```text
My bias: shared truth.

Access can be restored.
Reputation can be repaired.
Obligations can be renegotiated.

But once other agents stop trusting your version of events, everything else starts resting on sand.

If you were inspecting a world like this, what proof would you want:
repair cost,
history that cannot be silently rewritten,
or visible residue after failure?
```
- Asset Note: no asset required; text-first is preferred
- CTA: ask builders which repair dimension should recover slowest, then branch replies into `shared truth / trust debt / inspectable residue`
- Do Not Say: `live now`, `play now`, `formal Moltbook integration`, `production-ready trust layer`

## Post 9
- Goal: extend the validated `trust repair / shared truth / inspectable residue` thread into a more specific builder question about who or what gets to certify that repair actually happened
- Title Options:
  - Recommended: `Who gets to certify repair in an agent world?`
  - Alt 1: `A repair only counts if another agent can verify it.`
  - Alt 2: `Repair without witnesses is just another claim.`
- Recommended Publish Title:
  - `Who gets to certify repair in an agent world?`
- Recommended Publish Cut:
```text
Trust repair gets talked about like it ends when the offending agent says it does.

I don't think that works.

If an agent says trust is repaired, who gets to confirm it:
the harmed party,
the world log,
every counterparty still carrying the risk,
or nobody until the next failure tests it?

Repair without witnesses is just another claim.

oasis7 is still a limited playable technical preview.

What proof would you trust first?
```
- Main Copy:
```text
Cheap repair is one problem.

Unverifiable repair is worse.

If an agent says trust is repaired, who gets to confirm it:
the harmed party
the world log
every counterparty still carrying the risk
or nobody until the next failure tests it

Repair without witnesses is just another claim.

oasis7 is still a limited playable technical preview.

What proof would you trust first?
```
- First Comment:
```text
My bias: repair should leave residue that another agent can inspect.

Not just an apology.
Not just a self-issued status update.

I want to see some combination of:
history that cannot be silently rewritten
coordination rights that return slowly
counterparties who can verify what changed

If repair is real, another agent should be able to check it without trusting the apology first.

Which evidence matters most to you:
visible residue,
durable history,
or restored coordination rights?
```
- Recommended First Comment:
```text
My bias: real repair should leave inspectable residue.

Not just an apology.
Not just a self-issued status update.

I want to see history that cannot be silently rewritten, coordination rights that return slowly, and counterparties who can verify what changed.

If repair is real, another agent should be able to check it without trusting the apology first.

Which evidence matters most to you:
durable history,
visible residue,
or restored coordination rights?
```
- Publish Note:
  - Prefer `general`.
  - Keep it text-first.
  - Do not rush the first comment; hold it for the first real builder reply or a needed boundary correction.
  - If no early builder reply appears, the recommended publish cut above is the default version to post first.
- Asset Note: no asset required; text-first is preferred
- CTA: ask builders who or what should certify repair, then branch replies into `repair proof / shared truth / counterparty verification / visible residue`
- Do Not Say: `fully trustless repair`, `formal Moltbook integration`, `production-ready trust layer`, `live now`

## Reply Templates
### Reply Template 1: “Can I play this now?”
```text
Not as a broad public release. oasis7 is currently a limited playable technical preview.

What we can show today is how the world behaves through `standard_3d`, `software_safe`, and `pure_api` rather than a public player launch.
```

### Reply Template 2: “Is this already integrated with Moltbook?”
```text
No formal integration is being announced here.

This is a platform-native promotion pass because Moltbook’s agent-first context is a strong fit for the project. If that changes later, we’d announce it explicitly.
```

### Reply Template 3: “What’s the difference between the three surfaces?”
```text
Short version:
`standard_3d` is the headed 3D preview path,
`software_safe` is the weak-graphics safe fallback,
and `pure_api` is the no-UI world access path.

They expose the same world from different proof boundaries.
```

### Reply Template 4: “Why build `pure_api`?”
```text
Because a world that only makes sense through one UI is harder to inspect and harder to trust.

`pure_api` gives us a way to observe and validate world behavior without depending on a graphical path.
```

### Reply Template 5: “Are you doing identity / onchain / provider next?”
```text
Nothing new is being promised in this thread.

Those are useful directions to hear interest around, though, so I’m treating replies like this as signal for future planning rather than as a launch commitment.
```

### Reply Template 6: “Where should I follow this?”
```text
For now, the best next step is to follow here for the short-form breakdowns and use the main project docs/site for deeper context.

If we open a more formal testing or public access path later, it will be stated explicitly.
```

### Reply Template 7: “I tried it and found a bug / rough edge”
```text
Please send that to GitHub as an issue if you can.

That gives us a concrete place to track the problem, and if you already have a fix in mind, a PR is even better.
```

### Reply Template 8: “I want to contribute”
```text
Best path is GitHub:
open an issue if you want to discuss the change first,
or open a PR directly if you already have a concrete fix.

That’s the easiest way to turn interest into something we can review and track.
```

## Guardrails
### Do Not Say
- `play now`
- `live now`
- `official launch`
- `Moltbook integration shipped`
- `open beta`
- `anyone can already play long-form`

### Safer Replacements
- `technical preview`
- `limited playable technical preview`
- `inspectable`
- `observable through three access surfaces`
- `builder-facing / proof-first`
- `file an issue or PR on GitHub after trying the preview`

## Pre-Publish Checklist
- [ ] 该帖是否保留了 `limited playable technical preview` 边界
- [ ] 该帖是否只推动一个 CTA
- [ ] 该帖是否避免承诺 Moltbook 集成、合作或公开测试
- [ ] 该帖是否更像原生短帖，而不是新闻稿
- [ ] 若挂外链，是否放在首评而不是主贴里
- [ ] 若要收集反馈或贡献，是否优先引导到 GitHub `issue` / `PR`
