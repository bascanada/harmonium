use fundsp::hacker32::*;
use crate::sequencer::{Sequencer, RhythmMode, StepTrigger};
use crate::harmony::HarmonyNavigator;
use crate::progression::{Progression, ChordStep, ChordQuality};
use crate::log;
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

pub struct BlockRateAdapter {
    block: Box<dyn AudioUnit>,
    sample_rate: f64,
}

impl BlockRateAdapter {
    pub fn new(mut block: Box<dyn AudioUnit>, sample_rate: f64) -> Self {
        block.set_sample_rate(sample_rate);
        block.allocate();
        Self { block, sample_rate }
    }

    pub fn get_stereo(&mut self) -> (f32, f32) {
        self.block.get_stereo()
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
}

pub struct HarmoniumEngine {
    pub config: SessionConfig,
    pub target_state: Arc<Mutex<EngineParams>>,
    pub harmony_state: Arc<Mutex<HarmonyState>>,  // État harmonique pour l'UI
    pub event_queue: Arc<Mutex<Vec<VisualizationEvent>>>, // Queue d'événements pour l'UI
    current_state: CurrentState,
    // === POLYRYTHMIE: Plusieurs séquenceurs avec cycles différents ===
    sequencer_primary: Sequencer,    // Cycle principal (16 steps)
    sequencer_secondary: Sequencer,  // Cycle secondaire (12 steps) - déphasage de Steve Reich
    harmony: HarmonyNavigator,
    node: BlockRateAdapter,
    // === LEAD (Mélodie/Harmonies) ===
    frequency_lead: Shared,
    gate_lead: Shared,
    // === BASSE (Fondation) ===
    frequency_bass: Shared,
    gate_bass: Shared,
    // === BATTERIE (Nouveau) ===
    gate_snare: Shared,
    gate_hat: Shared,
    // === EFFETS GLOBAUX ===
    cutoff: Shared,
    resonance: Shared,
    distortion: Shared,
    fm_ratio: Shared,      // Ratio modulateur/carrier (1.0 = unison, 2.0 = octave)
    fm_amount: Shared,     // Profondeur de modulation FM (0.0 = off, 1.0 = intense)
    timbre_mix: Shared,    // 0.0 = Organique, 1.0 = FM
    reverb_mix: Shared,    // Dry/wet reverb (0.0 = sec, 1.0 = 100% reverb)
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
    // === ARTICULATION DYNAMIQUE (Anti-Legato) ===
    gate_timer_lead: usize,               // Compteur dégressif pour la durée de la note lead
    gate_timer_bass: usize,               // Compteur dégressif pour la durée de la note basse
    gate_timer_snare: usize,
    gate_timer_hat: usize,
}

impl HarmoniumEngine {
    pub fn new(sample_rate: f64, target_state: Arc<Mutex<EngineParams>>) -> Self {
        let mut rng = rand::thread_rng();
        let initial_params = target_state.lock().unwrap().clone();
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

        // === 1. DSP GRAPH CONSTRUCTION ===
        
        // Paramètres partagés
        let frequency_lead = shared(440.0);
        let gate_lead = shared(0.0);
        let frequency_bass = shared(110.0);
        let gate_bass = shared(0.0);
        let gate_snare = shared(0.0);
        let gate_hat = shared(0.0);
        
        let cutoff = shared(1000.0);
        let resonance = shared(1.0);
        let distortion = shared(0.0);
        let fm_ratio = shared(2.0);
        let fm_amount = shared(0.3);
        let timbre_mix = shared(0.0);
        let reverb_mix = shared(0.25);

        // --- INSTRUMENT 1: LEAD (FM/Organic Hybrid) ---
        let drift_lfo = lfo(|t| (t * 0.3).sin() * 2.0); 
        let freq_lead_mod = var(&frequency_lead) + drift_lfo;

        // FM Path
        let mod_freq = freq_lead_mod.clone() * var(&fm_ratio);
        let modulator = mod_freq >> sine();
        let car_freq = freq_lead_mod.clone() + (modulator * var(&fm_amount) * freq_lead_mod.clone());
        let fm_voice = car_freq >> saw();

        // Organic Path
        let osc_organic = (freq_lead_mod.clone() >> triangle()) * 0.8 
                        + (freq_lead_mod.clone() >> square()) * 0.2;
        let breath = (noise() >> lowpass_hz(2000.0, 0.5)) * 0.15;
        let organic_voice = (osc_organic + breath) >> lowpass_hz(1200.0, 1.0);

        // Mix & Envelope
        let env_lead = var(&gate_lead) >> adsr_live(0.005, 0.2, 0.5, 0.15);
        let lead_mix = (organic_voice * (1.0 - var(&timbre_mix))) + (fm_voice * var(&timbre_mix));
        let lead_out = (lead_mix * env_lead | var(&cutoff) | var(&resonance)) >> lowpass() >> pan(0.3);

        // --- INSTRUMENT 2: BASS ---
        let bass_osc = (var(&frequency_bass) >> sine()) * 0.7 + (var(&frequency_bass) >> saw()) * 0.3;
        let env_bass = var(&gate_bass) >> adsr_live(0.005, 0.1, 0.6, 0.1);
        let bass_out = ((bass_osc * env_bass) >> lowpass_hz(800.0, 0.5)) >> pan(0.0);

        // --- INSTRUMENT 3: SNARE (Noise Burst + Tone) ---
        // Bruit blanc filtré passe-bande pour le "claquement"
        let snare_noise = noise() >> bandpass_hz(1500.0, 0.8);
        // Onde triangle rapide pour le corps (pitch drop rapide)
        // Note: fundsp statique limite les env de pitch complexes, on fait simple
        let snare_tone = sine_hz(180.0) >> saw(); 
        let snare_src = (snare_noise * 0.8) + (snare_tone * 0.2);
        let env_snare = var(&gate_snare) >> adsr_live(0.001, 0.1, 0.0, 0.1);
        let snare_out = (snare_src * env_snare) >> pan(-0.2);

        // --- INSTRUMENT 4: HAT (High Frequency Noise) ---
        // Bruit rose filtré passe-haut
        let hat_src = noise() >> highpass_hz(6000.0, 0.8);
        // Enveloppe très courte
        let env_hat = var(&gate_hat) >> adsr_live(0.001, 0.05, 0.0, 0.05);
        let hat_out = (hat_src * env_hat * 0.4) >> pan(0.2);

        // --- MIXAGE FINAL ---
        let mix = lead_out + bass_out + snare_out + hat_out;
        
        let node = BlockRateAdapter::new(Box::new(mix), sample_rate);

        // Séquenceurs
        let sequencer_primary = Sequencer::new(steps, initial_pulses, bpm);
        let secondary_pulses = std::cmp::min((initial_params.density * 8.0) as usize + 1, 12);
        let sequencer_secondary = Sequencer::new_with_rotation(12, secondary_pulses, bpm, 0);
        
        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);
        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

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
            current_state: CurrentState::default(),
            sequencer_primary,
            sequencer_secondary,
            harmony,
            node,
            frequency_lead, gate_lead,
            frequency_bass, gate_bass,
            gate_snare, gate_hat, // Nouveaux champs
            cutoff, resonance, distortion,
            fm_ratio, fm_amount, timbre_mix, reverb_mix,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
            last_rotation: 0,
            measure_counter: 0,
            current_progression,
            progression_index: 0,
            last_valence_choice: initial_params.valence,
            last_tension_choice: initial_params.tension,
            gate_timer_lead: 0,
            gate_timer_bass: 0,
            gate_timer_snare: 0,
            gate_timer_hat: 0,
        }
    }

    pub fn process(&mut self) -> (f32, f32) {
        let target = self.target_state.lock().unwrap().clone();
        
        // === GESTION DES GATES ===
        if self.gate_timer_lead > 0 { self.gate_timer_lead -= 1; if self.gate_timer_lead == 0 { self.gate_lead.set_value(0.0); } }
        if self.gate_timer_bass > 0 { self.gate_timer_bass -= 1; if self.gate_timer_bass == 0 { self.gate_bass.set_value(0.0); } }
        if self.gate_timer_snare > 0 { self.gate_timer_snare -= 1; if self.gate_timer_snare == 0 { self.gate_snare.set_value(0.0); } }
        if self.gate_timer_hat > 0 { self.gate_timer_hat -= 1; if self.gate_timer_hat == 0 { self.gate_hat.set_value(0.0); } }

        // === MORPHING ===
        self.current_state.arousal += (target.arousal - self.current_state.arousal) * 0.06;
        self.current_state.valence += (target.valence - self.current_state.valence) * 0.04;
        self.current_state.density += (target.density - self.current_state.density) * 0.02;
        self.current_state.tension += (target.tension - self.current_state.tension) * 0.08;
        self.current_state.smoothness += (target.smoothness - self.current_state.smoothness) * 0.05;
        let target_bpm = target.compute_bpm();
        self.current_state.bpm += (target_bpm - self.current_state.bpm) * 0.05;

        // === DSP UPDATES ===
        self.fm_ratio.set_value(1.0 + (self.current_state.tension * 4.0));
        self.fm_amount.set_value(self.current_state.tension * 0.8);
        self.timbre_mix.set_value(self.current_state.tension.clamp(0.0, 1.0));
        self.cutoff.set_value(500.0 + (self.current_state.tension * 3500.0));
        self.resonance.set_value(1.0 + (self.current_state.tension * 4.0));
        
        // Restored missing DSP updates
        self.distortion.set_value(self.current_state.arousal * 0.8);
        self.reverb_mix.set_value(0.1 + (self.current_state.valence.abs() * 0.4));

        // === LOGIQUE SÉQUENCEUR ===
        let target_algo = target.algorithm;
        let mode_changed = self.sequencer_primary.mode != target_algo;
        
        if mode_changed {
            self.sequencer_primary.mode = target_algo;
            if target_algo == RhythmMode::PerfectBalance {
                self.sequencer_primary.upgrade_to_48_steps();
            }
        }

        // Rotation Logic (Restored)
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

        // Secondary Sequencer Logic (Restored)
        let secondary_pulses = std::cmp::min((self.current_state.density * 8.0) as usize + 1, 12);
        if secondary_pulses != self.sequencer_secondary.pulses {
            self.sequencer_secondary.pulses = secondary_pulses;
            self.sequencer_secondary.set_rotation(8 - (target_rotation % 8));
            self.sequencer_secondary.regenerate_pattern();
        }

        // Timing
        let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
        let new_samples_per_step = (self.node.sample_rate() * 60.0 / (self.current_state.bpm as f64) / steps_per_beat) as usize;
        if new_samples_per_step != self.samples_per_step { self.samples_per_step = new_samples_per_step; }

        // === TICK ===
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            
            let trigger_primary = self.sequencer_primary.tick();
            let trigger_secondary = if self.sequencer_primary.mode == RhythmMode::Euclidean {
                self.sequencer_secondary.tick()
            } else {
                StepTrigger::default()
            };

            // === HARMONY & PROGRESSION (Restored) ===
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

            // UI Update - Sync ALL sequencer fields for visualization
            if let Ok(mut state) = self.harmony_state.lock() {
                state.current_step = self.sequencer_primary.current_step;
                state.primary_steps = self.sequencer_primary.steps;
                state.primary_pulses = self.sequencer_primary.pulses;
                state.primary_rotation = self.sequencer_primary.rotation;
                state.primary_pattern = self.sequencer_primary.pattern.iter().map(|t| t.is_any()).collect();

                // Secondary sequencer (Euclidean mode only)
                state.secondary_steps = self.sequencer_secondary.steps;
                state.secondary_pulses = self.sequencer_secondary.pulses;
                state.secondary_rotation = self.sequencer_secondary.rotation;
                state.secondary_pattern = self.sequencer_secondary.pattern.iter().map(|t| t.is_any()).collect();
            }

            // === VOICE DISTRIBUTION ===
            
            // 1. KICK -> BASS
            if trigger_primary.kick {
                let root = if let Ok(s) = self.harmony_state.lock() { s.chord_root_offset } else { 0 };
                let midi = 36 + root;
                let freq = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);
                self.frequency_bass.set_value(freq);
                self.gate_bass.set_value(trigger_primary.velocity);
                self.gate_timer_bass = (self.samples_per_step as f32 * 0.6) as usize;
                
                if let Ok(mut q) = self.event_queue.lock() {
                    q.push(VisualizationEvent { note_midi: midi as u8, instrument: 0, step: self.sequencer_primary.current_step, duration_samples: 2000 });
                }
            }

            // 2. SNARE
            if trigger_primary.snare {
                self.gate_snare.set_value(trigger_primary.velocity);
                self.gate_timer_snare = (self.samples_per_step as f32 * 0.3) as usize;
                if let Ok(mut q) = self.event_queue.lock() {
                    q.push(VisualizationEvent { note_midi: 38, instrument: 2, step: self.sequencer_primary.current_step, duration_samples: 1000 });
                }
            }

            // 3. HAT
            if trigger_primary.hat || trigger_secondary.hat {
                let vel = if trigger_primary.hat { trigger_primary.velocity } else { 0.5 };
                self.gate_hat.set_value(vel);
                self.gate_timer_hat = (self.samples_per_step as f32 * 0.1) as usize;
            }

            // 4. LEAD
            let play_lead = trigger_primary.kick || trigger_primary.snare || trigger_secondary.is_any();
            if play_lead {
                let is_strong = trigger_primary.kick;
                let freq = self.harmony.next_note_hybrid(is_strong);
                self.frequency_lead.set_value(freq);
                
                let dur_factor = if trigger_primary.kick { 0.8 } else { 0.4 };
                let duration = (self.samples_per_step as f32 * dur_factor) as usize;
                
                self.gate_lead.set_value(0.8);
                self.gate_timer_lead = duration;
                
                let midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
                if let Ok(mut q) = self.event_queue.lock() {
                    q.push(VisualizationEvent { note_midi: midi, instrument: 1, step: self.sequencer_primary.current_step, duration_samples: duration });
                }
            }
        }
        
        self.sample_counter += 1;
        self.node.get_stereo()
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
