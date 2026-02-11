use serde::{Deserialize, Serialize};

/// Cel shading configuration for anime-style step lighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelShading {
    /// Number of discrete shadow steps (typically 2-4).
    pub shadow_steps: u32,
    /// Shadow color (R, G, B, A).
    pub shadow_color: [f32; 4],
    /// Highlight color (R, G, B, A).
    pub highlight_color: [f32; 4],
    /// Thresholds for each step boundary (length = shadow_steps - 1).
    pub thresholds: Vec<f32>,
}

impl Default for CelShading {
    fn default() -> Self {
        Self {
            shadow_steps: 2,
            shadow_color: [0.2, 0.15, 0.25, 1.0],
            highlight_color: [1.0, 1.0, 1.0, 1.0],
            thresholds: vec![0.5],
        }
    }
}

impl CelShading {
    /// Quantize a lighting value (0..1) into discrete steps.
    /// Branchless: `(lighting > threshold) as u32` compiles to cmov.
    #[inline(always)]
    pub fn quantize(&self, lighting: f32) -> f32 {
        if self.thresholds.is_empty() {
            return lighting;
        }
        let mut step = 0u32;
        for &threshold in &self.thresholds {
            // Branchless: bool-to-int, compiler emits cmov
            step += (lighting > threshold) as u32;
        }
        // Division exorcism: precompute reciprocal
        let rcp_steps = 1.0 / self.shadow_steps as f32;
        step as f32 * rcp_steps
    }
}

/// SDF-based outline configuration.
/// Uses epsilon-distance: `abs(sdf_distance) < epsilon` for contour detection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OutlineConfig {
    /// Outline width in world units.
    pub width: f32,
    /// Outline color (R, G, B, A).
    pub color: [f32; 4],
    /// SDF epsilon for contour detection.
    pub epsilon: f32,
    /// Fade outline with depth distance.
    pub depth_fade: f32,
}

impl Default for OutlineConfig {
    fn default() -> Self {
        Self {
            width: 0.02,
            color: [0.0, 0.0, 0.0, 1.0],
            epsilon: 0.005,
            depth_fade: 0.0,
        }
    }
}

impl OutlineConfig {
    /// Check if a given SDF distance falls within the outline region.
    #[inline(always)]
    pub fn is_outline(&self, sdf_distance: f32) -> bool {
        sdf_distance.abs() < self.epsilon + self.width
    }

    /// Compute outline alpha based on SDF distance and depth.
    /// Branchless: multiply-by-mask pattern, reciprocal division exorcism.
    #[inline(always)]
    pub fn outline_alpha(&self, sdf_distance: f32, depth: f32) -> f32 {
        let total_width = self.epsilon + self.width;
        let rcp_total_width = 1.0 / total_width;
        let abs_dist = sdf_distance.abs();

        // Branchless: in_range mask (0.0 or 1.0)
        let in_range = (abs_dist < total_width) as u32 as f32;
        // Edge factor: smooth falloff via reciprocal multiply
        let edge_factor = (1.0 - (abs_dist * rcp_total_width).min(1.0)) * in_range;
        // Depth factor: branchless via clamp
        let depth_factor = (1.0 - depth * self.depth_fade).max(0.0);

        edge_factor * depth_factor * self.color[3]
    }
}

/// Combined anime shading configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeShading {
    pub cel_shading: CelShading,
    pub outline: OutlineConfig,
    /// Ambient occlusion strength (0 = off).
    pub ao_strength: f32,
    /// Rim light intensity (0 = off).
    pub rim_light: f32,
}

impl Default for AnimeShading {
    fn default() -> Self {
        Self {
            cel_shading: CelShading::default(),
            outline: OutlineConfig::default(),
            ao_strength: 0.3,
            rim_light: 0.2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cel_shading_quantize() {
        let cel = CelShading {
            shadow_steps: 3,
            thresholds: vec![0.3, 0.7],
            ..Default::default()
        };
        assert_eq!(cel.quantize(0.1), 0.0 / 3.0);
        assert_eq!(cel.quantize(0.5), 1.0 / 3.0);
        assert_eq!(cel.quantize(0.9), 2.0 / 3.0);
    }

    #[test]
    fn test_outline_detection() {
        let outline = OutlineConfig {
            width: 0.02,
            epsilon: 0.005,
            ..Default::default()
        };
        assert!(outline.is_outline(0.01));
        assert!(!outline.is_outline(0.1));
    }

    #[test]
    fn test_outline_alpha() {
        let outline = OutlineConfig::default();
        let alpha = outline.outline_alpha(0.0, 0.0);
        assert!(alpha > 0.0);
        let alpha_far = outline.outline_alpha(1.0, 0.0);
        assert_eq!(alpha_far, 0.0);
    }

    #[test]
    fn test_anime_shading_default() {
        let shading = AnimeShading::default();
        assert_eq!(shading.cel_shading.shadow_steps, 2);
        assert!(shading.ao_strength > 0.0);
        assert!(shading.rim_light > 0.0);
    }
}
