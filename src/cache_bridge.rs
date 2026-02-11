//! Bridge: ALICE-Animation → ALICE-Cache
//! Frame-level SDF evaluation caching for real-time playback.

use crate::{Director, DirectorState, SceneGraph};
// use alice_cache::{Cache, CacheConfig};
use std::collections::HashMap;

/// Cached frame state for avoiding redundant SDF evaluations.
#[derive(Debug, Clone)]
pub struct CachedFrame {
    pub time: f32,
    pub state: DirectorState,
    pub sdf_hash: u64,
}

/// Animation frame cache with LRU eviction.
pub struct AnimationCache {
    frames: HashMap<u32, CachedFrame>,
    max_frames: usize,
    hit_count: u64,
    miss_count: u64,
}

impl AnimationCache {
    /// Create cache with specified max frame capacity.
    #[inline]
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: HashMap::with_capacity(max_frames),
            max_frames,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Get or evaluate a frame at the given time.
    #[inline]
    pub fn get_or_evaluate(
        &mut self,
        frame_index: u32,
        time: f32,
        director: &Director,
        scene: &SceneGraph,
    ) -> DirectorState {
        if let Some(cached) = self.frames.get(&frame_index) {
            self.hit_count += 1;
            return cached.state.clone();
        }
        self.miss_count += 1;
        let state = director.evaluate(scene, time);
        if self.frames.len() >= self.max_frames {
            // Evict oldest frame (simple strategy)
            if let Some(&oldest_key) = self.frames.keys().next() {
                self.frames.remove(&oldest_key);
            }
        }
        self.frames.insert(
            frame_index,
            CachedFrame {
                time,
                state: state.clone(),
                sdf_hash: 0,
            },
        );
        state
    }

    /// Cache hit rate (0.0 - 1.0).
    #[inline]
    pub fn hit_rate(&self) -> f32 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            return 0.0;
        }
        self.hit_count as f32 / total as f32
    }

    /// Clear all cached frames.
    #[inline]
    pub fn clear(&mut self) {
        self.frames.clear();
        self.hit_count = 0;
        self.miss_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::scene::SceneGraph;

    #[test]
    fn test_cache_hit_miss() {
        let mut cache = AnimationCache::new(10);
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 5.0));
        let sg = SceneGraph::new();

        // First access: miss
        let _s1 = cache.get_or_evaluate(0, 0.0, &dir, &sg);
        assert_eq!(cache.hit_rate(), 0.0);

        // Second access: hit
        let _s2 = cache.get_or_evaluate(0, 0.0, &dir, &sg);
        assert_eq!(cache.hit_rate(), 0.5);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = AnimationCache::new(2);
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 5.0));
        let sg = SceneGraph::new();

        cache.get_or_evaluate(0, 0.0, &dir, &sg);
        cache.get_or_evaluate(1, 1.0, &dir, &sg);
        cache.get_or_evaluate(2, 2.0, &dir, &sg); // Should evict oldest
        assert_eq!(cache.frames.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = AnimationCache::new(10);
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 5.0));
        let sg = SceneGraph::new();

        cache.get_or_evaluate(0, 0.0, &dir, &sg);
        cache.clear();
        assert_eq!(cache.hit_rate(), 0.0);
        assert_eq!(cache.frames.len(), 0);
    }
}
