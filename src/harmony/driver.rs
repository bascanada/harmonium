//! HarmonicDriver - Orchestrateur principal du système harmonique
//!
//! Le HarmonicDriver choisit dynamiquement entre:
//! - Steedman Grammar (Tension < 0.5): Progressions fonctionnelles narratives
//! - Neo-Riemannian (Tension > 0.7, triades): Transformations géométriques P/L/R
//! - Parsimonious (Tension > 0.7, tétracordes): Voice-leading parsimonieux
//!
//! Le Lydian Chromatic Concept (LCC) agit comme filtre vertical global.

use super::chord::{Chord, ChordType, PitchClass};
use super::lydian_chromatic::LydianChromaticConcept;
use super::neo_riemannian::NeoRiemannianEngine;
use super::parsimonious::ParsimoniousDriver;
use super::steedman_grammar::{SteedmanGrammar, GrammarStyle};
use super::pivot::{PivotDetector, PivotType};
use super::{HarmonyContext, HarmonyDecision, HarmonyStrategy, RngCore, TransitionType};
use std::sync::Arc;

/// Mode de stratégie actuel (version V2 avec Parsimonious)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrategyMode {
    /// Utilise Steedman Grammar (harmonie fonctionnelle)
    Steedman,
    /// Utilise Neo-Riemannian (transformations géométriques pour triades)
    NeoRiemannian,
    /// Utilise Parsimonious Driver (voice-leading pour tous types d'accords)
    Parsimonious,
    /// En transition entre les stratégies
    Transitioning { progress: f32 },
}

impl StrategyMode {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyMode::Steedman => "Steedman",
            StrategyMode::NeoRiemannian => "Neo-Riemannian",
            StrategyMode::Parsimonious => "Parsimonious",
            StrategyMode::Transitioning { .. } => "Transitioning",
        }
    }
}

/// Orchestrateur principal du système harmonique V3
///
/// Gère trois stratégies harmoniques:
/// - Steedman: Harmonie fonctionnelle (basse tension)
/// - Neo-Riemannian: Transformations géométriques pour triades (haute tension)
/// - Parsimonious: Voice-leading parsimonieux pour tous accords (haute tension + tétracordes)
pub struct HarmonicDriver {
    /// Moteur Steedman Grammar (enhanced V2)
    steedman: SteedmanGrammar,
    /// Moteur Neo-Riemannian (triades uniquement - fast path)
    neo_riemannian: NeoRiemannianEngine,
    /// Moteur Parsimonious (tous types d'accords)
    parsimonious: ParsimoniousDriver,
    /// Contexte LCC partagé
    lcc: Arc<LydianChromaticConcept>,
    /// Détecteur de pivots
    pivot: PivotDetector,

    // === État ===
    /// Accord courant
    current_chord: Chord,
    /// Stratégie actuelle
    current_strategy: StrategyMode,
    /// Position dans la phrase (en mesures)
    phrase_position: usize,
    /// Dernière tension observée
    last_tension: f32,
    /// Tonique globale
    global_key: PitchClass,
}

impl HarmonicDriver {
    /// Crée un nouveau HarmonicDriver
    pub fn new(initial_key: PitchClass) -> Self {
        let lcc = Arc::new(LydianChromaticConcept::new());

        Self {
            steedman: SteedmanGrammar::new(lcc.clone()),
            neo_riemannian: NeoRiemannianEngine::new(lcc.clone()),
            parsimonious: ParsimoniousDriver::new(lcc.clone()),
            pivot: PivotDetector::new(lcc.clone()),
            lcc,
            current_chord: Chord::new(initial_key, ChordType::Major),
            current_strategy: StrategyMode::Steedman,
            phrase_position: 0,
            last_tension: 0.5,
            global_key: initial_key,
        }
    }

    /// Définit la tonique globale
    pub fn set_key(&mut self, key: PitchClass) {
        self.global_key = key % 12;
    }

    /// Définit le style de la grammaire Steedman
    pub fn set_grammar_style(&self, style: GrammarStyle) {
        self.steedman.set_style(style);
    }

    /// Retourne le nom de la stratégie actuelle
    pub fn current_strategy_name(&self) -> &'static str {
        self.current_strategy.name()
    }

    /// Retourne l'accord courant
    pub fn current_chord(&self) -> &Chord {
        &self.current_chord
    }

    /// Sélectionne la stratégie appropriée basée sur le contexte
    fn select_strategy(&self, ctx: &HarmonyContext) -> StrategyMode {
        let is_tetrad = ctx.current_chord.chord_type.is_tetrad();
        let (steedman_w, _neo_w) = self.pivot.crossfade_weight(ctx.tension);

        if steedman_w > 0.99 {
            // Zone Steedman pure (tension < 0.5)
            StrategyMode::Steedman
        } else if ctx.tension > 0.7 {
            // Haute tension: choisir selon le type d'accord
            if is_tetrad {
                // Tétracordes: utiliser Parsimonious (peut gérer les 7èmes)
                StrategyMode::Parsimonious
            } else {
                // Triades: utiliser Neo-Riemannian (fast path)
                StrategyMode::NeoRiemannian
            }
        } else {
            // Zone de transition (0.5 - 0.7): utiliser Parsimonious comme bridge
            StrategyMode::Parsimonious
        }
    }

    /// Génère le prochain accord basé sur l'état émotionnel
    pub fn next_chord(&mut self, tension: f32, valence: f32, rng: &mut dyn RngCore) -> HarmonyDecision {
        // Construire le contexte
        let ctx = HarmonyContext {
            current_chord: self.current_chord.clone(),
            global_key: self.global_key,
            tension,
            valence,
            measure_in_phrase: self.phrase_position / 4,
            beat_in_measure: self.phrase_position % 4,
        };

        // Sélectionner la stratégie appropriée
        let selected_strategy = self.select_strategy(&ctx);
        let (steedman_w, neo_w) = self.pivot.crossfade_weight(tension);

        let decision = match selected_strategy {
            StrategyMode::Steedman => {
                self.current_strategy = StrategyMode::Steedman;
                self.steedman.next_chord(&ctx, rng)
            }
            StrategyMode::NeoRiemannian => {
                // Vérifier que l'accord courant est une triade
                if ctx.current_chord.chord_type.is_triad() {
                    self.current_strategy = StrategyMode::NeoRiemannian;
                    self.neo_riemannian.next_chord(&ctx, rng)
                } else {
                    // Fallback: utiliser Parsimonious pour les tétracordes
                    self.current_strategy = StrategyMode::Parsimonious;
                    self.parsimonious.next_chord(&ctx, rng)
                }
            }
            StrategyMode::Parsimonious => {
                self.current_strategy = StrategyMode::Parsimonious;
                self.parsimonious.next_chord(&ctx, rng)
            }
            StrategyMode::Transitioning { .. } => {
                // Zone de transition avec pivot
                self.current_strategy = StrategyMode::Transitioning { progress: neo_w };
                self.handle_transition(&ctx, steedman_w, neo_w, rng)
            }
        };

        // Mettre à jour l'état
        self.current_chord = decision.next_chord.clone();
        self.phrase_position += 1;
        self.last_tension = tension;

        decision
    }

    /// Gère la transition entre stratégies
    fn handle_transition(
        &self,
        ctx: &HarmonyContext,
        steedman_w: f32,
        neo_w: f32,
        rng: &mut dyn RngCore,
    ) -> HarmonyDecision {
        // Vérifier si l'accord courant est un pivot
        let pivot_type = self.pivot.is_pivot_chord(&ctx.current_chord);

        if pivot_type != PivotType::None {
            // On est sur un pivot: laisser la stratégie dominante prendre le relais
            if neo_w > steedman_w {
                self.neo_riemannian.next_chord(ctx, rng)
            } else {
                self.steedman.next_chord(ctx, rng)
            }
        } else {
            // Pas de pivot: en créer un si nécessaire
            let target = if neo_w > steedman_w {
                self.neo_riemannian.next_chord(ctx, rng).next_chord
            } else {
                self.steedman.next_chord(ctx, rng).next_chord
            };

            // Générer un accord pivot
            let pivot_chord = self.pivot.create_pivot(&ctx.current_chord, &target, ctx.tension);

            // Obtenir la gamme LCC pour le pivot
            let parent = self.lcc.parent_lydian(&pivot_chord);
            let level = self.lcc.level_for_tension(ctx.tension);
            let suggested_scale = self.lcc.get_scale(parent, level);

            HarmonyDecision {
                next_chord: pivot_chord,
                transition_type: TransitionType::Pivot,
                suggested_scale,
            }
        }
    }

    /// Retourne la gamme LCC actuelle pour la génération mélodique
    pub fn get_current_scale(&self, tension: f32) -> Vec<PitchClass> {
        let parent = self.lcc.parent_lydian(&self.current_chord);
        let level = self.lcc.level_for_tension(tension);
        self.lcc.get_scale(parent, level)
    }

    /// Vérifie si une note mélodique est valide dans le contexte actuel
    pub fn is_valid_melody_note(&self, note: PitchClass, tension: f32) -> bool {
        self.lcc.is_valid_note(note, &self.current_chord, tension)
    }

    /// Retourne le poids d'une note pour la génération mélodique
    pub fn melody_note_weight(&self, note: PitchClass, tension: f32) -> f32 {
        self.lcc.note_weight(note, &self.current_chord, tension)
    }

    /// Réinitialise la position dans la phrase
    pub fn reset_phrase(&mut self) {
        self.phrase_position = 0;
    }

    /// Retourne le décalage root en demi-tons depuis la tonique globale
    pub fn root_offset(&self) -> i32 {
        ((self.current_chord.root as i32) - (self.global_key as i32) + 12) % 12
    }

    /// Retourne si l'accord courant est mineur
    pub fn is_minor(&self) -> bool {
        self.current_chord.chord_type.is_minor()
    }

    /// Convertit vers le format ChordQuality de l'ancien système (pour compatibilité)
    pub fn to_basic_quality(&self) -> super::basic::ChordQuality {
        self.current_chord.to_basic_quality()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRng(u64);

    impl RngCore for TestRng {
        fn next_f32(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
            ((self.0 >> 16) & 0x7fff) as f32 / 32768.0
        }

        fn next_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
            let f = self.next_f32();
            range.start + (f * (range.end - range.start) as f32) as usize
        }
    }

    #[test]
    fn test_low_tension_uses_steedman() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(42);

        // Tension basse -> Steedman
        let _decision = driver.next_chord(0.3, 0.5, &mut rng);
        assert_eq!(driver.current_strategy, StrategyMode::Steedman);
    }

    #[test]
    fn test_high_tension_uses_neo_riemannian() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(42);

        // Tension haute -> Neo-Riemannian
        let _decision = driver.next_chord(0.9, 0.5, &mut rng);
        assert_eq!(driver.current_strategy, StrategyMode::NeoRiemannian);
    }

    #[test]
    fn test_transition_zone() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(42);

        // Tension moyenne -> Parsimonious (bridge entre Steedman et Neo-Riemannian)
        let _decision = driver.next_chord(0.6, 0.5, &mut rng);
        // En zone de transition (0.5-0.7), Parsimonious sert de bridge
        assert!(matches!(driver.current_strategy, StrategyMode::Parsimonious));
    }

    #[test]
    fn test_scale_generation() {
        let driver = HarmonicDriver::new(0); // C

        // Basse tension -> gamme Lydienne (consonante)
        let scale = driver.get_current_scale(0.0);
        assert!(scale.contains(&0)); // C
        assert!(scale.contains(&6)); // F# (caractéristique du Lydien)

        // Haute tension -> gamme chromatique
        let scale = driver.get_current_scale(1.0);
        assert_eq!(scale.len(), 12);
    }

    #[test]
    fn test_root_offset() {
        let mut driver = HarmonicDriver::new(0); // C

        // Après quelques accords, vérifier que root_offset est correct
        driver.current_chord = Chord::new(7, ChordType::Major); // G
        assert_eq!(driver.root_offset(), 7);

        driver.current_chord = Chord::new(5, ChordType::Major); // F
        assert_eq!(driver.root_offset(), 5);
    }

    // ========================================
    // Tests de progression (pas de X → X)
    // ========================================

    #[test]
    fn test_steedman_produces_chord_changes() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(42);

        // Basse tension = Steedman
        let tension = 0.3;
        let valence = 0.5;

        let mut changes = 0;
        let iterations = 10;

        for _ in 0..iterations {
            let old_chord = driver.current_chord.clone();
            let decision = driver.next_chord(tension, valence, &mut rng);

            if decision.next_chord.root != old_chord.root
                || decision.next_chord.chord_type != old_chord.chord_type
            {
                changes += 1;
            }
        }

        // Au moins 50% des itérations devraient produire un changement
        assert!(
            changes >= iterations / 2,
            "Steedman devrait produire des changements d'accords: {} changes sur {}",
            changes,
            iterations
        );
    }

    #[test]
    fn test_neo_riemannian_produces_chord_changes() {
        let mut driver = HarmonicDriver::new(0); // C
        driver.current_chord = Chord::new(0, ChordType::Major); // Triade pour Neo-Riemannian
        let mut rng = TestRng(123);

        // Haute tension + triade = Neo-Riemannian
        let tension = 0.9;
        let valence = 0.0;

        let mut changes = 0;
        let iterations = 10;

        for _ in 0..iterations {
            let old_chord = driver.current_chord.clone();
            let decision = driver.next_chord(tension, valence, &mut rng);

            if decision.next_chord.root != old_chord.root
                || decision.next_chord.chord_type != old_chord.chord_type
            {
                changes += 1;
            }
        }

        // Neo-Riemannian devrait TOUJOURS changer l'accord (P, L, R transforment)
        assert!(
            changes >= iterations * 8 / 10,
            "Neo-Riemannian devrait produire des changements: {} changes sur {}",
            changes,
            iterations
        );
    }

    #[test]
    fn test_parsimonious_produces_chord_changes() {
        let mut driver = HarmonicDriver::new(0); // C
        driver.current_chord = Chord::new(0, ChordType::Major7); // Tétracorde pour Parsimonious
        let mut rng = TestRng(456);

        // Haute tension + tétracorde = Parsimonious
        let tension = 0.85;
        let valence = 0.5;

        let mut changes = 0;
        let iterations = 10;

        for _ in 0..iterations {
            let old_chord = driver.current_chord.clone();
            let decision = driver.next_chord(tension, valence, &mut rng);

            if decision.next_chord.root != old_chord.root
                || decision.next_chord.chord_type != old_chord.chord_type
            {
                changes += 1;
            }
        }

        // Parsimonious devrait produire des changements (voisins trouvés)
        assert!(
            changes >= iterations / 2,
            "Parsimonious devrait produire des changements: {} changes sur {}",
            changes,
            iterations
        );
    }

    #[test]
    fn test_progression_variety() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(789);

        // Collecter les accords sur 20 itérations avec tension variable
        let mut chords_seen: std::collections::HashSet<(u8, ChordType)> = std::collections::HashSet::new();

        for i in 0..20 {
            // Varier la tension pour tester différentes stratégies
            let tension = (i as f32 / 20.0) * 0.8 + 0.1; // 0.1 à 0.9
            let valence = if i % 2 == 0 { 0.5 } else { -0.5 };

            let decision = driver.next_chord(tension, valence, &mut rng);
            chords_seen.insert((decision.next_chord.root, decision.next_chord.chord_type));
        }

        // On devrait voir au moins 5 accords différents sur 20 itérations
        assert!(
            chords_seen.len() >= 5,
            "La progression devrait avoir de la variété: {} accords uniques sur 20",
            chords_seen.len()
        );
    }

    #[test]
    fn test_strategy_switch_continues_progression() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(999);

        // Commencer avec Steedman (basse tension)
        let decision1 = driver.next_chord(0.3, 0.5, &mut rng);
        assert_eq!(driver.current_strategy, StrategyMode::Steedman);
        let chord_after_steedman = decision1.next_chord.clone();

        // Passer à Neo-Riemannian (haute tension, garder triade)
        driver.current_chord = Chord::new(chord_after_steedman.root, ChordType::Major);
        let chord_before_neo = driver.current_chord.clone(); // Capturer AVANT

        let decision2 = driver.next_chord(0.9, 0.5, &mut rng);
        assert_eq!(driver.current_strategy, StrategyMode::NeoRiemannian);

        // Neo-Riemannian devrait produire un accord différent
        assert!(
            decision2.next_chord.root != chord_before_neo.root
                || decision2.next_chord.chord_type != chord_before_neo.chord_type,
            "Le switch vers Neo-Riemannian devrait changer l'accord: {} → {}",
            chord_before_neo.name(),
            decision2.next_chord.name()
        );
    }

    #[test]
    fn test_tetrad_high_tension_uses_parsimonious() {
        let mut driver = HarmonicDriver::new(0); // C
        driver.current_chord = Chord::new(0, ChordType::Dominant7); // Tétracorde
        let mut rng = TestRng(111);

        // Haute tension + tétracorde = Parsimonious (pas Neo-Riemannian)
        let decision = driver.next_chord(0.85, 0.5, &mut rng);

        assert_eq!(
            driver.current_strategy,
            StrategyMode::Parsimonious,
            "Tétracorde à haute tension devrait utiliser Parsimonious"
        );

        // Devrait produire un changement
        assert!(
            decision.next_chord.root != 0 || decision.next_chord.chord_type != ChordType::Dominant7,
            "Parsimonious devrait changer l'accord"
        );
    }

    #[test]
    fn test_no_stuck_on_same_chord() {
        let mut driver = HarmonicDriver::new(0); // C
        let mut rng = TestRng(222);

        // Test avec différentes configurations
        let configs = [
            (0.2, 0.5, ChordType::Major),      // Steedman
            (0.6, 0.0, ChordType::Minor),      // Parsimonious bridge
            (0.9, 0.5, ChordType::Major),      // Neo-Riemannian
            (0.85, -0.5, ChordType::Minor7),   // Parsimonious tetrad
        ];

        for (tension, valence, start_type) in configs {
            driver.current_chord = Chord::new(0, start_type);

            let mut same_count = 0;
            for _ in 0..5 {
                let old = driver.current_chord.clone();
                let decision = driver.next_chord(tension, valence, &mut rng);

                if decision.next_chord.root == old.root
                    && decision.next_chord.chord_type == old.chord_type
                {
                    same_count += 1;
                }
            }

            // Ne devrait pas rester bloqué plus de 2 fois sur 5
            assert!(
                same_count <= 2,
                "Config ({}, {}, {:?}): bloqué {} fois sur 5",
                tension,
                valence,
                start_type,
                same_count
            );
        }
    }
}
