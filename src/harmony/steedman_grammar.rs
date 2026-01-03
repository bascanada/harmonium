//! Steedman Grammar - Grammaire générative pour progressions harmoniques
//!
//! Implémentation basée sur les travaux de Mark Steedman sur les grammaires
//! combinatoires pour la musique jazz/pop.
//!
//! Règles de réécriture: V -> ii-V, I -> vi-I, etc.

use super::chord::{Chord, ChordType, PitchClass};
use super::{HarmonyContext, HarmonyDecision, HarmonyStrategy, RngCore, TransitionType};
use super::lydian_chromatic::LydianChromaticConcept;
use std::sync::{Arc, RwLock};

/// Chiffres romains (degrés de la gamme)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RomanNumeral {
    I,
    II,
    III,
    IV,
    V,
    VI,
    VII,
    /// Dominante secondaire: V/V
    VofV,
    /// Dominante secondaire: V/ii
    VofII,
    /// Dominante secondaire: V/IV
    VofIV,
}

impl RomanNumeral {
    /// Intervalle en demi-tons depuis la tonique
    pub fn interval(&self) -> u8 {
        match self {
            RomanNumeral::I => 0,
            RomanNumeral::II => 2,
            RomanNumeral::III => 4,
            RomanNumeral::IV => 5,
            RomanNumeral::V => 7,
            RomanNumeral::VI => 9,
            RomanNumeral::VII => 11,
            RomanNumeral::VofV => 2,   // V/V = II7 (D7 en C)
            RomanNumeral::VofII => 9,  // V/ii = VI7 (A7 en C)
            RomanNumeral::VofIV => 0,  // V/IV = I7 (C7 en C)
        }
    }

    /// Nom pour affichage
    pub fn name(&self) -> &'static str {
        match self {
            RomanNumeral::I => "I",
            RomanNumeral::II => "ii",
            RomanNumeral::III => "iii",
            RomanNumeral::IV => "IV",
            RomanNumeral::V => "V",
            RomanNumeral::VI => "vi",
            RomanNumeral::VII => "vii°",
            RomanNumeral::VofV => "V/V",
            RomanNumeral::VofII => "V/ii",
            RomanNumeral::VofIV => "V/IV",
        }
    }
}

/// Règle de réécriture de la grammaire
#[derive(Clone, Debug)]
pub struct RewriteRule {
    /// Symbole de gauche (ce qu'on remplace)
    pub lhs: RomanNumeral,
    /// Symbole(s) de droite (par quoi on remplace)
    pub rhs: Vec<RomanNumeral>,
    /// Poids de la règle (probabilité relative)
    pub weight: f32,
    /// Valence minimale pour appliquer cette règle
    pub min_valence: f32,
    /// Valence maximale
    pub max_valence: f32,
}

/// Grammaire de Steedman pour progressions harmoniques
///
/// Utilise RwLock pour permettre la mutabilité intérieure thread-safe,
/// ce qui permet de maintenir l'état des expansions en cours
/// (ex: V → ii-V garde le V en attente après avoir joué ii)
pub struct SteedmanGrammar {
    /// Règles de réécriture
    rules: Vec<RewriteRule>,
    /// Contexte LCC
    lcc: Arc<LydianChromaticConcept>,
    /// État mutable: numeral courant + progression en attente
    state: RwLock<GrammarState>,
}

/// État interne mutable de la grammaire
#[derive(Clone, Debug)]
struct GrammarState {
    /// Accord courant (en numéraux romains)
    current_numeral: RomanNumeral,
    /// Progression générée en attente (ex: après V→ii-V, contient [V])
    pending_progression: Vec<RomanNumeral>,
}

impl SteedmanGrammar {
    /// Crée une nouvelle grammaire avec les règles jazz/pop standard
    pub fn new(lcc: Arc<LydianChromaticConcept>) -> Self {
        let mut grammar = Self {
            rules: Vec::new(),
            lcc,
            state: RwLock::new(GrammarState {
                current_numeral: RomanNumeral::I,
                pending_progression: Vec::new(),
            }),
        };

        grammar.init_rules();
        grammar
    }

    /// Initialise les règles de réécriture
    fn init_rules(&mut self) {
        // === PRÉPARATIONS DE DOMINANTE ===

        // V -> ii-V (la plus commune en jazz)
        self.add_rule(RomanNumeral::V, vec![RomanNumeral::II, RomanNumeral::V], 0.85, -1.0, 1.0);

        // V -> IV-V (approche plagale, rock/pop)
        self.add_rule(RomanNumeral::V, vec![RomanNumeral::IV, RomanNumeral::V], 0.5, 0.0, 1.0);

        // V -> V/V-V (dominante secondaire)
        self.add_rule(RomanNumeral::V, vec![RomanNumeral::VofV, RomanNumeral::V], 0.3, -1.0, 1.0);

        // === ÉLABORATIONS DE TONIQUE ===

        // I -> vi-I (mouvement déceptif inversé)
        self.add_rule(RomanNumeral::I, vec![RomanNumeral::VI, RomanNumeral::I], 0.4, -0.5, 0.5);

        // I -> IV-I (plagal)
        self.add_rule(RomanNumeral::I, vec![RomanNumeral::IV, RomanNumeral::I], 0.6, 0.2, 1.0);

        // I -> iii-I (mouvement par tierce)
        self.add_rule(RomanNumeral::I, vec![RomanNumeral::III, RomanNumeral::I], 0.3, -1.0, 0.3);

        // === PRÉPARATIONS DE SOUS-DOMINANTE ===

        // IV -> ii-IV
        self.add_rule(RomanNumeral::IV, vec![RomanNumeral::II, RomanNumeral::IV], 0.4, -1.0, 1.0);

        // === CADENCES ===

        // V -> I (cadence authentique, implicite mais utile)
        self.add_rule(RomanNumeral::V, vec![RomanNumeral::I], 0.9, -1.0, 1.0);

        // IV -> I (cadence plagale)
        self.add_rule(RomanNumeral::IV, vec![RomanNumeral::I], 0.5, 0.2, 1.0);

        // V -> vi (cadence déceptive)
        self.add_rule(RomanNumeral::V, vec![RomanNumeral::VI], 0.3, -0.5, 0.5);
    }

    /// Ajoute une règle de réécriture
    fn add_rule(&mut self, lhs: RomanNumeral, rhs: Vec<RomanNumeral>, weight: f32, min_valence: f32, max_valence: f32) {
        self.rules.push(RewriteRule {
            lhs,
            rhs,
            weight,
            min_valence,
            max_valence,
        });
    }

    /// Trouve les règles applicables pour un symbole et une valence
    fn applicable_rules(&self, symbol: RomanNumeral, valence: f32) -> Vec<&RewriteRule> {
        self.rules
            .iter()
            .filter(|r| r.lhs == symbol && valence >= r.min_valence && valence <= r.max_valence)
            .collect()
    }

    /// Sélectionne une règle de manière pondérée
    fn select_rule(&self, rules: &[&RewriteRule], rng: &mut dyn RngCore) -> Option<Vec<RomanNumeral>> {
        if rules.is_empty() {
            return None;
        }

        let total_weight: f32 = rules.iter().map(|r| r.weight).sum();
        let mut choice = rng.next_f32() * total_weight;

        for rule in rules {
            choice -= rule.weight;
            if choice <= 0.0 {
                return Some(rule.rhs.clone());
            }
        }

        // Fallback: première règle
        rules.first().map(|r| r.rhs.clone())
    }

    /// Convertit un numeral en accord concret
    pub fn realize(&self, numeral: RomanNumeral, key: PitchClass, valence: f32) -> Chord {
        let interval = numeral.interval();
        let root = (key + interval) % 12;

        let chord_type = match numeral {
            RomanNumeral::I => {
                if valence > 0.3 {
                    ChordType::Major7
                } else {
                    ChordType::Major
                }
            }
            RomanNumeral::II => {
                if valence < -0.3 {
                    ChordType::Minor7
                } else {
                    ChordType::Minor
                }
            }
            RomanNumeral::III => ChordType::Minor,
            RomanNumeral::IV => ChordType::Major,
            RomanNumeral::V => {
                if valence > 0.3 {
                    ChordType::Dominant7
                } else {
                    ChordType::Major
                }
            }
            RomanNumeral::VI => ChordType::Minor,
            RomanNumeral::VII => ChordType::HalfDiminished,
            RomanNumeral::VofV | RomanNumeral::VofII | RomanNumeral::VofIV => ChordType::Dominant7,
        };

        Chord::new(root, chord_type)
    }

    /// Génère le prochain accord de la progression (stateful via RwLock)
    fn generate_next_stateful(&self, ctx: &HarmonyContext, rng: &mut dyn RngCore) -> RomanNumeral {
        let mut state = self.state.write().unwrap();

        // Si on a une progression en attente, la consommer
        if !state.pending_progression.is_empty() {
            let next = state.pending_progression.remove(0);
            state.current_numeral = next;
            return next;
        }

        // Sinon, essayer d'étendre le symbole courant
        let rules = self.applicable_rules(state.current_numeral, ctx.valence);

        let next_numeral = if let Some(expansion) = self.select_rule(&rules, rng) {
            if expansion.len() > 1 {
                // Stocker les symboles suivants dans la progression en attente
                // Ex: V → ii-V devient: retourne ii, garde [V] en attente
                state.pending_progression = expansion[1..].to_vec();
                expansion[0]
            } else if !expansion.is_empty() {
                expansion[0]
            } else {
                self.default_progression_for(state.current_numeral, ctx, rng)
            }
        } else {
            // Fallback: mouvement par quinte ou progression standard
            self.default_progression_for(state.current_numeral, ctx, rng)
        };

        state.current_numeral = next_numeral;
        next_numeral
    }

    /// Progression par défaut quand aucune règle ne s'applique
    fn default_progression_for(&self, current: RomanNumeral, ctx: &HarmonyContext, rng: &mut dyn RngCore) -> RomanNumeral {
        let choice = rng.next_f32();

        // Progression cyclique basée sur le cercle des quintes
        match current {
            RomanNumeral::I => {
                if ctx.valence > 0.0 && choice < 0.4 {
                    RomanNumeral::V
                } else if choice < 0.7 {
                    RomanNumeral::IV
                } else {
                    RomanNumeral::VI
                }
            }
            RomanNumeral::II => RomanNumeral::V,
            RomanNumeral::III => RomanNumeral::VI,
            RomanNumeral::IV => {
                if choice < 0.6 {
                    RomanNumeral::V
                } else {
                    RomanNumeral::I
                }
            }
            RomanNumeral::V => RomanNumeral::I,
            RomanNumeral::VI => {
                if choice < 0.5 {
                    RomanNumeral::II
                } else {
                    RomanNumeral::IV
                }
            }
            RomanNumeral::VII => RomanNumeral::I,
            RomanNumeral::VofV => RomanNumeral::V,
            RomanNumeral::VofII => RomanNumeral::II,
            RomanNumeral::VofIV => RomanNumeral::IV,
        }
    }

    /// Retourne true si une expansion est en attente
    pub fn has_pending(&self) -> bool {
        self.state.read().map(|s| !s.pending_progression.is_empty()).unwrap_or(false)
    }

    /// Retourne le numeral courant (pour debug/UI)
    pub fn current_numeral(&self) -> RomanNumeral {
        self.state.read().map(|s| s.current_numeral).unwrap_or(RomanNumeral::I)
    }
}

impl HarmonyStrategy for SteedmanGrammar {
    fn next_chord(&self, ctx: &HarmonyContext, rng: &mut dyn RngCore) -> HarmonyDecision {
        // Utilise la méthode stateful avec RefCell pour maintenir l'état
        // des expansions en cours (ex: V → ii-V garde le V en attente)
        let next_numeral = self.generate_next_stateful(ctx, rng);
        let next_chord = self.realize(next_numeral, ctx.global_key, ctx.valence);

        // Obtenir la gamme LCC
        let parent = self.lcc.parent_lydian(&next_chord);
        let level = self.lcc.level_for_tension(ctx.tension);
        let suggested_scale = self.lcc.get_scale(parent, level);

        HarmonyDecision {
            next_chord,
            transition_type: TransitionType::Functional,
            suggested_scale,
        }
    }

    fn name(&self) -> &'static str {
        "Steedman Grammar"
    }
}

impl SteedmanGrammar {
    /// Déduit le numeral romain d'un accord basé sur la tonique globale
    fn deduce_numeral(&self, chord: &Chord, key: PitchClass) -> RomanNumeral {
        let interval = (chord.root + 12 - key) % 12;

        match interval {
            0 => RomanNumeral::I,
            2 => RomanNumeral::II,
            4 => RomanNumeral::III,
            5 => RomanNumeral::IV,
            7 => RomanNumeral::V,
            9 => RomanNumeral::VI,
            11 => RomanNumeral::VII,
            _ => RomanNumeral::I, // Fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grammar() -> SteedmanGrammar {
        SteedmanGrammar::new(Arc::new(LydianChromaticConcept::new()))
    }

    #[test]
    fn test_realize_basic() {
        let grammar = make_grammar();

        // I en C = C Major
        let chord = grammar.realize(RomanNumeral::I, 0, 0.5);
        assert_eq!(chord.root, 0);
        assert!(matches!(chord.chord_type, ChordType::Major | ChordType::Major7));

        // V en C = G (7 ou 12)
        let chord = grammar.realize(RomanNumeral::V, 0, 0.5);
        assert_eq!(chord.root, 7);

        // ii en C = D Minor
        let chord = grammar.realize(RomanNumeral::II, 0, 0.0);
        assert_eq!(chord.root, 2);
        assert!(chord.chord_type.is_minor());
    }

    #[test]
    fn test_deduce_numeral() {
        let grammar = make_grammar();

        // C en key C = I
        assert_eq!(grammar.deduce_numeral(&Chord::new(0, ChordType::Major), 0), RomanNumeral::I);

        // G en key C = V
        assert_eq!(grammar.deduce_numeral(&Chord::new(7, ChordType::Major), 0), RomanNumeral::V);

        // Am en key C = vi
        assert_eq!(grammar.deduce_numeral(&Chord::new(9, ChordType::Minor), 0), RomanNumeral::VI);
    }

    #[test]
    fn test_applicable_rules() {
        let grammar = make_grammar();

        // V devrait avoir plusieurs règles applicables
        let rules = grammar.applicable_rules(RomanNumeral::V, 0.0);
        assert!(!rules.is_empty());

        // La règle V -> ii-V devrait être présente
        let has_ii_v = rules.iter().any(|r| r.rhs == vec![RomanNumeral::II, RomanNumeral::V]);
        assert!(has_ii_v);
    }
}
