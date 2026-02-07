//! Emotional Preset Bank
//!
//! Defines the 4 corner presets for emotional morphing based on Russell's Circumplex Model.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::types::SynthPreset;

// Embed TOML file at compile time
const DEFAULT_PRESETS_TOML: &str = include_str!("presets.toml");

// Parse on first access (thread-safe singleton)
static DEFAULT_PRESET_BANK: std::sync::LazyLock<EmotionalPresetBank> =
    std::sync::LazyLock::new(|| {
        toml::from_str(DEFAULT_PRESETS_TOML).unwrap_or_else(|e| {
            panic!("Failed to parse embedded presets.toml: {}", e);
        })
    });

/// Holds the 4 corner presets for emotional morphing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalPresetBank {
    /// Joy/Victory (Valence +, Arousal +) - Q1
    pub joy: InstrumentPresets,

    /// Anger/Stress (Valence -, Arousal +) - Q2
    pub anger: InstrumentPresets,

    /// Sadness/Dark (Valence -, Arousal -) - Q3
    pub sadness: InstrumentPresets,

    /// Calm/Serenity (Valence +, Arousal -) - Q4
    pub calm: InstrumentPresets,
}

/// Presets for all instrument types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentPresets {
    pub bass: SynthPreset,
    pub lead: SynthPreset,
    pub snare: SynthPreset,
    pub hat: SynthPreset,
    pub poly: SynthPreset,
}

impl EmotionalPresetBank {
    /// Load default presets from embedded TOML
    pub fn default_presets() -> Self {
        DEFAULT_PRESET_BANK.clone()
    }

    /// Load from external TOML file (runtime)
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let bank: Self = toml::from_str(&contents)?;
        Ok(bank)
    }

    /// Save to TOML file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }
}

// All preset data moved to presets.toml
// Hardcoded functions (joy_presets, anger_presets, sadness_presets, calm_presets) removed
