use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use harmonium_core::sequencer::RhythmMode;
use harmonium_core::harmony::HarmonyMode;
use harmonium_core::params::EngineParams;

/// Bevy-compatible mirror of core RhythmMode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect, Serialize, Deserialize)]
pub enum BevyRhythmMode {
    #[default]
    Euclidean,
    PerfectBalance,
    ClassicGroove,
}

impl From<BevyRhythmMode> for RhythmMode {
    fn from(mode: BevyRhythmMode) -> Self {
        match mode {
            BevyRhythmMode::Euclidean => RhythmMode::Euclidean,
            BevyRhythmMode::PerfectBalance => RhythmMode::PerfectBalance,
            BevyRhythmMode::ClassicGroove => RhythmMode::ClassicGroove,
        }
    }
}

impl From<RhythmMode> for BevyRhythmMode {
    fn from(mode: RhythmMode) -> Self {
        match mode {
            RhythmMode::Euclidean => BevyRhythmMode::Euclidean,
            RhythmMode::PerfectBalance => BevyRhythmMode::PerfectBalance,
            RhythmMode::ClassicGroove => BevyRhythmMode::ClassicGroove,
        }
    }
}

/// Bevy-compatible mirror of core HarmonyMode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect, Serialize, Deserialize)]
pub enum BevyHarmonyMode {
    #[default]
    Basic,
    Driver,
}

impl From<BevyHarmonyMode> for HarmonyMode {
    fn from(mode: BevyHarmonyMode) -> Self {
        match mode {
            BevyHarmonyMode::Basic => HarmonyMode::Basic,
            BevyHarmonyMode::Driver => HarmonyMode::Driver,
        }
    }
}

impl From<HarmonyMode> for BevyHarmonyMode {
    fn from(mode: HarmonyMode) -> Self {
        match mode {
            HarmonyMode::Basic => BevyHarmonyMode::Basic,
            HarmonyMode::Driver => BevyHarmonyMode::Driver,
        }
    }
}

/// Bevy-compatible mirror of EngineParams
/// This allows full editing in Bevy Inspector
#[derive(Clone, Debug, Reflect, Serialize, Deserialize)]
pub struct BevyEngineParams {
    pub arousal: f32,
    pub valence: f32,
    pub density: f32,
    pub tension: f32,
    pub smoothness: f32,
    
    pub algorithm: BevyRhythmMode,
    // Note: complex types like Vec might need deeper mirroring or wrapping if they are specific types
    // but Vec<i32> and Vec<bool> are fine.
    pub channel_routing: Vec<i32>,
    pub muted_channels: Vec<bool>,
    
    pub harmony_mode: BevyHarmonyMode,

    pub record_wav: bool,
    pub record_midi: bool,
    pub record_abc: bool,

    pub enable_synthesis_morphing: bool,
    
    pub gain_lead: f32,
    pub gain_bass: f32,
    pub gain_snare: f32,
    pub gain_hat: f32,

    pub vel_base_bass: u8,
    pub vel_base_snare: u8,

    pub poly_steps: usize,
    pub fixed_kick: bool,
}

impl Default for BevyEngineParams {
    fn default() -> Self {
        Self {
            arousal: 0.5,
            valence: 0.0, // Neutral
            density: 0.5,
            tension: 0.3,
            smoothness: 0.7,
            algorithm: BevyRhythmMode::default(),
            channel_routing: vec![-1; 16],
            muted_channels: vec![false; 16],
            harmony_mode: BevyHarmonyMode::default(),
            record_wav: false,
            record_midi: false,
            record_abc: false,
            enable_synthesis_morphing: true,
            gain_lead: 1.0,
            gain_bass: 1.0,
            gain_snare: 1.0,
            gain_hat: 1.0,
            vel_base_bass: 100,
            vel_base_snare: 100,
            poly_steps: 16,
            fixed_kick: false,
        }
    }
}

impl From<BevyEngineParams> for EngineParams {
    fn from(params: BevyEngineParams) -> Self {
        EngineParams {
            arousal: params.arousal,
            valence: params.valence,
            density: params.density,
            tension: params.tension,
            smoothness: params.smoothness,
            algorithm: params.algorithm.into(),
            channel_routing: params.channel_routing,
            muted_channels: params.muted_channels,
            harmony_mode: params.harmony_mode.into(),
            record_wav: params.record_wav,
            record_midi: params.record_midi,
            record_abc: params.record_abc,
            enable_synthesis_morphing: params.enable_synthesis_morphing,
            gain_lead: params.gain_lead,
            gain_bass: params.gain_bass,
            gain_snare: params.gain_snare,
            gain_hat: params.gain_hat,
            vel_base_bass: params.vel_base_bass,
            vel_base_snare: params.vel_base_snare,
            poly_steps: params.poly_steps,
            fixed_kick: params.fixed_kick,
        }
    }
}

impl From<EngineParams> for BevyEngineParams {
    fn from(params: EngineParams) -> Self {
        BevyEngineParams {
            arousal: params.arousal,
            valence: params.valence,
            density: params.density,
            tension: params.tension,
            smoothness: params.smoothness,
            algorithm: params.algorithm.into(),
            channel_routing: params.channel_routing,
            muted_channels: params.muted_channels,
            harmony_mode: params.harmony_mode.into(),
            record_wav: params.record_wav,
            record_midi: params.record_midi,
            record_abc: params.record_abc,
            enable_synthesis_morphing: params.enable_synthesis_morphing,
            gain_lead: params.gain_lead,
            gain_bass: params.gain_bass,
            gain_snare: params.gain_snare,
            gain_hat: params.gain_hat,
            vel_base_bass: params.vel_base_bass,
            vel_base_snare: params.vel_base_snare,
            poly_steps: params.poly_steps,
            fixed_kick: params.fixed_kick,
        }
    }
}
