# ALICE-Animation

**Anime-Focused SDF Direction Engine**

> "20KBのASDFファイルで、200MBの映像を置き換える。"

ALICE-AnimationはALICE-SDFの上に構築されたアニメ制作特化の演出エンジンです。シーングラフ、ディレクター（カット/シーン/エピソード管理）、アニメカメラ（嘘パース含む）、NPRレンダリング設定をSDF数式ベースで統合し、数十KBのバイナリファイルで映像配信を実現します。

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                   EpisodePackage                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐     │
│  │ SceneGraph │  │  Director  │  │AnimeShading│     │
│  │  Actors    │  │   Cuts     │  │  Cel/NPR   │     │
│  │  Timeline  │  │  Scenes    │  │  Outline   │     │
│  │  Transform │  │  Camera    │  │  AO/Rim    │     │
│  └─────┬──────┘  └─────┬──────┘  └────────────┘     │
│        │               │                              │
│        ▼               ▼                              │
│  ┌─────────────────────────────┐                     │
│  │       ALICE-SDF (core)      │                     │
│  │  AnimatedSdf, Timeline,     │                     │
│  │  ProjectiveTransform,       │                     │
│  │  LatticeDeform, SdfSkinning │                     │
│  └─────────────────────────────┘                     │
└──────────────────────────────────────────────────────┘
         ↓ serialize (ANIM binary, CRC32)
    [Magic 4B][Ver 2B][Flags 2B][Size 4B][CRC 4B][Body]
         = 20-50 KB per episode
```

## Features

| Module | Description |
|--------|-------------|
| `scene` | SceneGraph with Actor hierarchy, parent-child transforms, AnimatedSdf evaluation |
| `director` | Cut/Scene/Episode sequencing, sorted binary-search cut lookup O(log n) |
| `camera` | Keyframed CameraTrack (position/target/FOV), CameraWork presets (Pan/Tilt/Dolly/Zoom/Orbit/Shake), FMA-optimized shake |
| `npr` | CelShading (branchless quantize), OutlineConfig (epsilon SDF contour), AnimeShading |
| `episode` | Binary serialize/deserialize with CRC32 integrity, EpisodePackage bundle |
| `lip_sync` | (feature `voice`) Japanese phoneme classification (F1/F2 formant → あいうえお), voice-to-animation sync |
| `camera::FakePerspective` | Anime-style exaggerated foreshortening via ProjectiveTransform / LatticeDeform (金田パース) |

## Optional Features

| Feature | Dependency | Description |
|---------|-----------|-------------|
| `voice` | ALICE-Voice | Lip sync from ParametricParams formants |
| `view` | ALICE-View | Camera3D bridge for real-time rendering |
| `streaming` | ALICE-Streaming-Protocol | SdfSceneDescriptor for streaming delivery |
| `physics` | ALICE-Physics | Physics-driven animation |

## Performance (カリカリ)

- `ActorTransform`: `#[repr(C, align(32))]` — SIMD-friendly, cache-aligned
- `SceneGraph`: Vec-based O(1) actor lookup by index (no HashMap)
- `Director`: sorted Vec + binary search O(log n) cut lookup
- `CameraTrack::evaluate()`: `#[inline(always)]`, FMA shake
- `CelShading::quantize()`: branchless (cmov) step counting
- `OutlineConfig::outline_alpha()`: branchless multiply-by-mask, reciprocal division exorcism
- Release profile: `opt-level=3, lto=fat, codegen-units=1, strip=true, panic=abort`

## Compression Ratio

| Content | Traditional | ALICE-Animation |
|---------|------------|----------------|
| 1 episode (24 min) | 200-500 MB (H.265) | **20-50 KB** (ASDF) |
| Resolution | Fixed 1080p/4K | **Infinite** (SDF) |
| Re-edit | Full re-render | **Keyframe edit only** |

## Quick Start

```rust
use alice_animation::*;
use alice_sdf::SdfNode;

// Build scene
let mut sg = SceneGraph::new();
let hero = sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
let villain = sg.add_actor(Actor::new("villain", SdfNode::box3d(1.0, 1.0, 1.0)));

// Build director with cuts
let mut dir = Director::new("Episode 1");
dir.add_cut(Cut::new("intro", 0.0, 3.0).with_actors(vec![hero]));
dir.add_cut(Cut::new("battle", 3.0, 8.0).with_actors(vec![hero, villain]));

// Package and serialize (~KB)
let meta = EpisodeMetadata::new("Pilot", 1, 8.0);
let episode = EpisodePackage::new(meta, sg, dir, AnimeShading::default());

let mut buf = Vec::new();
episode::serialize_episode(&episode, &mut buf).unwrap();
println!("Episode size: {} bytes", buf.len()); // ~KB, not MB
```

## License

**Proprietary** — Copyright (c) 2026 Moroya Sakamoto. All rights reserved.

This is the **authoring engine layer** (防壁) of the ALICE license strategy. Commercial licensing required for production use. Contact the copyright holder for inquiries.

## Author

Moroya Sakamoto
