---
name: audio-systems
version: "2.0.0"
description: Use when implementing or tuning game audio systems, music, spatial audio, sound effects, voice, or audio middleware integration.
sasmp_version: "1.3.0"
bonded_agent: 04-audio-sound-design
bond_type: PRIMARY_BOND

parameters:
  - name: middleware
    type: string
    required: false
    validation:
      enum: [wwise, fmod, unity_audio, unreal_audio, custom]
  - name: audio_type
    type: string
    required: false
    validation:
      enum: [sfx, music, voice, ambient, ui]

retry_policy:
  enabled: true
  max_attempts: 3
  backoff: exponential

observability:
  log_events: [start, complete, error]
  metrics: [active_voices, cpu_usage, memory_usage]
---

# Audio & Sound Systems

## When to Use

Use this skill when:

- audio implementation, middleware, music, SFX, voice, or spatial sound behavior is in scope
- a task needs game audio design or integration guidance

Do not use this skill when:

- the task does not match the trigger conditions above


## Audio Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    GAME AUDIO PIPELINE                       │
├─────────────────────────────────────────────────────────────┤
│  SOURCES: SFX | Music | Voice | Ambient                     │
│                         ↓                                    │
│  MIDDLEWARE: Wwise / FMOD / Engine Audio                    │
│                         ↓                                    │
│  PROCESSING: 3D Spatial | Reverb | EQ | Compression         │
│                         ↓                                    │
│  MIXING: Master → Submixes → Individual Tracks              │
│                         ↓                                    │
│  OUTPUT: Speakers / Headphones (Stereo/Surround/Binaural)  │
└─────────────────────────────────────────────────────────────┘
```

## Audio Programming

### Unity Audio Manager

```csharp
// ✅ Production-Ready: Audio Manager
public class AudioManager : MonoBehaviour
{
    public static AudioManager Instance { get; private set; }

    [System.Serializable]
    public class SoundBank
    {
        public string id;
        public AudioClip[] clips;
        [Range(0f, 1f)] public float volume = 1f;
        [Range(0.1f, 3f)] public float pitchVariation = 0.1f;
    }

    [SerializeField] private SoundBank[] _soundBanks;
    [SerializeField] private int _poolSize = 20;

    private Dictionary<string, SoundBank> _bankLookup;
    private Queue<AudioSource> _sourcePool;

    private void Awake()
    {
        if (Instance != null) { Destroy(gameObject); return; }
        Instance = this;
        DontDestroyOnLoad(gameObject);

        InitializePool();
        BuildLookup();
    }

    public void PlaySound(string id, Vector3 position)
    {
        if (!_bankLookup.TryGetValue(id, out var bank)) return;
        if (bank.clips.Length == 0) return;

        var source = GetPooledSource();
        source.transform.position = position;
        source.clip = bank.clips[Random.Range(0, bank.clips.Length)];
        source.volume = bank.volume;
        source.pitch = 1f + Random.Range(-bank.pitchVariation, bank.pitchVariation);
        source.Play();

        StartCoroutine(ReturnToPool(source, source.clip.length));
    }

    private AudioSource GetPooledSource()
    {
        if (_sourcePool.Count > 0) return _sourcePool.Dequeue();
        return CreateNewSource();
    }

    private void InitializePool() { /* ... */ }
    private void BuildLookup() { /* ... */ }
    private AudioSource CreateNewSource() { /* ... */ }
    private IEnumerator ReturnToPool(AudioSource s, float delay) { /* ... */ }
}
```

### FMOD Integration

```csharp
// ✅ Production-Ready: FMOD Event Player
public class FMODEventPlayer : MonoBehaviour
{
    [SerializeField] private FMODUnity.EventReference _eventRef;

    private FMOD.Studio.EventInstance _instance;
    private bool _isPlaying;

    public void Play()
    {
        if (_isPlaying) Stop();

        _instance = FMODUnity.RuntimeManager.CreateInstance(_eventRef);
        _instance.set3DAttributes(FMODUnity.RuntimeUtils.To3DAttributes(transform));
        _instance.start();
        _isPlaying = true;
    }

    public void SetParameter(string name, float value)
    {
        if (_isPlaying)
            _instance.setParameterByName(name, value);
    }

    public void Stop(bool allowFadeout = true)
    {
        if (!_isPlaying) return;

        _instance.stop(allowFadeout
            ? FMOD.Studio.STOP_MODE.ALLOWFADEOUT
            : FMOD.Studio.STOP_MODE.IMMEDIATE);
        _instance.release();
        _isPlaying = false;
    }

    private void OnDestroy() => Stop(false);
}
```

## Spatial Audio

```
3D AUDIO CONFIGURATION:
┌─────────────────────────────────────────────────────────────┐
│  ATTENUATION CURVES:                                         │
│                                                              │
│  Volume │████████████                                       │
│         │            ████                                   │
│         │                ████                               │
│         │                    ████                           │
│         └──────────────────────────→ Distance               │
│         0m     10m    30m    50m                            │
│                                                              │
│  Min Distance: 1m (full volume)                             │
│  Max Distance: 50m (inaudible)                              │
│  Rolloff: Logarithmic (realistic)                           │
└─────────────────────────────────────────────────────────────┘
```

## Music Systems

### Adaptive Music State Machine

```
MUSIC STATE TRANSITIONS:
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│   [EXPLORATION] ←──────────→ [TENSION]                      │
│        │                        │                           │
│        ↓                        ↓                           │
│   [DISCOVERY]              [COMBAT]                         │
│                                 │                           │
│                                 ↓                           │
│                            [VICTORY] / [DEFEAT]             │
│                                                              │
│  Transition Rules:                                           │
│  • Crossfade on beat boundaries                             │
│  • 2-4 bar transition windows                               │
│  • Intensity parameter controls layers                      │
└─────────────────────────────────────────────────────────────┘
```

## Mixing Guidelines

| Bus | Content | Target Level |
|-----|---------|--------------|
| Master | Final mix | -3dB peak |
| Music | BGM, Stingers | -12dB to -6dB |
| SFX | Gameplay sounds | -6dB to 0dB |
| Voice | Dialogue, VO | -6dB to -3dB |
| Ambient | Environment | -18dB to -12dB |

## 🔧 Troubleshooting

```
┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Audio popping/clicking                             │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Add fade in/out (5-10ms)                                  │
│ → Check sample rate mismatches                              │
│ → Increase audio buffer size                                │
│ → Use audio source pooling                                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Too many simultaneous sounds                       │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Implement voice limiting per category                     │
│ → Priority system (important sounds steal)                  │
│ → Distance-based culling                                    │
│ → Virtual voices (middleware feature)                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Music transitions are jarring                      │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Align transitions to musical bars                         │
│ → Use crossfades (1-4 seconds)                              │
│ → Prepare transition stingers                               │
│ → Match keys between sections                               │
└─────────────────────────────────────────────────────────────┘
```

## Optimization

| Platform | Max Voices | Compression | Streaming |
|----------|------------|-------------|-----------|
| Mobile | 16-32 | High (Vorbis) | Required |
| Console | 64-128 | Medium | Large files |
| PC | 128-256 | Low | Optional |

---

**Use this skill**: When implementing audio, designing sound, or composing music.

## Guardrails

- Verify audio behavior in the target runtime or middleware path when possible.
- Do not treat placeholder audio as final player-facing feedback.
