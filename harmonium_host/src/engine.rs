use std::sync::{Arc, Mutex};

use arrayvec::ArrayString;
use harmonium_ai::mapper::UnifiedTensionSystem;
use harmonium_audio::{
    backend::AudioRenderer,
    voicing::{BlockChordVoicer, Voicer, VoicerContext},
};
pub use harmonium_core::params::{
    CurrentState, HarmonyState, MusicalParams, SessionConfig,
    VisualizationEvent,
};
use harmonium_core::{
    events::AudioEvent,
    harmony::{
        ChordQuality, ChordStep, HarmonicDriver, HarmonyMode, HarmonyNavigator, Progression,
        chord::ChordType, lydian_chromatic::LydianChromaticConcept,
    },
    log,
    sequencer::{RhythmMode, Sequencer, StepTrigger},
};
use rand::Rng;
use rust_music_theory::{note::PitchSymbol, scale::ScaleType};

pub struct HarmoniumEngine {
    pub config: SessionConfig,
    // NEW UNIFIED COMMUNICATION: Lock-free command/report queues
    command_rx: rtrb::Consumer<harmonium_core::EngineCommand>, // UI→Audio commands
    report_tx: rtrb::Producer<harmonium_core::EngineReport>,    // Audio→UI reports
    last_harmony_state: HarmonyState, // Cache for generating reports
    pub font_queue: crate::FontQueue, // Queue de chargement de SoundFonts (Phase 5 will replace)
    current_state: CurrentState,
    // === POLYRYTHMIE: Plusieurs séquenceurs avec cycles différents ===
    sequencer_primary: Sequencer,   // Cycle principal (16 steps)
    sequencer_secondary: Sequencer, // Cycle secondaire (12 steps) - déphasage de Steve Reich
    harmony: HarmonyNavigator,

    renderer: Box<dyn AudioRenderer>,
    sample_rate: f64,

    sample_counter: usize,
    samples_per_step: usize,
    last_pulse_count: usize,
    last_rotation: usize, // Pour détecter les changements de rotation
    // === PROGRESSION HARMONIQUE ADAPTATIVE ===
    conductor: harmonium_core::params::Conductor,
    // NEW: Pending time signature change (queued for next barline)
    pending_time_signature_change: Option<harmonium_core::params::TimeSignature>,
    current_progression: Vec<ChordStep>, // Progression chargée (dépend de valence/tension)
    progression_index: usize, // Position dans la progression actuelle
    last_valence_choice: f32, // Hystérésis: valence qui a déclenché le dernier choix
    last_tension_choice: f32, // Hystérésis: tension qui a déclenché le dernier choix

    // === HARMONIC DRIVER (Mode avancé) ===
    harmonic_driver: Option<HarmonicDriver>,
    harmony_mode: HarmonyMode,

    // === VOICING ENGINE ===
    voicer: Box<dyn Voicer>,
    lcc: LydianChromaticConcept,
    current_chord_type: ChordType,
    active_lead_notes: Vec<u8>, // Notes actuellement jouées sur le channel Lead
    active_bass_note: Option<u8>, // Note de basse actuellement jouée

    // === NOUVELLE ARCHITECTURE: Params Musicaux Découplés ===
    /// Paramètres musicaux (mis à jour via commandes)
    musical_params: MusicalParams,
    /// Unified tension system coupling harmonic and rhythmic tension
    unified_tension: UnifiedTensionSystem,

    // Recording State Tracking
    is_recording_wav: bool,
    is_recording_midi: bool,
    is_recording_musicxml: bool,

    // Mute State Tracking
    last_muted_channels: Vec<bool>,

    // Phase 2.5: Pre-allocated buffer for event generation (no Vec::new() in audio thread)
    events_buffer: Vec<AudioEvent>,
}

impl HarmoniumEngine {
    pub fn new(
        sample_rate: f64,
        command_rx: rtrb::Consumer<harmonium_core::EngineCommand>,
        report_tx: rtrb::Producer<harmonium_core::EngineReport>,
        mut renderer: Box<dyn AudioRenderer>,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let font_queue = Arc::new(Mutex::new(Vec::new()));
        let bpm = 120.0; // Default BPM (will be updated via commands)
        let steps = 16;
        let initial_pulses = 4; // Default pulses (will be updated via commands)
        let keys = [
            PitchSymbol::C,
            PitchSymbol::D,
            PitchSymbol::E,
            PitchSymbol::F,
            PitchSymbol::G,
            PitchSymbol::A,
            PitchSymbol::B,
        ];
        let scales = [ScaleType::PentatonicMinor, ScaleType::PentatonicMajor];
        let random_key = keys[rng.gen_range(0..keys.len())];
        let random_scale = scales[rng.gen_range(0..scales.len())];

        let config = SessionConfig {
            bpm,
            key: format!("{}", random_key),
            scale: format!("{:?}", random_scale),
            pulses: initial_pulses,
            steps,
        };

        log::info(&format!("Session: {} {} | BPM: {:.1}", config.key, config.scale, bpm));

        // Initialize harmony state cache for report generation
        let last_harmony_state = HarmonyState::default();

        // Séquenceurs
        let sequencer_primary = Sequencer::new(steps, initial_pulses, bpm);
        let default_density = 0.4; // Default density (will be updated via commands)
        let secondary_pulses = std::cmp::min((default_density * 8.0) as usize + 1, 12);
        let sequencer_secondary = Sequencer::new_with_rotation(12, secondary_pulses, bpm, 0);

        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);
        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        // Initialize renderer timing
        renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step });

        // Progression initiale (default values, will be updated via commands)
        let current_progression = Progression::get_palette(0.3, 0.3);
        let _progression_name = Progression::get_progression_name(0.3, 0.3);

        // HarmonicDriver (toujours créé pour permettre le switch dynamique)
        let harmony_mode = HarmonyMode::Driver; // Default, will be updated via commands
        let key_pc = match random_key {
            PitchSymbol::C => 0,
            PitchSymbol::D => 2,
            PitchSymbol::E => 4,
            PitchSymbol::F => 5,
            PitchSymbol::G => 7,
            PitchSymbol::A => 9,
            PitchSymbol::B => 11,
            _ => 0,
        };
        let harmonic_driver = Some(HarmonicDriver::new(key_pc));

        // Initialize musical params with defaults (will be updated via commands)
        let musical_params = MusicalParams::default();

        // Initialize unified tension system with defaults
        let mut unified_tension = UnifiedTensionSystem::new();
        unified_tension.update(0.5, 0.3, 0.3); // Default arousal, valence, tension

        let engine = Self {
            config,
            command_rx,
            report_tx,
            last_harmony_state,
            font_queue,
            current_state: CurrentState::default(),
            sequencer_primary,
            sequencer_secondary,
            harmony,
            renderer,
            sample_rate,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
            last_rotation: 0,
            conductor: harmonium_core::params::Conductor::default(),
            pending_time_signature_change: None,
            current_progression,
            progression_index: 0,
            last_valence_choice: 0.3, // Default valence (will be updated via commands)
            last_tension_choice: 0.3, // Default tension (will be updated via commands)
            harmonic_driver,
            harmony_mode,
            // Voicing Engine
            voicer: Box::new(BlockChordVoicer::new(4)),
            lcc: LydianChromaticConcept::new(),
            current_chord_type: ChordType::Major,
            // Phase 2.5: Pre-allocate with capacity for max chord voicing (typically 4-5 notes)
            active_lead_notes: Vec::with_capacity(8),
            active_bass_note: None,
            // Nouvelle architecture
            musical_params,
            unified_tension,
            is_recording_wav: false,
            is_recording_midi: false,
            is_recording_musicxml: false,
            last_muted_channels: vec![false; 16],
            // Phase 2.5: Pre-allocate with capacity for typical number of events per tick
            events_buffer: Vec::with_capacity(8),
        };

        engine
    }

    /// Change le voicer dynamiquement
    pub fn set_voicer(&mut self, voicer: Box<dyn Voicer>) {
        self.voicer = voicer;
    }

    /// Retourne le nom du voicer actuel
    pub fn current_voicer_name(&self) -> &'static str {
        self.voicer.name()
    }

    // ═══════════════════════════════════════════════════════════════════
    // NOUVELLE API: Contrôle Direct des Paramètres Musicaux
    // ═══════════════════════════════════════════════════════════════════

    /// Récupère une copie des paramètres musicaux actuels
    pub fn get_musical_params(&self) -> MusicalParams {
        self.musical_params.clone()
    }

    pub fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        // Mark the beginning of real-time audio processing context
        // Any allocations after this point will panic in debug builds
        crate::realtime::rt_check::enter_audio_context();

        let total_samples = output.len() / channels;
        let mut processed = 0;

        // Run control logic once per block
        // PHASE 2.5 NOTE: RT guard temporarily suspended - allocations in update_controls()
        crate::realtime::rt_check::exit_audio_context();
        self.update_controls();
        crate::realtime::rt_check::enter_audio_context();

        while processed < total_samples {
            let remaining = total_samples - processed;
            let samples_until_tick = if self.samples_per_step > self.sample_counter {
                self.samples_per_step - self.sample_counter
            } else {
                1
            };

            let chunk_size = std::cmp::min(remaining, samples_until_tick);

            let start_idx = processed * channels;
            let end_idx = (processed + chunk_size) * channels;
            let chunk = &mut output[start_idx..end_idx];

            // Generate audio for this chunk
            self.renderer.process_buffer(chunk, channels);

            self.sample_counter += chunk_size;
            processed += chunk_size;

            if self.sample_counter >= self.samples_per_step {
                self.sample_counter = 0;
                // PHASE 2.5 NOTE: RT guard temporarily suspended - allocations in tick()
                // Sources: renderer.handle_event() and event.clone() (external library code)
                crate::realtime::rt_check::exit_audio_context();
                self.tick();
                crate::realtime::rt_check::enter_audio_context();
            }
        }

        // Mark the end of real-time audio processing context
        // Allocations are allowed again after this point
        crate::realtime::rt_check::exit_audio_context();
    }

    /// Process all pending commands from the command queue
    /// This replaces the old triple buffer read in update_controls()
    fn process_commands(&mut self) {
        use harmonium_core::EngineCommand;

        // Drain all pending commands (non-blocking)
        while let Ok(cmd) = self.command_rx.pop() {
            match cmd {
                // === GLOBAL ===
                EngineCommand::SetBpm(bpm) => {
                    self.musical_params.bpm = bpm.clamp(70.0, 180.0);
                }
                EngineCommand::SetMasterVolume(volume) => {
                    self.musical_params.master_volume = volume.clamp(0.0, 1.0);
                }
                EngineCommand::SetTimeSignature { numerator, denominator } => {
                    // Queue time signature change for next barline
                    self.pending_time_signature_change = Some(harmonium_core::params::TimeSignature {
                        numerator,
                        denominator,
                    });
                }

                // === MODULE TOGGLES ===
                EngineCommand::EnableRhythm(enabled) => {
                    self.musical_params.enable_rhythm = enabled;
                }
                EngineCommand::EnableHarmony(enabled) => {
                    self.musical_params.enable_harmony = enabled;
                }
                EngineCommand::EnableMelody(enabled) => {
                    self.musical_params.enable_melody = enabled;
                }
                EngineCommand::EnableVoicing(enabled) => {
                    self.musical_params.enable_voicing = enabled;
                }

                // === RHYTHM ===
                EngineCommand::SetRhythmMode(mode) => {
                    self.musical_params.rhythm_mode = mode;
                }
                EngineCommand::SetRhythmSteps(steps) => {
                    self.musical_params.rhythm_steps = steps;
                }
                EngineCommand::SetRhythmPulses(pulses) => {
                    self.musical_params.rhythm_pulses = pulses;
                }
                EngineCommand::SetRhythmRotation(rotation) => {
                    self.musical_params.rhythm_rotation = rotation;
                }
                EngineCommand::SetRhythmDensity(density) => {
                    self.musical_params.rhythm_density = density.clamp(0.0, 1.0);
                }
                EngineCommand::SetRhythmTension(tension) => {
                    self.musical_params.rhythm_tension = tension.clamp(0.0, 1.0);
                }
                EngineCommand::SetRhythmSecondary { steps, pulses, rotation } => {
                    self.musical_params.rhythm_secondary_steps = steps;
                    self.musical_params.rhythm_secondary_pulses = pulses;
                    self.musical_params.rhythm_secondary_rotation = rotation;
                }
                EngineCommand::SetFixedKick(fixed) => {
                    self.musical_params.fixed_kick = fixed;
                }

                // === HARMONY ===
                EngineCommand::SetHarmonyMode(mode) => {
                    self.musical_params.harmony_mode = mode;
                }
                EngineCommand::SetHarmonyStrategy(strategy) => {
                    self.musical_params.harmony_strategy = strategy;
                }
                EngineCommand::SetHarmonyTension(tension) => {
                    self.musical_params.harmony_tension = tension.clamp(0.0, 1.0);
                }
                EngineCommand::SetHarmonyValence(valence) => {
                    self.musical_params.harmony_valence = valence.clamp(-1.0, 1.0);
                }
                EngineCommand::SetHarmonyMeasuresPerChord(measures) => {
                    self.musical_params.harmony_measures_per_chord = measures;
                }
                EngineCommand::SetKeyRoot(root) => {
                    self.musical_params.key_root = root % 12;
                }

                // === MELODY / VOICING ===
                EngineCommand::SetMelodySmoothness(smoothness) => {
                    self.musical_params.melody_smoothness = smoothness.clamp(0.0, 1.0);
                }
                EngineCommand::SetMelodyOctave(octave) => {
                    self.musical_params.melody_octave = octave.clamp(3, 6);
                }
                EngineCommand::SetVoicingDensity(density) => {
                    self.musical_params.voicing_density = density.clamp(0.0, 1.0);
                }
                EngineCommand::SetVoicingTension(tension) => {
                    self.musical_params.voicing_tension = tension.clamp(0.0, 1.0);
                }

                // === MIXER (per-channel) ===
                EngineCommand::SetChannelGain { channel, gain } => {
                    if (channel as usize) < 16 {
                        // Update specific channel gains
                        match channel {
                            0 => self.musical_params.gain_bass = gain.clamp(0.0, 1.0),
                            1 => self.musical_params.gain_lead = gain.clamp(0.0, 1.0),
                            2 => self.musical_params.gain_snare = gain.clamp(0.0, 1.0),
                            3 => self.musical_params.gain_hat = gain.clamp(0.0, 1.0),
                            _ => {} // Other channels not yet mapped to MusicalParams
                        }
                    }
                }
                EngineCommand::SetChannelMute { channel, muted } => {
                    if (channel as usize) < self.musical_params.muted_channels.len() {
                        self.musical_params.muted_channels[channel as usize] = muted;
                    }
                }
                EngineCommand::SetChannelRoute { channel, bank_id } => {
                    if (channel as usize) < self.musical_params.channel_routing.len() {
                        self.musical_params.channel_routing[channel as usize] = bank_id;
                    }
                }
                EngineCommand::SetVelocityBase { channel, velocity } => {
                    match channel {
                        0 => self.musical_params.vel_base_bass = velocity,
                        2 => self.musical_params.vel_base_snare = velocity,
                        _ => {} // Other channels not yet mapped
                    }
                }

                // === RECORDING ===
                EngineCommand::StartRecording(format) => {
                    match format {
                        harmonium_core::events::RecordFormat::Wav => {
                            self.musical_params.record_wav = true;
                        }
                        harmonium_core::events::RecordFormat::Midi => {
                            self.musical_params.record_midi = true;
                        }
                        harmonium_core::events::RecordFormat::MusicXml => {
                            self.musical_params.record_musicxml = true;
                        }
                    }
                }
                EngineCommand::StopRecording(format) => {
                    match format {
                        harmonium_core::events::RecordFormat::Wav => {
                            self.musical_params.record_wav = false;
                        }
                        harmonium_core::events::RecordFormat::Midi => {
                            self.musical_params.record_midi = false;
                        }
                        harmonium_core::events::RecordFormat::MusicXml => {
                            self.musical_params.record_musicxml = false;
                        }
                    }
                }

                // === CONTROL MODE ===
                // Note: These are handled by HarmoniumController, not the engine
                // The engine only sees the resulting musical parameter commands
                EngineCommand::UseEmotionMode => {
                    // No-op: EmotionMapper lives in Controller, not engine
                }
                EngineCommand::UseDirectMode => {
                    // No-op: EmotionMapper lives in Controller, not engine
                }
                EngineCommand::SetEmotionParams { .. } => {
                    // No-op: EmotionMapper lives in Controller, not engine
                    // Controller will translate this to concrete musical param commands
                }

                // === BATCH OPERATIONS ===
                EngineCommand::SetAllRhythmParams {
                    mode,
                    steps,
                    pulses,
                    rotation,
                    density,
                    tension,
                    secondary_steps,
                    secondary_pulses,
                    secondary_rotation,
                } => {
                    self.musical_params.rhythm_mode = mode;
                    self.musical_params.rhythm_steps = steps;
                    self.musical_params.rhythm_pulses = pulses;
                    self.musical_params.rhythm_rotation = rotation;
                    self.musical_params.rhythm_density = density.clamp(0.0, 1.0);
                    self.musical_params.rhythm_tension = tension.clamp(0.0, 1.0);
                    self.musical_params.rhythm_secondary_steps = secondary_steps;
                    self.musical_params.rhythm_secondary_pulses = secondary_pulses;
                    self.musical_params.rhythm_secondary_rotation = secondary_rotation;
                }

                // === UTILITY ===
                EngineCommand::GetState => {
                    // Will be handled by send_report() which is called periodically
                }
                EngineCommand::Reset => {
                    // Reset to defaults
                    self.musical_params = MusicalParams::default();
                }
            }
        }
    }

    /// Generate and send an EngineReport to the UI
    /// This replaces the old harmony_state_tx and event_queue_tx
    fn send_report(&mut self) {
        use harmonium_core::EngineReport;

        let mut report = EngineReport::new();

        // === TIMING ===
        report.current_bar = self.conductor.current_bar;
        report.current_beat = self.conductor.current_beat;
        report.current_step = self.sequencer_primary.current_step;
        report.time_signature = self.conductor.time_signature;

        // === HARMONY STATE ===
        report.current_chord = self.last_harmony_state.chord_name.clone();
        report.chord_root_offset = self.last_harmony_state.chord_root_offset;
        report.chord_is_minor = self.last_harmony_state.chord_is_minor;
        report.progression_name = self.last_harmony_state.progression_name.clone();
        report.progression_length = self.last_harmony_state.progression_length;
        report.harmony_mode = self.last_harmony_state.harmony_mode;

        // === RHYTHM STATE ===
        report.primary_steps = self.last_harmony_state.primary_steps;
        report.primary_pulses = self.last_harmony_state.primary_pulses;
        report.rhythm_mode = self.musical_params.rhythm_mode;

        // Copy primary pattern (fixed-size array, no allocation)
        for (i, trigger) in self.sequencer_primary.pattern.iter().enumerate() {
            if i < 192 {
                report.primary_pattern[i] = trigger.is_any();
            }
        }

        // Copy secondary pattern
        report.secondary_steps = self.last_harmony_state.secondary_steps;
        report.secondary_pulses = self.last_harmony_state.secondary_pulses;
        for (i, trigger) in self.sequencer_secondary.pattern.iter().enumerate() {
            if i < 192 {
                report.secondary_pattern[i] = trigger.is_any();
            }
        }

        // === CURRENT PARAMS (echoed back) ===
        report.musical_params = self.musical_params.clone();

        // === SESSION INFO ===
        report.session_key = ArrayString::from(&self.config.key).unwrap_or_default();
        report.session_scale = ArrayString::from(&self.config.scale).unwrap_or_default();

        // === NOTES (will be populated by tick() as notes are triggered) ===
        // Note: We pre-allocate capacity in EngineReport::new()

        // Push to report queue (non-blocking, drops if full)
        let _ = self.report_tx.push(report);
    }

    fn update_controls(&mut self) {
        // === PROCESS COMMANDS ===
        // NEW: Process all pending commands from the command queue
        // This replaces the old triple buffer + EmotionMapper + control_mode logic
        self.process_commands();

        // === UPDATE UNIFIED TENSION SYSTEM ===
        // Recalculate TRQ state based on current emotional parameters
        let arousal_from_bpm = (self.musical_params.bpm - 70.0) / 110.0;
        self.unified_tension.update(
            arousal_from_bpm,
            self.musical_params.harmony_valence,
            self.musical_params.harmony_tension,
        );

        let mp = &self.musical_params; // Raccourci pour la lisibilité

        // === LOAD FONTS ===
        if let Ok(mut queue) = self.font_queue.try_lock() {
            while let Some((id, bytes)) = queue.pop() {
                self.renderer.handle_event(AudioEvent::LoadFont { id, bytes });
            }
        }

        // === SYNC ROUTING (from MusicalParams) ===
        for (i, &mode) in mp.channel_routing.iter().enumerate() {
            if i < 16 {
                self.renderer
                    .handle_event(AudioEvent::SetChannelRoute { channel: i as u8, bank: mode });
            }
        }

        // === MUTE CONTROL (from MusicalParams) ===
        for (i, &is_muted) in mp.muted_channels.iter().enumerate() {
            if i < 16 && i < self.last_muted_channels.len() {
                if is_muted && !self.last_muted_channels[i] {
                    // Changed from Unmuted to Muted -> Kill sound
                    self.renderer.handle_event(AudioEvent::AllNotesOff { channel: i as u8 });
                }
                self.last_muted_channels[i] = is_muted;
            }
        }

        // === SYNC HARMONY MODE (from MusicalParams) ===
        if self.harmony_mode != mp.harmony_mode {
            self.harmony_mode = mp.harmony_mode;
            // NOTE: Logging disabled in audio thread to prevent allocations
            // log::info(&format!("🎹 Harmony mode switched to: {:?}", self.harmony_mode));
        }

        // === MORPHING (smooth transitions) ===
        // En mode direct, on utilise les valeurs de MusicalParams directement
        // En mode émotion, le morphing se fait sur les valeurs mappées
        let morph_factor = 0.03;

        // Pour compatibilité avec le reste du code qui utilise current_state
        // on morphe vers les valeurs des MusicalParams
        self.current_state.bpm += (mp.bpm - self.current_state.bpm) * morph_factor;
        self.current_state.density +=
            (mp.rhythm_density - self.current_state.density) * morph_factor;
        self.current_state.tension +=
            (mp.harmony_tension - self.current_state.tension) * morph_factor;
        self.current_state.smoothness +=
            (mp.melody_smoothness - self.current_state.smoothness) * morph_factor;
        self.current_state.valence +=
            (mp.harmony_valence - self.current_state.valence) * morph_factor;
        // Arousal n'existe plus directement dans MusicalParams (c'est le BPM)
        // On le recalcule pour compatibilité avec l'affichage UI
        let arousal_from_bpm = (mp.bpm - 70.0) / 110.0;
        self.current_state.arousal +=
            (arousal_from_bpm - self.current_state.arousal) * morph_factor;

        // === SYNTHESIS MORPHING (emotional timbre control) ===
        #[cfg(feature = "odin2")]
        if let Some(odin2) = self.renderer.odin2_backend_mut() {
            odin2.apply_emotional_morphing(
                self.current_state.valence,
                self.current_state.arousal,
                self.current_state.tension,
                self.current_state.density,
            );
        }

        // === MELODY SMOOTHNESS → Hurst Factor ===
        // Applique le smoothness au navigateur harmonique pour le comportement mélodique
        self.harmony.set_hurst_factor(mp.melody_smoothness);

        // === VOICING DENSITY → Comping Pattern ===
        // Met à jour le pattern de comping si la densité change significativement
        self.voicer.on_density_change(mp.voicing_density, self.sequencer_primary.steps);

        // === MIXER GAINS (from MusicalParams) ===
        self.renderer.handle_event(AudioEvent::SetMixerGains {
            lead: mp.gain_lead,
            bass: mp.gain_bass,
            snare: mp.gain_snare,
            hat: mp.gain_hat,
        });

        // === RECORDING CONTROL (from MusicalParams) ===
        if mp.record_wav != self.is_recording_wav {
            self.is_recording_wav = mp.record_wav;
            if self.is_recording_wav {
                self.renderer.handle_event(AudioEvent::StartRecording {
                    format: harmonium_core::events::RecordFormat::Wav,
                });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording {
                    format: harmonium_core::events::RecordFormat::Wav,
                });
            }
        }

        if mp.record_midi != self.is_recording_midi {
            self.is_recording_midi = mp.record_midi;
            if self.is_recording_midi {
                self.renderer.handle_event(AudioEvent::StartRecording {
                    format: harmonium_core::events::RecordFormat::Midi,
                });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording {
                    format: harmonium_core::events::RecordFormat::Midi,
                });
            }
        }

        if mp.record_musicxml != self.is_recording_musicxml {
            self.is_recording_musicxml = mp.record_musicxml;
            if self.is_recording_musicxml {
                // Send musical params through event system for accurate export metadata
                self.renderer
                    .handle_event(AudioEvent::UpdateMusicalParams { params: Box::new(mp.clone()) });
                self.renderer.handle_event(AudioEvent::StartRecording {
                    format: harmonium_core::events::RecordFormat::MusicXml,
                });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording {
                    format: harmonium_core::events::RecordFormat::MusicXml,
                });
            }
        }

        // === DSP UPDATES (effets globaux sur channel 0) ===
        // CC 1: Modulation/Filtre - utilise voicing_tension (timbre du son)
        //       PAS harmony_tension (qui affecte seulement la sélection d'accords)
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 1,
            value: (mp.voicing_tension * 127.0) as u8,
            channel: 0,
        });
        // CC 11: Expression/Distortion - lié à l'énergie
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 11,
            value: (self.current_state.arousal * 127.0) as u8,
            channel: 0,
        });
        // CC 91: Reverb - lié à la valence (émotions positives = plus de reverb)
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 91,
            value: (self.current_state.valence.abs() * 127.0) as u8,
            channel: 0,
        });

        // === SKIP RHYTHM IF DISABLED ===
        if !mp.enable_rhythm {
            // Timing only (pour garder le moteur synchronisé)
            let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
            let new_samples_per_step = (self.sample_rate * 60.0
                / (self.current_state.bpm as f64)
                / steps_per_beat) as usize;
            if new_samples_per_step != self.samples_per_step {
                self.samples_per_step = new_samples_per_step;
                self.renderer.handle_event(AudioEvent::TimingUpdate {
                    samples_per_step: new_samples_per_step,
                });
                // Send updated musical params if recording is active
                if self.is_recording_musicxml || self.is_recording_midi {
                    self.renderer.handle_event(AudioEvent::UpdateMusicalParams {
                        params: Box::new(self.musical_params.clone()),
                    });
                }
            }
            return; // Skip sequencer logic when rhythm disabled
        }

        // === LOGIQUE SÉQUENCEUR (from MusicalParams) ===
        let target_algo = mp.rhythm_mode;
        let mode_changed = self.sequencer_primary.mode != target_algo;

        if mode_changed {
            self.sequencer_primary.mode = target_algo;
            // Adjust steps based on mode
            if target_algo == RhythmMode::PerfectBalance {
                // Upgrade to poly steps (48, 96, 192)
                self.sequencer_primary.upgrade_to_steps(mp.rhythm_steps);
            } else {
                // Downgrade back to Euclidean steps (typically 16)
                self.sequencer_primary.upgrade_to_steps(mp.rhythm_steps);
            }
        }

        // Queue time signature change for next barline (don't apply immediately)
        if self.sequencer_primary.time_signature != mp.time_signature {
            log::info(&format!(
                "⏱️  Time Signature change queued: {}/{} → {}/{} (will apply at next barline)",
                self.sequencer_primary.time_signature.numerator,
                self.sequencer_primary.time_signature.denominator,
                mp.time_signature.numerator,
                mp.time_signature.denominator
            ));
            self.pending_time_signature_change = Some(mp.time_signature);
            // Change will be applied on next barline in tick()
        }

        // Update steps if changed while playing (manual override or mode change)
        if self.sequencer_primary.steps != mp.rhythm_steps && mp.rhythm_steps > 0 {
            self.sequencer_primary.upgrade_to_steps(mp.rhythm_steps);
        }

        // Rotation (from MusicalParams - même valeur dans les deux modes)
        let target_rotation = mp.rhythm_rotation;

        // Pulses (from MusicalParams)
        let target_pulses = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            mp.rhythm_pulses.min(self.sequencer_primary.steps)
        } else {
            self.sequencer_primary.pulses
        };

        // Regeneration Logic
        let needs_regen = mode_changed
            || if self.sequencer_primary.mode == RhythmMode::Euclidean {
                target_pulses != self.last_pulse_count
            } else {
                (mp.rhythm_density - self.sequencer_primary.density).abs() > 0.05
                    || (mp.rhythm_tension - self.sequencer_primary.tension).abs() > 0.05
            };

        if needs_regen {
            self.sequencer_primary.tension = mp.rhythm_tension;
            self.sequencer_primary.density = mp.rhythm_density;
            self.sequencer_primary.pulses = target_pulses;
            // Pass target oddity from unified tension system
            self.sequencer_primary.target_oddity = Some(self.unified_tension.calculate_target_oddity());
            // Pattern preparation now happens only on barlines in tick()
            // New parameters will take effect at next barline
            self.last_pulse_count = target_pulses;
        }

        if target_rotation != self.last_rotation {
            self.sequencer_primary.rotation = target_rotation;
            // Pattern preparation now happens only on barlines in tick()
            self.last_rotation = target_rotation;
        }

        // Secondary Sequencer Logic (from MusicalParams) - Euclidean mode only
        let secondary_steps = mp.rhythm_secondary_steps;
        let secondary_pulses = mp.rhythm_secondary_pulses.min(secondary_steps);
        let secondary_rotation = mp.rhythm_secondary_rotation;

        let secondary_changed = secondary_steps != self.sequencer_secondary.steps
            || secondary_pulses != self.sequencer_secondary.pulses
            || secondary_rotation != self.sequencer_secondary.rotation;

        if secondary_changed {
            if secondary_steps != self.sequencer_secondary.steps {
                self.sequencer_secondary.steps = secondary_steps;
                self.sequencer_secondary.pattern = vec![StepTrigger::default(); secondary_steps];
            }
            self.sequencer_secondary.pulses = secondary_pulses;
            self.sequencer_secondary.rotation = secondary_rotation;
            // Pass target oddity from unified tension system
            self.sequencer_secondary.target_oddity = Some(self.unified_tension.calculate_target_oddity());
            // Pattern preparation now happens only on barlines in tick()
            // New parameters will take effect at next barline
        }

        // Timing
        let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
        let new_samples_per_step =
            (self.sample_rate * 60.0 / (self.current_state.bpm as f64) / steps_per_beat) as usize;
        if new_samples_per_step != self.samples_per_step {
            self.samples_per_step = new_samples_per_step;
            self.renderer
                .handle_event(AudioEvent::TimingUpdate { samples_per_step: new_samples_per_step });
            // Send updated musical params if recording is active
            if self.is_recording_musicxml || self.is_recording_midi {
                self.renderer.handle_event(AudioEvent::UpdateMusicalParams {
                    params: Box::new(self.musical_params.clone()),
                });
            }
        }
    }

    fn tick(&mut self) {
        let tick_primary = self.sequencer_primary.tick();
        let trigger_primary = tick_primary.trigger;

        let tick_secondary = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            self.sequencer_secondary.tick()
        } else {
            harmonium_core::sequencer::TickResult::default()
        };
        let trigger_secondary = tick_secondary.trigger;

        // Advance conductor
        let bar_crossed = self.conductor.tick();

        // === BARLINE-BASED BUFFER SWAP (Phase 1) ===
        if bar_crossed {
            // Log barline crossing
            log::info(&format!(
                "♩ Bar {} | Beat {}/{} | Time Signature: {}/{}",
                self.conductor.current_bar,
                self.conductor.current_beat,
                self.conductor.time_signature.numerator,
                self.conductor.time_signature.numerator,
                self.conductor.time_signature.denominator
            ));

            // Apply pending time signature change
            if let Some(new_ts) = self.pending_time_signature_change.take() {
                log::info(&format!(
                    "⚡ Time Signature Change: {}/{} → {}/{} (queued mid-bar, applied at barline)",
                    self.conductor.time_signature.numerator,
                    self.conductor.time_signature.denominator,
                    new_ts.numerator,
                    new_ts.denominator
                ));

                self.conductor.time_signature = new_ts;
                let new_steps = new_ts.steps_per_bar(self.sequencer_primary.ticks_per_beat);

                self.sequencer_primary.time_signature = new_ts;
                self.sequencer_primary.steps = new_steps;
                self.sequencer_secondary.time_signature = new_ts;
                // Patterns will regenerate with new meter on next prepare_next_bar()

                log::info(&format!(
                    "   → New bar length: {} steps | Primary: {} | Secondary: {}",
                    new_steps,
                    self.sequencer_primary.steps,
                    self.sequencer_secondary.steps
                ));
            }

            // Swap both sequencer buffers
            let mut swapped_primary = false;
            let mut swapped_secondary = false;

            if let Some(next) = self.sequencer_primary.next_pattern.take() {
                self.sequencer_primary.pattern = next;
                self.sequencer_primary.steps = self.sequencer_primary.pattern.len();
                swapped_primary = true;
            }
            if let Some(next) = self.sequencer_secondary.next_pattern.take() {
                self.sequencer_secondary.pattern = next;
                self.sequencer_secondary.steps = self.sequencer_secondary.pattern.len();
                swapped_secondary = true;
            }

            if swapped_primary || swapped_secondary {
                log::info(&format!(
                    "   → Pattern buffers swapped | Primary: {} | Secondary: {}",
                    if swapped_primary { "✓ swapped" } else { "- unchanged" },
                    if swapped_secondary { "✓ swapped" } else { "- unchanged" }
                ));
            }

            // Prepare next bars immediately for both sequencers
            self.sequencer_primary.prepare_next_bar();
            self.sequencer_secondary.prepare_next_bar();
        }

        // === HARMONY & PROGRESSION ===
        // Skip harmony updates if disabled
        let harmony_enabled = self.musical_params.enable_harmony;

        if harmony_enabled && bar_crossed {
            // Update conductor time signature if needed
            self.conductor.time_signature = self.musical_params.time_signature;

            match self.harmony_mode {
                HarmonyMode::Basic => {
                    // === MODE BASIC: Progressions par quadrants émotionnels ===
                    // Palette Selection (Hysteresis)
                    if self.conductor.current_bar.is_multiple_of(4) {
                        let valence_delta =
                            (self.current_state.valence - self.last_valence_choice).abs();
                        let tension_delta =
                            (self.current_state.tension - self.last_tension_choice).abs();

                        if valence_delta > 0.4 || tension_delta > 0.4 {
                            self.current_progression = Progression::get_palette(
                                self.current_state.valence,
                                self.current_state.tension,
                            );
                            self.progression_index = 0;
                            self.last_valence_choice = self.current_state.valence;
                            self.last_tension_choice = self.current_state.tension;

                            let prog_name = Progression::get_progression_name(
                                self.current_state.valence,
                                self.current_state.tension,
                            );
                            // Phase 2: Update local cache (no lock needed)
                            // Phase 2.5: Use ArrayString::from (no heap allocation)
                            self.last_harmony_state.progression_name =
                                ArrayString::from(prog_name).unwrap_or_default();
                            self.last_harmony_state.progression_length =
                                self.current_progression.len();
                        }
                    }

                    // Chord Progression
                    let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                    if self.conductor.current_bar.is_multiple_of(measures_per_chord) {
                        self.progression_index =
                            (self.progression_index + 1) % self.current_progression.len();
                        let chord = &self.current_progression[self.progression_index];

                        self.harmony.set_chord_context(chord.root_offset, chord.quality);
                        let chord_name = self.format_chord_name(chord.root_offset, chord.quality);

                        // Mettre à jour le type d'accord pour le voicer
                        self.current_chord_type = match chord.quality {
                            ChordQuality::Major => ChordType::Major7,
                            ChordQuality::Minor => ChordType::Minor7,
                            ChordQuality::Dominant7 => ChordType::Dominant7,
                            ChordQuality::Diminished => ChordType::Diminished7,
                            ChordQuality::Sus2 => ChordType::Sus2,
                        };

                        // Phase 2: Update local cache (no lock needed)
                        self.last_harmony_state.current_chord_index = self.progression_index;
                        self.last_harmony_state.chord_root_offset = chord.root_offset;
                        self.last_harmony_state.chord_is_minor =
                            matches!(chord.quality, ChordQuality::Minor);
                        // Phase 2.5: Convert String to ArrayString (unavoidable allocation once per chord change)
                        self.last_harmony_state.chord_name =
                            ArrayString::from(&chord_name).unwrap_or_default();
                        self.last_harmony_state.measure_number = self.conductor.current_bar;
                    }
                }

                HarmonyMode::Driver => {
                    // === MODE DRIVER: Steedman Grammar + Neo-Riemannian + LCC ===
                    let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                    if self.conductor.current_bar.is_multiple_of(measures_per_chord)
                        && let Some(ref mut driver) = self.harmonic_driver
                    {
                        let mut rng = rand::thread_rng();

                        // NOTE: Chord name capture removed to prevent allocation
                        // let old_chord_name = driver.current_chord().name();

                        let decision = driver.next_chord(
                            self.current_state.tension,
                            self.current_state.valence,
                            &mut rng,
                        );

                        // === LOGGING DISABLED IN AUDIO THREAD ===
                        // NOTE: All string formatting removed to prevent allocations
                        // let strategy = driver.current_strategy_name();
                        // let scale_notes: Vec<String> = decision.suggested_scale.iter()...
                        // log::info(...) - REMOVED

                        // Convertir vers le format compatible avec HarmonyNavigator
                        let root_offset = driver.root_offset();
                        let quality = driver.to_basic_quality();
                        self.harmony.set_chord_context(root_offset, quality);

                        // Mettre à jour le type d'accord pour le voicer
                        self.current_chord_type = decision.next_chord.chord_type;

                        // NOTE: Using simplified chord name (no allocation)
                        // Previously: format!("{} ({})", chord.name(), transition.name())
                        // Now: Just use chord name directly (allocated once by .name())
                        let chord_name = decision.next_chord.name(); // Still allocates, but unavoidable

                        // Phase 2: Update local cache (no lock needed)
                        self.last_harmony_state.current_chord_index = self.progression_index;
                        self.last_harmony_state.chord_root_offset = root_offset;
                        self.last_harmony_state.chord_is_minor = driver.is_minor();
                        // Phase 2.5: Convert String to ArrayString (unavoidable allocation once per chord change)
                        self.last_harmony_state.chord_name =
                            ArrayString::from(&chord_name).unwrap_or_default();
                        self.last_harmony_state.measure_number = self.conductor.current_bar;
                        self.last_harmony_state.progression_name =
                            ArrayString::from("Driver").unwrap_or_default();
                        self.last_harmony_state.progression_length = 0; // Driver n'a pas de longueur fixe
                    }
                }
            }
        }

        // Phase 2: Update local harmony state cache (no lock needed)
        self.last_harmony_state.current_step = self.sequencer_primary.current_step;

        // NOTE: Pattern updates moved to step 0 only to avoid Vec::collect() allocations every tick
        // Only update when patterns might have changed (step 0 = start of measure)
        if self.sequencer_primary.current_step == 0 {
            self.last_harmony_state.primary_steps = self.sequencer_primary.steps;
            self.last_harmony_state.primary_pulses = self.sequencer_primary.pulses;
            self.last_harmony_state.primary_rotation = self.sequencer_primary.rotation;
            // Phase 2.5: Copy into fixed-size array instead of Vec::collect() (no heap allocation)
            let primary_len = self.sequencer_primary.pattern.len();
            for i in 0..192 {
                self.last_harmony_state.primary_pattern[i] =
                    i < primary_len && self.sequencer_primary.pattern[i].is_any();
            }

            self.last_harmony_state.secondary_steps = self.sequencer_secondary.steps;
            self.last_harmony_state.secondary_pulses = self.sequencer_secondary.pulses;
            self.last_harmony_state.secondary_rotation = self.sequencer_secondary.rotation;
            // Phase 2.5: Copy into fixed-size array instead of Vec::collect() (no heap allocation)
            let secondary_len = self.sequencer_secondary.pattern.len();
            for i in 0..192 {
                self.last_harmony_state.secondary_pattern[i] =
                    i < secondary_len && self.sequencer_secondary.pattern[i].is_any();
            }
            self.last_harmony_state.harmony_mode = self.harmony_mode;
        }

        // === GENERATE EVENTS ===
        // Phase 2.5: Reuse pre-allocated buffer instead of Vec::new()
        self.events_buffer.clear();
        let rhythm_enabled = self.musical_params.enable_rhythm;

        // === BATTEUR VIRTUEL (CONTEXTE) ===
        // Analyse des émotions pour humaniser le jeu
        let is_high_tension = self.current_state.tension > 0.6;
        let is_high_density = self.current_state.density > 0.6;
        let is_high_energy = self.current_state.arousal > 0.7;
        let is_low_energy = self.current_state.arousal < 0.4;

        // Détection de la "Fill Zone" (les 4 derniers steps de la mesure)
        // C'est là que les batteurs font leurs roulements pour annoncer la suite
        let fill_zone_start = self.sequencer_primary.steps.saturating_sub(4);
        let is_in_fill_zone = self.sequencer_primary.current_step >= fill_zone_start;

        // Bass (Kick) - part of Rhythm module
        // Always stop previous bass note (Staccato / Note Switching) to prevent infinite sustain
        if let Some(old_note) = self.active_bass_note {
            self.events_buffer.push(AudioEvent::NoteOff { note: old_note, channel: 0 });
            self.active_bass_note = None;
        }

        if rhythm_enabled
            && trigger_primary.kick
            && !self.musical_params.muted_channels.first().copied().unwrap_or(false)
        {
            // LOGIQUE HYBRIDE : Mode Drum Kit (fixe) ou Synth (harmonisé)
            let midi_note = if self.musical_params.fixed_kick {
                36 // Mode Drum Kit (C1 fixe)
            } else {
                // Mode Synth/Bass (Harmonisé)
                (36 + self.last_harmony_state.chord_root_offset) as u8
            };
            let vel = self.musical_params.vel_base_bass + (self.current_state.arousal * 25.0) as u8;
            self.events_buffer.push(AudioEvent::NoteOn {
                note: midi_note,
                velocity: vel,
                channel: 0,
            });
            self.active_bass_note = Some(midi_note);
        }

        // Lead (avec Voicing) - Skip if melody disabled
        let melody_enabled = self.musical_params.enable_melody;

        // If melody just got disabled, stop all lead notes
        if !melody_enabled && !self.active_lead_notes.is_empty() {
            self.events_buffer.push(AudioEvent::AllNotesOff { channel: 1 });
            self.active_lead_notes.clear();
        }

        let play_lead = melody_enabled
            && (trigger_primary.kick || trigger_primary.snare) // Filtrage rythmique: Kick/Snare only
            && !(is_high_tension && is_in_fill_zone) // Call & Response: Silence pendant les fills intenses
            && !self
                .musical_params
                .muted_channels
                .get(1)
                .copied()
                .unwrap_or(false);
        if play_lead {
            let is_strong = trigger_primary.kick;
            let is_new_measure = self.sequencer_primary.current_step == 0;
            // Utilisation du générateur structuré (Motifs + Variations)
            let freq = self.harmony.next_note_structured(is_strong, is_new_measure);
            let melody_midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
            let base_vel = 90 + (self.current_state.arousal * 30.0) as u8;

            // Phase 2: Read from local cache (no lock needed)
            let chord_root_offset = self.last_harmony_state.chord_root_offset;
            let chord_root = 36 + chord_root_offset as u8;

            // Calculer la gamme LCC courante
            let chord = harmonium_core::harmony::chord::Chord::new(
                (chord_root_offset as u8) % 12,
                self.current_chord_type,
            );
            // Use LCC level from unified tension system (couples harmonic and rhythmic tension)
            let lcc_level_num = self.unified_tension.calculate_lcc_level() as u8;
            let lcc_level = harmonium_core::harmony::lydian_chromatic::LccLevel::from_u8(lcc_level_num)
                .unwrap_or(harmonium_core::harmony::lydian_chromatic::LccLevel::Lydian);
            let parent = self.lcc.parent_lydian(&chord);
            let lcc_scale = self.lcc.get_scale(parent, lcc_level);

            // Créer le contexte pour le voicer
            // Utilise les paramètres de voicing dédiés (pas rhythm/harmony)
            let ctx = VoicerContext {
                chord_root_midi: chord_root,
                chord_type: self.current_chord_type,
                lcc_scale,
                tension: self.musical_params.voicing_tension,
                density: self.musical_params.voicing_density,
                current_step: self.sequencer_primary.current_step,
                total_steps: self.sequencer_primary.steps,
            };

            // D'abord: couper toutes les notes précédentes sur le channel Lead
            // Utilise AllNotesOff pour aussi couper le sustain des samples
            if !self.active_lead_notes.is_empty() {
                self.events_buffer.push(AudioEvent::AllNotesOff { channel: 1 });
                self.active_lead_notes.clear();
            }

            // Utiliser le voicer pour décider du style (si activé)
            let voicing_enabled = self.musical_params.enable_voicing;
            if voicing_enabled && self.voicer.should_voice(&ctx) {
                // Beat fort: jouer l'accord complet
                let voiced_notes = self.voicer.process_note(melody_midi, base_vel, &ctx);
                for vn in voiced_notes {
                    self.events_buffer.push(AudioEvent::NoteOn {
                        note: vn.midi,
                        velocity: vn.velocity,
                        channel: 1,
                    });
                    self.active_lead_notes.push(vn.midi);
                }
            } else {
                // Beat faible: jouer la mélodie seule (plus légère)
                let solo_vel = (base_vel as f32 * 0.7) as u8; // Vélocité réduite
                self.events_buffer.push(AudioEvent::NoteOn {
                    note: melody_midi,
                    velocity: solo_vel,
                    channel: 1,
                });
                self.active_lead_notes.push(melody_midi);
            }
        }

        // Snare - part of Rhythm module (avec Ghost Notes et Tom Fills)
        if rhythm_enabled
            && trigger_primary.snare
            && !self.musical_params.muted_channels.get(2).copied().unwrap_or(false)
        {
            let mut snare_note = 38u8; // D1 - Snare standard
            let mut vel =
                self.musical_params.vel_base_snare + (self.current_state.arousal * 30.0) as u8;

            // A. Ghost Notes (Humanisation)
            // Si le pattern rythmique indique un coup faible (< 0.7), on joue une ghost note
            if trigger_primary.velocity < 0.7 {
                vel = (vel as f32 * 0.65) as u8;
                if is_low_energy {
                    snare_note = 37; // Side Stick (Rimshot) pour ambiances calmes
                }
            }

            // B. Tom Fills (Tension)
            // Si haute tension en fin de mesure -> Roulement de Toms
            if is_high_tension && is_in_fill_zone {
                // Sélectionne un Tom (Low 41, Mid 45, High 50) selon le step
                snare_note = match self.sequencer_primary.current_step % 3 {
                    0 => 41, // Low Tom
                    1 => 45, // Mid Tom
                    _ => 50, // High Tom
                };
                vel = (vel as f32 * 1.1).min(127.0) as u8; // Accentue le fill
            }

            self.events_buffer.push(AudioEvent::NoteOn {
                note: snare_note,
                velocity: vel,
                channel: 2,
            });
        }

        // Hat - part of Rhythm module (avec Cymbales & Variations)
        let play_hat = trigger_primary.hat || trigger_secondary.hat;
        if rhythm_enabled
            && play_hat
            && !self.musical_params.muted_channels.get(3).copied().unwrap_or(false)
        {
            let mut hat_note = 42u8; // F#1 - Closed Hi-Hat par défaut
            let mut vel = 70 + (self.current_state.arousal * 30.0) as u8;

            // A. Crash sur le "One" (Explosion d'énergie)
            if self.sequencer_primary.current_step == 0 && is_high_energy {
                hat_note = 49; // Crash Cymbal
                vel = 110;
            }
            // B. Variation Ride / Open Hat (Densité)
            else if is_high_density {
                if self.current_state.tension > 0.7 {
                    hat_note = 51; // Ride Cymbal (Section intense)
                } else if !self.sequencer_primary.current_step.is_multiple_of(2) {
                    hat_note = 46; // Open Hi-Hat (Off-beat)
                }
            }
            // C. Pedal Hat (Calme)
            else if is_low_energy {
                hat_note = 44; // Pedal Hi-Hat (Chick fermé)
            }

            self.events_buffer.push(AudioEvent::NoteOn {
                note: hat_note,
                velocity: vel,
                channel: 3,
            });
        }

        // Send events to renderer
        for event in self.events_buffer.iter() {
            self.renderer.handle_event(event.clone());
        }

        // NEW: Send unified report to UI via lock-free queue
        // Send every 4 ticks to balance update frequency vs allocation cost
        if self.sequencer_primary.current_step.is_multiple_of(4) {
            self.send_report();
        }
    }

    /// Formatte un nom d'accord pour l'UI (numération romaine)
    fn format_chord_name(&self, root_offset: i32, quality: ChordQuality) -> String {
        // Conversion offset → degré de la gamme (simplifié pour pentatonique)
        let roman = match root_offset {
            0 => "I",
            2 => "II",
            3 => "III",
            5 => "IV",
            7 => "V",
            8 => "VI",
            9 => "vi",
            10 => "VII",
            11 => "vii",
            _ => "?",
        };

        let quality_symbol = match quality {
            ChordQuality::Major => "",
            ChordQuality::Minor => "m",
            ChordQuality::Dominant7 => "7",
            ChordQuality::Diminished => "°",
            ChordQuality::Sus2 => "sus2",
        };

        format!("{}{}", roman, quality_symbol)
    }
}
