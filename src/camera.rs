use alice_sdf::animation::{Keyframe, Timeline, Track};
use alice_sdf::SdfNode;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Evaluated camera state at a single instant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CameraState {
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            fov: std::f32::consts::FRAC_PI_4,
        }
    }
}

impl CameraState {
    /// Compute the inverse view matrix for transforming SDF world coordinates.
    #[inline]
    pub fn inverse_view_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, Vec3::Y);
        view.inverse()
    }

    /// Forward direction vector.
    #[inline]
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }
}

/// Camera work presets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CameraWork {
    /// Static camera.
    Static,
    /// Horizontal pan.
    Pan { speed: f32 },
    /// Vertical tilt.
    Tilt { speed: f32 },
    /// Move camera forward/backward along view direction.
    Dolly { speed: f32 },
    /// Change FOV (zoom lens effect).
    Zoom { target_fov: f32 },
    /// Orbit around target.
    Orbit { radius: f32, speed: f32 },
    /// Camera shake effect.
    Shake { amplitude: f32, frequency: f32 },
}

/// Animated camera track with keyframed position, target, and FOV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraTrack {
    pub position_timeline: Timeline,
    pub target_timeline: Timeline,
    pub fov_track: Track,
    pub shake_amplitude: f32,
    pub shake_frequency: f32,
}

impl Default for CameraTrack {
    fn default() -> Self {
        let mut pos_tl = Timeline::new("camera_position");
        let mut px = Track::new("position.x");
        px.add_keyframe(Keyframe::new(0.0, 0.0));
        let mut py = Track::new("position.y");
        py.add_keyframe(Keyframe::new(0.0, 0.0));
        let mut pz = Track::new("position.z");
        pz.add_keyframe(Keyframe::new(0.0, 5.0));
        pos_tl.add_track(px);
        pos_tl.add_track(py);
        pos_tl.add_track(pz);

        let mut tgt_tl = Timeline::new("camera_target");
        let mut tx = Track::new("target.x");
        tx.add_keyframe(Keyframe::new(0.0, 0.0));
        let mut ty = Track::new("target.y");
        ty.add_keyframe(Keyframe::new(0.0, 0.0));
        let mut tz = Track::new("target.z");
        tz.add_keyframe(Keyframe::new(0.0, 0.0));
        tgt_tl.add_track(tx);
        tgt_tl.add_track(ty);
        tgt_tl.add_track(tz);

        let mut fov_track = Track::new("fov");
        fov_track.add_keyframe(Keyframe::new(0.0, std::f32::consts::FRAC_PI_4));

        Self {
            position_timeline: pos_tl,
            target_timeline: tgt_tl,
            fov_track,
            shake_amplitude: 0.0,
            shake_frequency: 0.0,
        }
    }
}

impl CameraTrack {
    /// Add a keyframe for camera position, target, and FOV at a given time.
    pub fn add_keyframe(&mut self, time: f32, position: Vec3, target: Vec3, fov: f32) {
        // Position tracks
        let names_pos = ["position.x", "position.y", "position.z"];
        let vals_pos = [position.x, position.y, position.z];
        for track in self.position_timeline.tracks.iter_mut() {
            for (i, name) in names_pos.iter().enumerate() {
                if track.name == *name {
                    track.add_keyframe(Keyframe::new(time, vals_pos[i]));
                }
            }
        }

        // Target tracks
        let names_tgt = ["target.x", "target.y", "target.z"];
        let vals_tgt = [target.x, target.y, target.z];
        for track in self.target_timeline.tracks.iter_mut() {
            for (i, name) in names_tgt.iter().enumerate() {
                if track.name == *name {
                    track.add_keyframe(Keyframe::new(time, vals_tgt[i]));
                }
            }
        }

        // FOV
        self.fov_track.add_keyframe(Keyframe::new(time, fov));
    }

    /// Evaluate camera state at a given time. Hot path — called every frame.
    #[inline(always)]
    pub fn evaluate(&self, time: f32) -> CameraState {
        let px = self
            .position_timeline
            .get_value("position.x", time)
            .unwrap_or(0.0);
        let py = self
            .position_timeline
            .get_value("position.y", time)
            .unwrap_or(0.0);
        let pz = self
            .position_timeline
            .get_value("position.z", time)
            .unwrap_or(5.0);

        let tx = self
            .target_timeline
            .get_value("target.x", time)
            .unwrap_or(0.0);
        let ty = self
            .target_timeline
            .get_value("target.y", time)
            .unwrap_or(0.0);
        let tz = self
            .target_timeline
            .get_value("target.z", time)
            .unwrap_or(0.0);

        let fov = self.fov_track.evaluate(time);

        let mut position = Vec3::new(px, py, pz);

        // Apply camera shake — FMA-optimized, precompute freq*TAU
        if self.shake_amplitude > 0.0 {
            let freq_tau = self.shake_frequency * std::f32::consts::TAU;
            let shake_x = (time * freq_tau).sin() * self.shake_amplitude;
            let shake_y = (time * freq_tau).mul_add(1.3, 0.0).cos()
                * self.shake_amplitude
                * 0.7;
            position.x += shake_x;
            position.y += shake_y;
        }

        CameraState {
            position,
            target: Vec3::new(tx, ty, tz),
            fov,
        }
    }

    /// Apply a camera work preset, adding keyframes automatically.
    pub fn apply_preset(&mut self, work: CameraWork, start: f32, duration: f32) {
        let end = start + duration;
        match work {
            CameraWork::Static => {}
            CameraWork::Pan { speed } => {
                let current = self.evaluate(start);
                self.add_keyframe(start, current.position, current.target, current.fov);
                let offset = Vec3::new(speed * duration, 0.0, 0.0);
                self.add_keyframe(
                    end,
                    current.position + offset,
                    current.target + offset,
                    current.fov,
                );
            }
            CameraWork::Tilt { speed } => {
                let current = self.evaluate(start);
                self.add_keyframe(start, current.position, current.target, current.fov);
                let offset = Vec3::new(0.0, speed * duration, 0.0);
                self.add_keyframe(end, current.position, current.target + offset, current.fov);
            }
            CameraWork::Dolly { speed } => {
                let current = self.evaluate(start);
                let dir = current.forward();
                self.add_keyframe(start, current.position, current.target, current.fov);
                self.add_keyframe(
                    end,
                    current.position + dir * speed * duration,
                    current.target,
                    current.fov,
                );
            }
            CameraWork::Zoom { target_fov } => {
                let current = self.evaluate(start);
                self.add_keyframe(start, current.position, current.target, current.fov);
                self.add_keyframe(end, current.position, current.target, target_fov);
            }
            CameraWork::Orbit { radius, speed } => {
                let current = self.evaluate(start);
                let steps = 8;
                for i in 0..=steps {
                    let t = start + (duration * i as f32 / steps as f32);
                    let angle = speed * (t - start);
                    let pos = current.target
                        + Vec3::new(radius * angle.cos(), current.position.y, radius * angle.sin());
                    self.add_keyframe(t, pos, current.target, current.fov);
                }
            }
            CameraWork::Shake {
                amplitude,
                frequency,
            } => {
                self.shake_amplitude = amplitude;
                self.shake_frequency = frequency;
            }
        }
    }
}

/// Distortion type for fake perspective effects.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DistortionType {
    /// ProjectiveTransform-based foreshortening.
    Projective,
    /// LatticeDeform-based free-form distortion (Kanada perspective).
    Lattice,
    /// Barrel/fisheye distortion.
    Fisheye,
}

/// Fake perspective configuration for anime-style exaggerated foreshortening.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakePerspective {
    pub name: String,
    pub distortion_type: DistortionType,
    pub strength: f32,
}

impl FakePerspective {
    pub fn new(name: impl Into<String>, distortion_type: DistortionType, strength: f32) -> Self {
        Self {
            name: name.into(),
            distortion_type,
            strength,
        }
    }

    /// Apply fake perspective distortion to an SDF node.
    pub fn apply(&self, sdf: SdfNode) -> SdfNode {
        match self.distortion_type {
            DistortionType::Projective => {
                // Build a perspective-like projection matrix with exaggerated foreshortening
                let s = self.strength;
                #[rustfmt::skip]
                let inv_matrix: [f32; 16] = [
                    1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    0.0, 0.0, 1.0, s * 0.1,
                    0.0, 0.0, 0.0, 1.0,
                ];
                sdf.projective_transform(inv_matrix, 1.0 + s.abs())
            }
            DistortionType::Lattice => {
                // Create a 2x2x2 lattice with exaggerated control points
                let s = self.strength;
                let control_points = vec![
                    // Near plane (z=0): normal
                    Vec3::new(-1.0, -1.0, -1.0),
                    Vec3::new(1.0, -1.0, -1.0),
                    Vec3::new(-1.0, 1.0, -1.0),
                    Vec3::new(1.0, 1.0, -1.0),
                    // Far plane (z=1): exaggerated scale
                    Vec3::new(-1.0 - s, -1.0 - s, 1.0),
                    Vec3::new(1.0 + s, -1.0 - s, 1.0),
                    Vec3::new(-1.0 - s, 1.0 + s, 1.0),
                    Vec3::new(1.0 + s, 1.0 + s, 1.0),
                ];
                sdf.lattice_deform(
                    control_points,
                    2,
                    2,
                    2,
                    Vec3::new(-2.0, -2.0, -2.0),
                    Vec3::new(2.0, 2.0, 2.0),
                )
            }
            DistortionType::Fisheye => {
                // Fisheye via projective with radial component
                let s = self.strength;
                #[rustfmt::skip]
                let inv_matrix: [f32; 16] = [
                    1.0 + s * 0.2, 0.0,           0.0, 0.0,
                    0.0,           1.0 + s * 0.2,  0.0, 0.0,
                    0.0,           0.0,            1.0, 0.0,
                    0.0,           0.0,            0.0, 1.0,
                ];
                sdf.projective_transform(inv_matrix, 1.0 + s.abs() * 0.3)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_default() {
        let state = CameraState::default();
        assert_eq!(state.position, Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(state.target, Vec3::ZERO);
    }

    #[test]
    fn test_camera_track_evaluate() {
        let mut track = CameraTrack::default();
        track.add_keyframe(
            0.0,
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::ZERO,
            std::f32::consts::FRAC_PI_4,
        );
        track.add_keyframe(
            5.0,
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::ZERO,
            std::f32::consts::FRAC_PI_4,
        );
        let mid = track.evaluate(2.5);
        assert!((mid.position.x - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_fake_perspective_projective() {
        let fp = FakePerspective::new("exaggerated", DistortionType::Projective, 1.0);
        let result = fp.apply(SdfNode::sphere(1.0));
        match result {
            SdfNode::ProjectiveTransform { .. } => {}
            _ => panic!("Expected ProjectiveTransform"),
        }
    }

    #[test]
    fn test_fake_perspective_lattice() {
        let fp = FakePerspective::new("kanada", DistortionType::Lattice, 0.5);
        let result = fp.apply(SdfNode::sphere(1.0));
        match result {
            SdfNode::LatticeDeform { .. } => {}
            _ => panic!("Expected LatticeDeform"),
        }
    }

    #[test]
    fn test_camera_work_preset() {
        let mut track = CameraTrack::default();
        track.apply_preset(CameraWork::Pan { speed: 2.0 }, 0.0, 5.0);
        let state = track.evaluate(5.0);
        assert!(state.position.x > 0.0);
    }
}
