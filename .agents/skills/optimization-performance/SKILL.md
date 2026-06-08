---
name: optimization-performance
version: "2.0.0"
description: Use when optimizing game performance, profiling frame rate, CPU/GPU bottlenecks, loading time, scalability, or multi-platform runtime behavior.
sasmp_version: "1.3.0"
bonded_agent: 02-game-programmer
bond_type: SECONDARY_BOND

parameters:
  - name: bottleneck
    type: string
    required: false
    validation:
      enum: [cpu, gpu, memory, loading, network]
  - name: platform
    type: string
    required: false
    validation:
      enum: [pc, console, mobile, vr, web]

retry_policy:
  enabled: true
  max_attempts: 3
  backoff: exponential

observability:
  log_events: [start, complete, error]
  metrics: [frame_time_ms, draw_calls, memory_mb, cpu_usage]
---

# Optimization & Performance

## When to Use

Use this skill when:

- frame rate, profiling, bottlenecks, loading, scalability, or platform performance is in scope
- a task needs performance optimization strategy or verification guidance

Do not use this skill when:

- the task does not match the trigger conditions above


## Performance Targets

```
FRAME BUDGETS:
┌─────────────────────────────────────────────────────────────┐
│  TARGET FPS │ FRAME TIME │ PLATFORM                        │
├─────────────┼────────────┼─────────────────────────────────┤
│  30 FPS     │ 33.3 ms    │ Console (heavy games)           │
│  60 FPS     │ 16.6 ms    │ PC, Console, Mobile             │
│  90 FPS     │ 11.1 ms    │ VR (minimum)                    │
│  120 FPS    │ 8.3 ms     │ Competitive games               │
│  144+ FPS   │ 6.9 ms     │ High-end PC                     │
└─────────────┴────────────┴─────────────────────────────────┘

FRAME TIME BREAKDOWN (16.6ms target):
┌─────────────────────────────────────────────────────────────┐
│  CPU: 8ms                                                    │
│  ├─ Game Logic:    3ms                                      │
│  ├─ Physics:       2ms                                      │
│  ├─ Animation:     1.5ms                                    │
│  └─ Audio/Other:   1.5ms                                    │
│                                                              │
│  GPU: 8ms                                                    │
│  ├─ Geometry:      2ms                                      │
│  ├─ Lighting:      3ms                                      │
│  ├─ Post-process:  2ms                                      │
│  └─ UI:            1ms                                      │
└─────────────────────────────────────────────────────────────┘
```

## Profiling Process

```
OPTIMIZATION WORKFLOW:
┌─────────────────────────────────────────────────────────────┐
│  1. MEASURE: Profile before optimizing                      │
│     → Never guess, always profile                          │
│                                                              │
│  2. IDENTIFY: Find the bottleneck                           │
│     → CPU-bound: Frame time > GPU time                     │
│     → GPU-bound: GPU time > CPU time                       │
│                                                              │
│  3. ANALYZE: Drill into specific systems                    │
│     → Which function? Which shader? Which asset?           │
│                                                              │
│  4. OPTIMIZE: Fix the actual bottleneck                     │
│     → One change at a time                                 │
│     → Measure before/after                                  │
│                                                              │
│  5. VERIFY: Confirm improvement                             │
│     → Check all platforms                                   │
│     → Test edge cases                                       │
│                                                              │
│  6. REPEAT: Move to next bottleneck                         │
└─────────────────────────────────────────────────────────────┘
```

## CPU Optimization

```
CPU OPTIMIZATION TECHNIQUES:
┌─────────────────────────────────────────────────────────────┐
│  ALGORITHMIC:                                                │
│  • Use appropriate data structures (spatial hashing)       │
│  • Early-out conditions                                     │
│  • Reduce O(n²) to O(n log n)                              │
├─────────────────────────────────────────────────────────────┤
│  CACHE:                                                      │
│  • Data-oriented design (SoA vs AoS)                       │
│  • Minimize cache misses                                    │
│  • Process data linearly                                    │
├─────────────────────────────────────────────────────────────┤
│  ALLOCATION:                                                 │
│  • Object pooling                                           │
│  • Avoid GC in hot paths                                    │
│  • Pre-allocate collections                                 │
├─────────────────────────────────────────────────────────────┤
│  THREADING:                                                  │
│  • Offload work to job system                              │
│  • Async loading                                            │
│  • Parallel processing                                      │
└─────────────────────────────────────────────────────────────┘
```

## GPU Optimization

```
GPU OPTIMIZATION TECHNIQUES:
┌─────────────────────────────────────────────────────────────┐
│  DRAW CALLS:                                                 │
│  • Static/dynamic batching                                  │
│  • GPU instancing                                           │
│  • Merge meshes                                             │
│  Target: <2000 on PC, <200 on mobile                        │
├─────────────────────────────────────────────────────────────┤
│  OVERDRAW:                                                   │
│  • Front-to-back rendering                                  │
│  • Occlusion culling                                        │
│  • Reduce transparency                                      │
├─────────────────────────────────────────────────────────────┤
│  SHADERS:                                                    │
│  • Reduce instruction count                                 │
│  • Use lower precision (half)                              │
│  • Minimize texture samples                                 │
├─────────────────────────────────────────────────────────────┤
│  GEOMETRY:                                                   │
│  • LOD systems                                              │
│  • Mesh simplification                                      │
│  • Frustum culling                                          │
└─────────────────────────────────────────────────────────────┘
```

## Platform-Specific Guidelines

```
MOBILE OPTIMIZATION:
┌─────────────────────────────────────────────────────────────┐
│  CRITICAL CONSTRAINTS:                                       │
│  • Thermal throttling                                       │
│  • Battery drain                                            │
│  • Memory limits (500MB-2GB)                               │
│                                                              │
│  TARGETS:                                                    │
│  • Draw calls: 100-200                                      │
│  • Triangles: 100K-500K/frame                              │
│  • Texture memory: 200-500MB                                │
│  • 30-60 FPS stable                                         │
└─────────────────────────────────────────────────────────────┘

VR OPTIMIZATION:
┌─────────────────────────────────────────────────────────────┐
│  CRITICAL: Maintain 90 FPS constantly                       │
│  • Single-pass stereo rendering                            │
│  • Fixed foveated rendering                                │
│  • Aggressive LOD                                           │
│  • Minimal post-processing                                  │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Troubleshooting

```
┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Frame rate drops during gameplay                   │
├─────────────────────────────────────────────────────────────┤
│ DEBUG:                                                       │
│ → Profile to find CPU vs GPU bound                          │
│ → Check for GC spikes                                       │
│ → Look for expensive operations in Update()               │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Move logic to FixedUpdate or coroutines                   │
│ → Implement object pooling                                  │
│ → Add LOD for distant objects                               │
│ → Reduce draw calls with batching                           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Long loading times                                 │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Async loading with progress bar                           │
│ → Stream assets in background                               │
│ → Compress assets more aggressively                         │
│ → Pre-warm caches                                           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Inconsistent frame pacing                          │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Enable VSync                                              │
│ → Use fixed timestep for physics                            │
│ → Spread heavy work across frames                           │
│ → Check for background processes                            │
└─────────────────────────────────────────────────────────────┘
```

## Profiling Tools

| Engine | CPU Profiler | GPU Profiler | Memory |
|--------|--------------|--------------|--------|
| Unity | Profiler | Frame Debugger | Memory Profiler |
| Unreal | Insights | RenderDoc | Memreport |
| Godot | Profiler | GPU Debugger | Built-in |
| Any | Platform tools | RenderDoc/PIX | Valgrind/Instruments |

---

**Use this skill**: When optimizing games, profiling performance, or supporting multiple platforms.

## Guardrails

- Prefer profiler or benchmark evidence over intuition.
- Do not optimize one metric while silently regressing correctness or player experience.
