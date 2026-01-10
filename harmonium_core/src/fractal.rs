use rand::Rng;

pub struct PinkNoise {
    rows: Vec<f32>,
    index: usize,
    range: f32,
}

impl PinkNoise {
    /// Crée un générateur avec `depth` octaves (ex: 5 pour une bonne précision)
    pub fn new(depth: usize) -> Self {
        PinkNoise {
            rows: vec![0.0; depth],
            index: 0,
            range: 1.0, // À ajuster selon l'amplitude voulue
        }
    }

    /// Génère la prochaine valeur (approximativement 1/f)
    /// Retourne une valeur flottante centrée autour de 0.0
    pub fn next(&mut self) -> f32 {
        let mut rng = rand::thread_rng();
        
        // Algorithme Voss-McCartney: on met à jour une rangée différente à chaque étape
        // Basé sur les bits qui changent dans le compteur binaire (trailing zeros)
        let trailing_zeros = self.index.trailing_zeros() as usize;
        
        // On met à jour la rangée correspondante (si elle existe)
        if trailing_zeros < self.rows.len() {
            // Nouvelle valeur aléatoire pour cette octave
            self.rows[trailing_zeros] = rng.gen_range(-1.0..1.0);
        }
        
        self.index = self.index.wrapping_add(1);
        
        // La somme des rangées donne le bruit rose
        let sum: f32 = self.rows.iter().sum();
        
        // Normalisation (approximative)
        sum / (self.rows.len() as f32) * self.range
    }
}
