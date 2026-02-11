//! Bridge: ALICE-Animation → ALICE-CDN
//! Episode distribution with edge caching and content routing.

use crate::episode::{EpisodeMetadata, EpisodePackage};
// use alice_cdn::{CdnClient, ContentDescriptor, CacheHint};

/// CDN-optimized episode descriptor for edge distribution.
#[derive(Debug, Clone)]
pub struct EpisodeCdnDescriptor {
    pub content_id: String,
    pub size_bytes: usize,
    pub cache_hint: CdnCacheHint,
    pub metadata: EpisodeMetadata,
}

/// Cache hint strategy for anime episodes.
#[derive(Debug, Clone, Copy)]
pub enum CdnCacheHint {
    /// Latest episode — cache at edge, high priority.
    Hot,
    /// Back-catalog — cache on demand.
    Warm,
    /// Rarely accessed — origin only.
    Cold,
}

/// Create a CDN content descriptor from an episode.
#[inline]
pub fn episode_to_cdn_descriptor(episode: &EpisodePackage, hint: CdnCacheHint) -> EpisodeCdnDescriptor {
    let size_bytes = episode.metadata.duration_seconds as usize * 6; // ~6 bytes/sec for SDF
    let content_id = format!("anim-ep{:04}-{}", episode.metadata.episode_number, episode.metadata.title);
    EpisodeCdnDescriptor {
        content_id,
        size_bytes,
        cache_hint: hint,
        metadata: episode.metadata.clone(),
    }
}

/// Estimate bandwidth savings vs traditional video.
#[inline]
pub fn bandwidth_savings_ratio(episode_size_bytes: usize, duration_seconds: f32) -> f32 {
    // Traditional: ~2 MB/s for 1080p video
    let traditional_bytes = (duration_seconds * 2_000_000.0) as usize;
    traditional_bytes as f32 / episode_size_bytes.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::npr::AnimeShading;
    use crate::scene::{Actor, SceneGraph};
    use alice_sdf::SdfNode;

    #[test]
    fn test_episode_to_cdn_descriptor() {
        let mut sg = SceneGraph::new();
        sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 120.0));
        let meta = EpisodeMetadata::new("CDN Test", 1, 120.0);
        let episode = EpisodePackage::new(meta, sg, dir, AnimeShading::default());

        let descriptor = episode_to_cdn_descriptor(&episode, CdnCacheHint::Hot);
        assert_eq!(descriptor.metadata.episode_number, 1);
        assert!(descriptor.size_bytes > 0);
    }

    #[test]
    fn test_bandwidth_savings() {
        let size_bytes = 50_000; // 50KB
        let duration = 120.0; // 2 minutes
        let ratio = bandwidth_savings_ratio(size_bytes, duration);
        assert!(ratio > 1.0); // Should show significant savings
    }
}
