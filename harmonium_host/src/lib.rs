use std::sync::{Arc, Mutex};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod timeline_engine;

// Decoupled architecture: MusicComposer (main thread) + PlaybackEngine (audio thread)
#[cfg(feature = "standalone")]
pub mod composer;
#[cfg(feature = "standalone")]
pub mod playback;

// Re-exports from workspace crates
#[cfg(feature = "ai")]
pub use harmonium_ai::ai;
pub use harmonium_ai::mapper;
pub use harmonium_audio::{backend, realtime, synthesis, voice_manager, voicing};
pub use harmonium_core::{events, fractal, harmony, log, params, sequencer};

// Real-time safety: Global allocator that panics on allocations in audio thread (debug builds only)
// Uses fully qualified path to avoid local mod ambiguity
#[cfg(debug_assertions)]
#[global_allocator]
static GLOBAL: harmonium_audio::realtime::rt_check::RTCheckAllocator =
    harmonium_audio::realtime::rt_check::RTCheckAllocator;

// Audio module (only for standalone/WASM builds with cpal)
#[cfg(feature = "standalone")]
pub mod audio;

// Native handle (standalone without wasm)
#[cfg(feature = "standalone")]
pub mod native_handle;
#[cfg(feature = "standalone")]
pub use native_handle::NativeHandle;

// VST Plugin module (only for VST builds)
#[cfg(feature = "vst")]
pub mod vst_plugin;

// VST GUI module (only for VST builds with GUI)
#[cfg(feature = "vst-gui")]
pub mod vst_gui;

// Re-exports pour compatibilité avec l'ancien code
// Re-export audio backend type (for runtime switching)
#[cfg(feature = "standalone")]
pub use audio::AudioBackendType;
pub use harmonium_ai::mapper::{EmotionMapper, MapperConfig};
// Re-exports pour la nouvelle architecture découplée
// Note: HarmonyStrategy removed if not in core/params or changed
pub use harmonium_core::params::{ControlMode, MusicalParams};
pub use harmonium_core::{
    harmony::{HarmonyMode, basic as progression, melody as harmony_melody},
    sequencer::RhythmMode,
};
// Re-export VST plugin when building with vst feature
#[cfg(feature = "vst")]
pub use vst_plugin::HarmoniumPlugin;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct RecordedData {
    format_str: String,
    data: Vec<u8>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl RecordedData {
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format_str.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[cfg(not(feature = "wasm"))]
impl RecordedData {
    pub fn format(&self) -> String {
        self.format_str.clone()
    }

    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

// Type aliases for complex types
pub type FontQueue = Arc<Mutex<Vec<(u32, Vec<u8>)>>>;
pub type FinishedRecordings = Arc<Mutex<Vec<(events::RecordFormat, Vec<u8>)>>>;
/// Shared measure pages: Composer writes by index, PlaybackEngine reads by index.
pub type SharedPages = Arc<Mutex<Vec<harmonium_core::timeline::Measure>>>;

// Handle and WASM bindings only available with wasm feature
// TODO: Phase 3 - Rebuild this API to use controller properly
#[cfg(all(feature = "standalone", feature = "wasm"))]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Handle {
    #[allow(dead_code)]
    stream: cpal::Stream,
    /// Unified controller for all engine communication
    controller: harmonium_core::HarmoniumController,
    /// Queue de chargement de SoundFonts
    font_queue: FontQueue,
    /// Enregistrements terminés
    finished_recordings: FinishedRecordings,
    /// Cached UI-side parameters for getters
    cached_params: harmonium_core::EngineParams,
    bpm: f32,
    key: String,
    scale: String,
    pulses: usize,
    steps: usize,
}

#[cfg(all(feature = "standalone", feature = "wasm"))]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Handle {
    // === Session Info (static, from initial config) ===

    pub fn get_bpm(&self) -> f32 {
        self.bpm
    }

    pub fn get_key(&self) -> String {
        self.key.clone()
    }

    pub fn get_scale(&self) -> String {
        self.scale.clone()
    }

    pub fn get_pulses(&self) -> usize {
        self.pulses
    }

    pub fn get_steps(&self) -> usize {
        self.steps
    }

    // === Emotion Controls ===

    /// Set arousal (0.0-1.0) - controls BPM (70-180)
    pub fn set_arousal(&mut self, arousal: f32) {
        self.cached_params.arousal = arousal.clamp(0.0, 1.0);
        let _ = self.controller.set_emotions(
            self.cached_params.arousal,
            self.cached_params.valence,
            self.cached_params.density,
            self.cached_params.tension,
        );
    }

    /// Set valence (-1.0 to 1.0) - major/minor bias
    pub fn set_valence(&mut self, valence: f32) {
        self.cached_params.valence = valence.clamp(-1.0, 1.0);
        let _ = self.controller.set_emotions(
            self.cached_params.arousal,
            self.cached_params.valence,
            self.cached_params.density,
            self.cached_params.tension,
        );
    }

    /// Set rhythmic density (0.0-1.0)
    pub fn set_density(&mut self, density: f32) {
        self.cached_params.density = density.clamp(0.0, 1.0);
        let _ = self.controller.set_emotions(
            self.cached_params.arousal,
            self.cached_params.valence,
            self.cached_params.density,
            self.cached_params.tension,
        );
    }

    /// Set harmonic tension (0.0-1.0)
    pub fn set_tension(&mut self, tension: f32) {
        self.cached_params.tension = tension.clamp(0.0, 1.0);
        let _ = self.controller.set_emotions(
            self.cached_params.arousal,
            self.cached_params.valence,
            self.cached_params.density,
            self.cached_params.tension,
        );
    }

    /// Set all emotion parameters at once
    pub fn set_params(&mut self, arousal: f32, valence: f32, density: f32, tension: f32) {
        self.cached_params.arousal = arousal.clamp(0.0, 1.0);
        self.cached_params.valence = valence.clamp(-1.0, 1.0);
        self.cached_params.density = density.clamp(0.0, 1.0);
        self.cached_params.tension = tension.clamp(0.0, 1.0);
        let _ = self.controller.set_emotions(
            self.cached_params.arousal,
            self.cached_params.valence,
            self.cached_params.density,
            self.cached_params.tension,
        );
    }

    // === Emotion Getters (cached UI-side values) ===

    pub fn get_target_arousal(&self) -> f32 {
        self.cached_params.arousal
    }

    pub fn get_target_valence(&self) -> f32 {
        self.cached_params.valence
    }

    pub fn get_target_density(&self) -> f32 {
        self.cached_params.density
    }

    pub fn get_target_tension(&self) -> f32 {
        self.cached_params.tension
    }

    pub fn get_computed_bpm(&self) -> f32 {
        self.cached_params.compute_bpm()
    }

    // === Rhythm Algorithm ===

    /// Set rhythm algorithm (0=Euclidean, 1=PerfectBalance, 2=ClassicGroove)
    pub fn set_algorithm(&mut self, algorithm: u8) {
        let mode = match algorithm {
            0 => RhythmMode::Euclidean,
            1 => RhythmMode::PerfectBalance,
            2 => RhythmMode::ClassicGroove,
            _ => RhythmMode::Euclidean,
        };
        let _ = self.controller.set_rhythm_mode(mode);
    }

    pub fn get_algorithm(&mut self) -> u8 {
        let _ = self.controller.poll_reports();
        match self.controller.get_state().map(|s| s.rhythm_mode) {
            Some(RhythmMode::Euclidean) => 0,
            Some(RhythmMode::PerfectBalance) => 1,
            Some(RhythmMode::ClassicGroove) => 2,
            None => 0,
        }
    }

    // === Harmony Mode ===

    /// Set harmony mode (0=Basic, 1=Driver)
    pub fn set_harmony_mode(&mut self, mode: u8) {
        let harmony_mode = match mode {
            0 => HarmonyMode::Basic,
            1 => HarmonyMode::Driver,
            _ => HarmonyMode::Driver,
        };
        let _ = self.controller.set_harmony_mode(harmony_mode);
    }

    pub fn get_harmony_mode(&mut self) -> u8 {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| match s.harmony_mode {
                HarmonyMode::Basic => 0,
                HarmonyMode::Driver => 1,
            })
            .unwrap_or(1)
    }

    // === Harmony State Getters (from engine reports) ===

    pub fn get_current_chord_name(&mut self) -> String {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| s.current_chord.to_string())
            .unwrap_or_else(|| "?".to_string())
    }

    pub fn get_current_chord_index(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.chord_root_offset as usize).unwrap_or(0)
    }

    pub fn is_current_chord_minor(&mut self) -> bool {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.chord_is_minor).unwrap_or(false)
    }

    pub fn get_current_measure(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.current_bar).unwrap_or(1)
    }

    pub fn get_current_cycle(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        // Cycle = bar / progression_length
        self.controller
            .get_state()
            .map(|s| {
                if s.progression_length > 0 { s.current_bar / s.progression_length + 1 } else { 1 }
            })
            .unwrap_or(1)
    }

    pub fn get_current_step(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.current_step).unwrap_or(0)
    }

    pub fn get_progression_name(&mut self) -> String {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| s.progression_name.to_string())
            .unwrap_or_else(|| "?".to_string())
    }

    pub fn get_progression_length(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.progression_length).unwrap_or(4)
    }

    // === Rhythm Visualization Getters ===

    pub fn get_primary_pulses(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_pulses).unwrap_or(4)
    }

    pub fn get_secondary_pulses(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_pulses).unwrap_or(3)
    }

    pub fn get_primary_rotation(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_rotation).unwrap_or(0)
    }

    pub fn get_secondary_rotation(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_rotation).unwrap_or(0)
    }

    pub fn get_primary_steps(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_steps).unwrap_or(16)
    }

    pub fn get_secondary_steps(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_steps).unwrap_or(12)
    }

    /// Get primary pattern as Vec<u8> (1=active, 0=silent) for WASM
    pub fn get_primary_pattern(&mut self) -> Vec<u8> {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| {
                let len = s.primary_steps.min(192);
                s.primary_pattern[..len].iter().map(|&b| if b { 1 } else { 0 }).collect()
            })
            .unwrap_or_else(|| vec![0; 16])
    }

    /// Get secondary pattern as Vec<u8>
    pub fn get_secondary_pattern(&mut self) -> Vec<u8> {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| {
                let len = s.secondary_steps.min(192);
                s.secondary_pattern[..len].iter().map(|&b| if b { 1 } else { 0 }).collect()
            })
            .unwrap_or_else(|| vec![0; 12])
    }

    /// Get newly generated measures as JSON array.
    ///
    /// Returns a JSON string like `[{index, tempo, time_sig_numerator, ...}, ...]`.
    /// Call this on each animation frame; the frontend should append the returned
    /// measures to its score cache for VexFlow rendering.
    /// Returns `"[]"` when no new measures are available.
    pub fn get_new_measures_json(&mut self) -> String {
        let measures = self.controller.poll_new_measures();
        serde_json::to_string(&measures).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get visualization events as flat array [note, channel, step, velocity, ...]
    pub fn get_events(&mut self) -> Vec<u32> {
        let mut result = Vec::new();
        let reports = self.controller.poll_reports();
        for report in &reports {
            for note in &report.notes {
                result.push(note.note_midi as u32);
                result.push(note.channel as u32);
                result.push(0u32); // step placeholder
                result.push(note.velocity as u32);
            }
        }
        result
    }

    // === Channel Routing & Muting ===

    /// Set channel routing (-1=FundSP, >=0=Bank ID)
    pub fn set_channel_routing(&mut self, channel: usize, mode: i32) {
        if channel < 16 {
            let _ = self.controller.send(harmonium_core::EngineCommand::SetChannelRoute {
                channel: channel as u8,
                bank_id: mode,
            });
        }
    }

    /// Set channel mute
    pub fn set_channel_muted(&mut self, channel: usize, is_muted: bool) {
        if channel < 16 {
            let _ = self.controller.set_channel_mute(channel as u8, is_muted);
        }
    }

    // === Mixer Controls ===

    pub fn set_gain_lead(&mut self, gain: f32) {
        self.cached_params.gain_lead = gain.clamp(0.0, 1.0);
        let _ = self.controller.set_channel_gain(1, self.cached_params.gain_lead);
    }

    pub fn set_gain_bass(&mut self, gain: f32) {
        self.cached_params.gain_bass = gain.clamp(0.0, 1.0);
        let _ = self.controller.set_channel_gain(0, self.cached_params.gain_bass);
    }

    pub fn set_gain_snare(&mut self, gain: f32) {
        self.cached_params.gain_snare = gain.clamp(0.0, 1.0);
        let _ = self.controller.set_channel_gain(2, self.cached_params.gain_snare);
    }

    pub fn set_gain_hat(&mut self, gain: f32) {
        self.cached_params.gain_hat = gain.clamp(0.0, 1.0);
        let _ = self.controller.set_channel_gain(3, self.cached_params.gain_hat);
    }

    pub fn set_vel_base_bass(&mut self, vel: u8) {
        self.cached_params.vel_base_bass = vel.min(127);
        let _ = self.controller.send(harmonium_core::EngineCommand::SetVelocityBase {
            channel: 0,
            velocity: self.cached_params.vel_base_bass,
        });
    }

    pub fn set_vel_base_snare(&mut self, vel: u8) {
        self.cached_params.vel_base_snare = vel.min(127);
        let _ = self.controller.send(harmonium_core::EngineCommand::SetVelocityBase {
            channel: 2,
            velocity: self.cached_params.vel_base_snare,
        });
    }

    pub fn get_gain_lead(&self) -> f32 {
        self.cached_params.gain_lead
    }

    pub fn get_gain_bass(&self) -> f32 {
        self.cached_params.gain_bass
    }

    pub fn get_gain_snare(&self) -> f32 {
        self.cached_params.gain_snare
    }

    pub fn get_gain_hat(&self) -> f32 {
        self.cached_params.gain_hat
    }

    pub fn get_vel_base_bass(&self) -> u8 {
        self.cached_params.vel_base_bass
    }

    pub fn get_vel_base_snare(&self) -> u8 {
        self.cached_params.vel_base_snare
    }

    /// Set polyrhythm steps (must be multiple of 4)
    pub fn set_poly_steps(&mut self, steps: usize) {
        let valid_steps = (steps / 4) * 4;
        self.cached_params.poly_steps = valid_steps.clamp(16, 384);
        let _ = self.controller.set_rhythm_steps(self.cached_params.poly_steps);
    }

    pub fn get_poly_steps(&self) -> usize {
        self.cached_params.poly_steps
    }

    /// Add a SoundFont to a specific bank
    pub fn add_soundfont(&self, bank_id: u32, sf2_bytes: Box<[u8]>) {
        if let Ok(mut queue) = self.font_queue.lock() {
            queue.push((bank_id, sf2_bytes.into_vec()));
        }
    }

    // === Playback Controls ===

    #[cfg(feature = "wasm")]
    pub fn resume(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(not(feature = "wasm"))]
    pub fn resume(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.play().map_err(|e| e.to_string())
    }

    #[cfg(feature = "wasm")]
    pub fn pause(&self) -> Result<(), JsValue> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(not(feature = "wasm"))]
    pub fn pause(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.pause().map_err(|e| e.to_string())
    }

    // === Recording ===

    pub fn start_recording_wav(&mut self) {
        let _ = self.controller.start_recording(events::RecordFormat::Wav);
    }

    pub fn stop_recording_wav(&mut self) {
        let _ = self.controller.stop_recording(events::RecordFormat::Wav);
    }

    pub fn start_recording_midi(&mut self) {
        let _ = self.controller.start_recording(events::RecordFormat::Midi);
    }

    pub fn stop_recording_midi(&mut self) {
        let _ = self.controller.stop_recording(events::RecordFormat::Midi);
    }

    pub fn start_recording_musicxml(&mut self) {
        let _ = self.controller.start_recording(events::RecordFormat::MusicXml);
    }

    pub fn stop_recording_musicxml(&mut self) {
        let _ = self.controller.stop_recording(events::RecordFormat::MusicXml);
    }

    pub fn pop_finished_recording(&self) -> Option<RecordedData> {
        if let Ok(mut queue) = self.finished_recordings.lock()
            && let Some((fmt, data)) = queue.pop()
        {
            let format_str = match fmt {
                events::RecordFormat::Wav => "wav".to_string(),
                events::RecordFormat::Midi => "midi".to_string(),
                events::RecordFormat::MusicXml => "musicxml".to_string(),
            };
            return Some(RecordedData { format_str, data });
        }
        None
    }

    // === Control Mode ===

    /// Switch to emotion mode (arousal/valence/density/tension sliders)
    pub fn use_emotion_mode(&mut self) {
        let _ = self.controller.use_emotion_mode();
    }

    /// Switch to direct technical control mode
    pub fn use_direct_mode(&mut self) {
        let _ = self.controller.use_direct_mode();
    }

    /// Returns true if in emotion mode
    pub fn is_emotion_mode(&self) -> bool {
        self.controller.get_mode() == harmonium_core::ControlMode::Emotion
    }

    // === Direct Mode Controls ===

    pub fn set_direct_bpm(&mut self, bpm: f32) {
        let _ = self.controller.set_bpm(bpm.clamp(30.0, 300.0));
    }

    pub fn get_direct_bpm(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.bpm).unwrap_or(120.0)
    }

    pub fn set_direct_enable_rhythm(&mut self, enabled: bool) {
        let _ = self.controller.enable_rhythm(enabled);
    }

    pub fn set_direct_enable_harmony(&mut self, enabled: bool) {
        let _ = self.controller.enable_harmony(enabled);
    }

    pub fn set_direct_enable_melody(&mut self, enabled: bool) {
        let _ = self.controller.enable_melody(enabled);
    }

    pub fn set_direct_enable_voicing(&mut self, enabled: bool) {
        let _ = self.controller.enable_voicing(enabled);
    }

    pub fn set_direct_fixed_kick(&mut self, enabled: bool) {
        self.cached_params.fixed_kick = enabled;
        let _ = self.controller.send(harmonium_core::EngineCommand::SetFixedKick(enabled));
    }

    pub fn get_direct_fixed_kick(&self) -> bool {
        self.cached_params.fixed_kick
    }

    /// Set all rhythm parameters at once
    #[allow(clippy::too_many_arguments)]
    pub fn set_all_rhythm_params(
        &mut self,
        mode: u8,
        steps: usize,
        pulses: usize,
        rotation: usize,
        density: f32,
        tension: f32,
        secondary_steps: usize,
        secondary_pulses: usize,
        secondary_rotation: usize,
    ) {
        let rhythm_mode = match mode {
            0 => RhythmMode::Euclidean,
            1 => RhythmMode::PerfectBalance,
            2 => RhythmMode::ClassicGroove,
            _ => RhythmMode::Euclidean,
        };
        let valid_steps = (steps / 4) * 4;
        let _ = self.controller.send(harmonium_core::EngineCommand::SetAllRhythmParams {
            mode: rhythm_mode,
            steps: valid_steps.clamp(16, 384),
            pulses: pulses.clamp(1, 32),
            rotation,
            density: density.clamp(0.0, 1.0),
            tension: tension.clamp(0.0, 1.0),
            secondary_steps: secondary_steps.clamp(4, 32),
            secondary_pulses: secondary_pulses.clamp(1, 32),
            secondary_rotation,
        });
    }

    pub fn set_direct_rhythm_mode(&mut self, mode: u8) {
        let rhythm_mode = match mode {
            0 => RhythmMode::Euclidean,
            1 => RhythmMode::PerfectBalance,
            2 => RhythmMode::ClassicGroove,
            _ => RhythmMode::Euclidean,
        };
        let _ = self.controller.set_rhythm_mode(rhythm_mode);
    }

    pub fn set_direct_rhythm_steps(&mut self, steps: usize) {
        let valid_steps = (steps / 4) * 4;
        let _ = self.controller.set_rhythm_steps(valid_steps.clamp(16, 384));
    }

    pub fn set_direct_rhythm_pulses(&mut self, pulses: usize) {
        let _ = self.controller.set_rhythm_pulses(pulses.clamp(1, 32));
    }

    pub fn set_direct_rhythm_rotation(&mut self, rotation: usize) {
        let _ = self.controller.set_rhythm_rotation(rotation);
    }

    pub fn set_direct_rhythm_density(&mut self, density: f32) {
        let _ = self.controller.set_rhythm_density(density.clamp(0.0, 1.0));
    }

    pub fn set_direct_rhythm_tension(&mut self, tension: f32) {
        let _ = self.controller.set_rhythm_tension(tension.clamp(0.0, 1.0));
    }

    pub fn set_direct_secondary_steps(&mut self, steps: usize) {
        let _ = self.controller.poll_reports();
        let cur_pulses = self.controller.get_state().map(|s| s.secondary_pulses).unwrap_or(3);
        let cur_rotation = self.controller.get_state().map(|s| s.secondary_rotation).unwrap_or(0);
        let _ = self.controller.send(harmonium_core::EngineCommand::SetRhythmSecondary {
            steps: steps.clamp(4, 32),
            pulses: cur_pulses,
            rotation: cur_rotation,
        });
    }

    pub fn set_direct_secondary_pulses(&mut self, pulses: usize) {
        let _ = self.controller.poll_reports();
        let cur_steps = self.controller.get_state().map(|s| s.secondary_steps).unwrap_or(12);
        let cur_rotation = self.controller.get_state().map(|s| s.secondary_rotation).unwrap_or(0);
        let _ = self.controller.send(harmonium_core::EngineCommand::SetRhythmSecondary {
            steps: cur_steps,
            pulses: pulses.clamp(1, 32),
            rotation: cur_rotation,
        });
    }

    pub fn set_direct_secondary_rotation(&mut self, rotation: usize) {
        let _ = self.controller.poll_reports();
        let cur_steps = self.controller.get_state().map(|s| s.secondary_steps).unwrap_or(12);
        let cur_pulses = self.controller.get_state().map(|s| s.secondary_pulses).unwrap_or(3);
        let _ = self.controller.send(harmonium_core::EngineCommand::SetRhythmSecondary {
            steps: cur_steps,
            pulses: cur_pulses,
            rotation,
        });
    }

    pub fn set_direct_harmony_mode(&mut self, mode: u8) {
        let harmony_mode = match mode {
            0 => HarmonyMode::Basic,
            1 => HarmonyMode::Driver,
            _ => HarmonyMode::Driver,
        };
        let _ = self.controller.set_harmony_mode(harmony_mode);
    }

    pub fn set_direct_harmony_tension(&mut self, tension: f32) {
        let _ = self.controller.set_harmony_tension(tension.clamp(0.0, 1.0));
    }

    pub fn set_direct_harmony_valence(&mut self, valence: f32) {
        let _ = self.controller.set_harmony_valence(valence.clamp(-1.0, 1.0));
    }

    pub fn set_direct_melody_smoothness(&mut self, smoothness: f32) {
        let _ = self.controller.set_melody_smoothness(smoothness.clamp(0.0, 1.0));
    }

    pub fn set_direct_voicing_density(&mut self, density: f32) {
        let _ = self.controller.set_voicing_density(density.clamp(0.0, 1.0));
    }

    pub fn set_direct_voicing_tension(&mut self, tension: f32) {
        let _ = self
            .controller
            .send(harmonium_core::EngineCommand::SetVoicingTension(tension.clamp(0.0, 1.0)));
    }

    pub fn get_direct_params_json(&mut self) -> String {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| serde_json::to_string(&s.musical_params).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string())
    }

    pub fn set_direct_params_json(&mut self, json: &str) {
        if let Ok(params) = serde_json::from_str::<MusicalParams>(json) {
            let _ = self.controller.set_bpm(params.bpm);
            let _ = self.controller.set_rhythm_mode(params.rhythm_mode);
            let _ = self.controller.set_rhythm_density(params.rhythm_density);
            let _ = self.controller.set_harmony_tension(params.harmony_tension);
            let _ = self.controller.set_harmony_valence(params.harmony_valence);
            let _ = self.controller.set_melody_smoothness(params.melody_smoothness);
        }
    }

    // === Direct Mode Getters (from engine reports) ===

    pub fn get_direct_enable_rhythm(&mut self) -> bool {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.enable_rhythm).unwrap_or(true)
    }

    pub fn get_direct_enable_harmony(&mut self) -> bool {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.enable_harmony).unwrap_or(true)
    }

    pub fn get_direct_enable_melody(&mut self) -> bool {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.enable_melody).unwrap_or(true)
    }

    pub fn get_direct_enable_voicing(&mut self) -> bool {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.enable_voicing).unwrap_or(false)
    }

    pub fn get_direct_rhythm_mode(&mut self) -> u8 {
        let _ = self.controller.poll_reports();
        self.controller
            .get_state()
            .map(|s| match s.rhythm_mode {
                RhythmMode::Euclidean => 0,
                RhythmMode::PerfectBalance => 1,
                RhythmMode::ClassicGroove => 2,
            })
            .unwrap_or(0)
    }

    pub fn get_direct_rhythm_steps(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_steps).unwrap_or(16)
    }

    pub fn get_direct_rhythm_pulses(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_pulses).unwrap_or(4)
    }

    pub fn get_direct_rhythm_rotation(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.primary_rotation).unwrap_or(0)
    }

    pub fn get_direct_rhythm_density(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.rhythm_density).unwrap_or(0.5)
    }

    pub fn get_direct_rhythm_tension(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.rhythm_tension).unwrap_or(0.3)
    }

    pub fn get_direct_secondary_steps(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_steps).unwrap_or(12)
    }

    pub fn get_direct_secondary_pulses(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_pulses).unwrap_or(3)
    }

    pub fn get_direct_secondary_rotation(&mut self) -> usize {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.secondary_rotation).unwrap_or(0)
    }

    pub fn get_direct_harmony_tension(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.harmony_tension).unwrap_or(0.3)
    }

    pub fn get_direct_harmony_valence(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.harmony_valence).unwrap_or(0.3)
    }

    pub fn get_direct_melody_smoothness(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.melody_smoothness).unwrap_or(0.7)
    }

    pub fn get_direct_voicing_density(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.voicing_density).unwrap_or(0.5)
    }

    pub fn get_direct_voicing_tension(&mut self) -> f32 {
        let _ = self.controller.poll_reports();
        self.controller.get_state().map(|s| s.musical_params.voicing_tension).unwrap_or(0.3)
    }
}

#[cfg(all(feature = "standalone", feature = "wasm"))]
#[wasm_bindgen]
pub fn start(sf2_bytes: Option<Box<[u8]>>) -> Result<Handle, JsValue> {
    start_with_backend(sf2_bytes, "fundsp")
}

/// Start Harmonium with a specific audio backend
/// backend: "fundsp" (default) or "odin2" (if compiled with odin2 feature)
#[cfg(all(feature = "standalone", feature = "wasm"))]
#[wasm_bindgen]
pub fn start_with_backend(sf2_bytes: Option<Box<[u8]>>, backend: &str) -> Result<Handle, JsValue> {
    console_error_panic_hook::set_once();

    // Parse backend type
    let backend_type = match backend.to_lowercase().as_str() {
        "fundsp" | "synth" | "default" => audio::AudioBackendType::FundSP,
        #[cfg(feature = "odin2")]
        "odin2" | "odin" => audio::AudioBackendType::Odin2,
        _ => {
            log::warn(&format!("Unknown backend '{}', using FundSP", backend));
            audio::AudioBackendType::FundSP
        }
    };

    let (stream, config, controller, font_queue, finished_recordings) =
        audio::create_timeline_stream_legacy(sf2_bytes.as_deref(), backend_type)
            .map_err(|e| JsValue::from_str(&e))?;

    Ok(Handle {
        stream,
        controller,
        font_queue,
        finished_recordings,
        cached_params: harmonium_core::EngineParams::default(),
        bpm: config.bpm,
        key: config.key,
        scale: config.scale,
        pulses: config.pulses,
        steps: config.steps,
    })
}

/// Get list of available audio backends
#[cfg(all(feature = "standalone", feature = "wasm"))]
#[wasm_bindgen]
pub fn get_available_backends() -> Vec<JsValue> {
    let mut backends = vec![JsValue::from_str("fundsp")];
    #[cfg(feature = "odin2")]
    backends.push(JsValue::from_str("odin2"));
    backends
}
