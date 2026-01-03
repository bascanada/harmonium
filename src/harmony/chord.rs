//! Module de représentation des accords
//!
//! Fournit une représentation enrichie des accords avec:
//! - PitchClass (0-11)
//! - ChordType étendu (Major, Minor, Aug, Dim, Dom7, etc.)
//! - Extensions et basse séparée
//! - Niveau LCC (Lydian Chromatic Concept)

/// Pitch class (0-11, où 0=C, 1=C#/Db, 2=D, etc.)
pub type PitchClass = u8;

/// Noms des notes pour affichage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteName {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B,
}

impl NoteName {
    /// Convertit un PitchClass en NoteName
    pub fn from_pitch_class(pc: PitchClass) -> Self {
        match pc % 12 {
            0 => NoteName::C,
            1 => NoteName::Cs,
            2 => NoteName::D,
            3 => NoteName::Ds,
            4 => NoteName::E,
            5 => NoteName::F,
            6 => NoteName::Fs,
            7 => NoteName::G,
            8 => NoteName::Gs,
            9 => NoteName::A,
            10 => NoteName::As,
            11 => NoteName::B,
            _ => unreachable!(),
        }
    }

    /// Convertit en PitchClass
    pub fn to_pitch_class(self) -> PitchClass {
        match self {
            NoteName::C => 0,
            NoteName::Cs => 1,
            NoteName::D => 2,
            NoteName::Ds => 3,
            NoteName::E => 4,
            NoteName::F => 5,
            NoteName::Fs => 6,
            NoteName::G => 7,
            NoteName::Gs => 8,
            NoteName::A => 9,
            NoteName::As => 10,
            NoteName::B => 11,
        }
    }

    /// Nom pour affichage
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteName::C => "C",
            NoteName::Cs => "C#",
            NoteName::D => "D",
            NoteName::Ds => "D#",
            NoteName::E => "E",
            NoteName::F => "F",
            NoteName::Fs => "F#",
            NoteName::G => "G",
            NoteName::Gs => "G#",
            NoteName::A => "A",
            NoteName::As => "A#",
            NoteName::B => "B",
        }
    }
}

impl std::fmt::Display for NoteName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Types d'accords étendus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordType {
    /// Accord majeur (1-3-5)
    Major,
    /// Accord mineur (1-b3-5)
    Minor,
    /// Accord augmenté (1-3-#5) - symétrique, utile pour pivots
    Augmented,
    /// Accord diminué (1-b3-b5)
    Diminished,
    /// Accord dominant 7 (1-3-5-b7)
    Dominant7,
    /// Accord majeur 7 (1-3-5-7)
    Major7,
    /// Accord mineur 7 (1-b3-5-b7)
    Minor7,
    /// Accord demi-diminué (1-b3-b5-b7)
    HalfDiminished,
    /// Accord diminué 7 (1-b3-b5-bb7) - symétrique, utile pour pivots
    Diminished7,
    /// Accord sus2 (1-2-5) - neutre, pas de tierce
    Sus2,
    /// Accord sus4 (1-4-5) - neutre, utile pour pivots
    Sus4,
}

impl ChordType {
    /// Retourne les intervalles (en demi-tons) depuis la fondamentale
    pub fn intervals(&self) -> Vec<u8> {
        match self {
            ChordType::Major => vec![0, 4, 7],
            ChordType::Minor => vec![0, 3, 7],
            ChordType::Augmented => vec![0, 4, 8],
            ChordType::Diminished => vec![0, 3, 6],
            ChordType::Dominant7 => vec![0, 4, 7, 10],
            ChordType::Major7 => vec![0, 4, 7, 11],
            ChordType::Minor7 => vec![0, 3, 7, 10],
            ChordType::HalfDiminished => vec![0, 3, 6, 10],
            ChordType::Diminished7 => vec![0, 3, 6, 9],
            ChordType::Sus2 => vec![0, 2, 7],
            ChordType::Sus4 => vec![0, 5, 7],
        }
    }

    /// Nom court pour affichage
    pub fn suffix(&self) -> &'static str {
        match self {
            ChordType::Major => "",
            ChordType::Minor => "m",
            ChordType::Augmented => "+",
            ChordType::Diminished => "dim",
            ChordType::Dominant7 => "7",
            ChordType::Major7 => "maj7",
            ChordType::Minor7 => "m7",
            ChordType::HalfDiminished => "m7b5",
            ChordType::Diminished7 => "dim7",
            ChordType::Sus2 => "sus2",
            ChordType::Sus4 => "sus4",
        }
    }

    /// Vérifie si c'est un accord majeur (inclut Maj7, Dom7)
    pub fn is_major(&self) -> bool {
        matches!(self, ChordType::Major | ChordType::Major7 | ChordType::Dominant7 | ChordType::Augmented)
    }

    /// Vérifie si c'est un accord mineur (inclut Min7, HalfDim)
    pub fn is_minor(&self) -> bool {
        matches!(self, ChordType::Minor | ChordType::Minor7 | ChordType::HalfDiminished | ChordType::Diminished | ChordType::Diminished7)
    }

    /// Vérifie si c'est un accord symétrique (utile pour pivots Neo-Riemannian)
    pub fn is_symmetric(&self) -> bool {
        matches!(self, ChordType::Augmented | ChordType::Diminished7)
    }

    /// Vérifie si c'est un accord ambigu (sans tierce claire)
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ChordType::Sus2 | ChordType::Sus4 | ChordType::Augmented | ChordType::Diminished7)
    }
}

/// Un accord avec contexte harmonique complet
#[derive(Clone, Debug)]
pub struct Chord {
    /// Fondamentale (0-11)
    pub root: PitchClass,
    /// Type d'accord
    pub chord_type: ChordType,
    /// Basse séparée (pour slash chords comme C/G)
    pub bass: Option<PitchClass>,
    /// Extensions (9, 11, 13) en pitch classes relatives
    pub extensions: Vec<PitchClass>,
    /// Niveau LCC (1-12, voir lydian_chromatic.rs)
    pub lcc_level: u8,
}

impl Default for Chord {
    fn default() -> Self {
        Chord {
            root: 0, // C
            chord_type: ChordType::Major,
            bass: None,
            extensions: Vec::new(),
            lcc_level: 1, // Lydian (le plus consonant)
        }
    }
}

impl Chord {
    /// Crée un nouvel accord simple
    pub fn new(root: PitchClass, chord_type: ChordType) -> Self {
        Chord {
            root: root % 12,
            chord_type,
            bass: None,
            extensions: Vec::new(),
            lcc_level: 1,
        }
    }

    /// Crée un accord avec basse différente (slash chord)
    pub fn with_bass(mut self, bass: PitchClass) -> Self {
        self.bass = Some(bass % 12);
        self
    }

    /// Ajoute une extension (9, 11, 13 en demi-tons depuis root)
    pub fn with_extension(mut self, interval: u8) -> Self {
        self.extensions.push(interval % 12);
        self
    }

    /// Définit le niveau LCC
    pub fn with_lcc_level(mut self, level: u8) -> Self {
        self.lcc_level = level.clamp(1, 12);
        self
    }

    /// Retourne les pitch classes de l'accord (sans extensions)
    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        self.chord_type
            .intervals()
            .iter()
            .map(|&interval| (self.root + interval) % 12)
            .collect()
    }

    /// Retourne toutes les pitch classes (avec extensions)
    pub fn all_pitch_classes(&self) -> Vec<PitchClass> {
        let mut pcs = self.pitch_classes();
        for &ext in &self.extensions {
            let pc = (self.root + ext) % 12;
            if !pcs.contains(&pc) {
                pcs.push(pc);
            }
        }
        pcs
    }

    /// Vérifie si l'accord est ambigu (utile pour pivots)
    pub fn is_ambiguous(&self) -> bool {
        self.chord_type.is_ambiguous()
    }

    /// Calcule la distance de voice-leading vers un autre accord
    /// (somme des mouvements minimaux en demi-tons)
    pub fn voice_leading_distance(&self, other: &Chord) -> u32 {
        let self_pcs = self.pitch_classes();
        let other_pcs = other.pitch_classes();

        // Algorithme glouton simplifié: pour chaque note de self,
        // trouver la note la plus proche dans other
        let mut total_distance = 0u32;
        let mut used = vec![false; other_pcs.len()];

        for &self_pc in &self_pcs {
            let mut min_dist = 12u32;
            let mut min_idx = 0;

            for (i, &other_pc) in other_pcs.iter().enumerate() {
                if used[i] {
                    continue;
                }
                // Distance minimale sur le cercle des pitch classes
                let dist = {
                    let d1 = ((other_pc as i32) - (self_pc as i32)).unsigned_abs();
                    let d2 = 12 - d1;
                    d1.min(d2)
                };
                if dist < min_dist {
                    min_dist = dist;
                    min_idx = i;
                }
            }

            if min_dist < 12 && min_idx < used.len() {
                used[min_idx] = true;
                total_distance += min_dist;
            }
        }

        total_distance
    }

    /// Nom complet de l'accord (ex: "C#m7")
    pub fn name(&self) -> String {
        let root_name = NoteName::from_pitch_class(self.root).as_str();
        let suffix = self.chord_type.suffix();
        let bass_str = self.bass.map_or(String::new(), |b| {
            format!("/{}", NoteName::from_pitch_class(b).as_str())
        });
        format!("{}{}{}", root_name, suffix, bass_str)
    }

    /// Convertit vers le ChordQuality de l'ancien système (pour compatibilité)
    pub fn to_basic_quality(&self) -> super::basic::ChordQuality {
        match self.chord_type {
            ChordType::Major | ChordType::Major7 | ChordType::Augmented => super::basic::ChordQuality::Major,
            ChordType::Minor | ChordType::Minor7 | ChordType::HalfDiminished => super::basic::ChordQuality::Minor,
            ChordType::Diminished | ChordType::Diminished7 => super::basic::ChordQuality::Diminished,
            ChordType::Dominant7 => super::basic::ChordQuality::Dominant7,
            ChordType::Sus2 | ChordType::Sus4 => super::basic::ChordQuality::Sus2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_classes() {
        // C Major = C, E, G = 0, 4, 7
        let c_maj = Chord::new(0, ChordType::Major);
        assert_eq!(c_maj.pitch_classes(), vec![0, 4, 7]);

        // A Minor = A, C, E = 9, 0, 4
        let a_min = Chord::new(9, ChordType::Minor);
        assert_eq!(a_min.pitch_classes(), vec![9, 0, 4]);

        // G Dominant7 = G, B, D, F = 7, 11, 2, 5
        let g7 = Chord::new(7, ChordType::Dominant7);
        assert_eq!(g7.pitch_classes(), vec![7, 11, 2, 5]);
    }

    #[test]
    fn test_chord_name() {
        assert_eq!(Chord::new(0, ChordType::Major).name(), "C");
        assert_eq!(Chord::new(0, ChordType::Minor).name(), "Cm");
        assert_eq!(Chord::new(1, ChordType::Dominant7).name(), "C#7");
        assert_eq!(Chord::new(4, ChordType::Minor7).name(), "Em7");
        assert_eq!(Chord::new(0, ChordType::Major).with_bass(7).name(), "C/G");
    }

    #[test]
    fn test_voice_leading_distance() {
        // C Major -> A Minor (relatif): C,E,G -> A,C,E
        // C->C (0), E->E (0), G->A (2) = total 2
        let c_maj = Chord::new(0, ChordType::Major);
        let a_min = Chord::new(9, ChordType::Minor);
        assert_eq!(c_maj.voice_leading_distance(&a_min), 2);

        // C Major -> C Minor (parallèle): C,E,G -> C,Eb,G
        // C->C (0), E->Eb (1), G->G (0) = total 1
        let c_min = Chord::new(0, ChordType::Minor);
        assert_eq!(c_maj.voice_leading_distance(&c_min), 1);
    }

    #[test]
    fn test_symmetric_chords() {
        assert!(ChordType::Augmented.is_symmetric());
        assert!(ChordType::Diminished7.is_symmetric());
        assert!(!ChordType::Major.is_symmetric());
        assert!(!ChordType::Minor.is_symmetric());
    }

    #[test]
    fn test_ambiguous_chords() {
        assert!(ChordType::Sus4.is_ambiguous());
        assert!(ChordType::Augmented.is_ambiguous());
        assert!(ChordType::Diminished7.is_ambiguous());
        assert!(!ChordType::Major.is_ambiguous());
    }
}
