use bevy::prelude::*;
use harmonium_core::sequencer::RhythmMode;
use harmonium_core::params::EngineParams;

/// The main component to attach to an entity (e.g. "MusicManager")
/// to control the generative engine.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct HarmoniumSource {
    /// 1. Enable/Disable note generation
    pub is_enabled: bool,
    
    /// 2. Technical configuration of the algorithm
    pub config: GenerativeConfig,
    
    /// 3. Synth configuration (Odin 2)
    pub synth: OdinConfig,

    /// 4. Manual / UI parameters for mixing
    pub manual_visual_params: EngineParams,
}

impl Default for HarmoniumSource {
    fn default() -> Self {
        Self {
            is_enabled: true,
            config: GenerativeConfig::default(),
            synth: OdinConfig::default(),
            manual_visual_params: EngineParams::default(),
        }
    }
}

/// Advanced technical parameters (similar to "Direct" mode)
#[derive(Clone, Reflect, Debug)]
pub struct GenerativeConfig {
    pub rhythm_mode: RhythmMode,
    pub steps: usize,      // 16, 32, 48...
    pub density: f32,      // 0.0 - 1.0 (Probability of note)
    pub tension: f32,      // 0.0 - 1.0 (Dissonance / Complexity)
    pub tempo: f32,        // BPM
}

impl Default for GenerativeConfig {
    fn default() -> Self {
        Self {
            rhythm_mode: RhythmMode::Euclidean,
            steps: 16,
            density: 0.5,
            tension: 0.3,
            tempo: 120.0,
        }
    }
}

use crate::assets::OdinAsset;

/// Configuration for Odin 2 presets
#[derive(Clone, Reflect, Default, Debug)]
pub struct OdinConfig {
    /// Handle to the loaded .odin asset
    /// Set this handle via AssetServer.load("my_preset.odin")
    pub preset: Handle<OdinAsset>,
    
    /// Or selection by Bank/Program if using SoundFonts
    pub bank_id: u32,
    pub program_id: u8,
}

/// Component to attach to game entities (Enemies, Zones, Items)
/// to give them a semantic meaning for the music AI.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct HarmoniumTag {
    /// Semantic keywords (e.g. ["danger", "rust", "mechanical"])
    pub tags: Vec<String>,
    /// Weight of the entity influence (e.g. Boss has higher weight than Minion)
    pub weight: f32,
}

impl HarmoniumTag {
    pub fn new(tags: &[&str], weight: f32) -> Self {
        Self {
            tags: tags.iter().map(|s| s.to_string()).collect(),
            weight,
        }
    }
}

/// Controls the "Mix" between Manual settings and AI analysis.
/// Usually attached to the Player or Camera.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AiDriver {
    /// 0.0 = Pure Manual (UI Sliders), 1.0 = Pure AI, 0.5 = Mix
    pub ai_influence: f32,
    
    /// Detection radius in game units
    pub detection_radius: f32,
    
    /// Scan frequency
    pub scan_timer: Timer,

    /// Cached AI target implementation
    pub ai_target: EngineParams,
}

impl Default for AiDriver {
    fn default() -> Self {
        Self {
            ai_influence: 1.0, // Full AI by default
            detection_radius: 50.0,
            scan_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            ai_target: EngineParams::default(),
        }
    }
}
