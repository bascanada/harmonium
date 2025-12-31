pub struct Sequencer {
    pub steps: usize,
    pub pulses: usize,
    pub pattern: Vec<bool>,
    pub rotation: usize,     // Rotation offset (Necklace → Bracelet transformation)
    pub current_step: usize,
    pub bpm: f32,
    pub last_tick_time: f64,
}

impl Sequencer {
    pub fn new(steps: usize, pulses: usize, bpm: f32) -> Self {
        Sequencer {
            steps,
            pulses,
            pattern: generate_euclidean_pattern(steps, pulses),
            rotation: 0,
            current_step: 0,
            bpm,
            last_tick_time: 0.0,
        }
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
    pub fn regenerate_pattern(&mut self) {
        self.pattern = rotate_pattern(&generate_euclidean_pattern(self.steps, self.pulses), self.rotation);
    }

    pub fn tick(&mut self) -> bool {
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
}
