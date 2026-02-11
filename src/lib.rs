pub mod scene;
pub mod director;
pub mod camera;
pub mod npr;
pub mod episode;

#[cfg(feature = "voice")]
pub mod lip_sync;

// Re-exports
pub use scene::{Actor, ActorId, ActorTransform, SceneGraph};
pub use director::{Cut, CutId, Director, DirectorState};
pub use camera::{CameraState, CameraTrack, CameraWork, FakePerspective};
pub use npr::{AnimeShading, CelShading, OutlineConfig};
pub use episode::{EpisodeMetadata, EpisodePackage};
