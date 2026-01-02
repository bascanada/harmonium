use crate::sequencer::{Sequencer, RhythmMode, StepTrigger};
use crate::harmony::HarmonyNavigator;
use crate::progression::{Progression, ChordStep, ChordQuality};
use crate::log;
use crate::events::AudioEvent;
use crate::backend::AudioRenderer;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;
use rand::Rng;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug)]
pub struct VisualizationEvent {
    pub note_midi: u8,
    pub instrument: u8, // 0=Bass, 1=Lead, 2=Snare, 3=Hat
    pub step: usize,
    pub duration_samples: usize,
}

/// État harmonique en lecture seule pour l'UI
/// Permet d'afficher l'accord courant, la mesure, le cycle, etc.
#[derive(Clone, Debug)]
pub struct HarmonyState {
    pub current_chord_index: usize,  // Position dans progression actuelle
    pub chord_root_offset: i32,      // Décalage en demi-tons (0=I, 5=IV, 7=V, 9=vi)
    pub chord_is_minor: bool,        // true si accord mineur
    pub chord_name: String,          // "I", "vi", "IV", "V"
    pub measure_number: usize,       // Numéro de mesure (1, 2, 3...)
    pub cycle_number: usize,         // Numéro de cycle complet (1, 2, 3...)
    pub current_step: usize,         // Step dans la mesure (0-15 ou 0-47)
    pub progression_name: String,    // Nom de la progression active ("Pop Energetic", etc.)
    pub progression_length: usize,   // Longueur de la progression (2-4 accords)
    // Visualisation Rythmique
    pub primary_steps: usize,        // Nombre de steps (16 ou 48)
    pub primary_pulses: usize,
    pub secondary_steps: usize,      // Nombre de steps secondaire (12)
    pub secondary_pulses: usize,
    pub primary_rotation: usize,
    pub secondary_rotation: usize,
    // Patterns réels pour visualisation
    pub primary_pattern: Vec<bool>,   // Pattern du séquenceur primaire
    pub secondary_pattern: Vec<bool>, // Pattern du séquenceur secondaire
}

impl Default for HarmonyState {
    fn default() -> Self {
        HarmonyState {
            current_chord_index: 0,
            chord_root_offset: 0,
            chord_is_minor: false,
            chord_name: "I".to_string(),
            measure_number: 1,
            cycle_number: 1,
            current_step: 0,
            progression_name: "Folk Peaceful (I-IV-I-V)".to_string(),
            progression_length: 4,
            primary_steps: 16,
            primary_pulses: 4,
            secondary_steps: 12,
            secondary_pulses: 3,
            primary_rotation: 0,
            secondary_rotation: 0,
            primary_pattern: vec![false; 16],
            secondary_pattern: vec![false; 12],
        }
    }
}

/// État cible (Target) - Ce que l'IA demande
/// Basé sur le modèle dimensionnel des émotions (Russell's Circumplex Model)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineParams {
    pub arousal: f32,   // 0.0 à 1.0 - Activation/Énergie → contrôle BPM
    pub valence: f32,   // -1.0 à 1.0 - Positif/Négatif → contrôle Harmonie (Majeur/Mineur)
    pub density: f32,   // 0.0 à 1.0 - Complexité rythmique
    pub tension: f32,   // 0.0 à 1.0 - Dissonance harmonique
    pub smoothness: f32, // 0.0 à 1.0 - Lissage mélodique (Hurst)
    #[serde(default)]
    pub algorithm: RhythmMode, // Euclidean (16 steps) ou PerfectBalance (48 steps)
    #[serde(default)]
    pub channel_routing: Vec<i32>, // -1 = FundSP, >=0 = Oxisynth Bank ID
    
    // Recording Control
    #[serde(default)]
    pub record_wav: bool,
    #[serde(default)]
    pub record_midi: bool,
}

impl Default for EngineParams {
    fn default() -> Self {
        EngineParams {
            arousal: 0.5,   // Énergie moyenne
            valence: 0.3,   // Légèrement positif
            density: 0.2,   // < 0.3 = Carré (4 côtés), > 0.3 = Hexagone (6 côtés)
            tension: 0.4,   // > 0.3 active le Triangle → polyrythme 4:3
            smoothness: 0.7, // Mélodie assez lisse par défaut
            algorithm: RhythmMode::Euclidean, // Mode classique par défaut
            channel_routing: vec![-1; 16], // Tout en FundSP par défaut
            record_wav: false,
            record_midi: false,
        }
    }
}

impl EngineParams {
    /// Calcule le BPM basé sur l'arousal (activation émotionnelle)
    /// Faible arousal (calme) → 70 BPM
    /// Haute arousal (excité) → 180 BPM
    pub fn compute_bpm(&self) -> f32 {
        70.0 + (self.arousal * 110.0)
    }
}

/// État actuel (Current) - Pour le lissage/morphing
#[derive(Clone, Debug)]
pub struct CurrentState {
    pub arousal: f32,
    pub valence: f32,
    pub density: f32,
    pub tension: f32,
    pub smoothness: f32,
    pub bpm: f32,  // Calculé à partir de arousal
}

impl Default for CurrentState {
    fn default() -> Self {
        CurrentState {
            arousal: 0.5,
            valence: 0.3,
            density: 0.4,
            tension: 0.2,
            smoothness: 0.7,
            bpm: 125.0,  // (0.5 * 110) + 70
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub bpm: f32,
    pub key: String,
    pub scale: String,
    pub pulses: usize,
    pub steps: usize,
}

pub struct HarmoniumEngine {
    pub config: SessionConfig,
    pub target_state: Arc<Mutex<EngineParams>>,
    pub harmony_state: Arc<Mutex<HarmonyState>>,  // État harmonique pour l'UI
    pub event_queue: Arc<Mutex<Vec<VisualizationEvent>>>, // Queue d'événements pour l'UI
    pub font_queue: Arc<Mutex<Vec<(u32, Vec<u8>)>>>, // Queue de chargement de SoundFonts
    current_state: CurrentState,
    // === POLYRYTHMIE: Plusieurs séquenceurs avec cycles différents ===
    sequencer_primary: Sequencer,    // Cycle principal (16 steps)
    sequencer_secondary: Sequencer,  // Cycle secondaire (12 steps) - déphasage de Steve Reich
    harmony: HarmonyNavigator,
    
    renderer: Box<dyn AudioRenderer>,
    sample_rate: f64,

    sample_counter: usize,
    samples_per_step: usize,
    last_pulse_count: usize,
    last_rotation: usize,  // Pour détecter les changements de rotation
    // === PROGRESSION HARMONIQUE ADAPTATIVE ===
    measure_counter: usize,               // Compte les mesures (16 steps = 1 mesure)
    current_progression: Vec<ChordStep>,  // Progression chargée (dépend de valence/tension)
    progression_index: usize,             // Position dans la progression actuelle
    last_valence_choice: f32,             // Hystérésis: valence qui a déclenché le dernier choix
    last_tension_choice: f32,             // Hystérésis: tension qui a déclenché le dernier choix
    
    // Optimization
    cached_target: EngineParams,
    
    // Recording State Tracking
    is_recording_wav: bool,
    is_recording_midi: bool,
}

impl HarmoniumEngine {
    pub fn new(sample_rate: f64, target_state: Arc<Mutex<EngineParams>>, mut renderer: Box<dyn AudioRenderer>) -> Self {
        let mut rng = rand::thread_rng();
        let initial_params = target_state.lock().unwrap().clone();
        let font_queue = Arc::new(Mutex::new(Vec::new()));
        let bpm = initial_params.compute_bpm();
        let steps = 16;
        let initial_pulses = std::cmp::min((initial_params.density * 11.0) as usize + 1, 16);
        let keys = [PitchSymbol::C, PitchSymbol::D, PitchSymbol::E, PitchSymbol::F, PitchSymbol::G, PitchSymbol::A, PitchSymbol::B];
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

        let harmony_state = Arc::new(Mutex::new(HarmonyState::default()));
        let event_queue = Arc::new(Mutex::new(Vec::new()));

        // Séquenceurs
        let sequencer_primary = Sequencer::new(steps, initial_pulses, bpm);
        let secondary_pulses = std::cmp::min((initial_params.density * 8.0) as usize + 1, 12);
        let sequencer_secondary = Sequencer::new_with_rotation(12, secondary_pulses, bpm, 0);
        
        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);
        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;
        
        // Initialize renderer timing
        renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step });

        // Progression initiale
        let current_progression = Progression::get_palette(initial_params.valence, initial_params.tension);
        let progression_name = Progression::get_progression_name(initial_params.valence, initial_params.tension);
        
        {
            let mut state = harmony_state.lock().unwrap();
            state.progression_name = progression_name.to_string();
            state.progression_length = current_progression.len();
        }

        Self {
            config,
            target_state,
            harmony_state,
            event_queue,
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
            measure_counter: 0,
            current_progression,
            progression_index: 0,
            last_valence_choice: initial_params.valence,
            last_tension_choice: initial_params.tension,
            cached_target: initial_params,
            is_recording_wav: false,
            is_recording_midi: false,
        }
    }



    pub fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        let total_samples = output.len() / channels;
        let mut processed = 0;
        
        // Run control logic once per block
        self.update_controls();
        
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
                self.tick();
            }
        }
    }

    fn update_controls(&mut self) {
        if let Ok(guard) = self.target_state.try_lock() {
            self.cached_target = guard.clone();
        }
        
        // === LOAD FONTS ===
        if let Ok(mut queue) = self.font_queue.try_lock() {
            while let Some((id, bytes)) = queue.pop() {
                self.renderer.handle_event(AudioEvent::LoadFont { id, bytes });
            }
        }

        let target = &self.cached_target;

        // === SYNC ROUTING ===
        for (i, &mode) in target.channel_routing.iter().enumerate() {
            if i < 16 {
                    self.renderer.handle_event(AudioEvent::SetChannelRoute { channel: i as u8, bank: mode });
            }
        }

        // === MORPHING ===
        self.current_state.arousal += (target.arousal - self.current_state.arousal) * 0.001;
        self.current_state.valence += (target.valence - self.current_state.valence) * 0.001;
        self.current_state.density += (target.density - self.current_state.density) * 0.0005;
        self.current_state.tension += (target.tension - self.current_state.tension) * 0.002;
        self.current_state.smoothness += (target.smoothness - self.current_state.smoothness) * 0.001;
        let target_bpm = target.compute_bpm();
        self.current_state.bpm += (target_bpm - self.current_state.bpm) * 0.001;

        // === RECORDING CONTROL ===
        if target.record_wav != self.is_recording_wav {
            self.is_recording_wav = target.record_wav;
            if self.is_recording_wav {
                self.renderer.handle_event(AudioEvent::StartRecording { format: crate::events::RecordFormat::Wav });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording { format: crate::events::RecordFormat::Wav });
            }
        }
        
        if target.record_midi != self.is_recording_midi {
            self.is_recording_midi = target.record_midi;
            if self.is_recording_midi {
                self.renderer.handle_event(AudioEvent::StartRecording { format: crate::events::RecordFormat::Midi });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording { format: crate::events::RecordFormat::Midi });
            }
        }

        // === DSP UPDATES ===
        self.renderer.handle_event(AudioEvent::ControlChange { ctrl: 1, value: (self.current_state.tension * 127.0) as u8 });
        self.renderer.handle_event(AudioEvent::ControlChange { ctrl: 11, value: (self.current_state.arousal * 127.0) as u8 });
        self.renderer.handle_event(AudioEvent::ControlChange { ctrl: 91, value: (self.current_state.valence.abs() * 127.0) as u8 });
        
        // === LOGIQUE SÉQUENCEUR ===
        let target_algo = target.algorithm;
        let mode_changed = self.sequencer_primary.mode != target_algo;
        
        if mode_changed {
            self.sequencer_primary.mode = target_algo;
            if target_algo == RhythmMode::PerfectBalance {
                self.sequencer_primary.upgrade_to_48_steps();
            }
        }

        // Rotation Logic
        let max_rotation = if self.sequencer_primary.mode == RhythmMode::PerfectBalance { 24 } else { 8 };
        let target_rotation = (self.current_state.tension * max_rotation as f32) as usize;

        // Regeneration Logic
        let target_pulses = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            std::cmp::min((self.current_state.density * 11.0) as usize + 1, self.sequencer_primary.steps)
        } else {
            self.sequencer_primary.pulses
        };

        let needs_regen = mode_changed || if self.sequencer_primary.mode == RhythmMode::Euclidean {
            target_pulses != self.last_pulse_count
        } else {
            (self.current_state.density - self.sequencer_primary.density).abs() > 0.05 ||
            (self.current_state.tension - self.sequencer_primary.tension).abs() > 0.05
        };

        if needs_regen {
            self.sequencer_primary.tension = self.current_state.tension;
            self.sequencer_primary.density = self.current_state.density;
            self.sequencer_primary.pulses = target_pulses;
            self.sequencer_primary.regenerate_pattern();
            self.last_pulse_count = target_pulses;
        }

        if target_rotation != self.last_rotation {
            self.sequencer_primary.set_rotation(target_rotation);
            self.last_rotation = target_rotation;
        }

        // Secondary Sequencer Logic
        let secondary_pulses = std::cmp::min((self.current_state.density * 8.0) as usize + 1, 12);
        if secondary_pulses != self.sequencer_secondary.pulses {
            self.sequencer_secondary.pulses = secondary_pulses;
            self.sequencer_secondary.set_rotation(8 - (target_rotation % 8));
            self.sequencer_secondary.regenerate_pattern();
        }

        // Timing
        let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
        let new_samples_per_step = (self.sample_rate * 60.0 / (self.current_state.bpm as f64) / steps_per_beat) as usize;
        if new_samples_per_step != self.samples_per_step { 
            self.samples_per_step = new_samples_per_step; 
            self.renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step: new_samples_per_step });
        }
    }

    fn tick(&mut self) {
        let trigger_primary = self.sequencer_primary.tick();
        let trigger_secondary = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            self.sequencer_secondary.tick()
        } else {
            StepTrigger::default()
        };

        // === HARMONY & PROGRESSION ===
        if self.sequencer_primary.current_step == 0 {
            self.measure_counter += 1;
            
            // Palette Selection (Hysteresis)
            if self.measure_counter % 4 == 0 {
                let valence_delta = (self.current_state.valence - self.last_valence_choice).abs();
                let tension_delta = (self.current_state.tension - self.last_tension_choice).abs();
                
                if valence_delta > 0.4 || tension_delta > 0.4 {
                    self.current_progression = Progression::get_palette(self.current_state.valence, self.current_state.tension);
                    self.progression_index = 0;
                    self.last_valence_choice = self.current_state.valence;
                    self.last_tension_choice = self.current_state.tension;
                    
                    let prog_name = Progression::get_progression_name(self.current_state.valence, self.current_state.tension);
                    if let Ok(mut state) = self.harmony_state.lock() {
                        state.progression_name = prog_name.to_string();
                        state.progression_length = self.current_progression.len();
                    }
                }
            }
            
            // Chord Progression
            let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
            if self.measure_counter % measures_per_chord == 0 {
                self.progression_index = (self.progression_index + 1) % self.current_progression.len();
                let chord = &self.current_progression[self.progression_index];
                
                self.harmony.set_chord_context(chord.root_offset, chord.quality);
                let chord_name = self.format_chord_name(chord.root_offset, chord.quality);
                
                if let Ok(mut state) = self.harmony_state.lock() {
                    state.current_chord_index = self.progression_index;
                    state.chord_root_offset = chord.root_offset;
                    state.chord_is_minor = matches!(chord.quality, ChordQuality::Minor);
                    state.chord_name = chord_name;
                    state.measure_number = self.measure_counter;
                }
            }
        }

        // UI Update
        if let Ok(mut state) = self.harmony_state.lock() {
            state.current_step = self.sequencer_primary.current_step;
            state.primary_steps = self.sequencer_primary.steps;
            state.primary_pulses = self.sequencer_primary.pulses;
            state.primary_rotation = self.sequencer_primary.rotation;
            state.primary_pattern = self.sequencer_primary.pattern.iter().map(|t| t.is_any()).collect();
            
            state.secondary_steps = self.sequencer_secondary.steps;
            state.secondary_pulses = self.sequencer_secondary.pulses;
            state.secondary_rotation = self.sequencer_secondary.rotation;
            state.secondary_pattern = self.sequencer_secondary.pattern.iter().map(|t| t.is_any()).collect();
        }

        // === GENERATE EVENTS ===
        let mut events = Vec::new();
        
        // Bass (Kick)
        if trigger_primary.kick {
            let root = if let Ok(s) = self.harmony_state.lock() { s.chord_root_offset } else { 0 };
            let midi = 36 + root;
            let vel = 100 + (self.current_state.arousal * 27.0) as u8;
            events.push(AudioEvent::NoteOn { note: midi as u8, velocity: vel, channel: 0 });
        }
        
        // Lead
        let play_lead = trigger_primary.kick || trigger_primary.snare || trigger_secondary.kick || trigger_secondary.snare || trigger_secondary.hat;
        if play_lead {
            let is_strong = trigger_primary.kick;
            let freq = self.harmony.next_note_hybrid(is_strong);
            let midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
            let vel = 90 + (self.current_state.arousal * 30.0) as u8;
            events.push(AudioEvent::NoteOn { note: midi, velocity: vel, channel: 1 });
        }
        
        // Snare
        if trigger_primary.snare {
             let vel = 80 + (self.current_state.arousal * 40.0) as u8;
             events.push(AudioEvent::NoteOn { note: 38, velocity: vel, channel: 2 });
        }
        
        // Hat
        if trigger_primary.hat || trigger_secondary.hat {
             let vel = 70 + (self.current_state.arousal * 30.0) as u8;
             events.push(AudioEvent::NoteOn { note: 42, velocity: vel, channel: 3 });
        }

        // Send events to renderer
        for event in events.iter() {
            self.renderer.handle_event(event.clone());
        }
        
        // Send events to UI
        if !events.is_empty() {
            if let Ok(mut queue) = self.event_queue.lock() {
                for event in events {
                    if let AudioEvent::NoteOn { note, channel, .. } = event {
                        queue.push(VisualizationEvent {
                            note_midi: note,
                            instrument: channel,
                            step: self.sequencer_primary.current_step,
                            duration_samples: self.samples_per_step,
                        });
                    }
                }
            }
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
