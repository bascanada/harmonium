//! Message Handler - Processes JSON messages from the webview

use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::engine::EngineParams;
use harmonium_core::params::ControlMode;
use harmonium_core::sequencer::RhythmMode;
use harmonium_core::harmony::HarmonyMode;
use crate::vst_plugin::HarmoniumParams;

/// Handle an incoming message from the webview
/// Returns true if the message was handled successfully
pub fn handle_message(
    message: &str,
    target_state: &Arc<Mutex<EngineParams>>,
    control_mode: &Arc<Mutex<ControlMode>>,
    params: &Arc<HarmoniumParams>,
) -> bool {
    let msg: Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let msg_type = msg.get("type").and_then(|t| t.as_str());
    let method = msg.get("method").and_then(|m| m.as_str());
    let msg_params = msg.get("params");

    match msg_type {
        Some("set") => handle_set(method, msg_params, target_state, control_mode, params),
        Some("action") => handle_action(method, control_mode, params),
        Some("get") => true, // Get requests are handled elsewhere
        _ => false,
    }
}

/// Handle setter messages
fn handle_set(
    method: Option<&str>,
    msg_params: Option<&Value>,
    target_state: &Arc<Mutex<EngineParams>>,
    control_mode: &Arc<Mutex<ControlMode>>,
    _daw_params: &Arc<HarmoniumParams>,
) -> bool {
    let method = match method {
        Some(m) => m,
        None => return false,
    };

    match method {
        // Emotional params - update target_state directly and set webview control flag
        "set_arousal" => {
            if let Some(v) = msg_params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()) {
                let value = v as f32;
                // Mark webview as controlling emotional params
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_emotions = true;
                }
                // Update target_state directly
                if let Ok(mut state) = target_state.lock() {
                    state.arousal = value;
                }
                return true;
            }
        }
        "set_valence" => {
            if let Some(v) = msg_params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()) {
                let value = v as f32;
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_emotions = true;
                }
                if let Ok(mut state) = target_state.lock() {
                    state.valence = value;
                }
                return true;
            }
        }
        "set_density" => {
            if let Some(v) = msg_params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()) {
                let value = v as f32;
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_emotions = true;
                }
                if let Ok(mut state) = target_state.lock() {
                    state.density = value;
                }
                return true;
            }
        }
        "set_tension" => {
            if let Some(v) = msg_params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()) {
                let value = v as f32;
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_emotions = true;
                }
                if let Ok(mut state) = target_state.lock() {
                    state.tension = value;
                }
                return true;
            }
        }

        // Direct params - all set webview_controls_direct flag
        "set_direct_bpm" => {
            if let Some(v) = msg_params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.bpm = v as f32;
                }
                return true;
            }
        }
        "set_direct_rhythm_mode" => {
            if let Some(v) = msg_params.and_then(|p| p.get("mode")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_mode = match v {
                        1 => RhythmMode::PerfectBalance,
                        2 => RhythmMode::ClassicGroove,
                        _ => RhythmMode::Euclidean,
                    };
                }
                return true;
            }
        }
        "set_direct_rhythm_steps" => {
            if let Some(v) = msg_params.and_then(|p| p.get("steps")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_steps = v as usize;
                }
                return true;
            }
        }
        "set_direct_rhythm_pulses" => {
            if let Some(v) = msg_params.and_then(|p| p.get("pulses")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_pulses = v as usize;
                }
                return true;
            }
        }
        "set_direct_rhythm_rotation" => {
            if let Some(v) = msg_params.and_then(|p| p.get("rotation")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_rotation = v as usize;
                }
                return true;
            }
        }
        "set_direct_rhythm_density" => {
            if let Some(v) = msg_params.and_then(|p| p.get("density")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_density = v as f32;
                }
                return true;
            }
        }
        "set_direct_rhythm_tension" => {
            if let Some(v) = msg_params.and_then(|p| p.get("tension")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_tension = v as f32;
                }
                return true;
            }
        }
        // Secondary rhythm params
        "set_direct_secondary_steps" => {
            if let Some(v) = msg_params.and_then(|p| p.get("steps")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_secondary_steps = v as usize;
                }
                return true;
            }
        }
        "set_direct_secondary_pulses" => {
            if let Some(v) = msg_params.and_then(|p| p.get("pulses")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_secondary_pulses = v as usize;
                }
                return true;
            }
        }
        "set_direct_secondary_rotation" => {
            if let Some(v) = msg_params.and_then(|p| p.get("rotation")).and_then(|v| v.as_i64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_secondary_rotation = v as usize;
                }
                return true;
            }
        }
        "set_direct_harmony_tension" => {
            if let Some(v) = msg_params.and_then(|p| p.get("tension")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.harmony_tension = v as f32;
                }
                return true;
            }
        }
        "set_direct_harmony_valence" => {
            if let Some(v) = msg_params.and_then(|p| p.get("valence")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.harmony_valence = v as f32;
                }
                return true;
            }
        }
        "set_direct_melody_smoothness" => {
            if let Some(v) = msg_params.and_then(|p| p.get("smoothness")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.melody_smoothness = v as f32;
                }
                return true;
            }
        }
        "set_direct_voicing_density" => {
            if let Some(v) = msg_params.and_then(|p| p.get("density")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.voicing_density = v as f32;
                }
                return true;
            }
        }
        "set_direct_voicing_tension" => {
            if let Some(v) = msg_params.and_then(|p| p.get("tension")).and_then(|v| v.as_f64()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.voicing_tension = v as f32;
                }
                return true;
            }
        }

        // Module enables
        "set_direct_enable_rhythm" => {
            if let Some(v) = msg_params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.enable_rhythm = v;
                }
                return true;
            }
        }
        "set_direct_enable_harmony" => {
            if let Some(v) = msg_params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.enable_harmony = v;
                }
                return true;
            }
        }
        "set_direct_enable_melody" => {
            if let Some(v) = msg_params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.enable_melody = v;
                }
                return true;
            }
        }
        "set_direct_enable_voicing" => {
            if let Some(v) = msg_params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.enable_voicing = v;
                }
                return true;
            }
        }
        "set_direct_fixed_kick" => {
            if let Some(v) = msg_params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) {
                if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.fixed_kick = v;
                }
                return true;
            }
        }

        // Channel controls
        "set_channel_muted" => {
            let channel = msg_params.and_then(|p| p.get("channel")).and_then(|v| v.as_i64());
            let muted = msg_params.and_then(|p| p.get("muted")).and_then(|v| v.as_bool());
            if let (Some(ch), Some(m)) = (channel, muted) {
                if let Ok(mut mode) = control_mode.lock() {
                    let idx = ch as usize;
                    if idx < mode.direct_params.muted_channels.len() {
                        mode.direct_params.muted_channels[idx] = m;
                    }
                }
                return true;
            }
        }
        "set_channel_gain" => {
            // Channel gains are handled separately in the audio backend
            // For now, just acknowledge the message
            return true;
        }

        // Algorithm and harmony mode
        "set_algorithm" => {
            if let Some(v) = msg_params.and_then(|p| p.get("mode")).and_then(|v| v.as_i64()) {
                if let Ok(mut state) = target_state.lock() {
                    state.algorithm = match v {
                        1 => RhythmMode::PerfectBalance,
                        2 => RhythmMode::ClassicGroove,
                        _ => RhythmMode::Euclidean,
                    };
                }
                return true;
            }
        }
        "set_harmony_mode" => {
            if let Some(v) = msg_params.and_then(|p| p.get("mode")).and_then(|v| v.as_i64()) {
                // Update both target_state and direct_params for consistency
                let mode = if v == 1 { HarmonyMode::Driver } else { HarmonyMode::Basic };
                if let Ok(mut state) = target_state.lock() {
                    state.harmony_mode = mode;
                }
                if let Ok(mut ctrl) = control_mode.lock() {
                    ctrl.webview_controls_direct = true;
                    ctrl.direct_params.harmony_mode = mode;
                }
                return true;
            }
        }

        "set_all_rhythm_params" => {
            if let (
                Some(mode_val),
                Some(steps),
                Some(pulses),
                Some(rot),
                Some(den),
                Some(ten),
                Some(sec_steps),
                Some(sec_pulses),
                Some(sec_rot)
            ) = (
                msg_params.and_then(|p| p.get("mode")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("steps")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("pulses")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("rotation")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("density")).and_then(|v| v.as_f64()),
                msg_params.and_then(|p| p.get("tension")).and_then(|v| v.as_f64()),
                msg_params.and_then(|p| p.get("secondarySteps")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("secondaryPulses")).and_then(|v| v.as_i64()),
                msg_params.and_then(|p| p.get("secondaryRotation")).and_then(|v| v.as_i64())
            ) {
                 if let Ok(mut mode) = control_mode.lock() {
                    mode.webview_controls_direct = true;
                    mode.direct_params.rhythm_mode = match mode_val {
                        1 => RhythmMode::PerfectBalance,
                        2 => RhythmMode::ClassicGroove,
                        _ => RhythmMode::Euclidean,
                    };
                    mode.direct_params.rhythm_steps = steps as usize;
                    mode.direct_params.rhythm_pulses = pulses as usize;
                    mode.direct_params.rhythm_rotation = rot as usize;
                    mode.direct_params.rhythm_density = den as f32;
                    mode.direct_params.rhythm_tension = ten as f32;
                    mode.direct_params.rhythm_secondary_steps = sec_steps as usize;
                    mode.direct_params.rhythm_secondary_pulses = sec_pulses as usize;
                    mode.direct_params.rhythm_secondary_rotation = sec_rot as usize;
                }
                return true;
            }
        }

        _ => {}
    }

    false
}

/// Handle action messages (mode switching, etc.)
fn handle_action(
    method: Option<&str>,
    control_mode: &Arc<Mutex<ControlMode>>,
    _params: &Arc<HarmoniumParams>,
) -> bool {
    let method = match method {
        Some(m) => m,
        None => return false,
    };

    match method {
        "use_emotion_mode" => {
            if let Ok(mut mode) = control_mode.lock() {
                mode.use_emotion_mode = true;
                mode.webview_controls_mode = true;
            }
            true
        }
        "use_direct_mode" => {
            if let Ok(mut mode) = control_mode.lock() {
                mode.use_emotion_mode = false;
                mode.webview_controls_mode = true;
                // EXPLICITLY set this to true to prevent DAW from overwriting params immediately
                mode.webview_controls_direct = true;
            }
            true
        }
        "init" => true,
        _ => false,
    }
}
