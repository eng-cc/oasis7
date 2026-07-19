# GitHub Pages Cinematic Consequence Refresh (2026-07-19)

- Owner role: `game_visual_interaction_designer`
- Task: #2449 (`task_409c0a8f6d714a0293e308e290c4ac5d`)
- Routes: `site/index.html`, `site/en/index.html`
- Image2 target: `doc/site/github-pages/assets/github-pages-cinematic-consequence-image2-2026-07-19.png`
- Browser baseline: local static Pages at `1440x900` and `390x844`

## Decision

Use **Cinematic consequence**. The homepage leads with one short resource-pressure
dilemma, one sentence defining player agency and persistence, one dominant event
chain action, the exact preview status, and one atmospheric world visual. The
next section turns the loop into an open three-beat rail instead of another card
grid.

The Image2 target is a hierarchy and rhythm reference. It is not a gameplay
capture, implementation proof, or a source of public claims. All meaningful copy,
status, actions, alt text, and the non-gameplay caption remain in HTML.

## Comparable-site findings

- Norland opens with a story-generating strategy promise, a dominant trailer or
  world visual, and a purchase action before its longer description.
- Factorio opens with one plain-language gameplay definition and immediate demo
  or purchase paths; deeper systems and community content follow later.
- Frostpunk and Dune: Awakening sell player burden and world fantasy before
  enumerating mechanics.
- Against the Storm likewise establishes role, hostile pressure, and settlement
  objective before feature detail.

The shared pattern is not "less information everywhere". It is one promise and
one action first, followed by progressive disclosure of systems and trust detail.
Oasis7 retains a stricter boundary because it is a limited playable technical
preview rather than a released game.

## Content and hierarchy changes

- Shorten the Chinese hero from a multi-clause choice list to
  `资源告急。文明会怎么选？`; keep the English route equivalent in intent.
- Make the event-chain action primary and demote the explanation action to a
  text link.
- Remove the three hero fact columns. Their meaning is already present in the
  headline, subtitle, and next section.
- Replace the first three bordered cards with an open consequence rail:
  pressure -> Agent response -> persistent result.
- Preserve the proof and download boundary sections below. Future compression of
  their trust-critical detail must use progressive disclosure, not deletion.

## Visual implementation brief

- Let the existing asteroid civilization art bleed behind the hero layout and
  use a strong dark-to-clear gradient to protect HTML copy.
- Reduce borders in the hero and first loop section; use spacing, scale, and a
  single rail to establish rhythm.
- On mobile, render promise and actions before the image. Maintain 44px actions,
  no horizontal overflow, a readable crop, and an adjacent non-gameplay caption.
- Do not add fake telemetry, fake metrics, autoplay, scroll-jacking, or claims
  baked into generated art.

## Acceptance

- At `1440x900`, the category, short H1, subtitle, dominant CTA, exact preview
  status, atmospheric visual, and start of the consequence rail are visible.
- At `390x844` and `360x800`, promise and CTA appear before the image, actions
  remain at least 44px high, and there is no horizontal overflow.
- Chinese and English routes retain the same sequence and preview boundary.
- Link, homepage claim/parity, manual sync, download, keyboard, no-JS, and
  reduced-motion checks remain valid.
- Real browser screenshots remain implementation evidence; the Image2 target
  does not replace them.
