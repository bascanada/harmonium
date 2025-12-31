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
    // === POLYRYTHMIE: Plusieurs séquenceurs avec cycles différents ===
    sequencer_primary: Sequencer,    // Cycle principal (16 steps)
    sequencer_secondary: Sequencer,  // Cycle secondaire (12 steps) - déphasage de Steve Reich
    harmony: HarmonyNavigator,
    node: BlockRateAdapter,
    frequency: Shared,
    gate: Shared,
    cutoff: Shared,
    resonance: Shared,
    distortion: Shared,
    fm_ratio: Shared,      // Ratio modulateur/carrier (1.0 = unison, 2.0 = octave)
    fm_amount: Shared,     // Profondeur de modulation FM (0.0 = off, 1.0 = intense)
    reverb_mix: Shared,    // Dry/wet reverb (0.0 = sec, 1.0 = 100% reverb)
    sample_counter: usize,
    samples_per_step: usize,
    last_pulse_count: usize,
    last_rotation: usize,  // Pour détecter les changements de rotation
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
        let fm_ratio = shared(2.0);     // Départ: octave (son de cloche)
        let fm_amount = shared(0.3);    // Modulation FM modérée
        let reverb_mix = shared(0.25);  // 25% reverb

        // === PATCH DSP RICHE: FM Synthesis + Spatial Effects ===
        
        // A. OSCILLATEURS: FM Synthesis (Carrier + Modulator)
        // Modulateur: fréquence = carrier * ratio (2.0 = octave, 3.0 = quinte+octave)
        let modulator_freq = var(&frequency) * var(&fm_ratio);
        let modulator = modulator_freq >> sine(); // Sine pour FM classique
        
        // Modulation de fréquence: carrier_freq + (modulator * fm_amount * freq)
        // Plus fm_amount est élevé, plus le spectre s'enrichit
        let carrier_freq = var(&frequency) + (modulator * var(&fm_amount) * var(&frequency));
        let carrier = carrier_freq >> saw(); // Saw pour richesse harmonique
        
        // B. ENVELOPPE: ADSR percussif pour articuler les notes
        let envelope = var(&gate) >> adsr_live(0.01, 0.15, 0.6, 0.3);
        let voice = carrier * envelope;
        
        // C. FILTRAGE: Lowpass dynamique (cutoff/resonance contrôlés par tension)
        let filtered = voice >> lowpass_hz(2000.0, 1.0);
        
        // D. EFFETS SPATIAUX: Delay simple (architecture parallèle)
        // Dry/Wet mix: pass() = signal sec, delay() * 0.3 = écho
        let spatial = filtered >> (pass() & delay(0.3) * 0.3);
        
        let node = spatial >> split::<U2>();
        let node = BlockRateAdapter::new(Box::new(node), sample_rate);

        // 2. Setup Logic Components - POLYRYTHMIE
        // Séquenceur principal: 16 steps (cycle standard)
        let sequencer_primary = Sequencer::new(steps, initial_pulses, bpm);
        
        // Séquenceur secondaire: 12 steps (déphasage à la Steve Reich)
        // Ratio 16:12 = 4:3 - crée un cycle complet tous les 48 steps
        let secondary_pulses = std::cmp::min((initial_params.density * 8.0) as usize + 1, 12);
        let sequencer_secondary = Sequencer::new_with_rotation(12, secondary_pulses, bpm, 0);
        
        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);

        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        Self {
            config,
            target_state,
            current_state: CurrentState::default(),
            sequencer_primary,
            sequencer_secondary,
            harmony,
            node,
            frequency,
            gate,
            cutoff,
            resonance,
            distortion,
            fm_ratio,
            fm_amount,
            reverb_mix,
            sample_counter: 0,
            samples_per_step,
            last_pulse_count: initial_pulses,
            last_rotation: 0,
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
        
        // C1. TENSION → FM Synthesis (brillance spectrale)
        // Faible tension: FM ratio proche de 1.0 (son doux, peu d'harmoniques)
        // Haute tension: FM ratio 3-5 (son métallique, cloche, bell-like)
        let target_fm_ratio = 1.0 + (self.current_state.tension * 4.0); // 1.0 → 5.0
        self.fm_ratio.set_value(target_fm_ratio);
        
        // Profondeur de modulation FM: plus de tension = plus d'inharmonicité
        let target_fm_amount = self.current_state.tension * 0.8; // 0.0 → 0.8
        self.fm_amount.set_value(target_fm_amount);
        
        // C2. VALENCE → Spatial Depth (espace sonore)
        // Valence positive: son ouvert, spacieux (plus de reverb)
        // Valence négative: son fermé, intime (sec)
        let target_reverb = 0.1 + (self.current_state.valence.abs() * 0.4); // 10% → 50%
        self.reverb_mix.set_value(target_reverb);
        
        // C3. AROUSAL → Attack Time (réactivité)
        // (Note: pour l'instant ADSR est fixe dans le graph, mais on pourrait le rendre variable)
        
        // Mapping Tension -> Cutoff (500Hz à 4000Hz)
        let target_cutoff = 500.0 + (self.current_state.tension * 3500.0);
        self.cutoff.set_value(target_cutoff);

        // Mapping Tension -> Résonance (1.0 à 5.0)
        let target_resonance = 1.0 + (self.current_state.tension * 4.0);
        self.resonance.set_value(target_resonance);

        // Mapping Arousal -> Distortion (0.0 à 0.8)
        let target_distortion = self.current_state.arousal * 0.8;
        self.distortion.set_value(target_distortion);

        // === ÉTAPE D: Mise à jour Séquenceurs (Logique Rythmique + Polyrythmie) ===
        
        // D1. Density → Pulses (séquenceur principal 16 steps)
        let target_pulses = std::cmp::min((self.current_state.density * 11.0) as usize + 1, 16);
        
        // D2. Tension → Rotation (géométrie rythmique à la Toussaint)
        // Plus de tension = plus de décalage rythmique (transformation Necklace → Bracelet)
        let target_rotation = (self.current_state.tension * 8.0) as usize; // 0-8 steps de rotation
        
        // Régénérer pattern principal si pulses changent
        if target_pulses != self.last_pulse_count {
            self.sequencer_primary.pulses = target_pulses;
            self.sequencer_primary.regenerate_pattern();
            self.last_pulse_count = target_pulses;
            log::info(&format!("🔄 Morphing Rhythm -> Pulses: {} | BPM: {:.1}", target_pulses, self.current_state.bpm));
        }
        
        // Appliquer rotation si tension change
        if target_rotation != self.last_rotation {
            self.sequencer_primary.set_rotation(target_rotation);
            self.last_rotation = target_rotation;
            log::info(&format!("🔀 Rotation shift: {} steps (Tension: {:.2})", target_rotation, self.current_state.tension));
        }
        
        // D3. Mettre à jour le séquenceur secondaire (polyrythmie 12 steps)
        let secondary_pulses = std::cmp::min((self.current_state.density * 8.0) as usize + 1, 12);
        if secondary_pulses != self.sequencer_secondary.pulses {
            self.sequencer_secondary.pulses = secondary_pulses;
            // Rotation inversée pour créer un déphasage intéressant
            self.sequencer_secondary.set_rotation(8 - target_rotation);
            self.sequencer_secondary.regenerate_pattern();
        }

        // Mise à jour du timing (samples_per_step basé sur le BPM actuel)
        let new_samples_per_step = (self.node.sample_rate() * 60.0 / (self.current_state.bpm as f64) / 4.0) as usize;
        if new_samples_per_step != self.samples_per_step {
            self.samples_per_step = new_samples_per_step;
        }

        // === ÉTAPE E: Logique de Tick des Séquenceurs (Polyrythmie) ===
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            
            // Tick des deux séquenceurs
            let trigger_primary = self.sequencer_primary.tick();
            let trigger_secondary = self.sequencer_secondary.tick();
            
            // Déterminer si on est sur un temps fort
            // Temps forts: début de mesure, beats 1 et 3 en 4/4
            let is_strong_beat = self.sequencer_primary.current_step % 4 == 0;
            
            // Logique de déclenchement: OR logique (un des deux suffit)
            // Alternative possible: AND (les deux doivent se synchroniser - plus rare, plus percussif)
            let trigger = trigger_primary || trigger_secondary;
            
            if trigger {
                // Génération mélodique probabiliste avec contexte rythmique
                let freq = self.harmony.next_note(is_strong_beat);
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
