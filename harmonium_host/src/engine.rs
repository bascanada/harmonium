use std::sync::{Arc, Mutex};

use harmonium_ai::mapper::EmotionMapper;
use harmonium_audio::{
    backend::AudioRenderer,
    voicing::{BlockChordVoicer, Voicer, VoicerContext},
};
pub use harmonium_core::params::{
    ControlMode, CurrentState, EngineParams, HarmonyState, MusicalParams, SessionConfig,
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
    tuning::TuningParams,
};
use rand::Rng;
use rust_music_theory::{note::PitchSymbol, scale::ScaleType};
use triple_buffer::Output;

/// Symbolic snapshot of the engine state for look-ahead simulation
pub struct SymbolicState {
    pub sequencer_primary: Sequencer,
    pub sequencer_secondary: Sequencer,
    pub harmony: HarmonyNavigator,
    pub harmonic_driver: Option<HarmonicDriver>,
    pub harmony_mode: HarmonyMode,
    pub current_progression: Vec<ChordStep>,
    pub progression_index: usize,
    pub measure_counter: usize,
    pub musical_params: MusicalParams,
    pub current_chord_type: ChordType,
    pub voicer: Box<dyn Voicer>,
    pub lcc: LydianChromaticConcept,
    pub last_harmony_state: HarmonyState,
    pub current_state: CurrentState,
}

impl Clone for SymbolicState {
    fn clone(&self) -> Self {
        Self {
            sequencer_primary: self.sequencer_primary.clone(),
            sequencer_secondary: self.sequencer_secondary.clone(),
            harmony: self.harmony.clone(),
            harmonic_driver: self.harmonic_driver.clone(),
            harmony_mode: self.harmony_mode,
            current_progression: self.current_progression.clone(),
            progression_index: self.progression_index,
            measure_counter: self.measure_counter,
            musical_params: self.musical_params.clone(),
            current_chord_type: self.current_chord_type,
            voicer: self.voicer.clone_box(),
            lcc: self.lcc.clone(),
            last_harmony_state: self.last_harmony_state.clone(),
            current_state: self.current_state.clone(),
        }
    }
}

impl SymbolicState {
    /// Advance the symbolic state by one tick and return generated events
    pub fn tick(&mut self, _samples_per_step: usize) -> (usize, Vec<AudioEvent>) {
        let mut events = Vec::new();
        let step_idx = self.sequencer_primary.current_step;

        let trigger_primary = self.sequencer_primary.tick();
        let trigger_secondary = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            self.sequencer_secondary.tick()
        } else {
            StepTrigger::default()
        };

        // --- HARMONY & PROGRESSION ---
        if self.musical_params.enable_harmony && step_idx == 0 {
            self.measure_counter += 1;

            match self.harmony_mode {
                HarmonyMode::Basic => {
                    // Chord Progression
                    let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                    if self.measure_counter.is_multiple_of(measures_per_chord) {
                        self.progression_index =
                            (self.progression_index + 1) % self.current_progression.len();
                        let chord = &self.current_progression[self.progression_index];

                        self.harmony.set_chord_context(chord.root_offset, chord.quality);

                        self.current_chord_type = match chord.quality {
                            ChordQuality::Major => ChordType::Major7,
                            ChordQuality::Minor => ChordType::Minor7,
                            ChordQuality::Dominant7 => ChordType::Dominant7,
                            ChordQuality::Diminished => ChordType::Diminished7,
                            ChordQuality::Sus2 => ChordType::Sus2,
                        };

                        self.last_harmony_state.current_chord_index = self.progression_index;
                        self.last_harmony_state.chord_root_offset = chord.root_offset;
                        self.last_harmony_state.chord_is_minor =
                            matches!(chord.quality, ChordQuality::Minor);
                        self.last_harmony_state.measure_number = self.measure_counter;
                        // Note: chord_name update omitted here for simplicity in symbolic,
                        // but could be added if needed for look-ahead display.
                    }
                }

                HarmonyMode::Driver => {
                    let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                    if self.measure_counter.is_multiple_of(measures_per_chord)
                        && let Some(ref mut driver) = self.harmonic_driver
                    {
                        let mut rng = rand::thread_rng();
                        let decision = driver.next_chord(
                            self.current_state.tension,
                            self.current_state.valence,
                            &mut rng,
                        );

                        let root_offset = driver.root_offset();
                        let quality = driver.to_basic_quality();
                        self.harmony.set_chord_context(root_offset, quality);

                        self.current_chord_type = decision.next_chord.chord_type;

                        self.last_harmony_state.current_chord_index = 0;
                        self.last_harmony_state.chord_root_offset = root_offset;
                        self.last_harmony_state.chord_is_minor = driver.is_minor();
                        self.last_harmony_state.measure_number = self.measure_counter;
                    }
                }
            }
        }

        self.last_harmony_state.current_step = step_idx;

        if step_idx == 0 {
            self.last_harmony_state.primary_steps = self.sequencer_primary.steps;
            self.last_harmony_state.primary_pulses = self.sequencer_primary.pulses;
            self.last_harmony_state.primary_rotation = self.sequencer_primary.rotation;
            let primary_len = self.sequencer_primary.pattern.len();
            for i in 0..192 {
                self.last_harmony_state.primary_pattern[i] =
                    i < primary_len && self.sequencer_primary.pattern[i].is_any();
            }

            self.last_harmony_state.secondary_steps = self.sequencer_secondary.steps;
            self.last_harmony_state.secondary_pulses = self.sequencer_secondary.pulses;
            self.last_harmony_state.secondary_rotation = self.sequencer_secondary.rotation;
            let secondary_len = self.sequencer_secondary.pattern.len();
            for i in 0..192 {
                self.last_harmony_state.secondary_pattern[i] =
                    i < secondary_len && self.sequencer_secondary.pattern[i].is_any();
            }
            self.last_harmony_state.harmony_mode = self.harmony_mode;
        }

        // --- GENERATE EVENTS ---
        let rhythm_enabled = self.musical_params.enable_rhythm;

        let is_high_tension = self.current_state.tension > 0.6;
        let is_high_density = self.current_state.density > 0.6;
        let is_high_energy = self.current_state.arousal > 0.7;
        let is_low_energy = self.current_state.arousal < 0.4;

        let fill_zone_start = self.sequencer_primary.steps.saturating_sub(4);
        let is_in_fill_zone = step_idx >= fill_zone_start;

        if rhythm_enabled
            && trigger_primary.kick
            && !self.musical_params.muted_channels.first().copied().unwrap_or(false)
        {
            let midi_note = if self.musical_params.fixed_kick {
                36
            } else {
                (36 + self.last_harmony_state.chord_root_offset) as u8
            };
            let vel = self.musical_params.vel_base_bass + (self.current_state.arousal * 25.0) as u8;
            events.push(AudioEvent::NoteOn { note: midi_note, velocity: vel, channel: 0 });
        }

        let melody_enabled = self.musical_params.enable_melody;
        let play_lead = melody_enabled
            && (trigger_primary.kick || trigger_primary.snare)
            && !(is_high_tension && is_in_fill_zone)
            && !self.musical_params.muted_channels.get(1).copied().unwrap_or(false);

        if play_lead {
            let is_strong = trigger_primary.kick;
            let is_new_measure = step_idx == 0;
            let freq = self.harmony.next_note_structured(is_strong, is_new_measure);
            let melody_midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
            let base_vel = 90 + (self.current_state.arousal * 30.0) as u8;

            let chord_root_offset = self.last_harmony_state.chord_root_offset;
            let chord_root = 36 + chord_root_offset as u8;

            let chord = harmonium_core::harmony::chord::Chord::new(
                (chord_root_offset as u8) % 12,
                self.current_chord_type,
            );
            let lcc_level = self.lcc.level_for_tension(self.current_state.tension);
            let parent = self.lcc.parent_lydian(&chord);
            let lcc_scale = self.lcc.get_scale(parent, lcc_level);

            let ctx = VoicerContext {
                chord_root_midi: chord_root,
                chord_type: self.current_chord_type,
                lcc_scale,
                tension: self.musical_params.voicing_tension,
                density: self.musical_params.voicing_density,
                current_step: step_idx,
                total_steps: self.sequencer_primary.steps,
            };

            if self.musical_params.enable_voicing && self.voicer.should_voice(&ctx) {
                let voiced_notes = self.voicer.process_note(melody_midi, base_vel, &ctx);
                for vn in voiced_notes {
                    events.push(AudioEvent::NoteOn {
                        note: vn.midi,
                        velocity: vn.velocity,
                        channel: 1,
                    });
                }
            } else {
                let solo_vel = (base_vel as f32 * 0.7) as u8;
                events.push(AudioEvent::NoteOn {
                    note: melody_midi,
                    velocity: solo_vel,
                    channel: 1,
                });
            }
        }

        if rhythm_enabled
            && trigger_primary.snare
            && !self.musical_params.muted_channels.get(2).copied().unwrap_or(false)
        {
            let mut snare_note = 38u8;
            let mut vel =
                self.musical_params.vel_base_snare + (self.current_state.arousal * 30.0) as u8;

            if trigger_primary.velocity < 0.7 {
                vel = (vel as f32 * 0.65) as u8;
                if is_low_energy {
                    snare_note = 37;
                }
            }

            if is_high_tension && is_in_fill_zone {
                snare_note = match step_idx % 3 {
                    0 => 41,
                    1 => 45,
                    _ => 50,
                };
                vel = (vel as f32 * 1.1).min(127.0) as u8;
            }

            events.push(AudioEvent::NoteOn { note: snare_note, velocity: vel, channel: 2 });
        }

        let play_hat = trigger_primary.hat || trigger_secondary.hat;
        if rhythm_enabled
            && play_hat
            && !self.musical_params.muted_channels.get(3).copied().unwrap_or(false)
        {
            let mut hat_note = 42u8;
            let mut vel = 70 + (self.current_state.arousal * 30.0) as u8;

            if step_idx == 0 && is_high_energy {
                hat_note = 49;
                vel = 110;
            } else if is_high_density {
                if self.current_state.tension > 0.7 {
                    hat_note = 51;
                } else if !step_idx.is_multiple_of(2) {
                    hat_note = 46;
                }
            } else if is_low_energy {
                hat_note = 44;
            }

            events.push(AudioEvent::NoteOn { note: hat_note, velocity: vel, channel: 3 });
        }

        (step_idx, events)
    }
}

pub struct HarmoniumEngine {
    pub config: SessionConfig,
    // Phase 3: Lock-free triple buffer for UI→Audio parameter updates
    target_params: Output<EngineParams>,
    // Lock-free queues for Audio→UI communication (Phase 2)
    harmony_state_tx: rtrb::Producer<HarmonyState>, // Audio thread writes harmony state
    event_queue_tx: rtrb::Producer<VisualizationEvent>, // Audio thread writes visualization events
    pub font_queue: crate::FontQueue, // Queue de chargement de SoundFonts (Phase 5 will replace)

    /// The generative symbolic part of the engine (cloneable for look-ahead)
    pub symbolic: SymbolicState,

    renderer: Box<dyn AudioRenderer>,
    sample_rate: f64,

    sample_counter: usize,
    samples_per_step: usize,
    last_pulse_count: usize,
    last_rotation: usize, // Pour détecter les changements de rotation

    _last_valence_choice: f32, // Hystérésis: valence qui a déclenché le dernier choix
    _last_tension_choice: f32, // Hystérésis: tension qui a déclenché le dernier choix

    _active_lead_notes: Vec<u8>, // Notes actuellement jouées sur le channel Lead
    active_bass_note: Option<u8>, // Note de basse actuellement jouée

    // === NOUVELLE ARCHITECTURE: Params Musicaux Découplés ===
    /// Mapper émotions → params musicaux
    emotion_mapper: EmotionMapper,
    /// État partagé pour le mode de contrôle (émotion vs direct)
    control_mode: Arc<Mutex<ControlMode>>,

    // Recording State Tracking
    is_recording_wav: bool,
    is_recording_midi: bool,
    is_recording_musicxml: bool,
    is_recording_truth: bool,

    // Mute State Tracking
    last_muted_channels: Vec<bool>,

    // Phase 2.5: Pre-allocated buffer for event generation (no Vec::new() in audio thread)
    events_buffer: Vec<AudioEvent>,

    // Phase 3: Tuning parameters for algorithmic tuning (LLM iteration loop)
    tuning: Option<TuningParams>,
}

impl HarmoniumEngine {
    pub fn new(
        sample_rate: f64,
        mut target_params: Output<EngineParams>,
        control_mode: Arc<Mutex<ControlMode>>,
        mut renderer: Box<dyn AudioRenderer>,
    ) -> (Self, rtrb::Consumer<HarmonyState>, rtrb::Consumer<VisualizationEvent>) {
        let mut rng = rand::thread_rng();
        let initial_params = target_params.read().clone();
        let font_queue = Arc::new(Mutex::new(Vec::new()));
        let bpm = initial_params.compute_bpm();
        let steps = 16;
        let initial_pulses = std::cmp::min((initial_params.density * 11.0) as usize + 1, 16);
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

        // Phase 2: Create lock-free SPSC queues for Audio→UI communication
        // harmony_state: 256 slots (~64 seconds buffer at 120 BPM, 4 updates/sec)
        // event_queue: 4096 slots (~60 seconds buffer at 120 BPM, every step)
        let (harmony_state_tx, harmony_state_rx) = rtrb::RingBuffer::new(256);
        let (event_queue_tx, event_queue_rx) = rtrb::RingBuffer::new(4096);
        let last_harmony_state = HarmonyState::default();

        // Séquenceurs
        let sequencer_primary = Sequencer::new(steps, initial_pulses, bpm);
        let secondary_pulses = std::cmp::min((initial_params.density * 8.0) as usize + 1, 12);
        let sequencer_secondary = Sequencer::new_with_rotation(12, secondary_pulses, bpm, 0);

        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);
        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        // Initialize renderer timing
        renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step });

        // Progression initiale
        let current_progression =
            Progression::get_palette(initial_params.valence, initial_params.tension);
        let _progression_name =
            Progression::get_progression_name(initial_params.valence, initial_params.tension);

        // HarmonicDriver (toujours créé pour permettre le switch dynamique)
        let harmony_mode = initial_params.harmony_mode;
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

        // Phase 2: Initialize harmony state cache with default values
        // (will be sent via queue in tick())
        // Note: No longer initializing mutex-based harmony_state

        // Créer le mapper et les params musicaux initiaux
        let emotion_mapper = EmotionMapper::new();
        let musical_params = emotion_mapper.map(&initial_params);

        // Initialize session key/scale in control_mode for UI
        if let Ok(mut mode) = control_mode.lock() {
            mode.session_key = config.key.clone();
            mode.session_scale = config.scale.clone();
        }

        let symbolic = SymbolicState {
            sequencer_primary,
            sequencer_secondary,
            harmony,
            harmonic_driver,
            harmony_mode,
            current_progression,
            progression_index: 0,
            measure_counter: 0,
            musical_params,
            current_chord_type: ChordType::Major,
            voicer: Box::new(BlockChordVoicer::new(4)),
            lcc: LydianChromaticConcept::new(),
            last_harmony_state,
            current_state: CurrentState::default(),
        };

        let engine = Self {
            config,
            target_params,
            harmony_state_tx,
            event_queue_tx,
            font_queue,
            symbolic,
            renderer,
            sample_rate,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
            last_rotation: 0,
            _last_valence_choice: initial_params.valence,
            _last_tension_choice: initial_params.tension,
            _active_lead_notes: Vec::with_capacity(8),
            active_bass_note: None,
            emotion_mapper,
            control_mode,
            is_recording_wav: false,
            is_recording_midi: false,
            is_recording_musicxml: false,
            is_recording_truth: false,
            last_muted_channels: vec![false; 16],
            events_buffer: Vec::with_capacity(8),
            tuning: None,
        };

        // Return engine and consumers (Phase 2: lock-free queues)
        (engine, harmony_state_rx, event_queue_rx)
    }

    /// Create engine with custom tuning parameters (Phase 3: LLM tuning loop)
    ///
    /// This constructor uses `TuningParams` to configure algorithm subsystems
    /// instead of hardcoded defaults, enabling iterative parameter optimization.
    pub fn with_tuning(
        sample_rate: f64,
        mut target_params: Output<EngineParams>,
        control_mode: Arc<Mutex<ControlMode>>,
        mut renderer: Box<dyn AudioRenderer>,
        tuning: TuningParams,
    ) -> (Self, rtrb::Consumer<HarmonyState>, rtrb::Consumer<VisualizationEvent>) {
        let mut rng = rand::thread_rng();
        let initial_params = target_params.read().clone();
        let font_queue = Arc::new(Mutex::new(Vec::new()));
        let bpm = initial_params.compute_bpm();
        let steps = 16;
        let initial_pulses = std::cmp::min((initial_params.density * 11.0) as usize + 1, 16);
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

        log::info(&format!("Session (tuned): {} {} | BPM: {:.1}", config.key, config.scale, bpm));

        // Phase 2: Create lock-free SPSC queues for Audio→UI communication
        let (harmony_state_tx, harmony_state_rx) = rtrb::RingBuffer::new(256);
        let (event_queue_tx, event_queue_rx) = rtrb::RingBuffer::new(4096);
        let last_harmony_state = HarmonyState::default();

        // === Sequencers with TuningParams ===
        let sequencer_primary =
            Sequencer::from_tuning(steps, initial_pulses, bpm, RhythmMode::Euclidean, &tuning);
        let secondary_pulses = std::cmp::min((initial_params.density * 8.0) as usize + 1, 12);
        let sequencer_secondary =
            Sequencer::from_tuning(12, secondary_pulses, bpm, RhythmMode::Euclidean, &tuning);

        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);
        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        // Initialize renderer timing
        renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step });

        // Progression initiale
        let current_progression =
            Progression::get_palette(initial_params.valence, initial_params.tension);

        // === HarmonicDriver with TuningParams ===
        let harmony_mode = initial_params.harmony_mode;
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
        let harmonic_driver = Some(HarmonicDriver::from_tuning(key_pc, &tuning));

        // Créer le mapper et les params musicaux initiaux
        let emotion_mapper = EmotionMapper::new();
        let musical_params = emotion_mapper.map(&initial_params);

        // Initialize session key/scale in control_mode for UI
        if let Ok(mut mode) = control_mode.lock() {
            mode.session_key = config.key.clone();
            mode.session_scale = config.scale.clone();
        }

        let symbolic = SymbolicState {
            sequencer_primary,
            sequencer_secondary,
            harmony,
            harmonic_driver,
            harmony_mode,
            current_progression,
            progression_index: 0,
            measure_counter: 0,
            musical_params,
            current_chord_type: ChordType::Major,
            voicer: Box::new(BlockChordVoicer::new(4)),
            lcc: LydianChromaticConcept::new(),
            last_harmony_state,
            current_state: CurrentState::default(),
        };

        let engine = Self {
            config,
            target_params,
            harmony_state_tx,
            event_queue_tx,
            font_queue,
            symbolic,
            renderer,
            sample_rate,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
            last_rotation: 0,
            _last_valence_choice: initial_params.valence,
            _last_tension_choice: initial_params.tension,
            _active_lead_notes: Vec::with_capacity(8),
            active_bass_note: None,
            emotion_mapper,
            control_mode,
            is_recording_wav: false,
            is_recording_midi: false,
            is_recording_musicxml: false,
            is_recording_truth: false,
            last_muted_channels: vec![false; 16],
            events_buffer: Vec::with_capacity(8),
            tuning: Some(tuning),
        };

        (engine, harmony_state_rx, event_queue_rx)
    }

    /// Returns the tuning parameters if available
    #[must_use]
    pub const fn tuning(&self) -> Option<&TuningParams> {
        self.tuning.as_ref()
    }

    /// Change le voicer dynamiquement
    pub fn set_voicer(&mut self, voicer: Box<dyn Voicer>) {
        self.symbolic.voicer = voicer;
    }

    /// Retourne le nom du voicer actuel
    pub fn current_voicer_name(&self) -> &'static str {
        self.symbolic.voicer.name()
    }

    /// Captures a symbolic snapshot of the engine for look-ahead simulation
    pub fn get_symbolic_snapshot(&self) -> SymbolicState {
        self.symbolic.clone()
    }

    /// Envoie un événement au moteur (via le renderer)
    pub fn handle_event(&mut self, event: AudioEvent) {
        self.renderer.handle_event(event);
    }

    // ═══════════════════════════════════════════════════════════════════
    // NOUVELLE API: Contrôle Direct des Paramètres Musicaux
    // ═══════════════════════════════════════════════════════════════════

    /// Récupère une copie des paramètres musicaux actuels
    pub fn get_musical_params(&self) -> MusicalParams {
        self.symbolic.musical_params.clone()
    }

    /// Récupère le mapper pour configuration (seuils, courbes, etc.)
    pub fn emotion_mapper_mut(&mut self) -> &mut EmotionMapper {
        &mut self.emotion_mapper
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

    fn update_controls(&mut self) {
        // === UPDATE MUSICAL PARAMS ===
        // Phase 3: Read latest params from triple buffer (lock-free, non-blocking)
        self.target_params.update();
        let target_params = self.target_params.read();

        // Selon le mode, on obtient les params soit du mapper, soit directement
        let use_emotion_mode = self.control_mode.lock().map(|m| m.use_emotion_mode).unwrap_or(true);

        if use_emotion_mode {
            // Mode émotionnel: EngineParams → EmotionMapper → MusicalParams
            self.symbolic.musical_params = self.emotion_mapper.map(target_params);
        } else {
            // Mode direct: MusicalParams depuis l'état partagé
            if let Ok(guard) = self.control_mode.try_lock() {
                self.symbolic.musical_params = guard.direct_params.clone();
            }
        }

        // Apply params from target_params (they work in BOTH modes)
        self.symbolic.musical_params.gain_lead = target_params.gain_lead;
        self.symbolic.musical_params.gain_bass = target_params.gain_bass;
        self.symbolic.musical_params.gain_snare = target_params.gain_snare;
        self.symbolic.musical_params.gain_hat = target_params.gain_hat;
        self.symbolic.musical_params.vel_base_bass = target_params.vel_base_bass;
        self.symbolic.musical_params.vel_base_snare = target_params.vel_base_snare;

        // Apply global enable overrides (work in BOTH modes)
        if let Ok(guard) = self.control_mode.try_lock() {
            self.symbolic.musical_params.enable_rhythm = guard.enable_rhythm;
            self.symbolic.musical_params.enable_harmony = guard.enable_harmony;
            self.symbolic.musical_params.enable_melody = guard.enable_melody;
            self.symbolic.musical_params.enable_voicing = guard.enable_voicing;
        }

        let mp = &self.symbolic.musical_params; // Raccourci pour la lisibilité

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
        if self.symbolic.harmony_mode != mp.harmony_mode {
            self.symbolic.harmony_mode = mp.harmony_mode;
        }

        // === MORPHING (smooth transitions) ===
        let morph_factor = 0.03;

        self.symbolic.current_state.bpm +=
            (mp.bpm - self.symbolic.current_state.bpm) * morph_factor;
        self.symbolic.current_state.density +=
            (mp.rhythm_density - self.symbolic.current_state.density) * morph_factor;
        self.symbolic.current_state.tension +=
            (mp.harmony_tension - self.symbolic.current_state.tension) * morph_factor;
        self.symbolic.current_state.smoothness +=
            (mp.melody_smoothness - self.symbolic.current_state.smoothness) * morph_factor;
        self.symbolic.current_state.valence +=
            (mp.harmony_valence - self.symbolic.current_state.valence) * morph_factor;

        let arousal_from_bpm = (mp.bpm - 70.0) / 110.0;
        self.symbolic.current_state.arousal +=
            (arousal_from_bpm - self.symbolic.current_state.arousal) * morph_factor;

        // === SYNTHESIS MORPHING (emotional timbre control) ===
        #[cfg(feature = "odin2")]
        if target_params.enable_synthesis_morphing
            && let Some(odin2) = self.renderer.odin2_backend_mut()
        {
            odin2.apply_emotional_morphing(
                self.symbolic.current_state.valence,
                self.symbolic.current_state.arousal,
                self.symbolic.current_state.tension,
                self.symbolic.current_state.density,
            );
        }

        // === MELODY SMOOTHNESS → Hurst Factor ===
        self.symbolic.harmony.set_hurst_factor(mp.melody_smoothness);

        // === VOICING DENSITY → Comping Pattern ===
        self.symbolic
            .voicer
            .on_density_change(mp.voicing_density, self.symbolic.sequencer_primary.steps);

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

        if mp.record_truth != self.is_recording_truth {
            self.is_recording_truth = mp.record_truth;
            if self.is_recording_truth {
                self.renderer.handle_event(AudioEvent::StartRecording {
                    format: harmonium_core::events::RecordFormat::Truth,
                });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording {
                    format: harmonium_core::events::RecordFormat::Truth,
                });
            }
        }

        // === DSP UPDATES (effets globaux sur channel 0) ===
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 1,
            value: (mp.voicing_tension * 127.0) as u8,
            channel: 0,
        });
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 11,
            value: (self.symbolic.current_state.arousal * 127.0) as u8,
            channel: 0,
        });
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 91,
            value: (self.symbolic.current_state.valence.abs() * 127.0) as u8,
            channel: 0,
        });

        // === SKIP RHYTHM IF DISABLED ===
        if !mp.enable_rhythm {
            // Timing only (pour garder le moteur synchronisé)
            let steps_per_beat = (self.symbolic.sequencer_primary.steps / 4) as f64;
            let new_samples_per_step = (self.sample_rate * 60.0
                / (self.symbolic.current_state.bpm as f64)
                / steps_per_beat) as usize;
            if new_samples_per_step != self.samples_per_step {
                self.samples_per_step = new_samples_per_step;
                self.renderer.handle_event(AudioEvent::TimingUpdate {
                    samples_per_step: new_samples_per_step,
                });
                // Send updated musical params if recording is active
                if self.is_recording_musicxml || self.is_recording_midi {
                    self.renderer.handle_event(AudioEvent::UpdateMusicalParams {
                        params: Box::new(self.symbolic.musical_params.clone()),
                    });
                }
            }
            return; // Skip sequencer logic when rhythm disabled
        }

        // === LOGIQUE SÉQUENCEUR (from MusicalParams) ===
        let target_algo = mp.rhythm_mode;
        let mode_changed = self.symbolic.sequencer_primary.mode != target_algo;

        if mode_changed {
            self.symbolic.sequencer_primary.mode = target_algo;
            self.symbolic.sequencer_primary.upgrade_to_steps(mp.rhythm_steps);
        }

        // Update steps if changed while playing
        if self.symbolic.sequencer_primary.steps != mp.rhythm_steps {
            self.symbolic.sequencer_primary.upgrade_to_steps(mp.rhythm_steps);
        }

        // Rotation (from MusicalParams - même valeur dans les deux modes)
        let target_rotation = mp.rhythm_rotation;

        // Pulses (from MusicalParams)
        let target_pulses = if self.symbolic.sequencer_primary.mode == RhythmMode::Euclidean {
            mp.rhythm_pulses.min(self.symbolic.sequencer_primary.steps)
        } else {
            self.symbolic.sequencer_primary.pulses
        };

        // Regeneration Logic
        let needs_regen = mode_changed
            || if self.symbolic.sequencer_primary.mode == RhythmMode::Euclidean {
                target_pulses != self.last_pulse_count
            } else {
                (mp.rhythm_density - self.symbolic.sequencer_primary.density).abs() > 0.05
                    || (mp.rhythm_tension - self.symbolic.sequencer_primary.tension).abs() > 0.05
            };

        if needs_regen {
            self.symbolic.sequencer_primary.tension = mp.rhythm_tension;
            self.symbolic.sequencer_primary.density = mp.rhythm_density;
            self.symbolic.sequencer_primary.pulses = target_pulses;
            self.symbolic.sequencer_primary.regenerate_pattern();
            self.last_pulse_count = target_pulses;
        }

        if target_rotation != self.last_rotation {
            self.symbolic.sequencer_primary.set_rotation(target_rotation);
            self.last_rotation = target_rotation;
        }

        // Secondary Sequencer Logic (from MusicalParams) - Euclidean mode only
        let secondary_steps = mp.rhythm_secondary_steps;
        let secondary_pulses = mp.rhythm_secondary_pulses.min(secondary_steps);
        let secondary_rotation = mp.rhythm_secondary_rotation;

        let secondary_changed = secondary_steps != self.symbolic.sequencer_secondary.steps
            || secondary_pulses != self.symbolic.sequencer_secondary.pulses
            || secondary_rotation != self.symbolic.sequencer_secondary.rotation;

        if secondary_changed {
            if secondary_steps != self.symbolic.sequencer_secondary.steps {
                self.symbolic.sequencer_secondary.steps = secondary_steps;
                self.symbolic.sequencer_secondary.pattern =
                    vec![StepTrigger::default(); secondary_steps];
            }
            self.symbolic.sequencer_secondary.pulses = secondary_pulses;
            self.symbolic.sequencer_secondary.rotation = secondary_rotation;
            self.symbolic.sequencer_secondary.regenerate_pattern();
        }

        // Timing
        let steps_per_beat = (self.symbolic.sequencer_primary.steps / 4) as f64;
        let new_samples_per_step = (self.sample_rate * 60.0
            / (self.symbolic.current_state.bpm as f64)
            / steps_per_beat) as usize;
        if new_samples_per_step != self.samples_per_step {
            self.samples_per_step = new_samples_per_step;
            self.renderer
                .handle_event(AudioEvent::TimingUpdate { samples_per_step: new_samples_per_step });
            // Send updated musical params if recording is active
            if self.is_recording_musicxml || self.is_recording_midi {
                self.renderer.handle_event(AudioEvent::UpdateMusicalParams {
                    params: Box::new(self.symbolic.musical_params.clone()),
                });
            }
        }
    }

    fn tick(&mut self) {
        // Advance the symbolic state and get generated events
        let (step_idx, events) = self.symbolic.tick(self.samples_per_step);

        // Handle physical side effects (audio engine only)
        self.events_buffer.clear();

        // Stop previous bass note (Staccato / Note Switching) to prevent infinite sustain
        if let Some(old_note) = self.active_bass_note {
            self.renderer.handle_event(AudioEvent::NoteOff { note: old_note, channel: 0 });
            self.active_bass_note = None;
        }

        for event in events {
            self.renderer.handle_event(event.clone());
            self.events_buffer.push(event.clone());

            // Phase 2: Send NoteOn events to UI via lock-free queue
            if let AudioEvent::NoteOn { note, channel, .. } = event {
                // Track active bass note for NoteOff next tick
                if channel == 0 {
                    self.active_bass_note = Some(note);
                }

                let vis_event = VisualizationEvent {
                    note_midi: note,
                    instrument: channel,
                    step: step_idx,
                    duration_samples: self.samples_per_step,
                };
                // Push to SPSC queue (non-blocking, drops if full)
                let _ = self.event_queue_tx.push(vis_event);
            }
        }

        // Update harmony state to queue for UI updates
        // We push every tick to ensure the UI has the most up-to-date information
        // (the queue is lock-free SPSC so this is fast)
        let _ = self.harmony_state_tx.push(self.symbolic.last_harmony_state.clone());
    }

    /// Formatte un nom d'accord pour l'UI (numération romaine)
    fn _format_chord_name(&self, root_offset: i32, quality: ChordQuality) -> String {
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

    // =========================================================================
    // DNA EXPORT
    // =========================================================================

    /// Export Musical DNA from a RecordingTruth
    ///
    /// This method extracts the complete Musical DNA profile from recorded
    /// events, including harmonic and rhythmic characteristics.
    ///
    /// # Arguments
    /// * `truth` - The RecordingTruth containing events and parameters
    ///
    /// # Returns
    /// A `MusicalDNA` struct containing the extracted profile
    #[must_use]
    pub fn export_dna(truth: &harmonium_core::truth::RecordingTruth) -> harmonium_core::MusicalDNA {
        harmonium_core::MusicalDNA::extract(truth)
    }

    /// Export Musical DNA to JSON string
    ///
    /// Convenience method that extracts DNA and serializes to JSON.
    ///
    /// # Errors
    /// Returns error if JSON serialization fails
    pub fn export_dna_json(
        truth: &harmonium_core::truth::RecordingTruth,
    ) -> Result<String, serde_json::Error> {
        let dna = Self::export_dna(truth);
        dna.to_json()
    }
}
