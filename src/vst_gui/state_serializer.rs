//! State Serializer - Converts engine state to JSON for the webview

use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::engine::EngineParams;
use crate::params::ControlMode;
use crate::sequencer::RhythmMode;

/// Serializable engine state for the webview
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    // Harmony state
    pub current_chord: String,
    pub current_measure: u32,
    pub current_step: u32,
    pub is_minor_chord: bool,
    pub progression_name: String,
    pub progression_length: u32,
    pub harmony_mode: u8,

    // Rhythm state - Primary
    pub primary_steps: u32,
    pub primary_pulses: u32,
    pub primary_rotation: u32,
    pub primary_pattern: Vec<bool>,

    // Rhythm state - Secondary
    pub secondary_steps: u32,
    pub secondary_pulses: u32,
    pub secondary_rotation: u32,
    pub secondary_pattern: Vec<bool>,

    // Control mode
    pub is_emotion_mode: bool,

    // Emotional params
    pub arousal: f32,
    pub valence: f32,
    pub density: f32,
    pub tension: f32,

    // Direct params
    pub bpm: f32,
    pub rhythm_mode: u8,
    pub enable_rhythm: bool,
    pub enable_harmony: bool,
    pub enable_melody: bool,
    pub enable_voicing: bool,
    pub fixed_kick: bool,

    // Direct rhythm params
    pub rhythm_density: f32,
    pub rhythm_tension: f32,

    // Direct harmony params
    pub harmony_tension: f32,
    pub harmony_valence: f32,

    // Direct melody/voicing params
    pub melody_smoothness: f32,
    pub voicing_density: f32,
    pub voicing_tension: f32,

    // Channel state
    pub channel_muted: Vec<bool>,
    pub channel_gains: Vec<f32>,

    // Session info
    pub key: String,
    pub scale: String,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            current_chord: "I".to_string(),
            current_measure: 1,
            current_step: 0,
            is_minor_chord: false,
            progression_name: String::new(),
            progression_length: 4,
            harmony_mode: 1,

            primary_steps: 16,
            primary_pulses: 4,
            primary_rotation: 0,
            primary_pattern: vec![],

            secondary_steps: 12,
            secondary_pulses: 3,
            secondary_rotation: 0,
            secondary_pattern: vec![],

            is_emotion_mode: true,

            arousal: 0.5,
            valence: 0.3,
            density: 0.5,
            tension: 0.3,

            bpm: 120.0,
            rhythm_mode: 0,
            enable_rhythm: true,
            enable_harmony: true,
            enable_melody: true,
            enable_voicing: false,
            fixed_kick: false,

            rhythm_density: 0.5,
            rhythm_tension: 0.3,

            harmony_tension: 0.3,
            harmony_valence: 0.3,

            melody_smoothness: 0.7,
            voicing_density: 0.5,
            voicing_tension: 0.3,

            channel_muted: vec![false, false, false, false],
            channel_gains: vec![0.6, 1.0, 0.5, 0.4],

            key: "C".to_string(),
            scale: "major".to_string(),
        }
    }
}

/// Collect current state from engine params and control mode
pub fn collect_state(
    target_state: &Arc<Mutex<EngineParams>>,
    control_mode: &Arc<Mutex<ControlMode>>,
) -> EngineState {
    let mut state = EngineState::default();

    // Get control mode info
    if let Ok(mode) = control_mode.lock() {
        state.is_emotion_mode = mode.use_emotion_mode;
        state.enable_rhythm = mode.enable_rhythm;
        state.enable_harmony = mode.enable_harmony;
        state.enable_melody = mode.enable_melody;
        state.enable_voicing = mode.enable_voicing;
        state.fixed_kick = mode.fixed_kick;

        // Live state from engine (updated by tick())
        state.current_step = mode.current_step;
        state.current_measure = mode.current_measure;
        state.primary_pattern = mode.primary_pattern.clone();
        state.secondary_pattern = mode.secondary_pattern.clone();
        state.current_chord = mode.current_chord.clone();
        state.is_minor_chord = mode.is_minor_chord;
        state.progression_name = mode.progression_name.clone();
        state.key = mode.session_key.clone();
        state.scale = mode.session_scale.clone();

        // Direct params
        let dp = &mode.direct_params;
        state.bpm = dp.bpm;
        state.rhythm_mode = match dp.rhythm_mode {
            RhythmMode::Euclidean => 0,
            RhythmMode::PerfectBalance => 1,
            RhythmMode::ClassicGroove => 2,
        };
        // Primary rhythm
        state.primary_steps = dp.rhythm_steps as u32;
        state.primary_pulses = dp.rhythm_pulses as u32;
        state.primary_rotation = dp.rhythm_rotation as u32;
        state.rhythm_density = dp.rhythm_density;
        state.rhythm_tension = dp.rhythm_tension;
        // Secondary rhythm
        state.secondary_steps = dp.rhythm_secondary_steps as u32;
        state.secondary_pulses = dp.rhythm_secondary_pulses as u32;
        state.secondary_rotation = dp.rhythm_secondary_rotation as u32;
        // Harmony
        state.harmony_tension = dp.harmony_tension;
        state.harmony_valence = dp.harmony_valence;
        state.harmony_mode = match dp.harmony_mode {
            crate::harmony::HarmonyMode::Basic => 0,
            crate::harmony::HarmonyMode::Driver => 1,
        };
        // Melody/Voicing
        state.melody_smoothness = dp.melody_smoothness;
        state.voicing_density = dp.voicing_density;
        state.voicing_tension = dp.voicing_tension;
        // Channel state
        state.channel_muted = dp.muted_channels.clone();
    }

    // Get emotional params
    if let Ok(params) = target_state.lock() {
        state.arousal = params.arousal;
        state.valence = params.valence;
        state.density = params.density;
        state.tension = params.tension;
    }

    state
}

/// Create a state update message
pub fn create_state_update_message(state: &EngineState) -> String {
    let msg = serde_json::json!({
        "type": "state_update",
        "data": state
    });
    msg.to_string()
}
