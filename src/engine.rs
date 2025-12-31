use fundsp::hacker32::*;
use crate::sequencer::Sequencer;
use crate::harmony::HarmonyNavigator;
use crate::progression::{Progression, ChordStep, ChordQuality};
use crate::log;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;
use rand::Rng;
use std::sync::{Arc, Mutex};

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
    pub current_step: usize,         // Step dans la mesure (0-15)
    pub progression_name: String,    // Nom de la progression active ("Pop Energetic", etc.)
    pub progression_length: usize,   // Longueur de la progression (2-4 accords)
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
        }
    }
}

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
    pub harmony_state: Arc<Mutex<HarmonyState>>,  // État harmonique pour l'UI
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
    // === PROGRESSION HARMONIQUE ADAPTATIVE ===
    measure_counter: usize,               // Compte les mesures (16 steps = 1 mesure)
    current_progression: Vec<ChordStep>,  // Progression chargée (dépend de valence/tension)
    progression_index: usize,             // Position dans la progression actuelle
    last_valence_choice: f32,             // Hystérésis: valence qui a déclenché le dernier choix
    last_tension_choice: f32,             // Hystérésis: tension qui a déclenché le dernier choix
    // === ARTICULATION DYNAMIQUE (Anti-Legato) ===
    gate_timer: usize,                    // Compteur dégressif pour la durée de la note
    current_gate_duration: usize,         // Durée cible de la note actuelle (en samples)
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
        // Release réduit à 0.1 (100ms) pour que les silences soient vraiment silencieux
        // Attack plus franche (0.005) pour une meilleure définition rythmique
        let envelope = var(&gate) >> adsr_live(0.005, 0.15, 0.6, 0.1);
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
            measure_counter: 0,
            current_progression,
            progression_index: 0,
            last_valence_choice: initial_params.valence,
            last_tension_choice: initial_params.tension,
            gate_timer: 0,
            current_gate_duration: 0,
        }
    }

    pub fn process(&mut self) -> (f32, f32) {
        // === ÉTAPE A: Récupérer l'état cible (Target) ===
        let target = {
            self.target_state.lock().unwrap().clone()
        }; // Lock relâché immédiatement
        
        // === GESTION DE LA DURÉE DE NOTE (ARTICULATION) ===
        // Si le timer arrive à 0, on coupe le son (Note Off)
        // Cela crée l'espace de "respiration" entre les notes
        if self.gate_timer > 0 {
            self.gate_timer -= 1;
            if self.gate_timer == 0 {
                self.gate.set_value(0.0);
            }
        }

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

        // === ÉTAPE E: Logique de Tick des Séquenceurs (Polyrythmie + Progression Harmonique) ===
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            
            // On force le Gate à 0 au début du step pour garantir le "re-trigger"
            // si la note précédente était très longue (Legato)
            self.gate.set_value(0.0);
            
            // Tick des deux séquenceurs
            let trigger_primary = self.sequencer_primary.tick();
            let trigger_secondary = self.sequencer_secondary.tick();
            
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
            let is_strong_beat = self.sequencer_primary.current_step % 4 == 0;
            
            // Mettre à jour le step courant dans harmony_state (pour l'UI)
            if let Ok(mut state) = self.harmony_state.lock() {
                state.current_step = self.sequencer_primary.current_step;
            }
            
            // === DÉCLENCHEMENT PUR (Sans aléatoire destructeur) ===
            // On respecte scrupuleusement le rythme euclidien généré
            // L'espace vient de la DENSITÉ (pulses), pas d'une suppression aléatoire
            let trigger = trigger_primary || trigger_secondary;
            
            if trigger {
                // 1. Génération mélodique probabiliste avec contexte rythmique
                let freq = self.harmony.next_note(is_strong_beat);
                self.frequency.set_value(freq);
                
                // === C'est ici que le Style émerge de la Géométrie ===
                
                // Formule d'Articulation :
                // Tension basse (0.0) → Legato fluide (90% du temps)
                // Tension haute (1.0) → Staccato agressif (15% du temps)
                let articulation_base = 0.90 - (self.current_state.tension * 0.75);
                
                // Humanisation légère sur le TEMPS (pas sur l'existence de la note)
                // Micro-variations de timing (±5%) pour éviter l'effet quantifié
                let mut rng = rand::thread_rng();
                let micro_timing = rng.gen_range(0.95..1.05);
                
                // Calcul de la durée en samples
                let mut duration = (self.samples_per_step as f32 * articulation_base * micro_timing) as usize;
                
                // Contraintes physiques
                // 1. Minimum vital (10ms @ 44.1kHz) pour entendre l'attaque
                if duration < 500 { duration = 500; }
                
                // 2. Maximum vital : on laisse TOUJOURS un petit espace à la fin (au moins 10%)
                // pour que l'enveloppe puisse redescendre avant la prochaine note.
                // C'est ça qui crée la définition rythmique.
                let max_duration = (self.samples_per_step as f32 * 0.90) as usize;
                if duration > max_duration { duration = max_duration; }
                
                self.current_gate_duration = duration;
                
                // 3. DÉCLENCHEMENT DE LA NOTE
                self.gate_timer = self.current_gate_duration;
                self.gate.set_value(1.0);
                
                // 4. ACCENTUATION DES TEMPS FORTS (pour future implémentation velocity)
                // Temps forts: vélocité 1.0, temps faibles: 0.7
                // let velocity = if is_strong_beat { 1.0 } else { 0.7 };
                // self.velocity_gain.set_value(velocity); // À implémenter dans le graph DSP
            }
            // Note: On ne met PLUS self.gate.set_value(0.0) dans le else
            // Le gate_timer s'occupe maintenant de fermer le gate au bon moment
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
