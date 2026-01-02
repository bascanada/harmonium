use serde::{Serialize, Deserialize};

// --- RHYTHM MODE (Strategy Pattern) ---

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum RhythmMode {
    #[default]
    Euclidean,      // Algorithme de Bjorklund (Classique)
    PerfectBalance, // Algorithme Additif (XronoMorph style) - 48 steps
}

/// Événement déclenché à chaque step du séquenceur
/// Indique quelles voix doivent jouer
#[derive(Clone, Copy, Debug, Default)]
pub struct StepTrigger {
    pub kick: bool,     // Fondation (Square/Octagon)
    pub snare: bool,    // Tension (Triangle/Backbeat)
    pub hat: bool,      // Remplissage (Euclidean Fills)
    pub velocity: f32,  // Dynamique générale (0.0 à 1.0)
}

impl StepTrigger {
    pub fn is_any(&self) -> bool {
        self.kick || self.snare || self.hat
    }
}

// --- SEQUENCER STRUCT ---

pub struct Sequencer {
    pub steps: usize,
    pub pulses: usize,
    pub pattern: Vec<StepTrigger>, // Remplacement de Vec<bool>
    pub rotation: usize,
    pub current_step: usize,
    pub bpm: f32,
    pub last_tick_time: f64,
    pub mode: RhythmMode,
    pub tension: f32,
    pub density: f32,
}

impl Sequencer {
    pub fn new(steps: usize, pulses: usize, bpm: f32) -> Self {
        Self::new_with_mode(steps, pulses, bpm, RhythmMode::Euclidean)
    }

    pub fn new_with_mode(steps: usize, pulses: usize, bpm: f32, mode: RhythmMode) -> Self {
        let mut seq = Sequencer {
            steps,
            pulses,
            pattern: vec![StepTrigger::default(); steps],
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

    pub fn new_with_rotation(steps: usize, pulses: usize, bpm: f32, rotation: usize) -> Self {
        let mut seq = Self::new(steps, pulses, bpm);
        seq.set_rotation(rotation);
        seq
    }

    pub fn set_rotation(&mut self, offset: usize) {
        self.rotation = offset % self.steps;
        // On régénère simplement pour appliquer la rotation
        // (Optimisation possible : rotation in-place, mais régénérer est plus sûr ici)
        self.regenerate_pattern();
    }

    pub fn regenerate_pattern(&mut self) {
        let mut raw = match self.mode {
            RhythmMode::Euclidean => {
                // Mode classique : On map le booléen sur Kick + Hat
                let bools = generate_euclidean_bools(self.steps, self.pulses);
                bools.into_iter().map(|b| StepTrigger {
                    kick: b,
                    snare: false,
                    hat: b, // Layering simple
                    velocity: if b { 1.0 } else { 0.0 },
                }).collect()
            }
            RhythmMode::PerfectBalance => {
                // Mode XronoMorph : 3 couches distinctes
                generate_balanced_layers_48(self.steps, self.density, self.tension)
            }
        };

        // Appliquer la rotation
        if self.rotation > 0 {
            raw.rotate_left(self.steps - self.rotation); // rotate_left fait un shift circulaire
        }
        
        self.pattern = raw;
    }

    pub fn upgrade_to_48_steps(&mut self) {
        if self.steps != 48 {
            self.steps = 48;
            self.current_step = 0;
            self.pattern = vec![StepTrigger::default(); 48];
            self.regenerate_pattern();
        }
    }

    pub fn tick(&mut self) -> StepTrigger {
        if self.pattern.is_empty() { return StepTrigger::default(); }
        let trigger = self.pattern[self.current_step];
        self.current_step = (self.current_step + 1) % self.steps;
        trigger
    }
}

/// Génère les booléens de Bjorklund (Legacy)
pub fn generate_euclidean_bools(steps: usize, pulses: usize) -> Vec<bool> {
    if pulses == 0 { return vec![false; steps]; }
    if pulses >= steps { return vec![true; steps]; }

    let mut pattern: Vec<Vec<u8>> = Vec::new();
    for _ in 0..pulses { pattern.push(vec![1]); }
    for _ in 0..(steps - pulses) { pattern.push(vec![0]); }

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

    let mut result = Vec::new();
    for group in pattern {
        for val in group {
            result.push(val == 1);
        }
    }
    result
}

// --- ALGORITHME PERFECT BALANCE : COUCHES SÉPARÉES ---

pub fn generate_balanced_layers_48(steps: usize, density: f32, tension: f32) -> Vec<StepTrigger> {
    let mut pattern = vec![StepTrigger::default(); steps];

    // --- COUCHE 1 : KICK (L'ANCRE) ---
    // Carré (4) ou Octogone (8) selon la densité
    let base_sides = if density < 0.35 { 4 } else { 8 };
    let interval_kick = steps / base_sides;
    
    for i in 0..base_sides {
        let pos = (i * interval_kick) % steps;
        pattern[pos].kick = true;
        pattern[pos].velocity = 1.0;
    }

    // --- COUCHE 2 : SNARE (LA TENSION) ---
    // Le Triangle (3) qui crée le polyrythme 4:3
    // Active uniquement si un peu de tension
    if tension > 0.15 {
        let tri_sides = 3;
        let interval_snare = steps / tri_sides; // 16 steps
        
        // Rotation dynamique : plus de tension = décalage du snare
        // Un décalage de 6 steps (1/8 de tour) est très musical
        let rotation = if tension > 0.6 { 6 } else { 0 };

        for i in 0..tri_sides {
            let pos = (rotation + i * interval_snare) % steps;
            pattern[pos].snare = true;
            // Si pas de kick ici, on accentue la vélocité du snare
            if !pattern[pos].kick {
                pattern[pos].velocity = 0.9;
            }
        }
    }

    // --- COUCHE 3 : HATS (LE GROOVE/FILL) ---
    // On projette une grille euclidienne de 16 steps sur les 48 steps
    // Cela évite le chaos aléatoire
    let fill_intensity = (density - 0.2).max(0.0);
    
    if fill_intensity > 0.0 {
        // Nombre de coups de hats (max 12 sur 16)
        let hat_pulses = (fill_intensity * 14.0) as usize;
        let hat_raw = generate_euclidean_bools(16, hat_pulses);
        
        for (i, &active) in hat_raw.iter().enumerate() {
            if active {
                // Projection 16 -> 48
                let pos = i * 3;
                pattern[pos].hat = true;
                
                // Gestion dynamique de la vélocité des hats
                // Si le step est vide (pas de kick/snare), c'est une "Ghost Note"
                if !pattern[pos].kick && !pattern[pos].snare {
                    pattern[pos].velocity = 0.4 + (tension * 0.3);
                }
            }
        }
    }

    // --- COUCHE 4 : TEXTURE/CHAOS (Ratchet) ---
    // Uniquement pour très haute densité (> 0.85)
    // Ajoute des coups sur les steps intermédiaires (micro-timing)
    if density > 0.85 {
        for i in 0..steps {
            if i % 3 != 0 && i % 2 == 0 { // Sur les steps pairs fins
                // Pseudo-random déterministe basé sur l'index
                if (i * 7 + (tension * 10.0) as usize) % 5 == 0 {
                    pattern[i].hat = true;
                    pattern[i].velocity = 0.25; // Très léger
                }
            }
        }
    }

    pattern
}
