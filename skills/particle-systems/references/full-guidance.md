# Full Guidance

This file preserves the detailed guidance that used to live in `particle-systems/SKILL.md`. The entrypoint now uses progressive disclosure: read this file only when the task needs the detailed patterns, examples, or command catalogue.

# Particle Systems

## Particle System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PARTICLE LIFECYCLE                        │
├─────────────────────────────────────────────────────────────┤
│  EMISSION                                                    │
│  ├─ Spawn Rate (particles/second)                           │
│  ├─ Burst (instant spawn count)                             │
│  └─ Shape (point, sphere, cone, mesh)                       │
│                    ↓                                         │
│  SIMULATION                                                  │
│  ├─ Velocity (initial + over lifetime)                      │
│  ├─ Forces (gravity, wind, turbulence)                      │
│  ├─ Collision (world, depth buffer)                         │
│  └─ Noise (procedural movement)                             │
│                    ↓                                         │
│  RENDERING                                                   │
│  ├─ Billboard (camera-facing quads)                         │
│  ├─ Mesh (3D geometry per particle)                         │
│  ├─ Trail (ribbon following path)                           │
│  └─ GPU Instancing (batched draw)                           │
│                    ↓                                         │
│  DEATH                                                       │
│  └─ Lifetime expired → recycle or destroy                   │
└─────────────────────────────────────────────────────────────┘
```

## Common Effect Recipes

```
EXPLOSION EFFECT:
┌─────────────────────────────────────────────────────────────┐
│  LAYERS:                                                     │
│  1. Flash (instant, bright, 0.1s)                           │
│  2. Core fireball (expanding sphere, 0.3s)                  │
│  3. Debris (physics-enabled chunks, 1-2s)                   │
│  4. Smoke (slow rising, fade out, 2-3s)                     │
│  5. Sparks (fast, gravity-affected, 0.5-1s)                 │
│  6. Shockwave (expanding ring, 0.2s)                        │
│                                                              │
│  SETTINGS:                                                   │
│  • Emission: Burst only (no rate)                           │
│  • Start speed: 5-20 (varies by layer)                      │
│  • Gravity: -9.8 for debris, 0 for smoke                    │
│  • Color: Orange→Red→Black over lifetime                    │
│  • Size: Start large, shrink (fireball) or grow (smoke)     │
└─────────────────────────────────────────────────────────────┘

FIRE EFFECT:
┌─────────────────────────────────────────────────────────────┐
│  LAYERS:                                                     │
│  1. Core flame (upward, orange-yellow, looping)             │
│  2. Ember particles (small, floating up)                    │
│  3. Smoke (dark, rises above flame)                         │
│  4. Light source (flickering point light)                   │
│                                                              │
│  SETTINGS:                                                   │
│  • Emission: Continuous (50-100/sec)                        │
│  • Velocity: Upward (2-5 units/sec)                         │
│  • Noise: Turbulence for natural movement                   │
│  • Color: White→Yellow→Orange→Red over lifetime             │
│  • Size: Start small, grow, then shrink                     │
│  • Blend: Additive for glow effect                          │
└─────────────────────────────────────────────────────────────┘

MAGIC SPELL EFFECT:
┌─────────────────────────────────────────────────────────────┐
│  LAYERS:                                                     │
│  1. Core glow (pulsing, bright center)                      │
│  2. Orbiting particles (circle around core)                 │
│  3. Trail particles (follow movement path)                  │
│  4. Impact burst (on hit/destination)                       │
│  5. Residual sparkles (lingering after effect)              │
│                                                              │
│  SETTINGS:                                                   │
│  • Emission: Rate + burst on cast/impact                    │
│  • Velocity: Custom curves for orbiting                     │
│  • Trails: Enable for mystical streaks                      │
│  • Color: Themed to element (blue=ice, red=fire)            │
│  • Blend: Additive for ethereal glow                        │
└─────────────────────────────────────────────────────────────┘
```

## Unity Particle System Setup

```csharp
// ✅ Production-Ready: Particle Effect Controller
public class ParticleEffectController : MonoBehaviour
{
    [Header("Effect Settings")]
    [SerializeField] private ParticleSystem mainEffect;
    [SerializeField] private ParticleSystem[] subEffects;
    [SerializeField] private AudioSource audioSource;
    [SerializeField] private Light effectLight;

    [Header("Pooling")]
    [SerializeField] private bool usePooling = true;
    [SerializeField] private float autoReturnDelay = 3f;

    private ParticleSystem.MainModule _mainModule;
    private float _originalLightIntensity;

    public event Action OnEffectComplete;

    private void Awake()
    {
        _mainModule = mainEffect.main;
        if (effectLight != null)
            _originalLightIntensity = effectLight.intensity;
    }

    public void Play(Vector3 position, Quaternion rotation)
    {
        transform.SetPositionAndRotation(position, rotation);

        // Play all particle systems
        mainEffect.Play();
        foreach (var effect in subEffects)
        {
            effect.Play();
        }

        // Play audio
        if (audioSource != null)
            audioSource.Play();

        // Animate light
        if (effectLight != null)
            StartCoroutine(AnimateLight());

        // Auto-return to pool
        if (usePooling)
            StartCoroutine(ReturnToPoolAfterDelay());
    }

    public void Stop()
    {
        mainEffect.Stop(true, ParticleSystemStopBehavior.StopEmitting);
        foreach (var effect in subEffects)
        {
            effect.Stop(true, ParticleSystemStopBehavior.StopEmitting);
        }
    }

    private IEnumerator AnimateLight()
    {
        effectLight.intensity = _originalLightIntensity;
        effectLight.enabled = true;

        float duration = _mainModule.duration;
        float elapsed = 0f;

        while (elapsed < duration)
        {
            elapsed += Time.deltaTime;
            float t = elapsed / duration;
            effectLight.intensity = Mathf.Lerp(_originalLightIntensity, 0f, t);
            yield return null;
        }

        effectLight.enabled = false;
    }

    private IEnumerator ReturnToPoolAfterDelay()
    {
        yield return new WaitForSeconds(autoReturnDelay);

        OnEffectComplete?.Invoke();

        // Reset for reuse
        Stop();
        if (effectLight != null)
        {
            effectLight.enabled = false;
            effectLight.intensity = _originalLightIntensity;
        }
    }

    public void SetColor(Color color)
    {
        var startColor = _mainModule.startColor;
        startColor.color = color;
        _mainModule.startColor = startColor;

        if (effectLight != null)
            effectLight.color = color;
    }

    public void SetScale(float scale)
    {
        transform.localScale = Vector3.one * scale;
    }
}
```

## GPU Particles (Compute Shader)

```hlsl
// ✅ Production-Ready: GPU Particle Compute Shader
#pragma kernel UpdateParticles

struct Particle
{
    float3 position;
    float3 velocity;
    float4 color;
    float size;
    float lifetime;
    float maxLifetime;
};

RWStructuredBuffer<Particle> particles;
float deltaTime;
float3 gravity;
float3 windDirection;
float windStrength;
float turbulenceStrength;
float time;

float noise3D(float3 p)
{
    // Simple 3D noise for turbulence
    return frac(sin(dot(p, float3(12.9898, 78.233, 45.164))) * 43758.5453);
}

[numthreads(256, 1, 1)]
void UpdateParticles(uint3 id : SV_DispatchThreadID)
{
    Particle p = particles[id.x];

    if (p.lifetime <= 0)
        return;

    // Apply forces
    float3 acceleration = gravity;

    // Wind
    acceleration += windDirection * windStrength;

    // Turbulence
    float3 turbulence = float3(
        noise3D(p.position + time) - 0.5,
        noise3D(p.position + time + 100) - 0.5,
        noise3D(p.position + time + 200) - 0.5
    ) * turbulenceStrength;
    acceleration += turbulence;

    // Update velocity and position
    p.velocity += acceleration * deltaTime;
    p.position += p.velocity * deltaTime;

    // Update lifetime
    p.lifetime -= deltaTime;

    // Fade out
    float lifetimeRatio = p.lifetime / p.maxLifetime;
    p.color.a = lifetimeRatio;
    p.size *= (0.5 + 0.5 * lifetimeRatio);

    particles[id.x] = p;
}
```

## Performance Optimization

```
PARTICLE BUDGET GUIDELINES:
┌─────────────────────────────────────────────────────────────┐
│  PLATFORM      │ MAX PARTICLES │ MAX DRAW CALLS │ NOTES    │
├────────────────┼───────────────┼────────────────┼──────────┤
│  Mobile Low    │ 500           │ 5              │ Simple   │
│  Mobile High   │ 2,000         │ 10             │ Moderate │
│  Console       │ 50,000        │ 50             │ Complex  │
│  PC Low        │ 10,000        │ 20             │ Moderate │
│  PC High       │ 1,000,000+    │ 100+           │ GPU sim  │
│  VR            │ 5,000         │ 15             │ 90 FPS!  │
└────────────────┴───────────────┴────────────────┴──────────┘

OPTIMIZATION TECHNIQUES:
┌─────────────────────────────────────────────────────────────┐
│  REDUCE COUNT:                                               │
│  • LOD system (fewer particles at distance)                 │
│  • Cull off-screen emitters                                 │
│  • Limit max particles per system                           │
│                                                              │
│  REDUCE OVERDRAW:                                            │
│  • Use smaller particle sizes                               │
│  • Reduce transparency layers                               │
│  • Use cutout instead of transparent                        │
│                                                              │
│  BATCH DRAWS:                                                │
│  • Share materials between systems                          │
│  • Use texture atlases                                      │
│  • Enable GPU instancing                                    │
│                                                              │
│  GPU SIMULATION:                                             │
│  • Use compute shaders for >10K particles                   │
│  • Move physics to GPU                                      │
│  • VFX Graph (Unity) / Niagara (Unreal)                    │
└─────────────────────────────────────────────────────────────┘
```

## Effect Pooling System

```csharp
// ✅ Production-Ready: VFX Pool Manager
public class VFXPoolManager : MonoBehaviour
{
    public static VFXPoolManager Instance { get; private set; }

    [System.Serializable]
    public class EffectPool
    {
        public string effectName;
        public ParticleEffectController prefab;
        public int initialSize = 5;
        public int maxSize = 20;
    }

    [SerializeField] private EffectPool[] effectPools;

    private Dictionary<string, Queue<ParticleEffectController>> _pools;
    private Dictionary<string, EffectPool> _poolConfigs;

    private void Awake()
    {
        Instance = this;
        InitializePools();
    }

    private void InitializePools()
    {
        _pools = new Dictionary<string, Queue<ParticleEffectController>>();
        _poolConfigs = new Dictionary<string, EffectPool>();

        foreach (var pool in effectPools)
        {
            _pools[pool.effectName] = new Queue<ParticleEffectController>();
            _poolConfigs[pool.effectName] = pool;

            // Pre-warm pool
            for (int i = 0; i < pool.initialSize; i++)
            {
                var effect = CreateEffect(pool);
                _pools[pool.effectName].Enqueue(effect);
            }
        }
    }

    private ParticleEffectController CreateEffect(EffectPool pool)
    {
        var effect = Instantiate(pool.prefab, transform);
        effect.gameObject.SetActive(false);
        effect.OnEffectComplete += () => ReturnToPool(pool.effectName, effect);
        return effect;
    }

    public ParticleEffectController SpawnEffect(
        string effectName,
        Vector3 position,
        Quaternion rotation)
    {
        if (!_pools.ContainsKey(effectName))
        {
            Debug.LogError($"Effect pool '{effectName}' not found!");
            return null;
        }

        var pool = _pools[effectName];
        ParticleEffectController effect;

        if (pool.Count > 0)
        {
            effect = pool.Dequeue();
        }
        else if (_poolConfigs[effectName].maxSize > pool.Count)
        {
            effect = CreateEffect(_poolConfigs[effectName]);
        }
        else
        {
            Debug.LogWarning($"Pool '{effectName}' exhausted!");
            return null;
        }

        effect.gameObject.SetActive(true);
        effect.Play(position, rotation);
        return effect;
    }

    private void ReturnToPool(string effectName, ParticleEffectController effect)
    {
        effect.gameObject.SetActive(false);
        _pools[effectName].Enqueue(effect);
    }
}

// Usage
public class WeaponController : MonoBehaviour
{
    public void OnHit(Vector3 hitPoint, Vector3 hitNormal)
    {
        VFXPoolManager.Instance.SpawnEffect(
            "ImpactSparks",
            hitPoint,
            Quaternion.LookRotation(hitNormal));
    }
}
```

## 🔧 Troubleshooting

```
┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Particles popping in/out                           │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Add fade-in at birth (size/alpha over lifetime)           │
│ → Increase soft particle distance                           │
│ → Check culling settings                                    │
│ → Extend lifetime with fade-out                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Low frame rate with particles                      │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Reduce max particle count                                 │
│ → Use LOD (fewer particles at distance)                     │
│ → Switch to GPU simulation                                  │
│ → Reduce overdraw (smaller/fewer particles)                 │
│ → Use simpler shaders                                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Particles clipping through geometry                │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Enable collision module                                   │
│ → Use depth fade/soft particles                             │
│ → Adjust near clip plane                                    │
│ → Position emitter away from surfaces                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Effect looks different in build                    │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Check quality settings (particle raycast budget)          │
│ → Verify shader compatibility                               │
│ → Test on target hardware                                   │
│ → Check for editor-only components                          │
└─────────────────────────────────────────────────────────────┘
```

## Engine-Specific Tools

| Engine | System | GPU Support | Visual Editor |
|--------|--------|-------------|---------------|
| Unity | Shuriken | VFX Graph | Yes (VFX Graph) |
| Unity | VFX Graph | Native | Yes |
| Unreal | Cascade | Limited | Yes |
| Unreal | Niagara | Native | Yes |
| Godot | GPUParticles | Yes | Inspector |

---

**Use this skill**: When creating visual effects, polishing gameplay, or optimizing particle performance.
