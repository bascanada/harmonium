//! Style profile — metadata wrapper around a [`TuningOverlay`].
//!
//! A `StyleProfile` is loaded from JSON or TOML and applied onto
//! [`TuningParams::default()`] to produce the final engine personality.

use serde::{Deserialize, Serialize};

use crate::tuning::{TuningOverlay, TuningParams};

/// A named style profile with metadata and a partial tuning overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleProfile {
    /// Human-readable style name (e.g. "Medium Bossa Nova").
    pub name: String,
    /// Free-text description of the style's musical characteristics.
    pub description: String,
    /// Suggested BPM range `(min, max)` for this style.
    pub tempo_range: (f32, f32),
    /// Searchable tags (e.g. `["latin", "jazz", "relaxed"]`).
    pub tags: Vec<String>,
    /// Partial tuning — only the parameters this style changes.
    pub tuning: TuningOverlay,
}

impl StyleProfile {
    /// Apply this profile's overlay onto `TuningParams::default()`.
    pub fn to_tuning_params(&self) -> TuningParams {
        TuningParams::default().with_overlay(&self.tuning)
    }

    /// Apply this profile's overlay onto a custom base.
    pub fn apply_to(&self, base: &TuningParams) -> TuningParams {
        base.with_overlay(&self.tuning)
    }

    /// Deserialize from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Serialize to a TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tuning::ClassicGrooveOverlay;

    fn sample_profile() -> StyleProfile {
        StyleProfile {
            name: "Test Bossa".into(),
            description: "Straight 8ths, anticipated bass".into(),
            tempo_range: (120.0, 160.0),
            tags: vec!["latin".into(), "jazz".into()],
            tuning: TuningOverlay {
                classic_groove: Some(ClassicGrooveOverlay {
                    ghost_note_velocity: Some(0.15),
                    hat_on_beat_velocity: Some(0.5),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }
    }

    #[test]
    fn json_roundtrip() {
        let profile = sample_profile();
        let json = profile.to_json().expect("serialize");
        let profile2 = StyleProfile::from_json(&json).expect("deserialize");
        assert_eq!(profile, profile2);
    }

    #[test]
    fn toml_roundtrip() {
        let profile = sample_profile();
        let toml_str = profile.to_toml().expect("serialize");
        let profile2 = StyleProfile::from_toml(&toml_str).expect("deserialize");
        assert_eq!(profile, profile2);
    }

    #[test]
    fn to_tuning_params_applies_overlay() {
        let profile = sample_profile();
        let tp = profile.to_tuning_params();
        assert_eq!(tp.classic_groove.ghost_note_velocity, 0.15);
        assert_eq!(tp.classic_groove.hat_on_beat_velocity, 0.5);
        // Unchanged fields keep defaults
        assert_eq!(tp.classic_groove.kick_anticipation_velocity, 0.7);
        assert_eq!(tp.melody, crate::tuning::MelodyParams::default());
    }

    #[test]
    fn apply_to_custom_base() {
        let mut base = TuningParams::default();
        base.melody.default_hurst_factor = 0.9;

        let profile = sample_profile();
        let tp = profile.apply_to(&base);

        // Base modification preserved
        assert_eq!(tp.melody.default_hurst_factor, 0.9);
        // Profile overlay applied
        assert_eq!(tp.classic_groove.ghost_note_velocity, 0.15);
    }
}
