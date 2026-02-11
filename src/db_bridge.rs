//! Bridge: ALICE-Animation → ALICE-DB
//! Episode persistence, metadata indexing, and search.

use crate::episode::{EpisodeMetadata, EpisodePackage};
// use alice_db::{Database, Record};

/// Episode record for database storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EpisodeRecord {
    pub id: String,
    pub title: String,
    pub episode_number: u32,
    pub duration_seconds: f32,
    pub size_bytes: usize,
    pub actor_count: usize,
    pub cut_count: usize,
    pub created_at: u64,
}

impl EpisodeRecord {
    /// Create a record from an EpisodePackage.
    #[inline]
    pub fn from_package(package: &EpisodePackage) -> Self {
        let id = format!(
            "ep-{:04}-{}",
            package.metadata.episode_number, package.metadata.title
        );
        Self {
            id,
            title: package.metadata.title.clone(),
            episode_number: package.metadata.episode_number,
            duration_seconds: package.metadata.duration_seconds,
            size_bytes: 0, // Set after serialization
            actor_count: package.scene_graph.actor_count(),
            cut_count: package.director.cut_count(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Update size after serialization.
    #[inline]
    pub fn with_size(mut self, size_bytes: usize) -> Self {
        self.size_bytes = size_bytes;
        self
    }
}

/// Query parameters for episode search.
#[derive(Debug, Clone, Default)]
pub struct EpisodeQuery {
    pub title_contains: Option<String>,
    pub min_duration: Option<f32>,
    pub max_duration: Option<f32>,
    pub min_episode_number: Option<u32>,
    pub max_episode_number: Option<u32>,
}

impl EpisodeQuery {
    /// Create a new empty query.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by title substring.
    #[inline]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title_contains = Some(title.into());
        self
    }

    /// Filter by duration range.
    #[inline]
    pub fn with_duration_range(mut self, min: f32, max: f32) -> Self {
        self.min_duration = Some(min);
        self.max_duration = Some(max);
        self
    }

    /// Filter by episode number range.
    #[inline]
    pub fn with_episode_range(mut self, min: u32, max: u32) -> Self {
        self.min_episode_number = Some(min);
        self.max_episode_number = Some(max);
        self
    }

    /// Check if a record matches this query.
    #[inline]
    pub fn matches(&self, record: &EpisodeRecord) -> bool {
        if let Some(ref title) = self.title_contains {
            if !record.title.contains(title) {
                return false;
            }
        }
        if let Some(min) = self.min_duration {
            if record.duration_seconds < min {
                return false;
            }
        }
        if let Some(max) = self.max_duration {
            if record.duration_seconds > max {
                return false;
            }
        }
        if let Some(min) = self.min_episode_number {
            if record.episode_number < min {
                return false;
            }
        }
        if let Some(max) = self.max_episode_number {
            if record.episode_number > max {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::npr::AnimeShading;
    use crate::scene::{Actor, SceneGraph};
    use alice_sdf::SdfNode;

    #[test]
    fn test_episode_record_from_package() {
        let mut sg = SceneGraph::new();
        sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 120.0));
        let meta = EpisodeMetadata::new("DB Test", 5, 120.0);
        let episode = EpisodePackage::new(meta, sg, dir, AnimeShading::default());

        let record = EpisodeRecord::from_package(&episode);
        assert_eq!(record.episode_number, 5);
        assert_eq!(record.duration_seconds, 120.0);
        assert_eq!(record.actor_count, 1);
        assert_eq!(record.cut_count, 1);
    }

    #[test]
    fn test_query_matches() {
        let record = EpisodeRecord {
            id: "ep-0005-Test".into(),
            title: "Test Episode".into(),
            episode_number: 5,
            duration_seconds: 120.0,
            size_bytes: 50000,
            actor_count: 2,
            cut_count: 3,
            created_at: 0,
        };

        let query = EpisodeQuery::new().with_title("Test");
        assert!(query.matches(&record));

        let query = EpisodeQuery::new().with_duration_range(100.0, 150.0);
        assert!(query.matches(&record));

        let query = EpisodeQuery::new().with_episode_range(1, 10);
        assert!(query.matches(&record));

        let query = EpisodeQuery::new().with_title("NotFound");
        assert!(!query.matches(&record));
    }
}
