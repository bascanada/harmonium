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
    let mut polygons = Vec::new();

    // === ÉTAPE 1 : LA RECETTE DU CHEF (Density/Tension → Géométrie) ===

    // A. La Fondation (Basse) - Toujours présente
    // Faible density: Carré (4 coups par cycle) = pulse régulier
    // Haute density: Octogone (8 coups) = double-time feel
    let base_sides = if density < 0.3 { 4 } else if density < 0.6 { 6 } else { 8 };
    polygons.push(BalancedPolygon::new(base_sides, 0, 1.0));

    // B. La Tension (Polyrythmie 4:3) - Le coeur du groove XronoMorph
    // 48 / 4 = 12 steps d'écart (Carré)
    // 48 / 3 = 16 steps d'écart (Triangle)
    // Superposer ces deux formes crée un polyrythme 4:3 parfait
    if tension > 0.3 {
        let tri_velocity = tension; // Plus de tension = triangle plus fort

        // Syncope: décaler le triangle pour créer du groove
        // tension haute (> 0.7) = décalage de 6 steps (1/8ème de tour)
        let tri_rotation = if tension > 0.7 { 6 } else if tension > 0.5 { 3 } else { 0 };

        polygons.push(BalancedPolygon::new(3, tri_rotation, tri_velocity));
    }

    // C. Le Remplissage (Haute densité) - Sparkles/Hi-hats
    // Hexagone (6 côtés) ou Dodécagone (12 côtés) pour les fills rapides
    if density > 0.65 {
        let fill_sides = if density > 0.85 { 12 } else { 6 };
        // Légère rotation pour éviter la collision avec la fondation
        polygons.push(BalancedPolygon::new(fill_sides, 2, 0.5));
    }

    // D. Contre-temps (Tension très haute) - Pentagone pour l'étrangeté
    // Le pentagone (5 côtés) ne divise pas 48 parfaitement, créant des accents décalés
    if tension > 0.85 {
        polygons.push(BalancedPolygon::new(5, 1, 0.4));
    }

    // === ÉTAPE 2 : LA SUPERPOSITION (Physique des Polygones) ===

    let mut accumulation = vec![0.0f32; steps];

    for poly in polygons {
        if poly.sides == 0 { continue; }

        // Calculer l'intervalle entre chaque sommet du polygone
        let interval = steps / poly.sides;

        // Placer chaque sommet du polygone sur le cercle rythmique
        for i in 0..poly.sides {
            let pos = (poly.rotation + i * interval) % steps;
            accumulation[pos] += poly.velocity;
        }
    }

    // === ÉTAPE 3 : QUANTIFICATION (Conversion en bool) ===
    // Tout ce qui a de l'énergie devient un trigger
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
        // Très basse density (< 0.3) + basse tension (≤ 0.3) = Carré seul
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Carré = 4 sommets
        assert_eq!(pulse_count, 4, "Square only should have exactly 4 pulses");

        // Vérifier les positions (0, 12, 24, 36)
        assert!(pattern[0], "Square vertex at 0");
        assert!(pattern[12], "Square vertex at 12");
        assert!(pattern[24], "Square vertex at 24");
        assert!(pattern[36], "Square vertex at 36");
    }

    #[test]
    fn test_polygon_hexagon_only() {
        // Density moyenne (0.3-0.6) + basse tension = Hexagone seul
        let pattern = generate_balanced_pattern_48(48, 0.4, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Hexagone = 6 sommets (48/6 = 8 steps d'intervalle)
        assert_eq!(pulse_count, 6, "Hexagon only should have exactly 6 pulses");

        // Vérifier les positions (0, 8, 16, 24, 32, 40)
        assert!(pattern[0], "Hexagon vertex at 0");
        assert!(pattern[8], "Hexagon vertex at 8");
        assert!(pattern[16], "Hexagon vertex at 16");
        assert!(pattern[24], "Hexagon vertex at 24");
        assert!(pattern[32], "Hexagon vertex at 32");
        assert!(pattern[40], "Hexagon vertex at 40");
    }

    #[test]
    fn test_polygon_octagon_only() {
        // Haute density (> 0.6) + basse tension = Octogone seul
        // Note: density > 0.65 ajoute aussi un fill polygon, donc on utilise 0.61
        let pattern = generate_balanced_pattern_48(48, 0.61, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Octogone = 8 sommets (48/8 = 6 steps d'intervalle)
        assert_eq!(pulse_count, 8, "Octagon only should have exactly 8 pulses");

        // Vérifier les positions (0, 6, 12, 18, 24, 30, 36, 42)
        assert!(pattern[0], "Octagon vertex at 0");
        assert!(pattern[6], "Octagon vertex at 6");
        assert!(pattern[12], "Octagon vertex at 12");
        assert!(pattern[18], "Octagon vertex at 18");
    }

    #[test]
    fn test_polygon_square_plus_triangle() {
        // Basse density + tension moyenne (0.3-0.7) = Carré + Triangle (polyrythme 4:3)
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.4);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Carré (4) + Triangle (3) avec collision au step 0 = 6 ou 7 pulses
        // Step 0: collision (Carré + Triangle)
        // Carré: 0, 12, 24, 36
        // Triangle (rotation 0): 0, 16, 32
        // Union: 0, 12, 16, 24, 32, 36 = 6 pulses
        assert!(pulse_count >= 6 && pulse_count <= 7,
            "Square + Triangle should have 6-7 pulses, got {}", pulse_count);

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
    fn test_polygon_very_high_density_with_fill() {
        // Très haute density (> 0.85) ajoute un dodécagone
        let pattern = generate_balanced_pattern_48(48, 0.9, 0.1);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Octogone (8) + Dodécagone (12) avec collisions
        // Devrait avoir significativement plus de pulses
        assert!(pulse_count >= 12,
            "High density should have many pulses from fill polygon, got {}", pulse_count);
    }

    #[test]
    fn test_polygon_extreme_tension_with_pentagon() {
        // Tension extrême (> 0.85) ajoute un pentagone
        let pattern = generate_balanced_pattern_48(48, 0.1, 0.9);
        let pulse_count = pattern.iter().filter(|&&x| x).count();

        // Carré (4) + Triangle (3) + Pentagone (5) avec collisions
        // Le pentagone ne divise pas 48 parfaitement, créant des accents irréguliers
        assert!(pulse_count >= 8,
            "Extreme tension should add pentagon, got {} pulses", pulse_count);
    }
}
