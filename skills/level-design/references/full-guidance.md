# Full Guidance

This file preserves the detailed guidance that used to live in `level-design/SKILL.md`. The entrypoint now uses progressive disclosure: read this file only when the task needs the detailed patterns, examples, or reference material.

# Level Design

## When to Use

Use this skill when:

- level pacing, layout, traversal, difficulty, encounters, or environmental storytelling are in scope
- a task needs reusable level design principles or critique

Do not use this skill when:

- the task does not match the trigger conditions above


## Level Design Philosophy

```
┌─────────────────────────────────────────────────────────────┐
│                    LEVEL DESIGN PILLARS                     │
├─────────────────────────────────────────────────────────────┤
│  1. FLOW: Guide the player naturally through space         │
│  2. PACING: Control intensity and rest moments             │
│  3. DISCOVERY: Reward exploration and curiosity            │
│  4. CLARITY: Player always knows where to go               │
│  5. CHALLENGE: Skill tests that teach and satisfy          │
└─────────────────────────────────────────────────────────────┘
```

## Level Structure Patterns

```
LINEAR LEVEL:
┌─────────────────────────────────────────────────────────────┐
│  [Start] → [Tutorial] → [Challenge] → [Boss] → [End]       │
│                                                              │
│  PROS: Easy to pace, clear direction                        │
│  CONS: Limited replay value, less exploration              │
│  BEST FOR: Story-driven games, action games                │
└─────────────────────────────────────────────────────────────┘

HUB & SPOKE:
┌─────────────────────────────────────────────────────────────┐
│              [Level A]                                       │
│                  ↑                                           │
│  [Level B] ← [HUB] → [Level C]                              │
│                  ↓                                           │
│              [Level D]                                       │
│                                                              │
│  PROS: Player choice, non-linear progression                │
│  CONS: Can feel disconnected                                │
│  BEST FOR: RPGs, Metroidvanias, open-world                 │
└─────────────────────────────────────────────────────────────┘

METROIDVANIA:
┌─────────────────────────────────────────────────────────────┐
│  ┌───┐   ┌───┐   ┌───┐                                      │
│  │ A │───│ B │───│ C │ (locked: need ability X)            │
│  └─┬─┘   └───┘   └───┘                                      │
│    │       │                                                 │
│  ┌─┴─┐   ┌─┴─┐                                              │
│  │ D │───│ E │ (grants ability X)                          │
│  └───┘   └───┘                                              │
│                                                              │
│  PROS: Rewarding exploration, ability gating                │
│  CONS: Can get lost, backtracking tedium                   │
│  BEST FOR: Exploration games, 2D platformers               │
└─────────────────────────────────────────────────────────────┘
```

## Pacing & Flow

```
INTENSITY GRAPH (Good Pacing):
┌─────────────────────────────────────────────────────────────┐
│  High │      ╱╲           ╱╲    ╱╲                          │
│       │     ╱  ╲    ╱╲   ╱  ╲  ╱  ╲    ╱╲                  │
│       │    ╱    ╲  ╱  ╲ ╱    ╲╱    ╲  ╱  ╲                 │
│  Low  │───╱──────╲╱────╲──────────────╲╱────────           │
│       └────────────────────────────────────────→ Time       │
│                                                              │
│  PATTERN: Build → Peak → Rest → Build → Peak → Rest        │
└─────────────────────────────────────────────────────────────┘

PACING ELEMENTS:
┌─────────────────────────────────────────────────────────────┤
│  BUILD-UP:                                                   │
│  • Introduce new mechanic safely                            │
│  • Increase difficulty gradually                            │
│  • Foreshadow upcoming challenge                            │
├─────────────────────────────────────────────────────────────┤
│  PEAK (Combat/Puzzle):                                       │
│  • Test player skills                                       │
│  • High stakes moments                                      │
│  • Boss encounters                                          │
├─────────────────────────────────────────────────────────────┤
│  REST:                                                       │
│  • Safe zones                                               │
│  • Story/exploration moments                                │
│  • Resource replenishment                                   │
│  • Save points                                              │
└─────────────────────────────────────────────────────────────┘
```

## Environmental Storytelling

```
STORYTELLING TECHNIQUES:
┌─────────────────────────────────────────────────────────────┐
│  VISUAL NARRATIVES:                                          │
│  • Abandoned objects tell stories                           │
│  • Environmental damage shows history                       │
│  • Character belongings reveal personality                  │
│  • Graffiti and notes provide context                       │
├─────────────────────────────────────────────────────────────┤
│  ATMOSPHERE:                                                 │
│  • Lighting sets mood (warm=safe, cold=danger)             │
│  • Sound design reinforces tone                             │
│  • Weather reflects narrative beats                         │
│  • Music cues emotional state                               │
├─────────────────────────────────────────────────────────────┤
│  DISCOVERY LAYERS:                                           │
│  • Surface: Obvious story elements                          │
│  • Hidden: Rewards for exploration                          │
│  • Deep: Lore for dedicated players                         │
└─────────────────────────────────────────────────────────────┘
```

## Whitebox Process

```
LEVEL DESIGN WORKFLOW:
┌─────────────────────────────────────────────────────────────┐
│  1. CONCEPT (Paper)                                          │
│     • Sketch rough layout                                   │
│     • Define key beats                                      │
│     • Identify player path                                  │
│                    ↓                                         │
│  2. WHITEBOX (Engine)                                        │
│     • Block out with primitives                             │
│     • Test scale and timing                                 │
│     • Place placeholder enemies                             │
│                    ↓                                         │
│  3. PLAYTEST                                                 │
│     • Get feedback early                                    │
│     • Iterate on layout                                     │
│     • Fix flow issues                                       │
│                    ↓                                         │
│  4. ART PASS                                                 │
│     • Replace with final art                                │
│     • Add lighting                                          │
│     • Polish visual details                                 │
│                    ↓                                         │
│  5. FINAL POLISH                                             │
│     • Audio integration                                     │
│     • VFX placement                                         │
│     • Performance optimization                              │
└─────────────────────────────────────────────────────────────┘
```

## Player Guidance

```
VISUAL GUIDANCE TECHNIQUES:
┌─────────────────────────────────────────────────────────────┐
│  LIGHTING:                                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │     [Dark]    [BRIGHT PATH]    [Dark]               │   │
│  │                    ↓                                  │   │
│  │              [OBJECTIVE]                             │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  LANDMARKS:                                                  │
│  • Tall structures visible from distance                    │
│  • Unique colors for important locations                    │
│  • Repeated motifs for path clarity                         │
│                                                              │
│  ARCHITECTURE:                                               │
│  • Lines leading to objectives                              │
│  • Framing important elements                               │
│  • Negative space around key items                          │
└─────────────────────────────────────────────────────────────┘
```

## Difficulty Progression

```
TEACHING THROUGH DESIGN:
┌─────────────────────────────────────────────────────────────┐
│  STEP 1: Safe Introduction                                   │
│  ┌─────────────────────────────────────────────┐           │
│  │  [Player] ──→ [Gap] ──→ [Platform]          │           │
│  │  (No enemies, can't die)                    │           │
│  └─────────────────────────────────────────────┘           │
│                                                              │
│  STEP 2: Add Stakes                                          │
│  ┌─────────────────────────────────────────────┐           │
│  │  [Player] ──→ [Gap+Spikes] ──→ [Platform]   │           │
│  │  (Same mechanic, consequence for failure)   │           │
│  └─────────────────────────────────────────────┘           │
│                                                              │
│  STEP 3: Add Complexity                                      │
│  ┌─────────────────────────────────────────────┐           │
│  │  [Player] ──→ [Moving Platform] ──→ [Goal]  │           │
│  │  (Timing + previously learned jump)         │           │
│  └─────────────────────────────────────────────┘           │
│                                                              │
│  STEP 4: Combine Mechanics                                   │
│  ┌─────────────────────────────────────────────┐           │
│  │  [Player] ──→ [Enemy] + [Gap] ──→ [Reward]  │           │
│  │  (Combat + platforming together)            │           │
│  └─────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Troubleshooting

```
┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Players get lost                                   │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Add stronger visual guides (lighting, landmarks)          │
│ → Use environmental cues (footprints, trails)               │
│ → Place NPCs or signs at decision points                    │
│ → Add map/compass UI elements                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Level feels too long/boring                        │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Add more pacing variety (peaks and valleys)               │
│ → Cut redundant sections                                    │
│ → Add shortcuts for backtracking                            │
│ → Place more rewards and discoveries                        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Difficulty spikes frustrating players              │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Add training area before hard section                     │
│ → Place checkpoint closer to challenge                      │
│ → Provide optional resources before boss                    │
│ → Playtest with different skill levels                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Players skip optional content                      │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Make secrets more visible (partial reveal)                │
│ → Place rewards near critical path                          │
│ → Use audio cues for hidden areas                           │
│ → Reward exploration with meaningful items                  │
└─────────────────────────────────────────────────────────────┘
```

## Level Metrics

| Metric | Action | Puzzle | RPG | Platformer |
|--------|--------|--------|-----|------------|
| Avg. Completion | 5-10 min | 10-20 min | 30-60 min | 3-8 min |
| Deaths/Level | 2-5 | 0-2 | 1-3 | 5-15 |
| Secrets | 2-3 | 1-2 | 5-10 | 3-5 |
| Checkpoints | Every 2 min | Puzzle start | Safe rooms | Every 30 sec |

---

**Use this skill**: When designing engaging levels, pacing gameplay, or creating environmental narratives.

## Guardrails

- Tie level recommendations to player goals, constraints, and observable flow.
- Do not present abstract level theory as verified playtest evidence.
