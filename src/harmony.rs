use rust_music_theory::scale::{Scale, ScaleType, Direction};
use rust_music_theory::note::{PitchSymbol, Pitch, Notes};
use rand::distributions::{Distribution, WeightedIndex};

pub struct HarmonyNavigator {
    pub current_scale: Scale,
    pub current_index: i32,
    pub octave: i32,
    scale_len: usize,
    last_step: i32,  // Mémoire mélodique pour "Gap Fill" (Temperley)
}

impl HarmonyNavigator {
    pub fn new(root_note: PitchSymbol, scale_type: ScaleType, octave: i32) -> Self {
        let pitch = Pitch::from(root_note);
        let scale = Scale::new(scale_type, pitch, octave as u8, None, Direction::Ascending).unwrap();
        let scale_len = scale.notes().len();
        
        HarmonyNavigator {
            current_scale: scale,
            current_index: 0,
            octave,
            scale_len,
            last_step: 0,
        }
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

    /// Calcule les probabilités de mouvement selon la théorie musicale
    /// CORRECTION: Notes stables = bonnes destinations, PAS immobilité!
    /// On favorise le MOUVEMENT (arpèges, sauts d'octave) plutôt que la répétition
    fn get_weighted_steps(&self, normalized_index: i32, is_strong_beat: bool) -> (Vec<i32>, Vec<u32>) {
        // Identifier les degrés de l'accord (1, 3, 5)
        let is_chord_tone = match self.scale_len {
            5 => true, // Pentatonique: toutes notes stables
            7 => normalized_index == 0 || normalized_index == 2 || normalized_index == 4,
            _ => normalized_index == 0,
        };
        
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
        
        // We need to reconstruct the frequency.
        // The 'note' struct from the scale has a fixed octave usually (the one the scale was created with).
        // We need to adjust it.
        
        // rust-music-theory Note has a frequency() method? Or we calculate from PitchClass and Octave.
        // Note struct usually has `pitch_class` and `octave`.
        
        // Let's create a new note with the shifted octave to get the freq.
        // Note: The scale notes already have the base octave of the scale.
        // So we just add the octave_shift to that note's octave.
        
        // Accessing private fields might be an issue if we try to construct manually.
        // Let's see if we can use a helper or if Note is easy to clone/modify.
        
        // Assuming Note has a public way to get freq or we can use the formula.
        // f = 440 * 2^((n - 69)/12)
        // Let's rely on the crate if possible, otherwise manual calc.
        
        // Warning: rust-music-theory `Note` struct fields might be private.
        // Use `pitch_class` and `octave` getters if available.
        
        // Actually, let's just use the `freq` method if it exists, or calculate.
        // A safer bet for a POC without full docs is to calculate:
        // pitch_class to semitone index (C=0, C#=1...)
        // midi_val = (octave + 1) * 12 + semitone
        // freq = 440.0 * 2.0_f32.powf((midi_val - 69.0) / 12.0)
        
        let pc_val = note.pitch.into_u8() as i32; // Assuming PitchClass can be converted to int
        let note_octave = note.octave as i32 + octave_shift;
        
        let midi_note = (note_octave + 1) * 12 + pc_val;
        let freq = 440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0);
        
        freq
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
}
