//! Harmonium VST3/CLAP Plugin
//!
//! This module wraps the HarmoniumEngine as a MIDI generator plugin.
//! It outputs MIDI notes that can drive any synth in the DAW.

use nih_plug::prelude::*;

// General MIDI Drum Map (Standard)
const GM_KICK: u8 = 36; // C1 - Bass Drum
const GM_SNARE: u8 = 38; // D1 - Snare
const GM_HIHAT_CLOSED: u8 = 42; // F#1 - Closed Hi-Hat
use std::sync::Arc;

use harmonium_audio::backend::vst_midi_backend::VstMidiBackend;
use harmonium_core::{
    HarmoniumController, EngineParams,
    harmony::HarmonyMode,
    params::{ControlMode, HarmonyState},
    sequencer::RhythmMode,
};

use crate::timeline_engine::TimelineEngine;

/// Snapshot of last-synced parameter values for dirty-tracking
#[derive(Default)]
#[allow(dead_code)]
struct ParamSnapshot {
    control_mode: bool,
    arousal: f32,
    valence: f32,
    density: f32,
    tension: f32,
    smoothness: f32,
    bpm: f32,
    rhythm_mode: bool,
    rhythm_steps: i32,
    rhythm_pulses: i32,
    rhythm_rotation: i32,
    harmony_mode: bool,
    enable_rhythm: bool,
    enable_harmony: bool,
    enable_melody: bool,
    enable_voicing: bool,
    mute_bass: bool,
    mute_lead: bool,
    mute_snare: bool,
    mute_hat: bool,
}

/// Main Harmonium VST Plugin
pub struct HarmoniumPlugin {
    params: Arc<HarmoniumParams>,
    engine: Option<TimelineEngine>,
    controller: Option<harmonium_core::HarmoniumController>,
    /// Target state for emotional mapping (shared with GUI)
    target_state: Arc<std::sync::Mutex<EngineParams>>,
    /// Control mode and live state (shared with GUI)
    control_mode_state: Arc<std::sync::Mutex<ControlMode>>,
    /// Harmony state receiver (shared with GUI)
    harmony_state_rx: Arc<std::sync::Mutex<Option<rtrb::Consumer<HarmonyState>>>>,
    sample_rate: f32,
    last_synced: ParamSnapshot,
}

/// Plugin parameters exposed to the DAW
#[derive(Params)]
pub struct HarmoniumParams {
    // ═══════════════════════════════════════════════════════════════════
    // MODE SELECTION
    // ═══════════════════════════════════════════════════════════════════
    /// Control mode: Emotional (mapped) or Technical (direct)
    #[id = "control_mode"]
    pub control_mode: BoolParam,

    // ═══════════════════════════════════════════════════════════════════
    // EMOTIONAL PARAMETERS (when control_mode = true)
    // ═══════════════════════════════════════════════════════════════════
    /// Arousal (0-1) - Energy level, affects BPM
    #[id = "arousal"]
    pub arousal: FloatParam,

    /// Valence (-1 to 1) - Positive/Negative emotion, affects harmony
    #[id = "valence"]
    pub valence: FloatParam,

    /// Density (0-1) - Rhythmic complexity
    #[id = "density"]
    pub density: FloatParam,

    /// Tension (0-1) - Harmonic dissonance
    #[id = "tension"]
    pub tension: FloatParam,

    /// Smoothness (0-1) - Melodic smoothness (Hurst factor)
    #[id = "smoothness"]
    pub smoothness: FloatParam,

    // ═══════════════════════════════════════════════════════════════════
    // TECHNICAL PARAMETERS (when control_mode = false)
    // ═══════════════════════════════════════════════════════════════════
    /// BPM (70-180)
    #[id = "bpm"]
    pub bpm: FloatParam,

    /// Rhythm Mode: Euclidean or PerfectBalance
    #[id = "rhythm_mode"]
    pub rhythm_mode: BoolParam,

    /// Rhythm Steps (4-192)
    #[id = "rhythm_steps"]
    pub rhythm_steps: IntParam,

    /// Rhythm Pulses (1-16)
    #[id = "rhythm_pulses"]
    pub rhythm_pulses: IntParam,

    /// Rhythm Rotation (0-15)
    #[id = "rhythm_rotation"]
    pub rhythm_rotation: IntParam,

    /// Harmony Mode: Basic or Driver
    #[id = "harmony_mode"]
    pub harmony_mode: BoolParam,

    // ═══════════════════════════════════════════════════════════════════
    // MODULE TOGGLES
    // ═══════════════════════════════════════════════════════════════════
    /// Enable Rhythm Module
    #[id = "enable_rhythm"]
    pub enable_rhythm: BoolParam,

    /// Enable Harmony Module
    #[id = "enable_harmony"]
    pub enable_harmony: BoolParam,

    /// Enable Melody Module
    #[id = "enable_melody"]
    pub enable_melody: BoolParam,

    /// Enable Voicing (harmonized chords on melody)
    #[id = "enable_voicing"]
    pub enable_voicing: BoolParam,

    // ═══════════════════════════════════════════════════════════════════
    // CHANNEL MUTES
    // ═══════════════════════════════════════════════════════════════════
    /// Mute Bass/Kick (Channel 0)
    #[id = "mute_bass"]
    pub mute_bass: BoolParam,

    /// Mute Lead/Melody (Channel 1)
    #[id = "mute_lead"]
    pub mute_lead: BoolParam,

    /// Mute Snare (Channel 2)
    #[id = "mute_snare"]
    pub mute_snare: BoolParam,

    /// Mute Hi-Hat (Channel 3)
    #[id = "mute_hat"]
    pub mute_hat: BoolParam,
}

impl Default for HarmoniumParams {
    fn default() -> Self {
        Self {
            // Control Mode
            control_mode: BoolParam::new("Emotion Mode", true).with_value_to_string(Arc::new(
                |v| {
                    if v { "Emotional".to_string() } else { "Technical".to_string() }
                },
            )),

            // Emotional Parameters
            arousal: FloatParam::new("Arousal", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Energy")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            valence: FloatParam::new("Valence", 0.3, FloatRange::Linear { min: -1.0, max: 1.0 })
                .with_unit(" Mood")
                .with_value_to_string(Arc::new(|v| {
                    if v > 0.3 {
                        format!("{:.0}% Happy", v * 100.0)
                    } else if v < -0.3 {
                        format!("{:.0}% Sad", v.abs() * 100.0)
                    } else {
                        "Neutral".to_string()
                    }
                })),

            density: FloatParam::new("Density", 0.4, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Complexity")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            tension: FloatParam::new("Tension", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Dissonance")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            smoothness: FloatParam::new(
                "Smoothness",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" Flow")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),

            // Technical Parameters
            bpm: FloatParam::new("BPM", 120.0, FloatRange::Linear { min: 70.0, max: 180.0 })
                .with_unit(" BPM")
                .with_value_to_string(formatters::v2s_f32_rounded(0)),

            rhythm_mode: BoolParam::new("Rhythm Mode", false).with_value_to_string(Arc::new(|v| {
                if v { "PerfectBalance".to_string() } else { "Euclidean".to_string() }
            })),

            rhythm_steps: IntParam::new("Steps", 16, IntRange::Linear { min: 4, max: 192 }),

            rhythm_pulses: IntParam::new("Pulses", 4, IntRange::Linear { min: 1, max: 16 }),

            rhythm_rotation: IntParam::new("Rotation", 0, IntRange::Linear { min: 0, max: 15 }),

            harmony_mode: BoolParam::new("Harmony Mode", true).with_value_to_string(Arc::new(
                |v| {
                    if v { "Driver (Advanced)".to_string() } else { "Basic".to_string() }
                },
            )),

            // Module Toggles
            enable_rhythm: BoolParam::new("Enable Rhythm", true),
            enable_harmony: BoolParam::new("Enable Harmony", true),
            enable_melody: BoolParam::new("Enable Melody", true),
            enable_voicing: BoolParam::new("Enable Voicing", false),

            // Channel Mutes
            mute_bass: BoolParam::new("Mute Bass", false),
            mute_lead: BoolParam::new("Mute Lead", false),
            mute_snare: BoolParam::new("Mute Snare", false),
            mute_hat: BoolParam::new("Mute Hat", false),
        }
    }
}

impl Default for HarmoniumPlugin {
    fn default() -> Self {
        let params = Arc::new(HarmoniumParams::default());

        Self {
            params,
            engine: None,
            controller: None,
            target_state: Arc::new(std::sync::Mutex::new(EngineParams::default())),
            control_mode_state: Arc::new(std::sync::Mutex::new(ControlMode::default())),
            harmony_state_rx: Arc::new(std::sync::Mutex::new(None)),
            sample_rate: 44100.0,
            last_synced: ParamSnapshot::default(),
        }
    }
}

impl HarmoniumPlugin {
    /// Map note to GM drum standard for drum channels
    /// Channel 0 (Kick) → GM_KICK (36)
    /// Channel 1 (Melody) → Keep original note (musical pitch)
    /// Channel 2 (Snare) → GM_SNARE (38)
    /// Channel 3 (Hi-Hat) → GM_HIHAT_CLOSED (42)
    fn map_note_for_channel(channel: u8, original_note: u8) -> u8 {
        match channel {
            0 => GM_KICK,         // Bass/Kick → C1
            1 => original_note,   // Melody → Keep musical note
            2 => GM_SNARE,        // Snare → D1
            3 => GM_HIHAT_CLOSED, // Hi-Hat → F#1
            _ => original_note,   // Unknown → Keep original
        }
    }

    /// Sync plugin parameters to engine state (only sends changed values)
    fn sync_params_to_engine(&mut self) {
        use harmonium_core::EngineCommand;

        if let Some(ref mut controller) = self.controller {
            let snap = &mut self.last_synced;

            // Control mode
            let emotion_mode = self.params.control_mode.value();
            if emotion_mode != snap.control_mode {
                snap.control_mode = emotion_mode;
                let _ = if emotion_mode {
                    controller.send(EngineCommand::UseEmotionMode)
                } else {
                    controller.send(EngineCommand::UseDirectMode)
                };
            }

            // Module enables
            macro_rules! sync_bool {
                ($param:ident, $cmd:expr) => {
                    let v = self.params.$param.value();
                    if v != snap.$param {
                        snap.$param = v;
                        let _ = controller.send($cmd(v));
                    }
                };
            }
            sync_bool!(enable_rhythm, EngineCommand::EnableRhythm);
            sync_bool!(enable_harmony, EngineCommand::EnableHarmony);
            sync_bool!(enable_melody, EngineCommand::EnableMelody);
            sync_bool!(enable_voicing, EngineCommand::EnableVoicing);

            // Channel mutes
            macro_rules! sync_mute {
                ($param:ident, $ch:expr) => {
                    let v = self.params.$param.value();
                    if v != snap.$param {
                        snap.$param = v;
                        let _ = controller.send(EngineCommand::SetChannelMute {
                            channel: $ch,
                            muted: v,
                        });
                    }
                };
            }
            sync_mute!(mute_bass, 0);
            sync_mute!(mute_lead, 1);
            sync_mute!(mute_snare, 2);
            sync_mute!(mute_hat, 3);

            if emotion_mode {
                let arousal = self.params.arousal.value();
                let valence = self.params.valence.value();
                let density = self.params.density.value();
                let tension = self.params.tension.value();
                if arousal != snap.arousal || valence != snap.valence
                    || density != snap.density || tension != snap.tension
                {
                    snap.arousal = arousal;
                    snap.valence = valence;
                    snap.density = density;
                    snap.tension = tension;
                    let _ = controller.send(EngineCommand::SetEmotionParams {
                        arousal, valence, density, tension,
                    });
                }
            } else {
                let bpm = self.params.bpm.value();
                if bpm != snap.bpm {
                    snap.bpm = bpm;
                    let _ = controller.send(EngineCommand::SetBpm(bpm));
                }

                let rm = self.params.rhythm_mode.value();
                if rm != snap.rhythm_mode {
                    snap.rhythm_mode = rm;
                    let rhythm_mode = if rm { RhythmMode::PerfectBalance } else { RhythmMode::Euclidean };
                    let _ = controller.send(EngineCommand::SetRhythmMode(rhythm_mode));
                }

                let steps = self.params.rhythm_steps.value();
                if steps != snap.rhythm_steps {
                    snap.rhythm_steps = steps;
                    let _ = controller.send(EngineCommand::SetRhythmSteps(steps as usize));
                }

                let pulses = self.params.rhythm_pulses.value();
                if pulses != snap.rhythm_pulses {
                    snap.rhythm_pulses = pulses;
                    let _ = controller.send(EngineCommand::SetRhythmPulses(pulses as usize));
                }

                let rot = self.params.rhythm_rotation.value();
                if rot != snap.rhythm_rotation {
                    snap.rhythm_rotation = rot;
                    let _ = controller.send(EngineCommand::SetRhythmRotation(rot as usize));
                }

                let hm = self.params.harmony_mode.value();
                if hm != snap.harmony_mode {
                    snap.harmony_mode = hm;
                    let harmony_mode = if hm { HarmonyMode::Driver } else { HarmonyMode::Basic };
                    let _ = controller.send(EngineCommand::SetHarmonyMode(harmony_mode));
                }
            }
        }
    }
}

impl Plugin for HarmoniumPlugin {
    const NAME: &'static str = "Harmonium";
    const VENDOR: &'static str = "BasCanada";
    const URL: &'static str = "https://github.com/bascanada/harmonium";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Enable GUI with vst-gui feature
    #[cfg(feature = "vst-gui")]
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        // MIDI Generator - no audio I/O needed, but we need at least one layout
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    // MIDI Configuration
    const MIDI_INPUT: MidiConfig = MidiConfig::None; // No MIDI input needed
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic; // We output MIDI!

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        // Create backend for the engine
        let engine_backend = Box::new(VstMidiBackend::new());

        // Create command/report queues for engine communication
        let (command_tx, command_rx) = rtrb::RingBuffer::<harmonium_core::EngineCommand>::new(1024);
        let (report_tx, report_rx) = rtrb::RingBuffer::<harmonium_core::EngineReport>::new(256);

        // Create engine with timeline architecture
        let engine =
            TimelineEngine::new(self.sample_rate as f64, command_rx, report_tx, engine_backend);

        self.engine = Some(engine);

        // Create controller for parameter updates
        self.controller = Some(HarmoniumController::new(command_tx, report_rx));

        true
    }

    fn reset(&mut self) {
        // Reset param snapshot so all params re-sync on next process()
        self.last_synced = ParamSnapshot::default();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Sync parameters from DAW to engine
        self.sync_params_to_engine();

        // Get the engine (should always exist after initialize)
        let engine = match &mut self.engine {
            Some(e) => e,
            None => {
                nih_log!("[PROCESS] Engine is None!");
                return ProcessStatus::Normal;
            }
        };

        // Process the audio buffer through the engine
        // This will trigger tick() which generates MIDI events
        let num_samples = buffer.samples();

        // Safety: ensure we have a valid buffer size
        if num_samples == 0 {
            return ProcessStatus::Normal;
        }

        // Use stereo for internal processing (engine expects interleaved stereo)
        let internal_channels = 2;
        let mut temp_buffer = vec![0.0f32; num_samples * internal_channels];
        engine.process_buffer(&mut temp_buffer, internal_channels);

        // Copy synthesized audio from temp_buffer back to VST output buffer
        // Note: nih-plug handles the mapping to channel buffers
        let mut samples_iter = temp_buffer.chunks_exact(internal_channels);
        for i in 0..num_samples {
            if let Some(frame) = samples_iter.next() {
                for channel in 0..buffer.channels().min(internal_channels) {
                    buffer.as_slice()[channel][i] = frame[channel];
                }
            }
        }

        // Poll for reports from engine and convert to MIDI events
        if let Some(ref mut controller) = self.controller {
            let reports = controller.poll_reports();
            for report in reports {
                // Use sample offset from report for accurate timing
                // Clamp to valid range (must be < num_samples)
                let timing = report.sample_offset.min((num_samples - 1) as u32);

                for note_event in &report.notes {
                    // Channel mapping: 0=Bass/Kick, 1=Lead/Melody, 2=Snare, 3=Hat
                    // Drum channels (0,2,3) are remapped to GM standard notes
                    // Melody channel (1) keeps original musical notes
                    let midi_note =
                        Self::map_note_for_channel(note_event.channel, note_event.note_midi);

                    if note_event.is_note_on {
                        context.send_event(NoteEvent::NoteOn {
                            timing,
                            voice_id: None,
                            channel: note_event.channel,
                            note: midi_note,
                            velocity: note_event.velocity as f32 / 127.0,
                        });
                    } else {
                        context.send_event(NoteEvent::NoteOff {
                            timing,
                            voice_id: None,
                            channel: note_event.channel,
                            note: midi_note,
                            velocity: 0.0,
                        });
                    }
                }
            }
        }

        ProcessStatus::Normal
    }

    /// Create the GUI editor (when vst-gui feature is enabled)
    #[cfg(feature = "vst-gui")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        crate::vst_gui::create_editor(
            self.target_state.clone(),
            self.control_mode_state.clone(),
            self.params.clone(),
            self.harmony_state_rx.clone(),
        )
    }
}

impl ClapPlugin for HarmoniumPlugin {
    const CLAP_ID: &'static str = "com.bascanada.harmonium";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("AI-powered generative music MIDI plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/bascanada/harmonium");
    const CLAP_SUPPORT_URL: Option<&'static str> =
        Some("https://github.com/bascanada/harmonium/issues");
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}

impl Vst3Plugin for HarmoniumPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"HarmoniumMIDIGen";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Generator];
}

// Export the plugin
nih_export_clap!(HarmoniumPlugin);
nih_export_vst3!(HarmoniumPlugin);

#[cfg(feature = "standalone")]
nih_export_standalone!(HarmoniumPlugin);
