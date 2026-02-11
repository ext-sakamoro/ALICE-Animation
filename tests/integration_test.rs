//! Integration test: 2-actor 2-cut episode roundtrip.
//!
//! Creates a complete episode with hero + villain, 2 cuts (intro + battle),
//! serializes to ANIM binary, deserializes, and verifies integrity.

use alice_animation::*;
use alice_animation::episode::{serialize_episode, deserialize_episode};
use alice_animation::director::Cut;
use alice_animation::scene::Actor;
use alice_sdf::SdfNode;
use glam::Vec3;

#[test]
fn test_full_episode_roundtrip() {
    // 1. Build scene graph with 2 actors
    let mut sg = SceneGraph::new();
    let hero_id = sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
    let villain_id = sg.add_actor(
        Actor::new("villain", SdfNode::box3d(1.0, 1.0, 1.0))
            .with_transform(ActorTransform {
                position: Vec3::new(5.0, 0.0, 0.0),
                ..Default::default()
            }),
    );
    assert_eq!(sg.actor_count(), 2);

    // 2. Build director with 2 cuts
    let mut dir = Director::new("Integration Episode");
    let _cut1 = dir.add_cut(
        Cut::new("intro", 0.0, 3.0)
            .with_actors(vec![hero_id]),
    );
    let _cut2 = dir.add_cut(
        Cut::new("battle", 3.0, 8.0)
            .with_actors(vec![hero_id, villain_id]),
    );
    assert_eq!(dir.cut_count(), 2);
    assert_eq!(dir.duration(), 8.0);

    // 3. Verify director evaluation
    let state_intro = dir.evaluate(&sg, 1.0);
    assert!(state_intro.active_cut.is_some());
    let state_battle = dir.evaluate(&sg, 5.0);
    assert!(state_battle.active_cut.is_some());
    let state_none = dir.evaluate(&sg, 10.0);
    assert!(state_none.active_cut.is_none());

    // 4. Evaluate scene SDF
    let sdf = sg.evaluate_scene(0.0);
    match &sdf {
        SdfNode::Union { .. } => {} // 2 actors = Union
        _ => panic!("Expected Union of 2 actors"),
    }

    // 5. NPR shading setup
    let shading = AnimeShading {
        cel_shading: CelShading {
            shadow_steps: 3,
            thresholds: vec![0.3, 0.7],
            ..Default::default()
        },
        outline: OutlineConfig {
            width: 0.03,
            epsilon: 0.005,
            ..Default::default()
        },
        ..Default::default()
    };
    // Verify branchless quantize
    assert_eq!(shading.cel_shading.quantize(0.1), 0.0 / 3.0);
    assert_eq!(shading.cel_shading.quantize(0.5), 1.0 / 3.0);
    assert_eq!(shading.cel_shading.quantize(0.9), 2.0 / 3.0);
    // Verify branchless outline
    assert!(shading.outline.is_outline(0.01));
    assert!(!shading.outline.is_outline(1.0));

    // 6. Package
    let meta = EpisodeMetadata::new("Integration Test", 1, 8.0);
    let episode = EpisodePackage::new(meta, sg, dir, shading);

    // 7. Serialize → ANIM binary
    let mut buf = Vec::new();
    let written = serialize_episode(&episode, &mut buf).unwrap();
    assert!(written > 16, "Must have header + body");
    assert!(written < 50_000, "Episode should be compact (~KB)");

    // Verify ANIM magic header
    assert_eq!(&buf[0..4], b"ANIM");

    // 8. Deserialize and verify integrity
    let mut cursor = std::io::Cursor::new(&buf);
    let restored = deserialize_episode(&mut cursor).unwrap();

    assert_eq!(restored.metadata.title, "Integration Test");
    assert_eq!(restored.metadata.episode_number, 1);
    assert_eq!(restored.metadata.duration_seconds, 8.0);
    assert_eq!(restored.scene_graph.actor_count(), 2);
    assert_eq!(restored.director.cut_count(), 2);
    assert_eq!(restored.director.duration(), 8.0);

    // 9. Verify fake perspective
    let fp = FakePerspective::new("kanada_pers", camera::DistortionType::Projective, 1.5);
    let distorted = fp.apply(SdfNode::sphere(1.0));
    match distorted {
        SdfNode::ProjectiveTransform { .. } => {}
        _ => panic!("Expected ProjectiveTransform"),
    }

    println!("Integration test passed: {} bytes", written);
}

#[test]
fn test_camera_full_pipeline() {
    let mut track = CameraTrack::default();
    track.add_keyframe(
        0.0,
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::ZERO,
        std::f32::consts::FRAC_PI_4,
    );
    track.add_keyframe(
        5.0,
        Vec3::new(10.0, 5.0, 10.0),
        Vec3::new(5.0, 0.0, 0.0),
        std::f32::consts::FRAC_PI_3,
    );

    // Evaluate at multiple time points
    for i in 0..=50 {
        let t = i as f32 * 0.1;
        let state = track.evaluate(t);
        assert!(!state.position.is_nan());
        assert!(!state.target.is_nan());
        assert!(state.fov > 0.0);

        let inv = state.inverse_view_matrix();
        assert!(!inv.x_axis.is_nan());

        let fwd = state.forward();
        assert!(!fwd.is_nan());
    }
}
