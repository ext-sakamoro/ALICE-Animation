use alice_sdf::animation::{Keyframe, Timeline, Track};
use alice_voice::ParametricParams;
use serde::{Deserialize, Serialize};

/// Japanese vowel phonemes for mouth shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phoneme {
    /// Mouth closed
    Closed,
    /// あ (open wide)
    A,
    /// い (wide narrow)
    I,
    /// う (small round)
    U,
    /// え (half open)
    E,
    /// お (round open)
    O,
}

impl Phoneme {
    /// Mouth openness value (0.0 = closed, 1.0 = fully open).
    pub fn openness(&self) -> f32 {
        match self {
            Phoneme::Closed => 0.0,
            Phoneme::A => 1.0,
            Phoneme::I => 0.3,
            Phoneme::U => 0.4,
            Phoneme::E => 0.6,
            Phoneme::O => 0.7,
        }
    }

    /// Mouth width value (0.0 = narrow, 1.0 = wide).
    pub fn width(&self) -> f32 {
        match self {
            Phoneme::Closed => 0.3,
            Phoneme::A => 0.8,
            Phoneme::I => 1.0,
            Phoneme::U => 0.2,
            Phoneme::E => 0.9,
            Phoneme::O => 0.5,
        }
    }
}

/// A single phoneme keyframe with timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeKeyframe {
    pub time: f32,
    pub phoneme: Phoneme,
}

/// Lip sync animation track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LipSyncTrack {
    pub name: String,
    pub phonemes: Vec<PhonemeKeyframe>,
}

impl LipSyncTrack {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            phonemes: Vec::new(),
        }
    }

    /// Add a phoneme at a given time.
    pub fn add_phoneme(&mut self, time: f32, phoneme: Phoneme) {
        self.phonemes.push(PhonemeKeyframe { time, phoneme });
        self.phonemes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Convert to an ALICE-SDF Timeline with two tracks: "mouth.openness" and "mouth.width".
    pub fn to_timeline(&self) -> Timeline {
        let mut tl = Timeline::new(&self.name);

        let mut openness_track = Track::new("mouth.openness");
        let mut width_track = Track::new("mouth.width");

        for kf in &self.phonemes {
            openness_track.add_keyframe(Keyframe::new(kf.time, kf.phoneme.openness()));
            width_track.add_keyframe(Keyframe::new(kf.time, kf.phoneme.width()));
        }

        tl.add_track(openness_track);
        tl.add_track(width_track);
        tl
    }

    /// Duration of this lip sync track.
    pub fn duration(&self) -> f32 {
        self.phonemes.last().map(|kf| kf.time).unwrap_or(0.0)
    }
}

/// Classify a vowel phoneme from formant frequencies (F1, F2).
///
/// Based on Japanese vowel formant chart:
/// - あ (A): F1 ~700-800, F2 ~1200-1400
/// - い (I): F1 ~250-350, F2 ~2200-2600
/// - う (U): F1 ~300-400, F2 ~1000-1200
/// - え (E): F1 ~450-600, F2 ~1800-2200
/// - お (O): F1 ~500-600, F2 ~800-1000
fn classify_phoneme(f1: f32, f2: f32) -> Phoneme {
    // Low F1 + high F2 → い
    if f1 < 400.0 && f2 > 2000.0 {
        return Phoneme::I;
    }
    // Low F1 + low F2 → う
    if f1 < 450.0 && f2 < 1300.0 {
        return Phoneme::U;
    }
    // High F1 + mid F2 → あ
    if f1 > 600.0 && f2 > 1000.0 && f2 < 1600.0 {
        return Phoneme::A;
    }
    // Mid F1 + high F2 → え
    if f1 > 400.0 && f2 > 1600.0 {
        return Phoneme::E;
    }
    // Mid F1 + low F2 → お
    if f1 > 400.0 && f2 < 1100.0 {
        return Phoneme::O;
    }
    // Default: あ (most common)
    Phoneme::A
}

/// Convert ALICE-Voice parametric params to a lip sync track.
///
/// Each ParametricParams frame maps to a phoneme based on formant analysis.
pub fn sync_voice_to_animation(
    voice_params: &[ParametricParams],
    frame_duration: f32,
) -> LipSyncTrack {
    let mut track = LipSyncTrack::new("lip_sync");
    let mut prev_phoneme = Phoneme::Closed;

    for (i, params) in voice_params.iter().enumerate() {
        let time = i as f32 * frame_duration;

        // Extract F1 and F2 from formants
        let phoneme = if params.formants.len() >= 2 {
            let f1 = params.formants[0].frequency;
            let f2 = params.formants[1].frequency;
            // Skip if both frequencies are too low (silence)
            if f1 < 100.0 && f2 < 100.0 {
                Phoneme::Closed
            } else {
                classify_phoneme(f1, f2)
            }
        } else {
            Phoneme::Closed
        };

        // Only add keyframes on phoneme changes to reduce data
        if phoneme != prev_phoneme {
            track.add_phoneme(time, phoneme);
            prev_phoneme = phoneme;
        }
    }

    // Close mouth at end
    if prev_phoneme != Phoneme::Closed {
        let end_time = voice_params.len() as f32 * frame_duration;
        track.add_phoneme(end_time, Phoneme::Closed);
    }

    track
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phoneme_values() {
        assert_eq!(Phoneme::Closed.openness(), 0.0);
        assert_eq!(Phoneme::A.openness(), 1.0);
        assert!(Phoneme::I.width() > Phoneme::U.width());
    }

    #[test]
    fn test_classify_phoneme() {
        assert_eq!(classify_phoneme(750.0, 1300.0), Phoneme::A);
        assert_eq!(classify_phoneme(300.0, 2400.0), Phoneme::I);
        assert_eq!(classify_phoneme(350.0, 1100.0), Phoneme::U);
        assert_eq!(classify_phoneme(500.0, 1900.0), Phoneme::E);
        assert_eq!(classify_phoneme(500.0, 900.0), Phoneme::O);
    }

    #[test]
    fn test_lip_sync_track_to_timeline() {
        let mut track = LipSyncTrack::new("test");
        track.add_phoneme(0.0, Phoneme::A);
        track.add_phoneme(0.5, Phoneme::I);
        track.add_phoneme(1.0, Phoneme::Closed);

        let tl = track.to_timeline();
        assert_eq!(tl.tracks.len(), 2);

        let openness = tl.get_value("mouth.openness", 0.0).unwrap();
        assert_eq!(openness, 1.0); // A = fully open
    }
}
