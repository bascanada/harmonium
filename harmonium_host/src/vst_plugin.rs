//! Harmonium VST3/CLAP Plugin
//!
//! This module wraps the HarmoniumEngine as a MIDI generator plugin.
//! It outputs MIDI notes that can drive any synth in the DAW.

use nih_plug::prelude::*;

// General MIDI Drum Map (Standard)
const GM_KICK: u8 = 36;      // C1 - Bass Drum
const GM_SNARE: u8 = 38;     // D1 - Snare
const GM_HIHAT_CLOSED: u8 = 42; // F#1 - Closed Hi-Hat
use std::sync::{Arc, Mutex};

use crate::engine::{HarmoniumEngine, EngineParams};
use harmonium_audio::backend::vst_midi_backend::VstMidiBackend;
use harmonium_core::params::ControlMode;
use harmonium_core::sequencer::RhythmMode;
use harmonium_core::harmony::HarmonyMode;

/// Main Harmonium VST Plugin
pub struct HarmoniumPlugin {
    params: Arc<HarmoniumParams>,
    engine: Option<HarmoniumEngine>,
    // Phase 2: Lock-free consumers for Audio→UI communication
    harmony_state_rx_for_editor: Arc<Mutex<Option<rtrb::Consumer<crate::engine::HarmonyState>>>>,
    event_queue_rx: Option<rtrb::Consumer<crate::engine::VisualizationEvent>>,
    midi_backend: Arc<Mutex<VstMidiBackend>>,
    // Phase 3: Lock-free triple buffer for UI→Audio parameter updates
    target_params_input: triple_buffer::Input<EngineParams>,
    target_params_output: Option<triple_buffer::Output<EngineParams>>,
    // Phase 3: Webview-accessible state (synced from triple buffer)
    target_state: Arc<Mutex<EngineParams>>,
    control_mode: Arc<Mutex<ControlMode>>,
    sample_rate: f32,
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
            control_mode: BoolParam::new("Emotion Mode", true)
                .with_value_to_string(Arc::new(|v| {
                    if v { "Emotional".to_string() } else { "Technical".to_string() }
                })),

            // Emotional Parameters
            arousal: FloatParam::new("Arousal", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Energy")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            valence: FloatParam::new("Valence", 0.3, FloatRange::Linear { min: -1.0, max: 1.0 })
                .with_unit(" Mood")
                .with_value_to_string(Arc::new(|v| {
                    if v > 0.3 { format!("{:.0}% Happy", v * 100.0) }
                    else if v < -0.3 { format!("{:.0}% Sad", v.abs() * 100.0) }
                    else { "Neutral".to_string() }
                })),

            density: FloatParam::new("Density", 0.4, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Complexity")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            tension: FloatParam::new("Tension", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Dissonance")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            smoothness: FloatParam::new("Smoothness", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" Flow")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),

            // Technical Parameters
            bpm: FloatParam::new("BPM", 120.0, FloatRange::Linear { min: 70.0, max: 180.0 })
                .with_unit(" BPM")
                .with_value_to_string(formatters::v2s_f32_rounded(0)),

            rhythm_mode: BoolParam::new("Rhythm Mode", false)
                .with_value_to_string(Arc::new(|v| {
                    if v { "PerfectBalance".to_string() } else { "Euclidean".to_string() }
                })),

            rhythm_steps: IntParam::new("Steps", 16, IntRange::Linear { min: 4, max: 192 }),

            rhythm_pulses: IntParam::new("Pulses", 4, IntRange::Linear { min: 1, max: 16 }),

            rhythm_rotation: IntParam::new("Rotation", 0, IntRange::Linear { min: 0, max: 15 }),

            harmony_mode: BoolParam::new("Harmony Mode", true)
                .with_value_to_string(Arc::new(|v| {
                    if v { "Driver (Advanced)".to_string() } else { "Basic".to_string() }
                })),

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
        // Phase 3: Create triple buffer for lock-free UI→Audio parameter updates
        let (target_params_input, target_params_output) = triple_buffer::triple_buffer(&EngineParams::default());
        let control_mode = Arc::new(Mutex::new(ControlMode::default()));
        let midi_backend = Arc::new(Mutex::new(VstMidiBackend::new()));
        let target_state = Arc::new(Mutex::new(EngineParams::default()));
        let harmony_state_rx_for_editor = Arc::new(Mutex::new(None));

        Self {
            params,
            engine: None,
            harmony_state_rx_for_editor,  // Phase 2: Initialized in initialize()
            event_queue_rx: None,    // Phase 2: Initialized in initialize()
            midi_backend,
            target_params_input,
            target_params_output: Some(target_params_output),  // Phase 3: Stored until activate()
            target_state,
            control_mode,
            sample_rate: 44100.0,
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
            0 => GM_KICK,           // Bass/Kick → C1
            1 => original_note,      // Melody → Keep musical note
            2 => GM_SNARE,          // Snare → D1
            3 => GM_HIHAT_CLOSED,   // Hi-Hat → F#1
            _ => original_note,      // Unknown → Keep original
        }
    }

    /// Sync plugin parameters to engine state
    fn sync_params_to_engine(&mut self) {
        let daw_emotion_mode = self.params.control_mode.value();

        // Check if webview is controlling
        let (webview_controls_emotions, webview_controls_mode, webview_controls_direct) =
            if let Ok(mode) = self.control_mode.lock() {
                (mode.webview_controls_emotions, mode.webview_controls_mode, mode.webview_controls_direct)
            } else {
                (false, false, false)
            };

        // Phase 3: Sync webview changes from target_state to triple buffer
        // When webview controls emotions, copy target_state to triple buffer input
        if webview_controls_emotions {
            if let Ok(state) = self.target_state.lock() {
                // Clone current state and update with webview's emotional params
                let mut params = self.target_params_input.input_buffer_mut().clone();
                params.arousal = state.arousal;
                params.valence = state.valence;
                params.density = state.density;
                params.tension = state.tension;
                // Publish to engine via triple buffer
                self.target_params_input.write(params);
            }
        }

        // Get actual emotion mode (webview overrides DAW if controlling)
        let use_emotion_mode = if webview_controls_mode {
            if let Ok(mode) = self.control_mode.lock() {
                mode.use_emotion_mode
            } else {
                daw_emotion_mode
            }
        } else {
            daw_emotion_mode
        };

        // Update control mode (only update use_emotion_mode if webview isn't controlling it)
        if let Ok(mut mode) = self.control_mode.lock() {
            if !webview_controls_mode {
                mode.use_emotion_mode = daw_emotion_mode;
            }

            // Global enable overrides - these work in BOTH modes (unless webview is controlling)
            // Must be outside the direct-mode block!
            if !webview_controls_direct {
                mode.enable_rhythm = self.params.enable_rhythm.value();
                mode.enable_harmony = self.params.enable_harmony.value();
                mode.enable_melody = self.params.enable_melody.value();
                mode.enable_voicing = self.params.enable_voicing.value();
            }

            // Only sync direct params from DAW if webview is NOT controlling them
            if !use_emotion_mode && !webview_controls_direct {
                // Technical mode - update direct params
                mode.direct_params.bpm = self.params.bpm.value();
                mode.direct_params.rhythm_mode = if self.params.rhythm_mode.value() {
                    RhythmMode::PerfectBalance
                } else {
                    RhythmMode::Euclidean
                };
                mode.direct_params.rhythm_steps = self.params.rhythm_steps.value() as usize;
                mode.direct_params.rhythm_pulses = self.params.rhythm_pulses.value() as usize;
                mode.direct_params.rhythm_rotation = self.params.rhythm_rotation.value() as usize;
                mode.direct_params.harmony_mode = if self.params.harmony_mode.value() {
                    HarmonyMode::Driver
                } else {
                    HarmonyMode::Basic
                };
                mode.direct_params.muted_channels = vec![
                    self.params.mute_bass.value(),
                    self.params.mute_lead.value(),
                    self.params.mute_snare.value(),
                    self.params.mute_hat.value(),
                ];
            }
        }

        // Only sync emotional params from DAW if webview is NOT controlling
        if use_emotion_mode && !webview_controls_emotions {
            // Emotional mode - update target state from DAW params
            // Phase 3: Use triple buffer write instead of lock
            let mut params = self.target_params_input.input_buffer_mut().clone();
            params.arousal = self.params.arousal.value();
            params.valence = self.params.valence.value();
            params.density = self.params.density.value();
            params.tension = self.params.tension.value();
            params.smoothness = self.params.smoothness.value();
            params.algorithm = if self.params.rhythm_mode.value() {
                RhythmMode::PerfectBalance
            } else {
                RhythmMode::Euclidean
            };
            params.harmony_mode = if self.params.harmony_mode.value() {
                HarmonyMode::Driver
            } else {
                HarmonyMode::Basic
            };
            params.muted_channels = vec![
                self.params.mute_bass.value(),
                self.params.mute_lead.value(),
                self.params.mute_snare.value(),
                self.params.mute_hat.value(),
            ];
            self.target_params_input.write(params);
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

        // Create a wrapper backend that we can access from outside
        let backend = VstMidiBackend::new();
        self.midi_backend = Arc::new(Mutex::new(backend));

        // Create a backend clone for the engine
        // We need to create a new one because we can't clone Arc<Mutex<>> into Box<dyn>
        let engine_backend = Box::new(VstMidiBackend::new());

        // Phase 3: Take Output from Option, or create new triple buffer if called again
        let target_params_output = if let Some(output) = self.target_params_output.take() {
            output
        } else {
            // Reinitialize case - create new triple buffer
            let (new_input, new_output) = triple_buffer::triple_buffer(&EngineParams::default());
            self.target_params_input = new_input;
            new_output
        };

        // Phase 2-3: Engine now returns (engine, harmony_rx, event_rx) and takes Output<EngineParams>
        let (engine, harmony_state_rx, event_queue_rx) = HarmoniumEngine::new(
            self.sample_rate as f64,
            target_params_output,
            self.control_mode.clone(),
            engine_backend,
        );

        self.engine = Some(engine);
        // Move harmony_state_rx to editor wrapper (not used by process())
        *self.harmony_state_rx_for_editor.lock().unwrap() = Some(harmony_state_rx);
        self.event_queue_rx = Some(event_queue_rx);

        true
    }

    fn reset(&mut self) {
        // Clear any pending MIDI events
        if let Ok(mut backend) = self.midi_backend.lock() {
            backend.clear();
        }
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

        // Phase 2: Collect MIDI events from the event_queue consumer
        // and convert those to MIDI output
        if let Some(ref mut event_rx) = self.event_queue_rx {
            while let Ok(event) = event_rx.pop() {
                // Convert visualization event to MIDI output
                // Channel mapping: 0=Bass/Kick, 1=Lead/Melody, 2=Snare, 3=Hat
                //
                // Drum channels (0,2,3) are remapped to GM standard notes
                // Melody channel (1) keeps original musical notes
                let midi_note = Self::map_note_for_channel(event.instrument, event.note_midi);

                // Send NoteOn at start of buffer
                context.send_event(NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel: event.instrument,
                    note: midi_note,
                    velocity: 0.8,
                });

                // Send NoteOff near end of buffer (timing must be < num_samples)
                // Use last sample of buffer to ensure it's valid
                let note_off_timing = (num_samples - 1) as u32;
                context.send_event(NoteEvent::NoteOff {
                    timing: note_off_timing,
                    voice_id: None,
                    channel: event.instrument,
                    note: midi_note,
                    velocity: 0.0,
                });
            }
        }

        // Clear the audio output (we're a MIDI generator, not an audio plugin)
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                *sample = 0.0;
            }
        }

        ProcessStatus::Normal
    }

    /// Create the GUI editor (when vst-gui feature is enabled)
    #[cfg(feature = "vst-gui")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        crate::vst_gui::create_editor(
            self.target_state.clone(),
            self.control_mode.clone(),
            self.params.clone(),
            self.harmony_state_rx_for_editor.clone(),
        )
    }
}

impl ClapPlugin for HarmoniumPlugin {
    const CLAP_ID: &'static str = "com.bascanada.harmonium";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("AI-powered generative music MIDI plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/bascanada/harmonium");
    const CLAP_SUPPORT_URL: Option<&'static str> = Some("https://github.com/bascanada/harmonium/issues");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::NoteEffect,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for HarmoniumPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"HarmoniumMIDIGen";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Generator,
    ];
}

// Export the plugin
nih_export_clap!(HarmoniumPlugin);
nih_export_vst3!(HarmoniumPlugin);
