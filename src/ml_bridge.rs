//! Bridge: ALICE-Animation → ALICE-ML
//! AI-assisted animation: in-betweening, auto camera work, style transfer.

use crate::{ActorTransform, SceneGraph};
// use alice_ml::{Model, Tensor};
use glam::Vec3;

/// AI in-betweening: generate intermediate frames between two keyframes.
#[derive(Debug, Clone)]
pub struct InbetweenRequest {
    pub start_transform: ActorTransform,
    pub end_transform: ActorTransform,
    pub num_frames: usize,
    pub easing: EasingHint,
}

/// Easing hint for AI interpolation.
#[derive(Debug, Clone, Copy)]
pub enum EasingHint {
    /// Linear interpolation (baseline).
    Linear,
    /// Anime-style snappy motion (fast start, slow end).
    AnimeSnap,
    /// Anticipation → overshoot → settle.
    Overshoot,
    /// Follow-through with secondary motion.
    FollowThrough,
}

/// Result of AI in-betweening.
#[derive(Debug, Clone)]
pub struct InbetweenResult {
    pub frames: Vec<ActorTransform>,
    pub confidence: f32,
}

/// Generate in-between frames using linear interpolation (ML-ready interface).
#[inline]
pub fn generate_inbetweens(request: &InbetweenRequest) -> InbetweenResult {
    let mut frames = Vec::with_capacity(request.num_frames);
    let rcp_frames = 1.0 / (request.num_frames + 1) as f32;
    for i in 1..=request.num_frames {
        let t = i as f32 * rcp_frames;
        let t = apply_easing(t, request.easing);
        let position = request
            .start_transform
            .position
            .lerp(request.end_transform.position, t);
        let rotation = request
            .start_transform
            .rotation
            .slerp(request.end_transform.rotation, t);
        let scale = request
            .start_transform
            .scale
            .lerp(request.end_transform.scale, t);
        frames.push(ActorTransform {
            position,
            rotation,
            scale,
        });
    }
    InbetweenResult {
        frames,
        confidence: 1.0,
    }
}

/// Apply easing function to t (0.0 - 1.0).
#[inline(always)]
fn apply_easing(t: f32, easing: EasingHint) -> f32 {
    match easing {
        EasingHint::Linear => t,
        EasingHint::AnimeSnap => {
            // Cubic ease-out: 1 - (1-t)^3
            let inv = 1.0 - t;
            1.0 - inv * inv * inv
        }
        EasingHint::Overshoot => {
            // Back ease-out with overshoot
            let s = 1.70158_f32;
            let t1 = t - 1.0;
            t1.mul_add(t1.mul_add(t1 * (s + 1.0) + s, 0.0), 1.0) // FMA chain
        }
        EasingHint::FollowThrough => {
            // Elastic ease-out (simplified)
            if t <= 0.0 || t >= 1.0 {
                return t;
            }
            let p = 0.3_f32;
            let rcp_p = 1.0 / p;
            (2.0_f32.powf(-10.0 * t)
                * ((t - p * 0.25) * std::f32::consts::TAU * rcp_p).sin())
                + 1.0
        }
    }
}

/// Auto camera suggestion based on scene composition.
#[derive(Debug, Clone)]
pub struct CameraSuggestion {
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
    pub confidence: f32,
    pub rationale: &'static str,
}

/// Suggest camera placement based on actor positions.
#[inline]
pub fn suggest_camera(scene: &SceneGraph) -> CameraSuggestion {
    // Simple heuristic: center on actors, pull back to frame all
    let (sum, count) = scene.actor_positions_sum();
    if count == 0 {
        return CameraSuggestion {
            position: Vec3::new(0.0, 0.0, 10.0),
            target: Vec3::ZERO,
            fov: std::f32::consts::FRAC_PI_4,
            confidence: 0.5,
            rationale: "Default: no actors in scene",
        };
    }
    let rcp_count = 1.0 / count as f32;
    let center = sum * rcp_count;
    let distance = 5.0 + count as f32 * 2.0; // Pull back for more actors
    CameraSuggestion {
        position: Vec3::new(center.x, center.y + 2.0, center.z + distance),
        target: center,
        fov: std::f32::consts::FRAC_PI_4,
        confidence: 0.8,
        rationale: "Auto-framing: centered on actors",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Actor, SceneGraph};
    use alice_sdf::SdfNode;
    use glam::Quat;

    #[test]
    fn test_generate_inbetweens_linear() {
        let start = ActorTransform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let end = ActorTransform {
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let request = InbetweenRequest {
            start_transform: start,
            end_transform: end,
            num_frames: 3,
            easing: EasingHint::Linear,
        };

        let result = generate_inbetweens(&request);
        assert_eq!(result.frames.len(), 3);
        assert!(result.confidence > 0.0);

        // Check interpolation
        let mid = &result.frames[1];
        assert!((mid.position.x - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_easing_functions() {
        assert_eq!(apply_easing(0.0, EasingHint::Linear), 0.0);
        assert_eq!(apply_easing(1.0, EasingHint::Linear), 1.0);

        let snap = apply_easing(0.5, EasingHint::AnimeSnap);
        assert!(snap > 0.5); // Should be ahead of linear

        let overshoot = apply_easing(1.0, EasingHint::Overshoot);
        assert!(overshoot > 0.95); // Should settle near 1.0
    }

    #[test]
    fn test_suggest_camera() {
        let mut sg = SceneGraph::new();
        sg.add_actor(
            Actor::new("a", SdfNode::sphere(1.0)).with_transform(ActorTransform {
                position: Vec3::new(5.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            }),
        );
        sg.add_actor(
            Actor::new("b", SdfNode::sphere(1.0)).with_transform(ActorTransform {
                position: Vec3::new(-5.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            }),
        );

        let suggestion = suggest_camera(&sg);
        assert!(suggestion.confidence > 0.5);
        // Camera should be centered on actors
        assert!(suggestion.target.x.abs() < 1.0);
    }

    #[test]
    fn test_suggest_camera_empty_scene() {
        let sg = SceneGraph::new();
        let suggestion = suggest_camera(&sg);
        assert_eq!(suggestion.rationale, "Default: no actors in scene");
    }
}
