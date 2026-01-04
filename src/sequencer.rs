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

// --- ALGORITHME PERFECT BALANCE / WELL-FORMED (XronoMorph Style) ---
// Approche "Hocketing" : les couches s'imbriquent au lieu de se heurter

pub fn generate_balanced_layers_48(steps: usize, density: f32, tension: f32) -> Vec<StepTrigger> {
    let mut pattern = vec![StepTrigger::default(); steps];
    let mut occupied = vec![false; steps]; // Carte d'occupation pour éviter les clashes

    // --- COUCHE 1 : KICK (Fondation) ---
    // Toujours calé sur la noire (intervalle de 12 steps = 1 temps)
    let kick_interval = steps / 4; // 12 steps

    // Kicks principaux sur les temps (0, 12, 24, 36)
    for i in 0..4 {
        let pos = i * kick_interval;
        pattern[pos].kick = true;
        pattern[pos].velocity = 1.0;
        occupied[pos] = true;
    }

    // Kicks secondaires (croches) si densité > 0.45
    if density > 0.45 {
        for i in 0..4 {
            let pos = i * kick_interval + (kick_interval / 2); // Steps 6, 18, 30, 42
            pattern[pos].kick = true;
            pattern[pos].velocity = 0.85;
            occupied[pos] = true;
        }
    }

    // --- COUCHE 2 : SNARE (Tension vs Résolution) ---
    if tension > 0.1 {
        let use_backbeat = tension < 0.6;

        if use_backbeat {
            // Mode Backbeat (stable) : Snare sur temps 2 et 4
            // Dans 48 steps : temps 2 = step 12, temps 4 = step 36
            let backbeats = [12, 36];
            for &pos in backbeats.iter() {
                pattern[pos].snare = true;
                pattern[pos].kick = false; // Snare remplace kick pour clarté
                pattern[pos].velocity = 1.0;
                occupied[pos] = true;
            }
        } else {
            // Mode Polyrythmique (tendu) : Triangle QUANTIFIÉ sur grille compatible
            // Au lieu de 48/3=16 (qui clash), on utilise des positions compatibles
            // Snare sur steps 12, 28, 44 (espacés de ~16 mais alignés sur grille de 4)
            let snare_positions = [12, 28, 44];
            for &pos in snare_positions.iter() {
                pattern[pos].snare = true;
                pattern[pos].velocity = 0.9 + (tension * 0.1);
                occupied[pos] = true;
            }
        }
    }

    // --- COUCHE 3 : HATS (Gap Filling / Hocketing) ---
    // Les Hats remplissent les TROUS laissés par Kick/Snare

    let max_hats = ((steps as f32) * density * 0.7) as usize;

    // Grille de préférence pour les Hats (priorité musicale)
    let mut candidates: Vec<(usize, f32)> = Vec::new();

    for i in 0..steps {
        if occupied[i] { continue; } // On respecte Kick/Snare

        let mut weight: f32 = 0.0;

        // Poids basés sur la subdivision musicale (12 steps = 1 temps)
        if i % 6 == 0 {
            weight += 50.0;  // Croche (off-beat fort)
        } else if i % 3 == 0 {
            weight += 30.0;  // Double-croche (16th)
        } else if i % 4 == 0 {
            weight += 15.0;  // Triolet de croches
        } else if i % 2 == 0 {
            weight += 5.0;   // Subdivision fine
        } else {
            weight += 1.0;   // Granularité maximale
        }

        // Tension haute = favoriser les positions "bizarres" (chaos contrôlé)
        if tension > 0.7 && weight < 10.0 {
            weight += 35.0;
        }

        candidates.push((i, weight));
    }

    // Trier par poids décroissant (les plus musicaux d'abord)
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Remplir jusqu'à la densité voulue
    for (pos, _) in candidates.iter().take(max_hats) {
        pattern[*pos].hat = true;

        // Vélocité dynamique selon la subdivision
        let is_strong = *pos % 6 == 0;
        pattern[*pos].velocity = if is_strong {
            0.6 + (tension * 0.2)
        } else {
            0.3 + (tension * 0.2) // Ghost notes
        };
    }

    // --- COUCHE 4 : LAYERING OPTIONNEL (haute densité) ---
    // Permet aux Hats de se superposer au Kick pour plus de "drive"
    if density > 0.75 {
        for i in 0..4 {
            let pos = i * kick_interval; // Sur les kicks principaux
            if !pattern[pos].hat {
                pattern[pos].hat = true;
                pattern[pos].velocity = 0.9; // Fort pour ne pas disparaître
            }
        }
    }

    pattern
}
