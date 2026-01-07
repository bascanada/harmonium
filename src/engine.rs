use crate::sequencer::{Sequencer, RhythmMode, StepTrigger};
use crate::harmony::{HarmonyNavigator, Progression, ChordStep, ChordQuality, HarmonyMode, HarmonicDriver};
use crate::harmony::chord::ChordType;
use crate::harmony::lydian_chromatic::LydianChromaticConcept;
use crate::voicing::{Voicer, BlockChordVoicer, VoicerContext};
use crate::log;
use crate::events::AudioEvent;
use crate::backend::AudioRenderer;
use crate::params::{MusicalParams, ControlMode};
use crate::mapper::EmotionMapper;
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
    pub harmony_mode: HarmonyMode,   // Mode harmonique actuel (Basic ou Driver)
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
            harmony_mode: HarmonyMode::Driver,
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
    #[serde(default)]
    pub muted_channels: Vec<bool>, // true = Muted
    #[serde(default)]
    pub harmony_mode: HarmonyMode, // Basic (quadrants) ou Driver (Steedman/NeoRiemannian/LCC)

    // Recording Control
    #[serde(default)]
    pub record_wav: bool,
    #[serde(default)]
    pub record_midi: bool,
    #[serde(default)]
    pub record_abc: bool,

    // Synthesis Morphing Control
    #[serde(default = "default_true")]
    pub enable_synthesis_morphing: bool,

    // Mixer Gains (0.0 - 1.0)
    #[serde(default = "default_gain_lead")]
    pub gain_lead: f32,
    #[serde(default = "default_gain_bass")]
    pub gain_bass: f32,
    #[serde(default = "default_gain_snare")]
    pub gain_snare: f32,
    #[serde(default = "default_gain_hat")]
    pub gain_hat: f32,

    // Velocity Base (MIDI 0-127)
    #[serde(default = "default_vel_bass")]
    pub vel_base_bass: u8,
    #[serde(default = "default_vel_snare")]
    pub vel_base_snare: u8,

    // Polyrythm Steps (48, 96, 192...)
    #[serde(default = "default_poly_steps")]
    pub poly_steps: usize,
}

fn default_gain_lead() -> f32 { 1.0 }
fn default_gain_bass() -> f32 { 0.6 }
fn default_gain_snare() -> f32 { 0.5 }
fn default_gain_hat() -> f32 { 0.3 }
fn default_vel_bass() -> u8 { 85 }
fn default_vel_snare() -> u8 { 70 }
fn default_poly_steps() -> usize { 48 }
fn default_true() -> bool { true }

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
            muted_channels: vec![false; 16], // Tout activé par défaut
            harmony_mode: HarmonyMode::Driver, // Système BasicHarmony par défaut
            record_wav: false,
            record_midi: false,
            record_abc: false,
            enable_synthesis_morphing: true,
            // Mixer defaults
            gain_lead: default_gain_lead(),
            gain_bass: default_gain_bass(),
            gain_snare: default_gain_snare(),
            gain_hat: default_gain_hat(),
            vel_base_bass: default_vel_bass(),
            vel_base_snare: default_vel_snare(),
            poly_steps: default_poly_steps(),
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

    // === HARMONIC DRIVER (Mode avancé) ===
    harmonic_driver: Option<HarmonicDriver>,
    harmony_mode: HarmonyMode,

    // === VOICING ENGINE ===
    voicer: Box<dyn Voicer>,
    lcc: LydianChromaticConcept,
    current_chord_type: ChordType,
    active_lead_notes: Vec<u8>,  // Notes actuellement jouées sur le channel Lead

    // Optimization
    cached_target: EngineParams,

    // === NOUVELLE ARCHITECTURE: Params Musicaux Découplés ===
    /// Mapper émotions → params musicaux
    emotion_mapper: EmotionMapper,
    /// Paramètres musicaux calculés (ou définis directement)
    musical_params: MusicalParams,
    /// État partagé pour le mode de contrôle (émotion vs direct)
    control_mode: Arc<Mutex<ControlMode>>,

    // Recording State Tracking
    is_recording_wav: bool,
    is_recording_midi: bool,
    is_recording_abc: bool,

    // Mute State Tracking
    last_muted_channels: Vec<bool>,
}

impl HarmoniumEngine {
    pub fn new(
        sample_rate: f64,
        target_state: Arc<Mutex<EngineParams>>,
        control_mode: Arc<Mutex<ControlMode>>,
        mut renderer: Box<dyn AudioRenderer>,
    ) -> Self {
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

        {
            let mut state = harmony_state.lock().unwrap();
            state.harmony_mode = harmony_mode;
            match harmony_mode {
                HarmonyMode::Basic => {
                    state.progression_name = progression_name.to_string();
                    state.progression_length = current_progression.len();
                }
                HarmonyMode::Driver => {
                    // Le Driver utilise Steedman Grammar par défaut (tension < 0.5)
                    state.progression_name = "Driver: Steedman Grammar".to_string();
                    state.progression_length = 4; // Progression dynamique
                }
            }
        }

        // Créer le mapper et les params musicaux initiaux
        let emotion_mapper = EmotionMapper::new();
        let musical_params = emotion_mapper.map(&initial_params);

        // Initialize session key/scale in control_mode for UI
        if let Ok(mut mode) = control_mode.lock() {
            mode.session_key = config.key.clone();
            mode.session_scale = config.scale.clone();
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
            harmonic_driver,
            harmony_mode,
            // Voicing Engine
            voicer: Box::new(BlockChordVoicer::new(4)),
            lcc: LydianChromaticConcept::new(),
            current_chord_type: ChordType::Major,
            active_lead_notes: Vec::new(),
            cached_target: initial_params,
            // Nouvelle architecture
            emotion_mapper,
            musical_params,
            control_mode,
            is_recording_wav: false,
            is_recording_midi: false,
            is_recording_abc: false,
            last_muted_channels: vec![false; 16],
        }
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

    /// Récupère le mapper pour configuration (seuils, courbes, etc.)
    pub fn emotion_mapper_mut(&mut self) -> &mut EmotionMapper {
        &mut self.emotion_mapper
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
        // === UPDATE MUSICAL PARAMS ===
        // Selon le mode, on obtient les params soit du mapper, soit directement
        let use_emotion_mode = self.control_mode.lock()
            .map(|m| m.use_emotion_mode)
            .unwrap_or(true);

        if use_emotion_mode {
            // Mode émotionnel: EngineParams → EmotionMapper → MusicalParams
            if let Ok(guard) = self.target_state.try_lock() {
                self.cached_target = guard.clone();
            }
            self.musical_params = self.emotion_mapper.map(&self.cached_target);
        } else {
            // Mode direct: MusicalParams depuis l'état partagé
            if let Ok(guard) = self.control_mode.try_lock() {
                self.musical_params = guard.direct_params.clone();
            }
        }

        // Apply global enable overrides (work in BOTH modes)
        if let Ok(guard) = self.control_mode.try_lock() {
            self.musical_params.enable_rhythm = guard.enable_rhythm;
            self.musical_params.enable_harmony = guard.enable_harmony;
            self.musical_params.enable_melody = guard.enable_melody;
            self.musical_params.enable_voicing = guard.enable_voicing;
        }

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
                self.renderer.handle_event(AudioEvent::SetChannelRoute { channel: i as u8, bank: mode });
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
            log::info(&format!("🎹 Harmony mode switched to: {:?}", self.harmony_mode));
        }

        // === MORPHING (smooth transitions) ===
        // En mode direct, on utilise les valeurs de MusicalParams directement
        // En mode émotion, le morphing se fait sur les valeurs mappées
        let morph_factor = 0.03;

        // Pour compatibilité avec le reste du code qui utilise current_state
        // on morphe vers les valeurs des MusicalParams
        self.current_state.bpm += (mp.bpm - self.current_state.bpm) * morph_factor;
        self.current_state.density += (mp.rhythm_density - self.current_state.density) * morph_factor;
        self.current_state.tension += (mp.harmony_tension - self.current_state.tension) * morph_factor;
        self.current_state.smoothness += (mp.melody_smoothness - self.current_state.smoothness) * morph_factor;
        self.current_state.valence += (mp.harmony_valence - self.current_state.valence) * morph_factor;
        // Arousal n'existe plus directement dans MusicalParams (c'est le BPM)
        // On le recalcule pour compatibilité avec l'affichage UI
        let arousal_from_bpm = (mp.bpm - 70.0) / 110.0;
        self.current_state.arousal += (arousal_from_bpm - self.current_state.arousal) * morph_factor;

        // === SYNTHESIS MORPHING (emotional timbre control) ===
        #[cfg(feature = "odin2")]
        if self.cached_target.enable_synthesis_morphing {
            // The renderer is wrapped in RecorderBackend, so we need to unwrap it first
            if let Some(recorder) = self.renderer.as_any_mut().downcast_mut::<crate::backend::recorder::RecorderBackend>() {
                // Now get the inner backend and try to downcast it to Odin2Backend
                if let Some(odin2) = recorder.inner_mut().as_any_mut().downcast_mut::<crate::backend::odin2_backend::Odin2Backend>() {
                    odin2.apply_emotional_morphing(
                        self.current_state.valence,
                        self.current_state.arousal,
                        self.current_state.tension,
                        self.current_state.density,
                    );
                }
            }
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
                self.renderer.handle_event(AudioEvent::StartRecording { format: crate::events::RecordFormat::Wav });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording { format: crate::events::RecordFormat::Wav });
            }
        }

        if mp.record_midi != self.is_recording_midi {
            self.is_recording_midi = mp.record_midi;
            if self.is_recording_midi {
                self.renderer.handle_event(AudioEvent::StartRecording { format: crate::events::RecordFormat::Midi });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording { format: crate::events::RecordFormat::Midi });
            }
        }

        if mp.record_abc != self.is_recording_abc {
            self.is_recording_abc = mp.record_abc;
            if self.is_recording_abc {
                self.renderer.handle_event(AudioEvent::StartRecording { format: crate::events::RecordFormat::Abc });
            } else {
                self.renderer.handle_event(AudioEvent::StopRecording { format: crate::events::RecordFormat::Abc });
            }
        }

        // === DSP UPDATES (effets globaux sur channel 0) ===
        // CC 1: Modulation/Filtre - utilise voicing_tension (timbre du son)
        //       PAS harmony_tension (qui affecte seulement la sélection d'accords)
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 1,
            value: (mp.voicing_tension * 127.0) as u8,
            channel: 0
        });
        // CC 11: Expression/Distortion - lié à l'énergie
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 11,
            value: (self.current_state.arousal * 127.0) as u8,
            channel: 0
        });
        // CC 91: Reverb - lié à la valence (émotions positives = plus de reverb)
        self.renderer.handle_event(AudioEvent::ControlChange {
            ctrl: 91,
            value: (self.current_state.valence.abs() * 127.0) as u8,
            channel: 0
        });

        // === SKIP RHYTHM IF DISABLED ===
        if !mp.enable_rhythm {
            // Timing only (pour garder le moteur synchronisé)
            let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
            let new_samples_per_step = (self.sample_rate * 60.0 / (self.current_state.bpm as f64) / steps_per_beat) as usize;
            if new_samples_per_step != self.samples_per_step {
                self.samples_per_step = new_samples_per_step;
                self.renderer.handle_event(AudioEvent::TimingUpdate { samples_per_step: new_samples_per_step });
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

        // Update steps if changed while playing
        if self.sequencer_primary.steps != mp.rhythm_steps {
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
        let needs_regen = mode_changed || if self.sequencer_primary.mode == RhythmMode::Euclidean {
            target_pulses != self.last_pulse_count
        } else {
            (mp.rhythm_density - self.sequencer_primary.density).abs() > 0.05 ||
            (mp.rhythm_tension - self.sequencer_primary.tension).abs() > 0.05
        };

        if needs_regen {
            self.sequencer_primary.tension = mp.rhythm_tension;
            self.sequencer_primary.density = mp.rhythm_density;
            self.sequencer_primary.pulses = target_pulses;
            self.sequencer_primary.regenerate_pattern();
            self.last_pulse_count = target_pulses;
        }

        if target_rotation != self.last_rotation {
            self.sequencer_primary.set_rotation(target_rotation);
            self.last_rotation = target_rotation;
        }

        // Secondary Sequencer Logic (from MusicalParams) - Euclidean mode only
        let secondary_steps = mp.rhythm_secondary_steps;
        let secondary_pulses = mp.rhythm_secondary_pulses.min(secondary_steps);
        let secondary_rotation = mp.rhythm_secondary_rotation;

        let secondary_changed =
            secondary_steps != self.sequencer_secondary.steps ||
            secondary_pulses != self.sequencer_secondary.pulses ||
            secondary_rotation != self.sequencer_secondary.rotation;

        if secondary_changed {
            if secondary_steps != self.sequencer_secondary.steps {
                self.sequencer_secondary.steps = secondary_steps;
                self.sequencer_secondary.pattern = vec![StepTrigger::default(); secondary_steps];
            }
            self.sequencer_secondary.pulses = secondary_pulses;
            self.sequencer_secondary.rotation = secondary_rotation;
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
        // Skip harmony updates if disabled
        let harmony_enabled = self.musical_params.enable_harmony;

        if harmony_enabled && self.sequencer_primary.current_step == 0 {
            self.measure_counter += 1;

            match self.harmony_mode {
                HarmonyMode::Basic => {
                    // === MODE BASIC: Progressions par quadrants émotionnels ===
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

                        // Mettre à jour le type d'accord pour le voicer
                        self.current_chord_type = match chord.quality {
                            ChordQuality::Major => ChordType::Major7,
                            ChordQuality::Minor => ChordType::Minor7,
                            ChordQuality::Dominant7 => ChordType::Dominant7,
                            ChordQuality::Diminished => ChordType::Diminished7,
                            ChordQuality::Sus2 => ChordType::Sus2,
                        };

                        if let Ok(mut state) = self.harmony_state.lock() {
                            state.current_chord_index = self.progression_index;
                            state.chord_root_offset = chord.root_offset;
                            state.chord_is_minor = matches!(chord.quality, ChordQuality::Minor);
                            state.chord_name = chord_name;
                            state.measure_number = self.measure_counter;
                        }
                    }
                }

                HarmonyMode::Driver => {
                    // === MODE DRIVER: Steedman Grammar + Neo-Riemannian + LCC ===
                    let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                    if self.measure_counter % measures_per_chord == 0 {
                        if let Some(ref mut driver) = self.harmonic_driver {
                            let mut rng = rand::thread_rng();

                            // Capturer le nom de l'accord AVANT la transition
                            let old_chord_name = driver.current_chord().name();

                            let decision = driver.next_chord(
                                self.current_state.tension,
                                self.current_state.valence,
                                &mut rng,
                            );

                            // === LOGGING HARMONIQUE ===
                            let strategy = driver.current_strategy_name();
                            let scale_notes: Vec<String> = decision.suggested_scale
                                .iter()
                                .map(|pc| crate::harmony::chord::NoteName::from_pitch_class(*pc).to_string())
                                .collect();

                            log::info(&format!(
                                "🎵 [Driver] {} → {} | Strategy: {} | T:{:.2} V:{:.2} | Scale: [{}]",
                                old_chord_name,
                                decision.next_chord.name(),
                                strategy,
                                self.current_state.tension,
                                self.current_state.valence,
                                scale_notes.join(" ")
                            ));

                            // Convertir vers le format compatible avec HarmonyNavigator
                            let root_offset = driver.root_offset();
                            let quality = driver.to_basic_quality();
                            self.harmony.set_chord_context(root_offset, quality);

                            // Mettre à jour le type d'accord pour le voicer
                            self.current_chord_type = decision.next_chord.chord_type;

                            let chord_name = format!(
                                "{} ({})",
                                decision.next_chord.name(),
                                decision.transition_type.name()
                            );

                            if let Ok(mut state) = self.harmony_state.lock() {
                                state.current_chord_index = self.progression_index;
                                state.chord_root_offset = root_offset;
                                state.chord_is_minor = driver.is_minor();
                                state.chord_name = chord_name.clone();
                                state.measure_number = self.measure_counter;
                                state.progression_name = format!("Driver: {}", strategy);
                                state.progression_length = 0; // Driver n'a pas de longueur fixe
                            }
                        }
                    }
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
            state.harmony_mode = self.harmony_mode;
        }

        // === GENERATE EVENTS ===
        let mut events = Vec::new();
        let rhythm_enabled = self.musical_params.enable_rhythm;

        // Bass (Kick) - part of Rhythm module
        if rhythm_enabled && trigger_primary.kick && !self.musical_params.muted_channels.get(0).copied().unwrap_or(false) {
            let root = if let Ok(s) = self.harmony_state.lock() { s.chord_root_offset } else { 0 };
            let midi = 36 + root;
            let vel = self.cached_target.vel_base_bass + (self.current_state.arousal * 25.0) as u8;
            events.push(AudioEvent::NoteOn { note: midi as u8, velocity: vel, channel: 0 });
        }
        
        // Lead (avec Voicing) - Skip if melody disabled
        let melody_enabled = self.musical_params.enable_melody;

        // If melody just got disabled, stop all lead notes
        if !melody_enabled && !self.active_lead_notes.is_empty() {
            events.push(AudioEvent::AllNotesOff { channel: 1 });
            self.active_lead_notes.clear();
        }

        let play_lead = melody_enabled
                        && (trigger_primary.kick || trigger_primary.snare || trigger_secondary.kick || trigger_secondary.snare || trigger_secondary.hat)
                        && !self.musical_params.muted_channels.get(1).copied().unwrap_or(false);
        if play_lead {
            let is_strong = trigger_primary.kick;
            let freq = self.harmony.next_note_hybrid(is_strong);
            let melody_midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
            let base_vel = 90 + (self.current_state.arousal * 30.0) as u8;

            // Récupérer les infos harmoniques pour le voicer
            let (chord_root, chord_root_offset) = if let Ok(s) = self.harmony_state.lock() {
                (36 + s.chord_root_offset as u8, s.chord_root_offset)
            } else {
                (60, 0)
            };

            // Calculer la gamme LCC courante
            let chord = crate::harmony::chord::Chord::new(
                (chord_root_offset as u8) % 12,
                self.current_chord_type
            );
            let lcc_level = self.lcc.level_for_tension(self.current_state.tension);
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
                events.push(AudioEvent::AllNotesOff { channel: 1 });
                self.active_lead_notes.clear();
            }

            // Utiliser le voicer pour décider du style (si activé)
            let voicing_enabled = self.musical_params.enable_voicing;
            if voicing_enabled && self.voicer.should_voice(&ctx) {
                // Beat fort: jouer l'accord complet
                let voiced_notes = self.voicer.process_note(melody_midi, base_vel, &ctx);
                for vn in voiced_notes {
                    events.push(AudioEvent::NoteOn {
                        note: vn.midi,
                        velocity: vn.velocity,
                        channel: 1,
                    });
                    self.active_lead_notes.push(vn.midi);
                }
            } else {
                // Beat faible: jouer la mélodie seule (plus légère)
                let solo_vel = (base_vel as f32 * 0.7) as u8; // Vélocité réduite
                events.push(AudioEvent::NoteOn {
                    note: melody_midi,
                    velocity: solo_vel,
                    channel: 1,
                });
                self.active_lead_notes.push(melody_midi);
            }
        }
        
        // Snare - part of Rhythm module
        if rhythm_enabled && trigger_primary.snare && !self.musical_params.muted_channels.get(2).copied().unwrap_or(false) {
             let vel = self.cached_target.vel_base_snare + (self.current_state.arousal * 30.0) as u8;
             events.push(AudioEvent::NoteOn { note: 38, velocity: vel, channel: 2 });
        }

        // Hat - part of Rhythm module
        if rhythm_enabled && (trigger_primary.hat || trigger_secondary.hat) && !self.musical_params.muted_channels.get(3).copied().unwrap_or(false) {
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

        // Update live state for UI visualization (VST webview)
        if let Ok(mut mode) = self.control_mode.try_lock() {
            mode.current_step = self.sequencer_primary.current_step as u32;
            mode.current_measure = self.measure_counter as u32;
            // Convert StepTrigger patterns to bool (true = any trigger: kick, snare, or hat)
            mode.primary_pattern = self.sequencer_primary.pattern.iter().map(|t| t.is_any()).collect();
            mode.secondary_pattern = self.sequencer_secondary.pattern.iter().map(|t| t.is_any()).collect();

            // Get chord info from harmony_state
            if let Ok(state) = self.harmony_state.try_lock() {
                mode.current_chord = state.chord_name.clone();
                mode.is_minor_chord = state.chord_is_minor;
                mode.progression_name = state.progression_name.clone();
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
