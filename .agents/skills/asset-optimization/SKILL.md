---
name: asset-optimization
version: "2.0.0"
description: Use when optimizing game assets, compression, streaming, resource loading, asset bundles, memory use, or delivery pipelines.
sasmp_version: "1.3.0"
bonded_agent: 06-tools-pipeline
bond_type: PRIMARY_BOND

parameters:
  - name: asset_type
    type: string
    required: false
    validation:
      enum: [textures, meshes, audio, animations, all]
  - name: platform
    type: string
    required: false
    validation:
      enum: [pc, console, mobile, web]

retry_policy:
  enabled: true
  max_attempts: 3
  backoff: exponential

observability:
  log_events: [start, complete, error, import]
  metrics: [asset_size_mb, compression_ratio, import_time]
---

# Asset Optimization

## When to Use

Use this skill when:

- asset size, compression, streaming, loading, bundles, memory, or delivery pipeline decisions are in scope
- a task needs reusable asset-pipeline optimization guidance

Do not use this skill when:

- the task does not match the trigger conditions above


## Asset Pipeline Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    ASSET PIPELINE FLOW                       │
├─────────────────────────────────────────────────────────────┤
│  SOURCE ASSETS (Large, Editable):                            │
│  .psd, .fbx, .blend, .wav, .tga                             │
│                              ↓                               │
│  IMPORT SETTINGS:                                            │
│  Compression, Format, Quality, Platform overrides           │
│                              ↓                               │
│  PROCESSING:                                                 │
│  Compression, Mipmaps, LOD generation, Format conversion    │
│                              ↓                               │
│  RUNTIME ASSETS (Optimized):                                 │
│  .dds, .ktx, .ogg, engine-specific formats                  │
│                              ↓                               │
│  PACKAGING:                                                  │
│  Asset bundles, streaming chunks, platform builds           │
└─────────────────────────────────────────────────────────────┘
```

## Texture Optimization

```
TEXTURE COMPRESSION FORMATS:
┌─────────────────────────────────────────────────────────────┐
│  PLATFORM   │ FORMAT      │ QUALITY  │ SIZE/PIXEL         │
├─────────────┼─────────────┼──────────┼────────────────────┤
│  PC/Console │ BC7         │ Best     │ 1 byte             │
│  PC/Console │ BC1 (DXT1)  │ Good     │ 0.5 byte           │
│  iOS        │ ASTC 6x6    │ Great    │ 0.89 byte          │
│  Android    │ ETC2        │ Good     │ 0.5-1 byte         │
│  Mobile     │ ASTC 8x8    │ Good     │ 0.5 byte           │
│  Uncompressed│ RGBA32     │ Perfect  │ 4 bytes            │
└─────────────┴─────────────┴──────────┴────────────────────┘

TEXTURE SIZE GUIDELINES:
┌─────────────────────────────────────────────────────────────┐
│  Character (main):    2048x2048                             │
│  Character (NPC):     1024x1024                             │
│  Props (large):       1024x1024                             │
│  Props (small):       512x512 or 256x256                    │
│  UI elements:         Power of 2, vary by size             │
│  Environment:         2048x2048 (tiling)                   │
│  Mobile maximum:      1024x1024 (prefer 512)               │
└─────────────────────────────────────────────────────────────┘
```

## Mesh Optimization

```
POLYGON BUDGET GUIDELINES:
┌─────────────────────────────────────────────────────────────┐
│  PLATFORM   │ HERO CHAR │ NPC      │ PROP     │ SCENE     │
├─────────────┼───────────┼──────────┼──────────┼───────────┤
│  PC High    │ 100K      │ 30K      │ 10K      │ 10M       │
│  PC Med     │ 50K       │ 15K      │ 5K       │ 5M        │
│  Console    │ 80K       │ 25K      │ 8K       │ 8M        │
│  Mobile     │ 10K       │ 3K       │ 500      │ 500K      │
│  VR         │ 30K       │ 10K      │ 2K       │ 2M        │
└─────────────┴───────────┴──────────┴──────────┴───────────┘

LOD CONFIGURATION:
┌─────────────────────────────────────────────────────────────┐
│  LOD0: 100% triangles  │  0-10m   │ Full detail           │
│  LOD1: 50% triangles   │  10-30m  │ Reduced               │
│  LOD2: 25% triangles   │  30-60m  │ Low detail            │
│  LOD3: 10% triangles   │  60m+    │ Billboard/Impostor    │
└─────────────────────────────────────────────────────────────┘
```

## Audio Optimization

```
AUDIO COMPRESSION:
┌─────────────────────────────────────────────────────────────┐
│  TYPE        │ FORMAT  │ QUALITY   │ STREAMING            │
├─────────────┼─────────┼───────────┼──────────────────────┤
│  Music       │ Vorbis  │ 128-192   │ Always stream        │
│  SFX (short) │ ADPCM   │ High      │ Decompress on load   │
│  SFX (long)  │ Vorbis  │ 128       │ Stream if > 1MB      │
│  Voice       │ Vorbis  │ 96-128    │ Stream               │
│  Ambient     │ Vorbis  │ 96        │ Stream               │
└─────────────┴─────────┴───────────┴──────────────────────┘

AUDIO MEMORY BUDGET:
• Mobile: 20-50 MB
• Console: 100-200 MB
• PC: 200-500 MB
```

## Batch Processing Script

```python
# ✅ Production-Ready: Asset Batch Processor
import subprocess
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor

def process_textures(input_dir: Path, output_dir: Path, platform: str):
    """Batch process textures for target platform."""

    settings = {
        'pc': {'format': 'bc7', 'max_size': 4096},
        'mobile': {'format': 'astc', 'max_size': 1024},
        'console': {'format': 'bc7', 'max_size': 2048},
    }

    config = settings.get(platform, settings['pc'])

    textures = list(input_dir.glob('**/*.png')) + list(input_dir.glob('**/*.tga'))

    def process_single(texture: Path):
        output_path = output_dir / texture.relative_to(input_dir)
        output_path = output_path.with_suffix('.dds')
        output_path.parent.mkdir(parents=True, exist_ok=True)

        subprocess.run([
            'texconv',
            '-f', config['format'],
            '-w', str(config['max_size']),
            '-h', str(config['max_size']),
            '-m', '0',  # Generate all mipmaps
            '-o', str(output_path.parent),
            str(texture)
        ])

    with ThreadPoolExecutor(max_workers=8) as executor:
        executor.map(process_single, textures)
```

## 🔧 Troubleshooting

```
┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Build size too large                               │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Audit unused assets                                       │
│ → Increase texture compression                              │
│ → Enable mesh compression                                   │
│ → Split into downloadable content                           │
│ → Use texture atlases                                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Long import times                                  │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Use asset database caching                                │
│ → Import in batches                                         │
│ → Use faster SSD storage                                    │
│ → Pre-process assets in CI/CD                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ PROBLEM: Assets look blurry                                 │
├─────────────────────────────────────────────────────────────┤
│ SOLUTIONS:                                                   │
│ → Reduce compression for important assets                   │
│ → Increase texture resolution                               │
│ → Check mipmap settings                                     │
│ → Use appropriate filtering mode                            │
└─────────────────────────────────────────────────────────────┘
```

## Memory Budgets

| Platform | Textures | Meshes | Audio | Total |
|----------|----------|--------|-------|-------|
| Mobile Low | 100 MB | 50 MB | 30 MB | 200 MB |
| Mobile High | 500 MB | 200 MB | 100 MB | 1 GB |
| Console | 2 GB | 1 GB | 200 MB | 4 GB |
| PC | 4 GB | 2 GB | 500 MB | 8 GB |

---

**Use this skill**: When optimizing assets, managing memory, or streamlining pipelines.

## Guardrails

- Measure before and after when optimization claims matter.
- Do not trade away visual quality or loading correctness without task acceptance.
