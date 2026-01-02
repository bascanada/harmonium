use fundsp::hacker32::*;
use crate::sequencer::{Sequencer, RhythmMode};
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
    pub instrument: u8, // 0 = Bass, 1 = Lead
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
    // === EFFETS GLOBAUX ===
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
    // === PROGRESSION HARMONIQUE ADAPTATIVE ===
    measure_counter: usize,               // Compte les mesures (16 steps = 1 mesure)
    current_progression: Vec<ChordStep>,  // Progression chargée (dépend de valence/tension)
    progression_index: usize,             // Position dans la progression actuelle
    last_valence_choice: f32,             // Hystérésis: valence qui a déclenché le dernier choix
    last_tension_choice: f32,             // Hystérésis: tension qui a déclenché le dernier choix
    // === ARTICULATION DYNAMIQUE (Anti-Legato) ===
    gate_timer_lead: usize,               // Compteur dégressif pour la durée de la note lead
    gate_timer_bass: usize,               // Compteur dégressif pour la durée de la note basse
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

        // État harmonique partagé pour l'UI
        let harmony_state = Arc::new(Mutex::new(HarmonyState::default()));
        let event_queue = Arc::new(Mutex::new(Vec::new()));

        // 1. Setup Audio Graph avec DEUX INSTRUMENTS SÉPARÉS
        
        // --- LEAD (Mélodie/Harmonies) ---
        let frequency_lead = shared(440.0);
        let gate_lead = shared(0.0);
        
        // --- BASSE (Fondation) ---
        let frequency_bass = shared(110.0);
        let gate_bass = shared(0.0);
        
        // --- EFFETS GLOBAUX ---
        let cutoff = shared(1000.0);
        let resonance = shared(1.0);
        let distortion = shared(0.0);
        let fm_ratio = shared(2.0);     // Départ: octave (son de cloche)
        let fm_amount = shared(0.3);    // Modulation FM modérée
        let reverb_mix = shared(0.25);  // 25% reverb

        // === PATCH DSP: DEUX INSTRUMENTS SÉPARÉS ===
        
        // --- INSTRUMENT 1: LEAD (Aérien, FM, Reverb) ---
        // A. Oscillateurs FM (comme avant mais pour le lead)
        let modulator_freq_lead = var(&frequency_lead) * var(&fm_ratio);
        let modulator_lead = modulator_freq_lead >> sine();
        let carrier_freq_lead = var(&frequency_lead) + (modulator_lead * var(&fm_amount) * var(&frequency_lead));
        let carrier_lead = carrier_freq_lead >> saw();
        
        // B. Enveloppe ADSR fluide pour le lead
        let envelope_lead = var(&gate_lead) >> adsr_live(0.005, 0.2, 0.5, 0.15);
        let voice_lead = carrier_lead * envelope_lead;
        
        // C. Filtrage dynamique
        let filtered_lead = voice_lead >> lowpass_hz(2000.0, 1.0);
        
        // D. Pan légèrement à droite pour séparation stéréo
        let lead_output = filtered_lead >> pan(0.3);
        
        // --- INSTRUMENT 2: BASSE (Solide, Simple, Sub) ---
        // --- INSTRUMENT 2: BASSE (Modifié pour plus de présence) ---
        // A. Oscillateurs: Sine pur (Sub) + Saw filtrée (Texture)
        let bass_sub = var(&frequency_bass) >> sine();
        // On ajoute un peu de Saw pour que la basse perce le mix, pas juste du "boum"
        let bass_texture = var(&frequency_bass) >> saw(); 
        let bass_osc = bass_sub * 0.7 + bass_texture * 0.3;
        
        // B. Enveloppe: On utilise la MÊME sortie d'enveloppe pour le volume et le filtre
        let envelope_bass = var(&gate_bass) >> adsr_live(0.005, 0.1, 0.6, 0.1);
        
        // C. Filtre DYNAMIQUE (Wah): Le filtre s'ouvre quand la note frappe
        // Cutoff de base (300Hz) + Modulation par l'enveloppe (jusqu'à 1000Hz)
        // let bass_filter_freq = 300.0 + (envelope_bass.clone() * 1000.0); // Note: fundsp gère ça différemment
        // Pour simplifier avec fundsp statique, on garde un lowpass fixe mais plus ouvert
        let filtered_bass = (bass_osc * envelope_bass) >> lowpass_hz(800.0, 0.5); 
        
        // D. PANNING: BASSE AU CENTRE (0.0) ! C'est vital pour l'énergie.
        let bass_output = filtered_bass >> pan(0.0);
        
        // --- MIXAGE FINAL ---
        let mix = lead_output + bass_output;
        
        // Note: Pour l'instant, on ne met pas de reverb globale sur le mix final 
        // pour éviter les erreurs de type complexes avec fundsp (stéréo vs mono).
        // La reverb est déjà appliquée sur le Lead (via spatial).
        // La basse reste sèche et centrée, ce qui est mieux pour le mix.
        let node = mix;
        
        // Si on veut vraiment de la reverb globale, il faut un graph stéréo complexe
        // Pour l'instant, la reverb est déjà dans le patch Lead (via spatial)
        // On va ajouter un peu de reverb sur la basse si nécessaire, mais généralement la basse est sèche.
        
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

        // === PROGRESSION HARMONIQUE INITIALE ===
        // Commencer avec une progression basée sur l'état émotionnel initial
        let current_progression = Progression::get_palette(initial_params.valence, initial_params.tension);
        let progression_name = Progression::get_progression_name(initial_params.valence, initial_params.tension);
        
        // Initialiser harmony_state avec la progression initiale
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
            frequency_lead,
            gate_lead,
            frequency_bass,
            gate_bass,
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
            measure_counter: 0,
            current_progression,
            progression_index: 0,
            last_valence_choice: initial_params.valence,
            last_tension_choice: initial_params.tension,
            gate_timer_lead: 0,
            gate_timer_bass: 0,
        }
    }

    pub fn process(&mut self) -> (f32, f32) {
        // === ÉTAPE A: Récupérer l'état cible (Target) ===
        let target = {
            self.target_state.lock().unwrap().clone()
        }; // Lock relâché immédiatement
        
        // === GESTION DES GATE TIMERS (ARTICULATION) ===
        // Timer LEAD (Mélodie)
        if self.gate_timer_lead > 0 {
            self.gate_timer_lead -= 1;
            if self.gate_timer_lead == 0 {
                self.gate_lead.set_value(0.0);
            }
        }
        
        // Timer BASS (Fondation)
        if self.gate_timer_bass > 0 {
            self.gate_timer_bass -= 1;
            if self.gate_timer_bass == 0 {
                self.gate_bass.set_value(0.0);
            }
        }

        // === ÉTAPE B: MORPHING - Interpolation linéaire (Lerp) ===
        // Facteurs de lissage (plus petit = plus fluide/lent)
        const AROUSAL_SMOOTHING: f32 = 0.06;
        const VALENCE_SMOOTHING: f32 = 0.04;  // Lent pour transitions harmoniques douces
        const DENSITY_SMOOTHING: f32 = 0.02;  // Plus lent pour éviter les changements brusques
        const TENSION_SMOOTHING: f32 = 0.08;  // Plus rapide pour la réactivité du timbre
        const SMOOTHNESS_SMOOTHING: f32 = 0.05;

        self.current_state.arousal += (target.arousal - self.current_state.arousal) * AROUSAL_SMOOTHING;
        self.current_state.valence += (target.valence - self.current_state.valence) * VALENCE_SMOOTHING;
        self.current_state.density += (target.density - self.current_state.density) * DENSITY_SMOOTHING;
        self.current_state.tension += (target.tension - self.current_state.tension) * TENSION_SMOOTHING;
        self.current_state.smoothness += (target.smoothness - self.current_state.smoothness) * SMOOTHNESS_SMOOTHING;

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

        // D0. GESTION DU CHANGEMENT DE MODE (Strategy Pattern)
        // Vérifie si l'algorithme a changé (Euclidean ↔ PerfectBalance)
        let target_algo = target.algorithm;
        let mode_changed = self.sequencer_primary.mode != target_algo;
        if mode_changed {
            self.sequencer_primary.mode = target_algo;

            // Si on passe en PerfectBalance, on UPGRADE la résolution à 48 steps
            // C'est le "nombre magique" pour les polyrythmes parfaits (4:3)
            if target_algo == RhythmMode::PerfectBalance {
                self.sequencer_primary.upgrade_to_48_steps();
                log::info("🚀 UPGRADE: Sequencer resolution -> 48 steps (High Precision Polyrhythm)");
            } else {
                // Retour en Euclidean: on garde 48 steps (compatible) ou on revient à 16
                // Pour éviter les glitches, on garde la haute résolution
                // L'algo Euclidean fonctionne avec n'importe quel nombre de steps
            }
        }

        // D1. Density → Pulses (pour mode Euclidean)
        let target_pulses = if self.sequencer_primary.mode == RhythmMode::Euclidean {
            std::cmp::min((self.current_state.density * 11.0) as usize + 1, self.sequencer_primary.steps)
        } else {
            // En mode PerfectBalance, les pulses sont calculés par l'algorithme géométrique
            self.sequencer_primary.pulses
        };

        // D2. Tension → Rotation (géométrie rythmique à la Toussaint)
        // Plus de tension = plus de décalage rythmique (transformation Necklace → Bracelet)
        let max_rotation = if self.sequencer_primary.mode == RhythmMode::PerfectBalance { 24 } else { 8 };
        let target_rotation = (self.current_state.tension * max_rotation as f32) as usize;

        // Régénérer pattern principal si:
        // - Mode a changé (besoin de recalculer avec le nouvel algorithme)
        // - Pulses changent (mode Euclidean)
        // - Density/tension changent (mode PerfectBalance - régénération continue)
        // IMPORTANT: Comparer AVANT de mettre à jour les valeurs du séquenceur!
        let needs_regen = mode_changed || if self.sequencer_primary.mode == RhythmMode::Euclidean {
            target_pulses != self.last_pulse_count
        } else {
            // En PerfectBalance, on régénère si density ou tension ont significativement changé
            (self.current_state.density - self.sequencer_primary.density).abs() > 0.05 ||
            (self.current_state.tension - self.sequencer_primary.tension).abs() > 0.05
        };

        if needs_regen {
            // D0.5. Mettre à jour les paramètres du séquenceur AVANT régénération
            // Ces valeurs sont utilisées par generate_balanced_pattern_48
            self.sequencer_primary.tension = self.current_state.tension;
            self.sequencer_primary.density = self.current_state.density;
            self.sequencer_primary.pulses = target_pulses;

            self.sequencer_primary.regenerate_pattern();
            self.last_pulse_count = target_pulses;

            if self.sequencer_primary.mode == RhythmMode::PerfectBalance {
                log::info(&format!("🔷 Morphing Geometry -> Density: {:.2} | Tension: {:.2} | 48 Steps",
                    self.current_state.density, self.current_state.tension));
            } else {
                log::info(&format!("🔄 Morphing Rhythm -> Pulses: {} | BPM: {:.1}", target_pulses, self.current_state.bpm));
            }
        }

        // Appliquer rotation si tension change
        if target_rotation != self.last_rotation {
            self.sequencer_primary.set_rotation(target_rotation);
            self.last_rotation = target_rotation;
            log::info(&format!("🔀 Rotation shift: {} steps (Tension: {:.2})", target_rotation, self.current_state.tension));
        }

        // D3. Mettre à jour le séquenceur secondaire (polyrythmie 12 steps)
        // En mode PerfectBalance, le séquenceur secondaire devient moins important
        // car le polyrythme est déjà intégré dans les 48 steps
        let secondary_pulses = std::cmp::min((self.current_state.density * 8.0) as usize + 1, 12);
        if secondary_pulses != self.sequencer_secondary.pulses {
            self.sequencer_secondary.pulses = secondary_pulses;
            // Rotation inversée pour créer un déphasage intéressant
            self.sequencer_secondary.set_rotation(8 - (target_rotation % 8));
            self.sequencer_secondary.regenerate_pattern();
        }

        // Mise à jour du timing (samples_per_step basé sur le BPM actuel et le nombre de steps)
        // En mode 48 steps: on divise par 12 au lieu de 4 pour garder la même durée de mesure
        // 48 steps / 4 beats = 12 steps par beat (au lieu de 4 en mode 16 steps)
        let steps_per_beat = (self.sequencer_primary.steps / 4) as f64;
        let new_samples_per_step = (self.node.sample_rate() * 60.0 / (self.current_state.bpm as f64) / steps_per_beat) as usize;
        if new_samples_per_step != self.samples_per_step {
            self.samples_per_step = new_samples_per_step;
        }

        // === ÉTAPE E: Logique de Tick des Séquenceurs (Polyrythmie + Progression Harmonique) ===
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            
            // On force les Gates à 0 au début du step pour garantir le "re-trigger"
            // si les notes précédentes étaient très longues (Legato)
            self.gate_lead.set_value(0.0);
            self.gate_bass.set_value(0.0);
            
            // Tick des séquenceurs
            let trigger_primary = self.sequencer_primary.tick();

            // En mode PerfectBalance, le polyrythme est encodé dans les 48 steps du primaire
            // Le secondaire est désactivé pour éviter le chaos (48/12 = 4 tours par mesure)
            let trigger_secondary = if self.sequencer_primary.mode == RhythmMode::PerfectBalance {
                false
            } else {
                self.sequencer_secondary.tick()
            };
            
            // === PROGRESSION HARMONIQUE ADAPTATIVE ===
            // Quand le séquenceur primaire revient au step 0, on débute une nouvelle mesure
            if self.sequencer_primary.current_step == 0 {
                self.measure_counter += 1;
                
                // === 1. SÉLECTION DE PALETTE (Macro-structure) ===
                // Toutes les 4 mesures, vérifier si l'état émotionnel a suffisamment changé
                // pour justifier un changement de progression harmonique (hystérésis)
                if self.measure_counter % 4 == 0 {
                    let valence_delta = (self.current_state.valence - self.last_valence_choice).abs();
                    let tension_delta = (self.current_state.tension - self.last_tension_choice).abs();
                    
                    // Seuil d'hystérésis: changer uniquement si déplacement significatif (> 0.4)
                    // Évite les oscillations chaotiques entre progressions
                    if valence_delta > 0.4 || tension_delta > 0.4 {
                        // Charger nouvelle palette basée sur l'état émotionnel actuel
                        self.current_progression = Progression::get_palette(
                            self.current_state.valence, 
                            self.current_state.tension
                        );
                        self.progression_index = 0; // Reset au début de la nouvelle progression
                        
                        // Mémoriser le choix pour hystérésis
                        self.last_valence_choice = self.current_state.valence;
                        self.last_tension_choice = self.current_state.tension;
                        
                        let prog_name = Progression::get_progression_name(
                            self.current_state.valence, 
                            self.current_state.tension
                        );
                        
                        // Mettre à jour l'UI avec le nouveau contexte harmonique
                        if let Ok(mut state) = self.harmony_state.lock() {
                            state.progression_name = prog_name.to_string();
                            state.progression_length = self.current_progression.len();
                        }
                        
                        log::info(&format!("🎼 New Harmonic Context: {} | Valence: {:.2}, Tension: {:.2}", 
                                          prog_name, self.current_state.valence, self.current_state.tension));
                    }
                }
                
                // === 2. AVANCEMENT DANS LA PROGRESSION (Micro-structure) ===
                // Vitesse de changement d'accord contrôlée par tension:
                // Haute tension (> 0.6): changements fréquents (chaque mesure)
                // Basse tension: changements lents (toutes les 4 mesures)
                let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
                
                if self.measure_counter % measures_per_chord == 0 {
                    // Avancer dans la progression actuelle (cyclique)
                    self.progression_index = (self.progression_index + 1) % self.current_progression.len();
                    
                    let current_chord = &self.current_progression[self.progression_index];
                    
                    // === 3. APPLICATION DE L'ACCORD AU NAVIGATEUR HARMONIQUE ===
                    self.harmony.set_chord_context(current_chord.root_offset, current_chord.quality);
                    
                    // Nommage des accords pour l'UI
                    let chord_name = self.format_chord_name(current_chord.root_offset, current_chord.quality);
                    
                    // Mettre à jour l'état harmonique pour l'UI
                    if let Ok(mut state) = self.harmony_state.lock() {
                        state.current_chord_index = self.progression_index;
                        state.chord_root_offset = current_chord.root_offset;
                        state.chord_is_minor = matches!(current_chord.quality, ChordQuality::Minor);
                        state.chord_name = chord_name.clone();
                        state.measure_number = self.measure_counter;
                        state.cycle_number = (self.measure_counter / (measures_per_chord * self.current_progression.len())) + 1;
                        state.current_step = self.sequencer_primary.current_step;
                    }
                    
                    log::info(&format!("🎵 Chord: {} | Measure: {} | Progression: {}/{}", 
                                      chord_name, self.measure_counter, 
                                      self.progression_index + 1, self.current_progression.len()));
                }
            }
            
            // Déterminer si on est sur un temps fort
            // Temps forts: début de mesure, beats 1 et 3 en 4/4
            let _is_strong_beat = self.sequencer_primary.current_step % 4 == 0;
            
            // Mettre à jour le step courant dans harmony_state (pour l'UI)
            if let Ok(mut state) = self.harmony_state.lock() {
                state.current_step = self.sequencer_primary.current_step;
                // Mettre à jour les infos rythmiques pour la visualisation
                state.primary_steps = self.sequencer_primary.steps;
                state.primary_pulses = self.sequencer_primary.pulses;
                state.primary_rotation = self.sequencer_primary.rotation;
                state.primary_pattern = self.sequencer_primary.pattern.clone();

                state.secondary_steps = self.sequencer_secondary.steps;
                state.secondary_pulses = self.sequencer_secondary.pulses;
                state.secondary_rotation = self.sequencer_secondary.rotation;
                state.secondary_pattern = self.sequencer_secondary.pattern.clone();
            }
            
            // === LA CLEF DU GROOVE : COHÉRENCE RYTHMIQUE ===
            
            // 1. BASSE : Le Pilier (Suit le rythme principal)
            let play_bass = trigger_primary;
            
            // 2. LEAD : La Texture
            // - Mode Euclidean: Lead joue sur Primary OU Secondary (polyrythme 16:12)
            // - Mode PerfectBalance: Lead joue seulement sur Primary (polyrythme encodé dans 48 steps)
            // Note: trigger_secondary est déjà `false` en PerfectBalance (voir ligne 531)
            let play_lead = trigger_primary || trigger_secondary;
            
            // --- INSTRUMENT 1: BASSE ---
            if play_bass {
                // La basse joue TOUJOURS la fondamentale (root) de l'accord actuel
                let chord_root = if let Ok(state) = self.harmony_state.lock() {
                    state.chord_root_offset
                } else {
                    0
                };
                
                // Convertir en fréquence MIDI (Octave basse: MIDI 36-48 = C2-C3)
                let bass_midi = 36 + chord_root; // C2 = MIDI 36
                let bass_freq = 440.0 * 2.0_f32.powf((bass_midi as f32 - 69.0) / 12.0);
                
                self.frequency_bass.set_value(bass_freq);
                
                // Articulation Basse : Toujours assez percussive (60% du step)
                let mut bass_duration = (self.samples_per_step as f32 * 0.6) as usize;
                if bass_duration < 800 { bass_duration = 800; }
                
                self.gate_timer_bass = bass_duration;
                self.gate_bass.set_value(1.0);

                // Event pour l'UI
                if let Ok(mut queue) = self.event_queue.lock() {
                    queue.push(VisualizationEvent {
                        note_midi: bass_midi as u8,
                        instrument: 0,
                        step: self.sequencer_primary.current_step,
                        duration_samples: bass_duration,
                    });
                }
            }
            
            // --- INSTRUMENT 2: LEAD ---
            if play_lead {
                // Mise à jour du facteur Hurst (lissage mélodique)
                self.harmony.set_hurst_factor(self.current_state.smoothness);

                // L'harmonie change intelligemment :
                // Si c'est un coup de Basse (Primary), le Lead joue une note structurante (Tierce/Quinte)
                // Si c'est un coup "Fantôme" (Secondary seul), le Lead peut oser une note de tension
                // Note: is_strong_beat || play_bass signifie qu'on joue "safe" sur les temps forts ET sur les coups de basse
                
                // IMPORTANT : On calcule si on est sur un temps fort pour aider Markov
                // Temps fort = début de mesure (0) ou temps 3 (si 16 steps/4 temps)
                // Ou simplement aligné avec la basse
                let is_strong_beat = play_bass || (self.sequencer_primary.current_step % 4 == 0);

                // --- CHANGEMENT ICI : Appel de la méthode HYBRIDE ---
                let lead_freq = self.harmony.next_note_hybrid(is_strong_beat);
                
                self.frequency_lead.set_value(lead_freq);
                
                // Calculer MIDI approximatif pour l'affichage
                let lead_midi = (69.0 + 12.0 * (lead_freq / 440.0).log2()).round() as u8;

                // Articulation Lead liée à la Tension
                let articulation_base = 0.90 - (self.current_state.tension * 0.75);
                
                // Humanisation légère sur le TEMPS (±5%)
                let mut rng = rand::thread_rng();
                let micro_timing = rng.gen_range(0.95..1.05);
                
                let mut lead_duration = (self.samples_per_step as f32 * articulation_base * micro_timing) as usize;
                
                // Variation : Si le Lead joue SEUL (sans la basse), on le fait plus court/léger
                if !play_bass {
                     lead_duration = (lead_duration as f32 * 0.7) as usize;
                }
                
                // Contraintes physiques
                if lead_duration < 500 { lead_duration = 500; }
                
                let max_lead_duration = (self.samples_per_step as f32 * 0.90) as usize;
                if lead_duration > max_lead_duration { lead_duration = max_lead_duration; }
                
                self.gate_timer_lead = lead_duration;
                self.gate_lead.set_value(1.0);

                // Event pour l'UI
                if let Ok(mut queue) = self.event_queue.lock() {
                    queue.push(VisualizationEvent {
                        note_midi: lead_midi,
                        instrument: 1,
                        step: self.sequencer_primary.current_step, // On utilise le step primaire comme référence temporelle
                        duration_samples: lead_duration,
                    });
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
