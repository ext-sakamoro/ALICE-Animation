pub mod scene;
pub mod director;
pub mod camera;
pub mod npr;
pub mod episode;

#[cfg(feature = "voice")]
pub mod lip_sync;

#[cfg(feature = "codec")]
pub mod codec_bridge;
#[cfg(feature = "cdn")]
pub mod cdn_bridge;
#[cfg(feature = "cache")]
pub mod cache_bridge;
#[cfg(feature = "db")]
pub mod db_bridge;
#[cfg(feature = "browser")]
pub mod browser_bridge;
#[cfg(feature = "ml")]
pub mod ml_bridge;

// Re-exports
pub use scene::{Actor, ActorId, ActorTransform, SceneGraph};
pub use director::{Cut, CutId, Director, DirectorState};
pub use camera::{CameraState, CameraTrack, CameraWork, FakePerspective};
pub use npr::{AnimeShading, CelShading, OutlineConfig};
pub use episode::{EpisodeMetadata, EpisodePackage};
