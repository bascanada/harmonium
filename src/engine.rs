use fundsp::hacker32::*;
use crate::sequencer::Sequencer;
use crate::harmony::HarmonyNavigator;
use crate::log;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;
use rand::Rng;
use std::sync::{Arc, Mutex};

/// État cible (Target) - Ce que l'IA demande
/// Basé sur le modèle dimensionnel des émotions (Russell's Circumplex Model)
#[derive(Clone, Debug)]
pub struct EngineParams {
    pub arousal: f32,   // 0.0 à 1.0 - Activation/Énergie → contrôle BPM
    pub valence: f32,   // -1.0 à 1.0 - Positif/Négatif → contrôle Harmonie (Majeur/Mineur)
    pub density: f32,   // 0.0 à 1.0 - Complexité rythmique
    pub tension: f32,   // 0.0 à 1.0 - Dissonance harmonique
}

impl Default for EngineParams {
    fn default() -> Self {
        EngineParams {
            arousal: 0.5,   // Énergie moyenne
            valence: 0.3,   // Légèrement positif
            density: 0.3,
            tension: 0.2,
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
    pub bpm: f32,  // Calculé à partir de arousal
}

impl Default for CurrentState {
    fn default() -> Self {
        CurrentState {
            arousal: 0.5,
            valence: 0.3,
            density: 0.3,
            tension: 0.2,
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
    current_state: CurrentState,
    sequencer: Sequencer,
    harmony: HarmonyNavigator,
    node: BlockRateAdapter,
    frequency: Shared,
    gate: Shared,
    cutoff: Shared,      // Contrôle dynamique du filtre
    resonance: Shared,   // Contrôle dynamique de la résonance
    distortion: Shared,  // Contrôle dynamique de la distortion
    sample_counter: usize,
    samples_per_step: usize,
    last_pulse_count: usize,
}

impl HarmoniumEngine {
    pub fn new(sample_rate: f64, target_state: Arc<Mutex<EngineParams>>) -> Self {
        let mut rng = rand::thread_rng();
        let initial_params = target_state.lock().unwrap().clone();
        let bpm = initial_params.compute_bpm(); // Calculé depuis arousal!
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

        log::info(&format!("Session: {} {} | BPM: {:.1} | Pulses: {}/{}", config.key, config.scale, bpm, initial_pulses, steps));

        // 1. Setup Audio Graph avec paramètres DYNAMIQUES
        let frequency = shared(440.0);
        let gate = shared(0.0);
        let cutoff = shared(1000.0);
        let resonance = shared(1.0);
        let distortion = shared(0.0);

        // Patch DSP Expressif: Saw >> Filtre Statique >> ADSR
        // Note: FundSP ne supporte pas facilement les paramètres dynamiques pour lowpass_hz
        // On utilisera une approche plus simple pour le moment
        let osc = var(&frequency) >> saw();
        let patch = osc * (var(&gate) >> adsr_live(0.05, 0.2, 0.5, 0.1)) >> lowpass_hz(2000.0, 1.0);
        
        let node = patch >> split::<U2>();
        let node = BlockRateAdapter::new(Box::new(node), sample_rate);

        // 2. Setup Logic Components
        let sequencer = Sequencer::new(steps, initial_pulses, bpm);
        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);

        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        Self {
            config,
            target_state,
            current_state: CurrentState::default(),
            sequencer,
            harmony,
            node,
            frequency,
            gate,
            cutoff,
            resonance,
            distortion,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
        }
    }

    pub fn process(&mut self) -> (f32, f32) {
        // === ÉTAPE A: Récupérer l'état cible (Target) ===
        let target = {
            self.target_state.lock().unwrap().clone()
        }; // Lock relâché immédiatement

        // === ÉTAPE B: MORPHING - Interpolation linéaire (Lerp) ===
        // Facteurs de lissage (plus petit = plus fluide/lent)
        const AROUSAL_SMOOTHING: f32 = 0.06;
        const VALENCE_SMOOTHING: f32 = 0.04;  // Lent pour transitions harmoniques douces
        const DENSITY_SMOOTHING: f32 = 0.02;  // Plus lent pour éviter les changements brusques
        const TENSION_SMOOTHING: f32 = 0.08;  // Plus rapide pour la réactivité du timbre

        self.current_state.arousal += (target.arousal - self.current_state.arousal) * AROUSAL_SMOOTHING;
        self.current_state.valence += (target.valence - self.current_state.valence) * VALENCE_SMOOTHING;
        self.current_state.density += (target.density - self.current_state.density) * DENSITY_SMOOTHING;
        self.current_state.tension += (target.tension - self.current_state.tension) * TENSION_SMOOTHING;

        // Calculer le BPM depuis l'arousal (activation émotionnelle)
        let target_bpm = target.compute_bpm();
        self.current_state.bpm += (target_bpm - self.current_state.bpm) * 0.05;

        // === ÉTAPE C: Mise à jour DSP (Timbre Dynamique) ===
        // Mapping Tension -> Cutoff (500Hz à 4000Hz)
        let target_cutoff = 500.0 + (self.current_state.tension * 3500.0);
        self.cutoff.set_value(target_cutoff);

        // Mapping Tension -> Résonance (1.0 à 5.0)
        let target_resonance = 1.0 + (self.current_state.tension * 4.0);
        self.resonance.set_value(target_resonance);

        // Mapping Arousal -> Distortion (0.0 à 0.8)
        let target_distortion = self.current_state.arousal * 0.8;
        self.distortion.set_value(target_distortion);

        // === ÉTAPE D: Mise à jour Séquenceur (Logique Rythmique) ===
        // Convertir density (0.0-1.0) en nombre de pulses (1 à 12)
        let target_pulses = std::cmp::min((self.current_state.density * 11.0) as usize + 1, 16);
        
        // *Astuce XronoMorph*: Ne régénérer le pattern que si le nombre entier change
        if target_pulses != self.last_pulse_count {
            self.sequencer.pulses = target_pulses;
            self.sequencer.pattern = crate::sequencer::generate_euclidean_pattern(self.sequencer.steps, target_pulses);
            self.last_pulse_count = target_pulses;
            log::info(&format!("🔄 Morphing Rhythm -> Pulses: {} | BPM: {:.1}", target_pulses, self.current_state.bpm));
        }

        // Mise à jour du timing (samples_per_step basé sur le BPM actuel)
        let new_samples_per_step = (self.node.sample_rate() * 60.0 / (self.current_state.bpm as f64) / 4.0) as usize;
        if new_samples_per_step != self.samples_per_step {
            self.samples_per_step = new_samples_per_step;
        }

        // === ÉTAPE E: Logique de Tick du Séquenceur ===
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            let trigger = self.sequencer.tick();
            if trigger {
                let freq = self.harmony.next_note();
                self.frequency.set_value(freq);
                self.gate.set_value(1.0);
            } else {
                self.gate.set_value(0.0);
            }
        }
        self.sample_counter += 1;

        self.node.get_stereo()
    }
}
