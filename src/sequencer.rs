use serde::{Serialize, Deserialize};

// --- RHYTHM MODE (Strategy Pattern) ---

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum RhythmMode {
    #[default]
    Euclidean,      // Algorithme de Bjorklund (Classique)
    PerfectBalance, // Algorithme Additif (XronoMorph style) - 48 steps
}

/// Une forme géométrique pure qui tourne sur le cercle
/// Représente un polygone régulier inscrit dans le cercle rythmique
#[derive(Clone, Debug)]
pub struct BalancedPolygon {
    pub sides: usize,    // Ex: 3 (Triangle), 4 (Carré), 6 (Hexagone)
    pub rotation: usize, // Décalage en steps (0 à 47)
    pub velocity: f32,   // Poids de la forme pour le mix (0.0 à 1.0)
}

impl BalancedPolygon {
    pub fn new(sides: usize, rotation: usize, velocity: f32) -> Self {
        Self { sides, rotation, velocity }
    }
}

// --- SEQUENCER STRUCT ---

pub struct Sequencer {
    pub steps: usize,
    pub pulses: usize,
    pub pattern: Vec<bool>,
    pub rotation: usize,     // Rotation offset (Necklace → Bracelet transformation)
    pub current_step: usize,
    pub bpm: f32,
    pub last_tick_time: f64,
    // --- NEW: Strategy Pattern fields ---
    pub mode: RhythmMode,
    pub tension: f32,        // Contrôle la complexité polyrythmique (0.0 à 1.0)
    pub density: f32,        // Contrôle le remplissage (0.0 à 1.0)
}

impl Sequencer {
    pub fn new(steps: usize, pulses: usize, bpm: f32) -> Self {
        Self::new_with_mode(steps, pulses, bpm, RhythmMode::Euclidean)
    }

    /// Créer un séquenceur avec un mode spécifique (Euclidean ou PerfectBalance)
    pub fn new_with_mode(steps: usize, pulses: usize, bpm: f32, mode: RhythmMode) -> Self {
        let mut seq = Sequencer {
            steps,
            pulses,
            pattern: vec![false; steps],
            rotation: 0,
            current_step: 0,
            bpm,
            last_tick_time: 0.0,
            mode,
            tension: 0.0,
            density: 0.5,
        };
        seq.regenerate_pattern();
        seq
    }

    /// Créer un séquenceur avec une rotation initiale (point de départ différent)
    pub fn new_with_rotation(steps: usize, pulses: usize, bpm: f32, rotation: usize) -> Self {
        let mut seq = Self::new(steps, pulses, bpm);
        seq.set_rotation(rotation);
        seq
    }

    /// Définir la rotation (décalage circulaire du pattern)
    /// Selon Toussaint: même rythme euclidien, style musical différent
    /// Ex: E(3,8) rotation 0 = Tresillo, rotation 2 = Rythme rock
    pub fn set_rotation(&mut self, offset: usize) {
        self.rotation = offset % self.steps;
        self.pattern = rotate_pattern(&generate_euclidean_pattern(self.steps, self.pulses), self.rotation);
    }

    /// Régénérer le pattern avec les paramètres actuels
    /// Utilise le Strategy Pattern pour choisir l'algorithme approprié
    pub fn regenerate_pattern(&mut self) {
        let raw = match self.mode {
            RhythmMode::Euclidean => {
                // Algorithme de Bjorklund classique
                generate_euclidean_pattern(self.steps, self.pulses)
            }
            RhythmMode::PerfectBalance => {
                // Algorithme de superposition de polygones (XronoMorph style)
                // Utilise density et tension au lieu de pulses
                generate_balanced_pattern_48(self.steps, self.density, self.tension)
            }
        };
        self.pattern = rotate_pattern(&raw, self.rotation);
    }

    /// Passer en mode haute résolution (48 steps) pour PerfectBalance
    pub fn upgrade_to_48_steps(&mut self) {
        if self.steps != 48 {
            self.steps = 48;
            self.current_step = 0;
            self.pattern = vec![false; 48];
            self.regenerate_pattern();
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.pattern.is_empty() { return false; }
        let trigger = self.pattern[self.current_step];
        self.current_step = (self.current_step + 1) % self.steps;
        trigger
    }
}

pub fn generate_euclidean_pattern(steps: usize, pulses: usize) -> Vec<bool> {
    if pulses == 0 {
        return vec![false; steps];
    }
    if pulses >= steps {
        return vec![true; steps];
    }

    // Basic implementation of Bjorklund's algorithm logic
    // Start with 'pulses' groups of [1] and 'steps-pulses' groups of [0]
    let mut pattern: Vec<Vec<u8>> = Vec::new();
    for _ in 0..pulses {
        pattern.push(vec![1]);
    }
    for _ in 0..(steps - pulses) {
        pattern.push(vec![0]);
    }

    let mut count = std::cmp::min(pulses, steps - pulses);
    let mut remainder = pattern.len() - count;

    while remainder > 1 && count > 0 {
        for i in 0..count {
            let last = pattern.pop().unwrap();
            pattern[i].extend(last);
        }
        remainder = pattern.len() - count;
        count = std::cmp::min(count, remainder);
    }

    // Flatten the pattern
    let mut result = Vec::new();
    for group in pattern {
        for val in group {
            result.push(val == 1);
        }
    }
    
    // The standard Bjorklund might need rotation to match musical expectations (like starting on a beat),
    // but this mathematically correct distribution is a good start.
    result
}

// --- ALGORITHME 2 : PERFECT BALANCE (48 STEPS) ---
// Pourquoi 48 ? C'est un nombre hautement composé:
// - Divisible par 2, 4, 8, 16 (Rythmes binaires)
// - Divisible par 3, 6, 12, 24 (Rythmes ternaires/triplés)
// Permet des polyrythmes 4:3 parfaits sans approximation.

/// Génère un rythme par superposition de polygones réguliers sur une grille de N steps
/// Inspiré de XronoMorph (Andrew Milne) et des Well-Formed Scales
///
/// # Arguments
/// * `steps` - Nombre de steps (48 recommandé pour les polyrythmes)
/// * `density` - Contrôle le remplissage (0.0 = sparse, 1.0 = dense)
/// * `tension` - Contrôle la complexité polyrythmique (0.0 = stable, 1.0 = complexe)
pub fn generate_balanced_pattern_48(steps: usize, density: f32, tension: f32) -> Vec<bool> {
    // On travaille sur un accumulateur de vélocité (0.0 à 1.0)
    let mut accumulation = vec![0.0f32; steps];

    // ==========================================
    // COUCHE 1 : LE SQUELETTE (Perfect Balance)
    // C'est ce qui donne la structure "Intelligente"
    // ==========================================
    
    let mut structural_polygons = Vec::new();

    // 1. L'Ancre (Basse) - Carré (4) ou Octogone (8)
    let base_sides = if density < 0.3 { 4 } else { 8 };
    structural_polygons.push(BalancedPolygon::new(base_sides, 0, 1.0)); // Vélocité MAX

    // 2. La Tension (Triangle - Polyrythme 4:3)
    // On abaisse le seuil à 0.1 pour que tu l'entendes tout de suite
    if tension > 0.1 {
        // Vélocité dynamique : plus c'est tendu, plus le triangle tape fort
        let tri_vel = 0.6 + (tension * 0.4); 
        // Rotation : Si tension haute, on décale pour syncoper
        let rotation = if tension > 0.6 { 6 } else { 0 };
        structural_polygons.push(BalancedPolygon::new(3, rotation, tri_vel));
    }

    // Appliquer le Squelette
    for poly in structural_polygons {
        let interval = steps / poly.sides;
        for i in 0..poly.sides {
            let pos = (poly.rotation + i * interval) % steps;
            accumulation[pos] = poly.velocity.max(accumulation[pos]); // On garde la vélocité max
        }
    }

    // ==========================================
    // COUCHE 2 : LA CHAIR (Euclidean Fills)
    // C'est ce qui règle ton problème de "Vide"
    // ==========================================

    // On génère un pattern Euclidien standard de 16 steps (semi-croches)
    // Le nombre de notes dépend directement de la densité (ex: 50% de densité = 8 notes)
    let fill_pulses = (density * 16.0).round() as usize;
    
    if fill_pulses > 0 {
        let fill_pattern = generate_euclidean_pattern(16, fill_pulses);
        
        // On projette ce pattern 16 steps sur la grille de 48 steps
        // 1 step Euclidien = 3 steps Haute Résolution (48 / 16 = 3)
        for (i, &is_active) in fill_pattern.iter().enumerate() {
            if is_active {
                let pos_48 = i * 3;
                
                // Si la case est vide (pas de squelette), on ajoute une "Ghost Note"
                if accumulation[pos_48] == 0.0 {
                    accumulation[pos_48] = 0.35; // Vélocité faible (Ghost Note)
                }
            }
        }
    }
    
    // ==========================================
    // COUCHE 3 : LE CHAOS (Hi-Hat / Texture fine)
    // Pour les densités très élevées (> 0.8), on ajoute du détail fin
    // ==========================================
    if density > 0.8 {
        // On remplit les contre-temps fins (les steps intermédiaires)
        for i in 0..steps {
            if i % 3 != 0 && accumulation[i] == 0.0 {
                 if i % 2 == 0 { // Un petit pattern arbitraire
                     accumulation[i] = 0.15; // Très faible, juste du "grain"
                 }
            }
        }
    }

    // Conversion en booléens (Trigger)
    // Note : Idéalement, ton moteur audio devrait utiliser la valeur float pour la vélocité !
    // Mais pour l'instant on retourne juste true si > 0.
    accumulation.into_iter().map(|val| val > 0.0).collect()
}

/// Rotation circulaire d'un pattern (Necklace → Bracelet)
/// Selon Toussaint (The Geometry of Musical Rhythm):
/// - Un "necklace" est une classe d'équivalence de rythmes
/// - Un "bracelet" est un rythme spécifique avec un point de départ
/// Ex: [1,0,0,1,0,0,1,0] rotation 2 → [0,1,0,0,1,0,0,1]
pub fn rotate_pattern(pattern: &[bool], offset: usize) -> Vec<bool> {
    if pattern.is_empty() || offset == 0 {
        return pattern.to_vec();
    }
    
    let len = pattern.len();
    let offset = offset % len; // Normaliser l'offset
    
    let mut rotated = Vec::with_capacity(len);
    for i in 0..len {
        rotated.push(pattern[(i + offset) % len]);
    }
    rotated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation() {
        // E(3,8) généré par Bjorklund: [T,F,F,T,F,F,T,F]
        let pattern = vec![true, false, false, true, false, false, true, false];
        let rotated = rotate_pattern(&pattern, 2);
        // Rotation de 2: prend à partir de l'index 2
        assert_eq!(rotated, vec![false, true, false, false, true, false, true, false]);
    }

    #[test]
    fn test_euclidean_e38() {
        // E(3,8) - Tresillo/Cuban rhythm
        let pattern = generate_euclidean_pattern(8, 3);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 3);
        assert_eq!(pattern.len(), 8);
        // Vérifier que c'est bien le pattern de Bjorklund
        assert_eq!(pattern, vec![true, false, false, true, false, false, true, false]);
    }

    #[test]
    fn test_euclidean_e58() {
        // E(5,8) - Cuban cinquillo
        let pattern = generate_euclidean_pattern(8, 5);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 5);
    }

    #[test]
    fn test_balanced_48_low_tension() {
        // Low tension (0.2) = only base polygon (Square or Hexagon)
        let pattern = generate_balanced_pattern_48(48, 0.5, 0.2);
        assert_eq!(pattern.len(), 48);

        // Should have at least 4 pulses (base polygon)
        let pulse_count = pattern.iter().filter(|&&x| x).count();
        assert!(pulse_count >= 4, "Expected at least 4 pulses, got {}", pulse_count);
    }

    #[test]
    fn test_balanced_48_high_tension() {
        // High tension (0.8) = base polygon + triangle (polyrhythm 4:3)
        let pattern = generate_balanced_pattern_48(48, 0.5, 0.8);
        assert_eq!(pattern.len(), 48);

        // Should have more pulses due to triangle overlay
        let pulse_count = pattern.iter().filter(|&&x| x).count();
        assert!(pulse_count >= 6, "Expected at least 6 pulses with polyrhythm, got {}", pulse_count);
    }

    #[test]
    fn test_balanced_48_polyrythm_4_3() {
        // Verify that Square (4 sides) and Triangle (3 sides) create proper 4:3 polyrhythm
        // 48 / 4 = 12 steps apart (Square vertices)
        // 48 / 3 = 16 steps apart (Triangle vertices)
        // density < 0.3 gives us a Square (4 sides)
        let pattern = generate_balanced_pattern_48(48, 0.2, 0.5);

        // Step 0 should always be active (both polygons start there)
        assert!(pattern[0], "Step 0 should be active (Square + Triangle origin)");

        // Step 12 should be active (Square vertex: 48/4 = 12)
        assert!(pattern[12], "Step 12 should be active (Square vertex)");

        // Step 16 should be active (Triangle vertex with tension > 0.3: 48/3 = 16)
        assert!(pattern[16], "Step 16 should be active (Triangle vertex)");

        // Step 24 should be active (Square vertex)
        assert!(pattern[24], "Step 24 should be active (Square vertex)");

        // Step 32 should be active (Triangle vertex)
        assert!(pattern[32], "Step 32 should be active (Triangle vertex)");

        // Step 36 should be active (Square vertex)
        assert!(pattern[36], "Step 36 should be active (Square vertex)");
    }

    #[test]
    fn test_sequencer_mode_switch() {
        // Test that mode switching works correctly
        let mut seq = Sequencer::new_with_mode(16, 4, 120.0, RhythmMode::Euclidean);
        assert_eq!(seq.mode, RhythmMode::Euclidean);
        assert_eq!(seq.steps, 16);

        // Switch to PerfectBalance and upgrade to 48 steps
        seq.mode = RhythmMode::PerfectBalance;
        seq.upgrade_to_48_steps();

        assert_eq!(seq.steps, 48);
        assert_eq!(seq.pattern.len(), 48);
    }

    /// Tests exhaustifs pour la génération de polygones
    /// Vérifie que chaque combinaison density/tension produit les polygones attendus

    #[test]
    fn test_polygon_square_only() {
        // Très basse density (< 0.3) + basse tension (≤ 0.1) = Carré seul
        // Note: density < 0.3 generates a Square (4 pulses)
        // fill_pulses = 0.1 * 16 = 1.6 -> 2 pulses
        // Total should be Square + modest fill
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Carré = 4 sommets
        assert!(pulse_count >= 4, "Square only should have at least 4 pulses");

        // Vérifier les positions du Squelette (0, 12, 24, 36)
        assert!(pattern[0], "Square vertex at 0");
        assert!(pattern[12], "Square vertex at 12");
        assert!(pattern[24], "Square vertex at 24");
        assert!(pattern[36], "Square vertex at 36");
    }

    #[test]
    fn test_balanced_48_medium_density() {
        // Density moyenne (0.4) -> Octogone (8) + Euclidean fills
        let pattern = generate_balanced_pattern_48(48, 0.4, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Octogone = 8 sommets
        assert!(pulse_count >= 8, "Medium density should have at least 8 pulses");

        // Vérifier quelques positions de l'Octogone
        assert!(pattern[0], "Octagon vertex at 0");
        assert!(pattern[6], "Octagon vertex at 6");
    }

    #[test]
    fn test_balanced_48_high_density() {
        // Haute density (0.61) -> Octogone (8) + Dense Euclidean fills
        let pattern = generate_balanced_pattern_48(48, 0.61, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        assert!(pulse_count >= 8, "High density should have many pulses");
    }

    #[test]
    fn test_polygon_square_plus_triangle() {
        // Basse density + tension moyenne (0.3-0.7) = Carré + Triangle (polyrythme 4:3)
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.4);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Carré (4) + Triangle (3) -> Squelette
        assert!(pulse_count >= 6,
            "Square + Triangle should have at least 6 pulses, got {}", pulse_count);

        // Vérifier le polyrythme 4:3
        assert!(pattern[0], "Origin (both polygons)");
        assert!(pattern[12], "Square vertex");
        assert!(pattern[16], "Triangle vertex");
        assert!(pattern[24], "Square vertex");
        assert!(pattern[32], "Triangle vertex");
        assert!(pattern[36], "Square vertex");
    }

    #[test]
    fn test_polygon_high_tension_syncope() {
        // Haute tension (> 0.7) décale le triangle de 6 steps
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.8);

        // Carré: 0, 12, 24, 36
        // Triangle (rotation 6): 6, 22, 38
        assert!(pattern[0], "Square vertex at 0");
        assert!(pattern[6], "Triangle vertex at 6 (syncopated)");
        assert!(pattern[12], "Square vertex at 12");
        assert!(pattern[22], "Triangle vertex at 22 (syncopated)");
    }

    #[test]
    fn test_balanced_48_very_high_density() {
        // Très haute density (0.9)
        let pattern = generate_balanced_pattern_48(48, 0.9, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Should be very dense
        assert!(pulse_count > 12, "Very high density should have > 12 pulses");
    }

    #[test]
    fn test_balanced_48_extreme_tension() {
        // Tension extrême (> 0.85) -> Triangle syncopated + Chaos (maybe)
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.9);
        
        // Triangle syncopated should be present
        assert!(pattern[6], "Syncopated triangle vertex at 6");
    }
}
