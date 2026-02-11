//! Bridge: ALICE-Animation → ALICE-Browser
//! Web-based anime player: SDF evaluation + NPR rendering in browser.

use crate::{DirectorState, EpisodePackage};
// use alice_browser::RenderTarget;

/// Web player configuration for browser-based anime playback.
#[derive(Debug, Clone)]
pub struct WebPlayerConfig {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub target_fps: f32,
    pub quality: RenderQuality,
    pub autoplay: bool,
}

/// Render quality presets for different bandwidth/device scenarios.
#[derive(Debug, Clone, Copy)]
pub enum RenderQuality {
    /// Low quality — mobile, slow connection (SDF eval at 1/4 resolution).
    Low,
    /// Medium quality — tablet, moderate connection.
    Medium,
    /// High quality — desktop, fast connection (full resolution SDF).
    High,
    /// Ultra quality — 4K display, local playback.
    Ultra,
}

impl RenderQuality {
    /// Resolution scale factor (0.25 - 2.0).
    #[inline]
    pub fn scale_factor(self) -> f32 {
        match self {
            RenderQuality::Low => 0.25,
            RenderQuality::Medium => 0.5,
            RenderQuality::High => 1.0,
            RenderQuality::Ultra => 2.0,
        }
    }
}

impl Default for WebPlayerConfig {
    fn default() -> Self {
        Self {
            canvas_width: 1920,
            canvas_height: 1080,
            target_fps: 24.0,
            quality: RenderQuality::High,
            autoplay: false,
        }
    }
}

/// Player state for a running episode.
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub current_time: f32,
    pub playing: bool,
    pub buffered_frames: usize,
    pub director_state: Option<DirectorState>,
}

impl PlayerState {
    /// Create a new player state at time zero.
    #[inline]
    pub fn new() -> Self {
        Self {
            current_time: 0.0,
            playing: false,
            buffered_frames: 0,
            director_state: None,
        }
    }

    /// Advance time by delta seconds.
    #[inline]
    pub fn advance(&mut self, delta_seconds: f32) {
        if self.playing {
            self.current_time += delta_seconds;
        }
    }

    /// Toggle play/pause.
    #[inline]
    pub fn toggle_play(&mut self) {
        self.playing = !self.playing;
    }

    /// Seek to a specific time.
    #[inline]
    pub fn seek(&mut self, time: f32) {
        self.current_time = time.max(0.0);
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Web player for episodes.
pub struct WebPlayer {
    pub config: WebPlayerConfig,
    pub state: PlayerState,
    pub episode: Option<EpisodePackage>,
}

impl WebPlayer {
    /// Create a new web player.
    #[inline]
    pub fn new(config: WebPlayerConfig) -> Self {
        Self {
            config,
            state: PlayerState::new(),
            episode: None,
        }
    }

    /// Load an episode.
    #[inline]
    pub fn load_episode(&mut self, episode: EpisodePackage) {
        self.episode = Some(episode);
        self.state.current_time = 0.0;
        self.state.playing = self.config.autoplay;
    }

    /// Update player state and render a frame.
    #[inline]
    pub fn update(&mut self, delta_seconds: f32) {
        self.state.advance(delta_seconds);
        if let Some(ref episode) = self.episode {
            let state = episode.director.evaluate(&episode.scene_graph, self.state.current_time);
            self.state.director_state = Some(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::episode::EpisodeMetadata;
    use crate::npr::AnimeShading;
    use crate::scene::{Actor, SceneGraph};
    use alice_sdf::SdfNode;

    #[test]
    fn test_render_quality_scale() {
        assert_eq!(RenderQuality::Low.scale_factor(), 0.25);
        assert_eq!(RenderQuality::High.scale_factor(), 1.0);
        assert_eq!(RenderQuality::Ultra.scale_factor(), 2.0);
    }

    #[test]
    fn test_player_state() {
        let mut state = PlayerState::new();
        assert_eq!(state.current_time, 0.0);
        assert!(!state.playing);

        state.toggle_play();
        state.advance(1.0);
        assert_eq!(state.current_time, 1.0);

        state.seek(5.0);
        assert_eq!(state.current_time, 5.0);
    }

    #[test]
    fn test_web_player() {
        let mut player = WebPlayer::new(WebPlayerConfig::default());
        let mut sg = SceneGraph::new();
        sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 10.0));
        let meta = EpisodeMetadata::new("Web Test", 1, 10.0);
        let episode = EpisodePackage::new(meta, sg, dir, AnimeShading::default());

        player.load_episode(episode);
        player.state.toggle_play();
        player.update(1.0);
        assert_eq!(player.state.current_time, 1.0);
    }
}
