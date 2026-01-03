//! Module de génération mélodique - HarmonyNavigator
//!
//! Génération de mélodies basée sur:
//! - Chaînes de Markov (probabilités conditionnelles)
//! - Bruit fractal 1/f (Pink Noise)
//! - Hybride Markov+Fractal

use rust_music_theory::scale::{Scale, ScaleType, Direction};
use rust_music_theory::note::{PitchSymbol, Pitch, Notes};
use rand::distributions::{Distribution, WeightedIndex};
use super::basic::ChordQuality;
use crate::fractal::PinkNoise;

pub struct HarmonyNavigator {
    pub current_scale: Scale,
    pub current_index: i32,
    pub octave: i32,
    scale_len: usize,
    last_step: i32,  // Mémoire mélodique pour "Gap Fill" (Temperley)
    // === PROGRESSION HARMONIQUE: Contexte d'accord local ===
    pub current_chord_notes: Vec<u8>, // Pitch classes de l'accord actuel (ex: [0,4,7] pour Do Maj)
    pub global_key_root: u8,          // Tonique globale du morceau (0=C, 1=C#, etc.)
    // === FRACTAL NOISE ===
    pink_noise: PinkNoise,
    hurst_factor: f32,
}

impl HarmonyNavigator {
    pub fn new(root_note: PitchSymbol, scale_type: ScaleType, octave: i32) -> Self {
        let pitch = Pitch::from(root_note);
        let scale = Scale::new(scale_type, pitch, octave as u8, None, Direction::Ascending).unwrap();
        let scale_len = scale.notes().len();

        // Départ: accord I majeur (tonique, tierce majeure, quinte, septième majeure)
        let global_key_root = pitch.into_u8();
        let current_chord_notes = vec![0, 4, 7, 11]; // I Maj7

        HarmonyNavigator {
            current_scale: scale,
            current_index: 0,
            octave,
            scale_len,
            last_step: 0,
            current_chord_notes,
            global_key_root,
            pink_noise: PinkNoise::new(5), // 5 octaves de profondeur
            hurst_factor: 0.7, // Valeur par défaut pour une mélodie "chantante"
        }
    }

    /// Change le contexte harmonique (accord courant)
    /// Root offset: décalage en demi-tons par rapport à la tonique globale
    /// Ex: root_offset=0 (I), root_offset=5 (IV), root_offset=7 (V), root_offset=9 (vi)
    ///
    /// Basé sur la théorie des progressions fonctionnelles (Tonique-Sous-Dominante-Dominante)
    pub fn set_chord_context(&mut self, root_offset: i32, quality: ChordQuality) {
        // Construction de l'accord selon sa qualité
        let (third, fifth, seventh) = match quality {
            ChordQuality::Major => (4, 7, 11),     // Maj7: 1-3-5-7
            ChordQuality::Minor => (3, 7, 10),     // m7: 1-♭3-5-♭7
            ChordQuality::Dominant7 => (4, 7, 10), // 7: 1-3-5-♭7 (tension)
            ChordQuality::Diminished => (3, 6, 9), // dim7: 1-♭3-♭5-♭♭7
            ChordQuality::Sus2 => (2, 7, 0),       // sus2: 1-2-5 (pas de tierce)
        };

        // Notes de l'accord en pitch classes (modulo 12)
        self.current_chord_notes = if quality == ChordQuality::Sus2 {
            // Sus2 n'a que 3 notes (pas de 7ème)
            vec![
                (root_offset % 12) as u8,
                ((root_offset + third) % 12) as u8,
                ((root_offset + fifth) % 12) as u8,
            ]
        } else {
            vec![
                (root_offset % 12) as u8,             // Fondamentale
                ((root_offset + third) % 12) as u8,   // Tierce (ou 2nde pour sus2)
                ((root_offset + fifth) % 12) as u8,   // Quinte (juste ou diminuée)
                ((root_offset + seventh) % 12) as u8, // Septième
            ]
        };
    }

    /// Vérifie si une note de la gamme fait partie de l'accord courant
    /// Ceci permet de distinguer notes d'accord (stables) vs notes de passage (transitoires)
    fn is_in_current_chord(&self, scale_degree: i32) -> bool {
        let notes = self.current_scale.notes();
        let len = notes.len() as i32;

        // Obtenir la note de la gamme à cette position
        let index = scale_degree.rem_euclid(len);
        let note = &notes[index as usize];

        // Convertir en pitch class (modulo 12)
        let pitch_class = note.pitch.into_u8();

        // Vérifier si cette pitch class est dans l'accord courant
        self.current_chord_notes.contains(&pitch_class)
    }

    /// Génère la prochaine note en utilisant des probabilités conditionnelles (Chaînes de Markov)
    /// Basé sur "Music and Probability" (David Temperley)
    pub fn next_note(&mut self, is_strong_beat: bool) -> f32 {
        let mut rng = rand::thread_rng();

        // Position normalisée dans la gamme (0 = tonique, 1 = 2ème degré, etc.)
        let normalized_index = self.current_index.rem_euclid(self.scale_len as i32);

        // === CHAÎNES DE MARKOV: Probabilités conditionnelles ===
        let (steps, weights) = self.get_weighted_steps(normalized_index, is_strong_beat);

        // Sélection pondérée
        let dist = WeightedIndex::new(&weights).unwrap();
        let chosen_step = steps[dist.sample(&mut rng)];

        // === GAP FILL (Temperley): Après un grand saut, revenir dans l'autre direction ===
        // Principe: Si le dernier mouvement était un saut > 2, compenser en revenant
        let final_step = if self.last_step.abs() > 2 {
            // Grand saut précédent: forcer un retour par mouvement contraire
            if self.last_step > 0 { -1 } else { 1 }
        } else {
            chosen_step
        };

        self.last_step = final_step; // Mémoriser pour la prochaine fois
        self.current_index += final_step;

        // Contrainte: rester dans une tessiture raisonnable (± 2 octaves)
        self.current_index = self.current_index.clamp(
            -(self.scale_len as i32 * 2),
            self.scale_len as i32 * 2
        );

        self.get_frequency()
    }

    /// Génère la prochaine note en utilisant du bruit fractal (1/f)
    /// Cela crée des mélodies plus organiques et structurées que les chaînes de Markov
    pub fn next_note_fractal(&mut self) -> f32 {
        // 1. Obtenir une valeur "tendance" du bruit fractal
        let fractal_drift = self.pink_noise.next();

        // 2. Mapper cette valeur à un index dans la gamme (autour d'un centre)
        // Cela remplace la marche aléatoire pure par une évolution structurée
        let center_index = 0; // Tonique centrale
        let target_index = center_index + (fractal_drift * 10.0) as i32; // Amplitude de ±10 degrés

        // 3. Lisser le mouvement (simulation de l'Exposant de Hurst)
        // Au lieu de sauter directement au target, on s'en rapproche
        // Un facteur faible (0.1) = très lisse (Hurst élevé), facteur fort (1.0) = erratique

        let diff = target_index - self.current_index;

        // Utilisation de hurst_factor pour contrôler la taille maximale du saut
        // Si hurst_factor est petit (ex: 0.1), on force des petits pas (mouvement conjoint)
        // Si hurst_factor est grand (ex: 1.0), on autorise des sauts plus grands vers la cible
        let max_step = (self.hurst_factor * 5.0).max(1.0) as i32;
        let step = diff.clamp(-max_step, max_step);

        self.current_index += step;

        // ... contraintes de gamme et conversion en fréquence ...
        self.get_frequency()
    }

    /// GÉNÉRATION HYBRIDE : Le Bruit Rose (GPS) guide les choix de Markov (Conducteur).
    /// is_strong_beat : Permet de favoriser les notes de l'accord sur les temps forts
    pub fn next_note_hybrid(&mut self, is_strong_beat: bool) -> f32 {
        let mut rng = rand::thread_rng();

        // 1. LE GPS (Bruit Fractal) : Quelle est la "tendance" globale ?
        // On récupère la valeur cible idéale dictée par le 1/f
        let fractal_drift = self.pink_noise.next();
        let center_index = 0; // Tonique centrale
        // Amplitude de ±12 degrés (environ 2 octaves) pour la cible
        let target_index = center_index + (fractal_drift * 12.0) as i32;

        // 2. LE CONDUCTEUR (Markov) : Quels sont les mouvements musicaux valides ?
        // On récupère les probabilités basées sur la théorie (tonique, sensible, etc.)
        let normalized_index = self.current_index.rem_euclid(self.scale_len as i32);
        let (steps, original_weights) = self.get_weighted_steps(normalized_index, is_strong_beat);

        // 3. LA FUSION : On biaise les poids vers la cible fractale
        let mut final_weights = Vec::with_capacity(original_weights.len());
        let current_dist = (target_index - self.current_index).abs();

        // Facteur d'influence du fractal (lié au paramètre smoothness/Hurst)
        // Hurst bas (0.1) = peu d'influence, on suit surtout Markov (local)
        // Hurst haut (1.0) = forte influence, on court vers la cible (global)
        let fractal_influence = 0.5 + (self.hurst_factor * 3.0);

        for (i, &step) in steps.iter().enumerate() {
            let predicted_index = self.current_index + step;
            let new_dist = (target_index - predicted_index).abs();

            let mut weight = original_weights[i] as f32;

            // Si ce pas nous rapproche de la cible fractale, on booste son poids
            if new_dist < current_dist {
                weight *= fractal_influence;
            } else {
                // Si on s'éloigne, on réduit légèrement le poids (mais on ne l'interdit pas !)
                weight *= 0.8;
            }

            final_weights.push(weight);
        }

        // 4. SÉLECTION PONDÉRÉE
        // On recrée une distribution avec les nouveaux poids
        let dist = WeightedIndex::new(&final_weights).unwrap_or_else(|_| {
            // Fallback de sécurité si tous les poids sont 0 (rare)
            WeightedIndex::new(vec![1.0; final_weights.len()]).unwrap()
        });

        let chosen_step = steps[dist.sample(&mut rng)];

        // === Gap Fill (Temperley) ===
        // Sécurité supplémentaire : si on vient de faire un grand saut,
        // on évite d'en refaire un dans la même direction
        let final_step = if self.last_step.abs() > 2 && chosen_step.abs() > 2 && chosen_step.signum() == self.last_step.signum() {
            // On force un petit mouvement ou un retour
            if chosen_step > 0 { -1 } else { 1 }
        } else {
            chosen_step
        };

        self.last_step = final_step;
        self.current_index += final_step;

        // Contraintes physiques (Tessiture)
        self.current_index = self.current_index.clamp(
            -(self.scale_len as i32 * 2),
            self.scale_len as i32 * 2
        );

        self.get_frequency()
    }

    /// Calcule les probabilités de mouvement selon la théorie musicale
    /// CORRECTION: Notes stables = bonnes destinations, PAS immobilité!
    /// On favorise le MOUVEMENT (arpèges, sauts d'octave) plutôt que la répétition
    /// + PROGRESSION HARMONIQUE: les notes stables changent selon l'accord courant!
    fn get_weighted_steps(&self, normalized_index: i32, is_strong_beat: bool) -> (Vec<i32>, Vec<u32>) {
        // === CONTEXTE HARMONIQUE: Identifier les degrés selon l'ACCORD ACTUEL ===
        // Plus sophistiqué que "1, 3, 5 statiques" - maintenant dynamique!
        let is_chord_tone = self.is_in_current_chord(normalized_index);

        let is_tonic = normalized_index == 0;
        let is_leading_tone = self.scale_len == 7 && normalized_index == 6;

        // Saut d'octave (7 en diatonique, 5 en pentatonique)
        let octave_jump = self.scale_len as i32;

        // === CAS 1: TONIQUE (La maison - affirmer l'accord, pas stagner!) ===
        if is_tonic {
            if is_strong_beat {
                // Affirmer l'accord par arpège (tierce +2, quinte +4) ou octave
                // Réduire "0" à 10% (juste pour effet rythmique occasionnel)
                (vec![0, 2, 4, -3, octave_jump, -octave_jump],
                 vec![10, 30, 25, 15, 10, 10])
            } else {
                // Temps faible: préparer le mouvement avec notes de passage
                (vec![1, -1, 2, -2, 0],
                 vec![30, 30, 15, 15, 10])
            }
        }
        // === CAS 2: SENSIBLE (7ème degré) - TRES forte attraction ===
        else if is_leading_tone {
            // 85% de résolution vers la tonique (+1)
            (vec![1, -1, 0, -2], vec![85, 10, 2, 3])
        }
        // === CAS 3: AUTRES NOTES D'ACCORD (Tierce, Quinte) ===
        else if is_chord_tone {
            if is_strong_beat {
                // Naviguer dans l'arpège vers tonique ou autre note d'accord
                (vec![0, -2, 2, -4, 1, -1],
                 vec![10, 30, 30, 10, 10, 10])
            } else {
                // Mouvement par notes de passage
                (vec![1, -1, 2, -2, 0],
                 vec![40, 40, 10, 5, 5])
            }
        }
        // === CAS 4: NOTES DE PASSAGE (Instables - doivent résoudre) ===
        else {
            // Résolution vers note stable voisine (mouvement conjoint dominant)
            (vec![1, -1, 0],
             vec![45, 45, 10])
        }
    }

    fn get_frequency(&self) -> f32 {
        let notes = self.current_scale.notes();
        let len = notes.len() as i32;

        // Calculate the actual note index and octave shift
        let mut index = self.current_index;
        let mut octave_shift = 0;

        while index < 0 {
            index += len;
            octave_shift -= 1;
        }
        while index >= len {
            index -= len;
            octave_shift += 1;
        }

        let note = &notes[index as usize];

        let pc_val = note.pitch.into_u8() as i32;
        let note_octave = note.octave as i32 + octave_shift;

        let midi_note = (note_octave + 1) * 12 + pc_val;
        let freq = 440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0);

        freq
    }

    pub fn set_hurst_factor(&mut self, factor: f32) {
        self.hurst_factor = factor.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_steps_tonic_strong_beat() {
        let navigator = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
        let (steps, weights) = navigator.get_weighted_steps(0, true);

        // Sur tonique + temps fort: MOUVEMENT favorisé (arpège) plutôt qu'immobilité
        // Les sauts d'arpège (+2 tierce, +4 quinte) doivent avoir plus de poids que "0"
        let stay_weight = steps.iter().position(|&s| s == 0).map(|i| weights[i]).unwrap_or(0);
        let arpeggiate_weight: u32 = steps.iter()
            .enumerate()
            .filter(|&(_, &s)| s == 2 || s == 4)
            .map(|(i, _)| weights[i])
            .sum();

        assert!(arpeggiate_weight > stay_weight,
                "Les arpèges ({}) doivent dominer l'immobilité ({})",
                arpeggiate_weight, stay_weight);
    }

    #[test]
    fn test_weighted_steps_chord_tone() {
        let navigator = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
        let (steps, weights) = navigator.get_weighted_steps(2, true); // 3ème degré = note d'accord

        // Note d'accord sur temps fort: doit favoriser stabilité et mouvements conjoints
        assert!(steps.contains(&0)); // Peut rester
        assert!(steps.contains(&1) || steps.contains(&-1)); // Ou bouger conjointement
        assert!(weights.iter().sum::<u32>() == 100); // Total des poids = 100%
    }

    #[test]
    fn test_probabilistic_movement_distribution() {
        let mut navigator = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);

        // Générer 100 notes et vérifier la distribution
        let mut movements = Vec::new();
        for _ in 0..100 {
            let prev_index = navigator.current_index;
            navigator.next_note(false);
            movements.push(navigator.current_index - prev_index);
        }

        // Vérifier que ce n'est pas uniforme (comme le serait un pur random walk)
        // Les mouvements conjoints (-1, 0, 1) devraient être plus fréquents que les sauts
        let conjunct = movements.iter().filter(|&&m| m.abs() <= 1).count();
        let disjunct = movements.iter().filter(|&&m| m.abs() > 1).count();

        assert!(conjunct > disjunct); // Mouvements conjoints dominants
    }

    #[test]
    fn test_chord_context_changes_stability() {
        let mut navigator = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);

        // Test 1: Sur accord I (C Maj: C, E, G, B), la note C (degré 0) est stable
        navigator.set_chord_context(0, ChordQuality::Major); // I Maj
        assert!(navigator.is_in_current_chord(0), "C devrait être dans l'accord I");

        // Test 2: Sur accord vi (A Min: A, C, E, G), la note C (degré 0) est TOUJOURS stable
        // Parce que C fait partie de l'accord de La mineur
        navigator.set_chord_context(9, ChordQuality::Minor); // vi Min (A = +9 demi-tons depuis C)
        // Note: En pentatonique C majeur, les degrés sont C, D, E, G, A
        // L'accord de A mineur contient A, C, E, G
        // Donc le degré 0 (C) devrait toujours être dedans
        assert!(navigator.is_in_current_chord(0), "C devrait être dans l'accord vi (A mineur contient C)");

        // Test 3: Vérifier que les pitch classes sont correctement calculées
        navigator.set_chord_context(5, ChordQuality::Major); // IV (F Maj: F, A, C, E)
        let chord_notes = &navigator.current_chord_notes;
        assert_eq!(chord_notes.len(), 4, "L'accord devrait avoir 4 notes");
        assert!(chord_notes.contains(&5), "F (pitch class 5) devrait être dans F Maj");
        assert!(chord_notes.contains(&9), "A (pitch class 9) devrait être dans F Maj");
        assert!(chord_notes.contains(&0), "C (pitch class 0) devrait être dans F Maj");
    }

    #[test]
    fn test_chord_progression_cycle() {
        let mut navigator = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);

        // Simuler la progression I-vi-IV-V
        let progression = [
            (0, ChordQuality::Major),  // I
            (9, ChordQuality::Minor),  // vi
            (5, ChordQuality::Major),  // IV
            (7, ChordQuality::Major),  // V
        ];

        for (root_offset, quality) in progression.iter() {
            navigator.set_chord_context(*root_offset, *quality);

            // Vérifier que les notes de l'accord sont bien définies
            assert_eq!(navigator.current_chord_notes.len(), 4,
                      "Chaque accord devrait avoir 4 notes (1, 3, 5, 7)");

            // Vérifier que les pitch classes sont dans la plage [0, 11]
            for &pc in navigator.current_chord_notes.iter() {
                assert!(pc < 12, "Pitch class {} devrait être < 12", pc);
            }
        }
    }
}
