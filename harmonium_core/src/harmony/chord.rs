//! Module de représentation des accords
//!
//! Fournit une représentation enrichie des accords avec:
//! - `PitchClass` (0-11)
//! - `ChordType` étendu (Major, Minor, Aug, Dim, Dom7, etc.)
//! - Extensions et basse séparée
//! - Niveau LCC (Lydian Chromatic Concept)

/// Pitch class (0-11, où 0=C, 1=C#/Db, 2=D, etc.)
pub type PitchClass = u8;

/// Noms des notes pour affichage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteName {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl NoteName {
    /// Convertit un `PitchClass` en `NoteName`
    #[must_use]
    pub fn from_pitch_class(pc: PitchClass) -> Self {
        match pc % 12 {
            0 => Self::C,
            1 => Self::Cs,
            2 => Self::D,
            3 => Self::Ds,
            4 => Self::E,
            5 => Self::F,
            6 => Self::Fs,
            7 => Self::G,
            8 => Self::Gs,
            9 => Self::A,
            10 => Self::As,
            11 => Self::B,
            _ => unreachable!(),
        }
    }

    /// Convertit en `PitchClass`
    #[must_use]
    pub const fn to_pitch_class(self) -> PitchClass {
        match self {
            Self::C => 0,
            Self::Cs => 1,
            Self::D => 2,
            Self::Ds => 3,
            Self::E => 4,
            Self::F => 5,
            Self::Fs => 6,
            Self::G => 7,
            Self::Gs => 8,
            Self::A => 9,
            Self::As => 10,
            Self::B => 11,
        }
    }

    /// Nom pour affichage
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Cs => "C#",
            Self::D => "D",
            Self::Ds => "D#",
            Self::E => "E",
            Self::F => "F",
            Self::Fs => "F#",
            Self::G => "G",
            Self::Gs => "G#",
            Self::A => "A",
            Self::As => "A#",
            Self::B => "B",
        }
    }
}

impl std::fmt::Display for NoteName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Types d'accords étendus
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    // === NOUVEAUX TYPES (V2) ===
    /// Accord mineur majeur 7 (1-b3-5-7) - ex: `CmMaj7`
    MinorMajor7,
    /// Accord augmenté 7 (1-3-#5-b7) - ex: C7#5
    Augmented7,
    /// Accord majeur 6 (1-3-5-6) - ex: C6
    Major6,
    /// Accord mineur 6 (1-b3-5-6) - ex: Cm6
    Minor6,
    /// Accord dominant 7 sus4 (1-4-5-b7) - ex: C7sus4
    Dominant7Sus4,
    /// Accord add9 (1-3-5-9) - ex: Cadd9
    Add9,
}

impl ChordType {
    /// Retourne les intervalles (en demi-tons) depuis la fondamentale
    #[must_use]
    pub fn intervals(&self) -> Vec<u8> {
        match self {
            Self::Major => vec![0, 4, 7],
            Self::Minor => vec![0, 3, 7],
            Self::Augmented => vec![0, 4, 8],
            Self::Diminished => vec![0, 3, 6],
            Self::Dominant7 => vec![0, 4, 7, 10],
            Self::Major7 => vec![0, 4, 7, 11],
            Self::Minor7 => vec![0, 3, 7, 10],
            Self::HalfDiminished => vec![0, 3, 6, 10],
            Self::Diminished7 => vec![0, 3, 6, 9],
            Self::Sus2 => vec![0, 2, 7],
            Self::Sus4 => vec![0, 5, 7],
            // Nouveaux types V2
            Self::MinorMajor7 => vec![0, 3, 7, 11],
            Self::Augmented7 => vec![0, 4, 8, 10],
            Self::Major6 => vec![0, 4, 7, 9],
            Self::Minor6 => vec![0, 3, 7, 9],
            Self::Dominant7Sus4 => vec![0, 5, 7, 10],
            Self::Add9 => vec![0, 2, 4, 7],
        }
    }

    /// Retourne le nombre de notes dans l'accord (cardinalité)
    #[must_use]
    pub fn cardinality(&self) -> usize {
        self.intervals().len()
    }

    /// Vérifie si c'est une triade (3 notes)
    #[must_use]
    pub fn is_triad(&self) -> bool {
        self.cardinality() == 3
    }

    /// Vérifie si c'est un tétracorde (4 notes)
    #[must_use]
    pub fn is_tetrad(&self) -> bool {
        self.cardinality() == 4
    }

    /// Nom court pour affichage
    #[must_use]
    pub const fn suffix(&self) -> &'static str {
        match self {
            Self::Major => "",
            Self::Minor => "m",
            Self::Augmented => "+",
            Self::Diminished => "dim",
            Self::Dominant7 => "7",
            Self::Major7 => "maj7",
            Self::Minor7 => "m7",
            Self::HalfDiminished => "m7b5",
            Self::Diminished7 => "dim7",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            // Nouveaux types V2
            Self::MinorMajor7 => "mMaj7",
            Self::Augmented7 => "7#5",
            Self::Major6 => "6",
            Self::Minor6 => "m6",
            Self::Dominant7Sus4 => "7sus4",
            Self::Add9 => "add9",
        }
    }

    /// Vérifie si c'est un accord majeur (inclut Maj7, Dom7, 6)
    #[must_use]
    pub const fn is_major(&self) -> bool {
        matches!(
            self,
            Self::Major
                | Self::Major7
                | Self::Dominant7
                | Self::Augmented
                | Self::Major6
                | Self::Augmented7
                | Self::Add9
        )
    }

    /// Vérifie si c'est un accord mineur (inclut Min7, `HalfDim`, mMaj7)
    #[must_use]
    pub const fn is_minor(&self) -> bool {
        matches!(
            self,
            Self::Minor
                | Self::Minor7
                | Self::HalfDiminished
                | Self::Diminished
                | Self::Diminished7
                | Self::MinorMajor7
                | Self::Minor6
        )
    }

    /// Vérifie si c'est un accord symétrique (utile pour pivots Neo-Riemannian)
    #[must_use]
    pub const fn is_symmetric(&self) -> bool {
        matches!(self, Self::Augmented | Self::Diminished7)
    }

    /// Vérifie si c'est un accord ambigu (sans tierce claire)
    #[must_use]
    pub const fn is_ambiguous(&self) -> bool {
        matches!(
            self,
            Self::Sus2 | Self::Sus4 | Self::Augmented | Self::Diminished7 | Self::Dominant7Sus4
        )
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
    /// Niveau LCC (1-12, voir `lydian_chromatic.rs`)
    pub lcc_level: u8,
}

impl Default for Chord {
    fn default() -> Self {
        Self {
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
    #[must_use]
    pub const fn new(root: PitchClass, chord_type: ChordType) -> Self {
        Self { root: root % 12, chord_type, bass: None, extensions: Vec::new(), lcc_level: 1 }
    }

    /// Crée un accord avec basse différente (slash chord)
    #[must_use]
    pub const fn with_bass(mut self, bass: PitchClass) -> Self {
        self.bass = Some(bass % 12);
        self
    }

    /// Ajoute une extension (9, 11, 13 en demi-tons depuis root)
    #[must_use]
    pub fn with_extension(mut self, interval: u8) -> Self {
        self.extensions.push(interval % 12);
        self
    }

    /// Définit le niveau LCC
    #[must_use]
    pub fn with_lcc_level(mut self, level: u8) -> Self {
        self.lcc_level = level.clamp(1, 12);
        self
    }

    /// Retourne les pitch classes de l'accord (sans extensions)
    #[must_use]
    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        self.chord_type.intervals().iter().map(|&interval| (self.root + interval) % 12).collect()
    }

    /// Retourne toutes les pitch classes (avec extensions)
    #[must_use]
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
    #[must_use]
    pub const fn is_ambiguous(&self) -> bool {
        self.chord_type.is_ambiguous()
    }

    /// Calcule la distance de voice-leading vers un autre accord
    /// (somme des mouvements minimaux en demi-tons)
    #[must_use]
    pub fn voice_leading_distance(&self, other: &Self) -> u32 {
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
                    let d1 = (i32::from(other_pc) - i32::from(self_pc)).unsigned_abs();
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
    #[must_use]
    pub fn name(&self) -> String {
        let root_name = NoteName::from_pitch_class(self.root).as_str();
        let suffix = self.chord_type.suffix();
        let bass_str = self
            .bass
            .map_or(String::new(), |b| format!("/{}", NoteName::from_pitch_class(b).as_str()));
        format!("{root_name}{suffix}{bass_str}")
    }

    /// Convertit vers le `ChordQuality` de l'ancien système (pour compatibilité)
    #[must_use]
    pub const fn to_basic_quality(&self) -> super::basic::ChordQuality {
        match self.chord_type {
            ChordType::Major
            | ChordType::Major7
            | ChordType::Augmented
            | ChordType::Major6
            | ChordType::Augmented7
            | ChordType::Add9 => super::basic::ChordQuality::Major,
            ChordType::Minor
            | ChordType::Minor7
            | ChordType::HalfDiminished
            | ChordType::MinorMajor7
            | ChordType::Minor6 => super::basic::ChordQuality::Minor,
            ChordType::Diminished | ChordType::Diminished7 => {
                super::basic::ChordQuality::Diminished
            }
            ChordType::Dominant7 | ChordType::Dominant7Sus4 => {
                super::basic::ChordQuality::Dominant7
            }
            ChordType::Sus2 | ChordType::Sus4 => super::basic::ChordQuality::Sus2,
        }
    }

    /// Identifie un accord à partir d'un ensemble de pitch classes.
    /// Retourne None si aucun type d'accord valide ne correspond.
    ///
    /// Cette méthode est utilisée par le `ParsimoniousDriver` pour valider
    /// les accords candidats générés par les mouvements de voix.
    #[must_use]
    pub fn identify(pitch_classes: &[PitchClass]) -> Option<Self> {
        if pitch_classes.is_empty() {
            return None;
        }

        // Normaliser et trier les pitch classes
        let mut pcs: Vec<PitchClass> = pitch_classes.iter().map(|&pc| pc % 12).collect();
        pcs.sort_unstable();
        pcs.dedup();

        // Essayer chaque note comme potentielle fondamentale
        for &potential_root in &pcs {
            // Calculer les intervalles depuis cette fondamentale
            let mut intervals: Vec<u8> =
                pcs.iter().map(|&pc| (pc + 12 - potential_root) % 12).collect();
            intervals.sort_unstable();

            // Essayer de matcher contre les types d'accords connus
            if let Some(chord_type) = Self::match_intervals(&intervals) {
                return Some(Self::new(potential_root, chord_type));
            }
        }

        None
    }

    /// Matche un ensemble d'intervalles à un `ChordType`
    fn match_intervals(intervals: &[u8]) -> Option<ChordType> {
        // Définir tous les patterns d'accords (ensembles d'intervalles)
        // Ordre important: plus spécifiques d'abord pour éviter les faux positifs
        let patterns: &[(&[u8], ChordType)] = &[
            // Triades
            (&[0, 4, 7], ChordType::Major),
            (&[0, 3, 7], ChordType::Minor),
            (&[0, 4, 8], ChordType::Augmented),
            (&[0, 3, 6], ChordType::Diminished),
            (&[0, 2, 7], ChordType::Sus2),
            (&[0, 5, 7], ChordType::Sus4),
            // Tétracordes (7èmes et 6èmes)
            (&[0, 4, 7, 10], ChordType::Dominant7),
            (&[0, 4, 7, 11], ChordType::Major7),
            (&[0, 3, 7, 10], ChordType::Minor7),
            (&[0, 3, 6, 10], ChordType::HalfDiminished),
            (&[0, 3, 6, 9], ChordType::Diminished7),
            (&[0, 3, 7, 11], ChordType::MinorMajor7),
            (&[0, 4, 8, 10], ChordType::Augmented7),
            (&[0, 4, 7, 9], ChordType::Major6),
            (&[0, 3, 7, 9], ChordType::Minor6),
            (&[0, 5, 7, 10], ChordType::Dominant7Sus4),
            (&[0, 2, 4, 7], ChordType::Add9),
        ];

        for (pattern, chord_type) in patterns {
            if intervals == *pattern {
                return Some(*chord_type);
            }
        }

        None
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
