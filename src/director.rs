use serde::{Deserialize, Serialize};

use crate::camera::{CameraState, CameraTrack};
use crate::scene::{ActorId, SceneGraph};

/// Unique cut identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CutId(pub u32);

/// A single cut (camera angle + active actors within a time range).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cut {
    pub name: String,
    pub start_time: f32,
    pub end_time: f32,
    pub camera: CameraTrack,
    pub active_actors: Vec<ActorId>,
    /// Precomputed reciprocal of duration (division exorcism).
    rcp_duration: f32,
}

impl Cut {
    pub fn new(name: impl Into<String>, start: f32, end: f32) -> Self {
        let dur = end - start;
        Self {
            name: name.into(),
            start_time: start,
            end_time: end,
            camera: CameraTrack::default(),
            active_actors: Vec::new(),
            rcp_duration: if dur > 0.0 { 1.0 / dur } else { 0.0 },
        }
    }

    /// Duration of this cut in seconds.
    #[inline]
    pub fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }

    /// Reciprocal of duration (precomputed, division exorcism).
    #[inline]
    pub fn rcp_duration(&self) -> f32 {
        self.rcp_duration
    }

    /// Check if a given time falls within this cut.
    #[inline]
    pub fn contains_time(&self, time: f32) -> bool {
        time >= self.start_time && time < self.end_time
    }

    /// Set camera track.
    pub fn with_camera(mut self, camera: CameraTrack) -> Self {
        self.camera = camera;
        self
    }

    /// Set active actors.
    pub fn with_actors(mut self, actors: Vec<ActorId>) -> Self {
        self.active_actors = actors;
        self
    }
}

/// A scene is a named group of sequential cuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub cuts: Vec<CutId>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cuts: Vec::new(),
        }
    }
}

/// An episode is the top-level container: a sequence of scenes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub name: String,
    pub scenes: Vec<Scene>,
}

impl Episode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            scenes: Vec::new(),
        }
    }
}

/// Snapshot of the director's evaluation at a specific time.
#[derive(Debug, Clone)]
pub struct DirectorState {
    pub time: f32,
    pub active_cut: Option<CutId>,
    pub camera_state: CameraState,
}

/// Director: manages cuts, scenes, and episode sequencing.
/// Sorted Vec storage for O(log n) binary-search cut lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Director {
    pub episode: Episode,
    /// Sorted by start_time for binary search O(log n) lookup.
    sorted_cuts: Vec<(CutId, Cut)>,
    next_id: u32,
}

impl Director {
    pub fn new(episode_name: impl Into<String>) -> Self {
        Self {
            episode: Episode::new(episode_name),
            sorted_cuts: Vec::new(),
            next_id: 0,
        }
    }

    /// Add a cut and return its ID. Maintains sorted order by start_time.
    pub fn add_cut(&mut self, cut: Cut) -> CutId {
        let id = CutId(self.next_id);
        self.next_id += 1;
        let start = cut.start_time;
        let pos = self
            .sorted_cuts
            .binary_search_by(|(_, c)| c.start_time.partial_cmp(&start).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|pos| pos);
        self.sorted_cuts.insert(pos, (id, cut));
        id
    }

    /// Get a cut by ID.
    pub fn get_cut(&self, id: CutId) -> Option<&Cut> {
        self.sorted_cuts.iter().find(|(cid, _)| *cid == id).map(|(_, c)| c)
    }

    /// Get a mutable cut.
    pub fn get_cut_mut(&mut self, id: CutId) -> Option<&mut Cut> {
        self.sorted_cuts.iter_mut().find(|(cid, _)| *cid == id).map(|(_, c)| c)
    }

    /// Add a scene to the episode.
    pub fn add_scene(&mut self, scene: Scene) {
        self.episode.scenes.push(scene);
    }

    /// Find the active cut at a given time. O(log n) binary search.
    pub fn find_active_cut(&self, time: f32) -> Option<(CutId, &Cut)> {
        // Binary search for the last cut whose start_time <= time
        let idx = self
            .sorted_cuts
            .binary_search_by(|(_, c)| {
                if c.start_time <= time {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap_or_else(|pos| pos);

        // Check the candidate (idx-1, since binary_search returns insertion point)
        if idx > 0 {
            let (id, cut) = &self.sorted_cuts[idx - 1];
            if cut.contains_time(time) {
                return Some((*id, cut));
            }
        }
        // Also check idx==0 edge case
        if !self.sorted_cuts.is_empty() {
            let (id, cut) = &self.sorted_cuts[0];
            if cut.contains_time(time) {
                return Some((*id, cut));
            }
        }
        None
    }

    /// Total duration across all cuts.
    #[inline]
    pub fn duration(&self) -> f32 {
        self.sorted_cuts
            .iter()
            .map(|(_, c)| c.end_time)
            .fold(0.0f32, f32::max)
    }

    /// Evaluate the director state at a given time.
    pub fn evaluate(&self, _scene_graph: &SceneGraph, time: f32) -> DirectorState {
        match self.find_active_cut(time) {
            Some((cut_id, cut)) => {
                let local_time = time - cut.start_time;
                let camera_state = cut.camera.evaluate(local_time);
                DirectorState {
                    time,
                    active_cut: Some(cut_id),
                    camera_state,
                }
            }
            None => DirectorState {
                time,
                active_cut: None,
                camera_state: CameraState::default(),
            },
        }
    }

    /// Number of cuts.
    #[inline]
    pub fn cut_count(&self) -> usize {
        self.sorted_cuts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cut_timing() {
        let cut = Cut::new("opening", 0.0, 5.0);
        assert!(cut.contains_time(0.0));
        assert!(cut.contains_time(2.5));
        assert!(!cut.contains_time(5.0));
        assert_eq!(cut.duration(), 5.0);
    }

    #[test]
    fn test_director_add_and_find() {
        let mut dir = Director::new("Episode 1");
        let c1 = dir.add_cut(Cut::new("intro", 0.0, 3.0));
        let c2 = dir.add_cut(Cut::new("battle", 3.0, 8.0));

        assert_eq!(dir.cut_count(), 2);
        assert_eq!(dir.find_active_cut(1.0).map(|(id, _)| id), Some(c1));
        assert_eq!(dir.find_active_cut(5.0).map(|(id, _)| id), Some(c2));
        assert!(dir.find_active_cut(10.0).is_none());
        assert_eq!(dir.duration(), 8.0);
    }

    #[test]
    fn test_director_evaluate() {
        let mut dir = Director::new("Test");
        let _c1 = dir.add_cut(Cut::new("cut1", 0.0, 5.0));
        let sg = SceneGraph::new();
        let state = dir.evaluate(&sg, 2.0);
        assert!(state.active_cut.is_some());
        assert_eq!(state.time, 2.0);
    }
}
