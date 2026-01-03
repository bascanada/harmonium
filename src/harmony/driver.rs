//! HarmonicDriver - Orchestrateur principal du système harmonique
//!
//! Le HarmonicDriver choisit dynamiquement entre:
//! - Steedman Grammar (Tension < 0.5): Progressions fonctionnelles narratives
//! - Neo-Riemannian (Tension > 0.7): Transformations géométriques atonales
//!
//! Le Lydian Chromatic Concept (LCC) agit comme filtre vertical global.

use super::chord::{Chord, ChordType, PitchClass};
use super::lydian_chromatic::LydianChromaticConcept;
use super::neo_riemannian::NeoRiemannianEngine;
use super::steedman_grammar::SteedmanGrammar;
use super::pivot::{PivotDetector, PivotType};
use super::{HarmonyContext, HarmonyDecision, HarmonyStrategy, RngCore, TransitionType};
use std::sync::Arc;

/// Mode de stratégie actuel
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrategyMode {
    /// Utilise Steedman Grammar (harmonie fonctionnelle)
    Steedman,
    /// Utilise Neo-Riemannian (transformations géométriques)
    NeoRiemannian,
    /// En transition entre les deux stratégies
    Transitioning { progress: f32 },
}

impl StrategyMode {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyMode::Steedman => "Steedman",
            StrategyMode::NeoRiemannian => "Neo-Riemannian",
            StrategyMode::Transitioning { .. } => "Transitioning",
        }
    }
}

/// Orchestrateur principal du système harmonique V2
pub struct HarmonicDriver {
    /// Moteur Steedman Grammar
    steedman: SteedmanGrammar,
    /// Moteur Neo-Riemannian
    neo_riemannian: NeoRiemannianEngine,
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

    /// Retourne le nom de la stratégie actuelle
    pub fn current_strategy_name(&self) -> &'static str {
        self.current_strategy.name()
    }

    /// Retourne l'accord courant
    pub fn current_chord(&self) -> &Chord {
        &self.current_chord
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

        // Déterminer la stratégie préférée
        let (steedman_w, neo_w) = self.pivot.crossfade_weight(tension);

        let decision = if steedman_w > 0.99 {
            // Zone Steedman pure
            self.current_strategy = StrategyMode::Steedman;
            self.steedman.next_chord(&ctx, rng)
        } else if neo_w > 0.99 {
            // Zone Neo-Riemannian pure
            self.current_strategy = StrategyMode::NeoRiemannian;
            self.neo_riemannian.next_chord(&ctx, rng)
        } else {
            // Zone de transition
            self.current_strategy = StrategyMode::Transitioning { progress: neo_w };
            self.handle_transition(&ctx, steedman_w, neo_w, rng)
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

        // Tension moyenne -> Transition
        let _decision = driver.next_chord(0.6, 0.5, &mut rng);
        assert!(matches!(driver.current_strategy, StrategyMode::Transitioning { .. }));
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
        let mut rng = TestRng(42);

        // Après quelques accords, vérifier que root_offset est correct
        driver.current_chord = Chord::new(7, ChordType::Major); // G
        assert_eq!(driver.root_offset(), 7);

        driver.current_chord = Chord::new(5, ChordType::Major); // F
        assert_eq!(driver.root_offset(), 5);
    }
}
