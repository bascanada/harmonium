//! Musical Notation Format Module
//!
//! This module provides a purely musical representation for score visualization,
//! separate from the audio/MIDI domain. It enables:
//! - VexFlow-compatible notation rendering
//! - MusicXML export
//! - Transposing instrument support
//! - Audio/Score synchronization via shared note IDs
//!
//! # Architecture
//! ```text
//! MusicKernel.tick()
//!       │
//!       │ note_id (shared)
//!       │
//!   ┌───┴───┐
//!   ▼       ▼
//! AudioEvent   ScoreNoteEvent
//! (audio)      (visualization)
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// NOTE ID GENERATION
// ═══════════════════════════════════════════════════════════════════

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique note ID (thread-safe)
///
/// This ID is shared between AudioEvent and ScoreNoteEvent to enable
/// synchronized playback highlighting.
#[must_use]
pub fn next_note_id() -> u64 {
    NOTE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Reset the note ID counter.
///
/// Intended for tests only — calling this in production will cause ID collisions.
#[doc(hidden)]
pub fn reset_note_id_counter() {
    NOTE_ID_COUNTER.store(1, Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════════════
// HARMONIUM SCORE - ROOT STRUCTURE
// ═══════════════════════════════════════════════════════════════════

/// Complete musical score for visualization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HarmoniumScore {
    /// Format version
    pub version: String,
    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Tempo in BPM
    pub tempo: f32,
    /// Time signature (numerator, denominator)
    pub time_signature: (u8, u8),
    /// Key signature
    pub key_signature: KeySignature,
    /// Parts (instruments)
    pub parts: Vec<Part>,
}

impl Default for HarmoniumScore {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            title: None,
            tempo: 120.0,
            time_signature: (4, 4),
            key_signature: KeySignature::default(),
            parts: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// KEY SIGNATURE
// ═══════════════════════════════════════════════════════════════════

/// Key signature with root, mode, and circle of fifths position
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeySignature {
    /// Root note name (e.g., "C", "F#", "Bb")
    pub root: String,
    /// Major or minor mode
    pub mode: KeyMode,
    /// Circle of fifths position (-7 to +7)
    /// Positive = sharps, Negative = flats
    pub fifths: i8,
}

impl Default for KeySignature {
    fn default() -> Self {
        Self { root: "C".to_string(), mode: KeyMode::Major, fifths: 0 }
    }
}

impl KeySignature {
    /// Create a key signature from root pitch class and mode
    #[must_use]
    pub fn from_pitch_class(root: u8, is_minor: bool) -> Self {
        let root_name = pitch_class_to_name(root, !is_minor);
        let mode = if is_minor { KeyMode::Minor } else { KeyMode::Major };
        let fifths = fifths_from_key(root, is_minor);
        Self { root: root_name, mode, fifths }
    }
}

/// Key mode
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    Major,
    Minor,
}

// ═══════════════════════════════════════════════════════════════════
// PART (INSTRUMENT)
// ═══════════════════════════════════════════════════════════════════

/// A part/instrument in the score
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    /// Part identifier (e.g., "lead", "bass", "drums")
    pub id: String,
    /// Display name (e.g., "Alto Saxophone")
    pub name: String,
    /// Clef type
    pub clef: Clef,
    /// Transposition (for transposing instruments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transposition: Option<Transposition>,
    /// Measures in this part
    pub measures: Vec<Measure>,
}

/// Clef types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Clef {
    Treble,
    Bass,
    Percussion,
}

// ═══════════════════════════════════════════════════════════════════
// TRANSPOSITION
// ═══════════════════════════════════════════════════════════════════

/// Transposition for transposing instruments
///
/// Concert C sounds as written [interval] + [octave adjustment]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transposition {
    /// Transposition interval
    pub interval: TransposeInterval,
    /// Octave shift (-1, 0, +1)
    #[serde(default)]
    pub octave_shift: i8,
}

impl Transposition {
    /// Create transposition for Bb instruments (clarinet, trumpet, tenor sax)
    #[must_use]
    pub const fn bb() -> Self {
        Self { interval: TransposeInterval::M2, octave_shift: 0 }
    }

    /// Create transposition for Eb instruments (alto sax, baritone sax)
    #[must_use]
    pub const fn eb() -> Self {
        Self { interval: TransposeInterval::m3, octave_shift: 0 }
    }

    /// Create transposition for F instruments (French horn)
    #[must_use]
    pub const fn f() -> Self {
        Self { interval: TransposeInterval::P5, octave_shift: 0 }
    }

    /// Create transposition for tenor sax (Bb, octave lower)
    #[must_use]
    pub const fn tenor_sax() -> Self {
        Self { interval: TransposeInterval::M2, octave_shift: -1 }
    }

    /// Get the semitone offset for this transposition
    #[must_use]
    pub const fn semitone_offset(&self) -> i8 {
        let interval_semitones = match self.interval {
            TransposeInterval::P1 => 0,
            TransposeInterval::M2 => 2,
            TransposeInterval::m3 => 3,
            TransposeInterval::P4 => 5,
            TransposeInterval::P5 => 7,
        };
        interval_semitones + (self.octave_shift * 12)
    }
}

/// Transposition intervals
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransposeInterval {
    /// Perfect unison (C instruments, no transposition)
    P1,
    /// Major 2nd up (Bb instruments)
    M2,
    /// Minor 3rd up (Eb instruments)
    #[allow(non_camel_case_types)]
    m3,
    /// Perfect 4th up
    P4,
    /// Perfect 5th up (F instruments)
    P5,
}

// ═══════════════════════════════════════════════════════════════════
// MEASURE
// ═══════════════════════════════════════════════════════════════════

/// A measure/bar in the score
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Measure {
    /// Measure number (1-indexed)
    pub number: usize,
    /// Optional time signature change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_signature: Option<(u8, u8)>,
    /// Optional key signature change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_signature: Option<KeySignature>,
    /// Note events
    pub events: Vec<ScoreNoteEvent>,
    /// Chord symbols
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chords: Vec<ChordSymbol>,
}

impl Measure {
    /// Create an empty measure
    #[must_use]
    pub fn new(number: usize) -> Self {
        Self {
            number,
            time_signature: None,
            key_signature: None,
            events: Vec::new(),
            chords: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SCORE NOTE EVENT (with ID for audio sync)
// ═══════════════════════════════════════════════════════════════════

/// A note event in the score, linked to AudioEvent via shared ID
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreNoteEvent {
    /// Unique ID matching the corresponding AudioEvent
    pub id: u64,
    /// Position in measure (1-indexed beat, e.g., 1.0, 1.5, 2.0)
    pub beat: f32,
    /// Event type
    #[serde(rename = "type")]
    pub event_type: NoteEventType,
    /// Pitches (for notes and chords)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pitches: Vec<Pitch>,
    /// Duration
    pub duration: Duration,
    /// Dynamic marking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<Dynamic>,
    /// Articulation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub articulation: Option<Articulation>,
}

/// Note event types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteEventType {
    Note,
    Rest,
    Chord,
    Drum,
}

// ═══════════════════════════════════════════════════════════════════
// PITCH (purely musical, no MIDI)
// ═══════════════════════════════════════════════════════════════════

/// Musical pitch representation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pitch {
    /// Note step (C, D, E, F, G, A, B)
    pub step: NoteStep,
    /// Octave (0-9, middle C = C4)
    pub octave: u8,
    /// Alteration (-2 = double-flat, -1 = flat, 0 = natural, 1 = sharp, 2 = double-sharp)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub alter: i8,
}

impl Pitch {
    /// Create a new pitch
    #[must_use]
    pub const fn new(step: NoteStep, octave: u8, alter: i8) -> Self {
        Self { step, octave, alter }
    }

    /// Convert to MIDI note number
    #[must_use]
    pub fn to_midi(&self) -> u8 {
        let base = match self.step {
            NoteStep::C => 0,
            NoteStep::D => 2,
            NoteStep::E => 4,
            NoteStep::F => 5,
            NoteStep::G => 7,
            NoteStep::A => 9,
            NoteStep::B => 11,
        };
        #[allow(clippy::cast_sign_loss)]
        let midi = (self.octave as i16 + 1) * 12 + base as i16 + self.alter as i16;
        midi.clamp(0, 127) as u8
    }

    /// Convert to VexFlow key string (e.g., "c/4", "f#/5")
    #[must_use]
    pub fn to_vexflow(&self) -> String {
        let step_str = match self.step {
            NoteStep::C => "c",
            NoteStep::D => "d",
            NoteStep::E => "e",
            NoteStep::F => "f",
            NoteStep::G => "g",
            NoteStep::A => "a",
            NoteStep::B => "b",
        };
        let alter_str = match self.alter {
            -2 => "bb",
            -1 => "b",
            0 => "",
            1 => "#",
            2 => "##",
            _ => "",
        };
        format!("{step_str}{alter_str}/{}", self.octave)
    }

    /// Convert to readable string (e.g., "C#4", "Bb3")
    #[must_use]
    pub fn to_string_notation(&self) -> String {
        let step_str = match self.step {
            NoteStep::C => "C",
            NoteStep::D => "D",
            NoteStep::E => "E",
            NoteStep::F => "F",
            NoteStep::G => "G",
            NoteStep::A => "A",
            NoteStep::B => "B",
        };
        let alter_str = match self.alter {
            -2 => "bb",
            -1 => "b",
            0 => "",
            1 => "#",
            2 => "##",
            _ => "",
        };
        format!("{step_str}{alter_str}{}", self.octave)
    }
}

/// Note steps (white keys)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NoteStep {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

// ═══════════════════════════════════════════════════════════════════
// DURATION (named, not numeric)
// ═══════════════════════════════════════════════════════════════════

/// Musical duration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Duration {
    /// Base duration
    pub base: DurationBase,
    /// Number of dots (0, 1, or 2)
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub dots: usize,
    /// Tuplet ratio (e.g., [3, 2] = triplet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuplet: Option<(u8, u8)>,
}

impl Duration {
    /// Create a new duration
    #[must_use]
    pub const fn new(base: DurationBase) -> Self {
        Self { base, dots: 0, tuplet: None }
    }

    /// Add a dot
    #[must_use]
    pub const fn dotted(mut self) -> Self {
        self.dots = 1;
        self
    }

    /// Convert to VexFlow duration string
    #[must_use]
    pub fn to_vexflow(&self) -> String {
        let base_str = match self.base {
            DurationBase::Whole => "w",
            DurationBase::Half => "h",
            DurationBase::Quarter => "q",
            DurationBase::Eighth => "8",
            DurationBase::Sixteenth => "16",
            DurationBase::ThirtySecond => "32",
        };
        let dot_str = "d".repeat(self.dots);
        format!("{base_str}{dot_str}")
    }

    /// Get duration in beats (quarter = 1.0)
    #[must_use]
    pub fn to_beats(&self) -> f32 {
        let base_beats = match self.base {
            DurationBase::Whole => 4.0,
            DurationBase::Half => 2.0,
            DurationBase::Quarter => 1.0,
            DurationBase::Eighth => 0.5,
            DurationBase::Sixteenth => 0.25,
            DurationBase::ThirtySecond => 0.125,
        };

        // Apply dots (each dot adds half of previous value)
        let mut total = base_beats;
        let mut add = base_beats / 2.0;
        for _ in 0..self.dots {
            total += add;
            add /= 2.0;
        }

        // Apply tuplet
        if let Some((num, denom)) = self.tuplet {
            total *= denom as f32 / num as f32;
        }

        total
    }
}

/// Base duration values
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DurationBase {
    Whole,
    Half,
    Quarter,
    Eighth,
    #[serde(rename = "16th")]
    Sixteenth,
    #[serde(rename = "32nd")]
    ThirtySecond,
}

// ═══════════════════════════════════════════════════════════════════
// DYNAMICS & ARTICULATIONS
// ═══════════════════════════════════════════════════════════════════

/// Dynamic markings
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Dynamic {
    #[serde(rename = "ppp")]
    Pianississimo,
    #[serde(rename = "pp")]
    Pianissimo,
    #[serde(rename = "p")]
    Piano,
    #[serde(rename = "mp")]
    MezzoPiano,
    #[serde(rename = "mf")]
    MezzoForte,
    #[serde(rename = "f")]
    Forte,
    #[serde(rename = "ff")]
    Fortissimo,
    #[serde(rename = "fff")]
    Fortississimo,
}

impl Dynamic {
    /// Convert MIDI velocity to dynamic
    #[must_use]
    pub const fn from_velocity(velocity: u8) -> Self {
        match velocity {
            0..=15 => Self::Pianississimo,
            16..=31 => Self::Pianissimo,
            32..=49 => Self::Piano,
            50..=69 => Self::MezzoPiano,
            70..=89 => Self::MezzoForte,
            90..=109 => Self::Forte,
            110..=124 => Self::Fortissimo,
            125..=127 => Self::Fortississimo,
            _ => Self::MezzoForte, // Unreachable but needed for exhaustiveness
        }
    }
}

/// Articulation markings
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Articulation {
    Staccato,
    Accent,
    Tenuto,
    Marcato,
}

// ═══════════════════════════════════════════════════════════════════
// DRUM SYMBOLS (simplified)
// ═══════════════════════════════════════════════════════════════════

/// Simplified drum symbols
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DrumSymbol {
    /// Kick drum
    K,
    /// Snare
    S,
    /// Hi-hat (closed)
    H,
    /// Hi-hat (open)
    Ho,
    /// Ride cymbal
    R,
    /// Crash cymbal
    C,
    /// Tom 1 (high)
    T1,
    /// Tom 2 (mid)
    T2,
    /// Tom 3 (low/floor)
    T3,
}

impl DrumSymbol {
    /// Convert channel to drum symbol
    #[must_use]
    pub const fn from_channel(channel: u8) -> Option<Self> {
        match channel {
            0 => Some(Self::K), // Bass/Kick channel
            2 => Some(Self::S), // Snare channel
            3 => Some(Self::H), // Hat channel
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// CHORD SYMBOLS
// ═══════════════════════════════════════════════════════════════════

/// Chord symbol for harmony annotation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChordSymbol {
    /// Position in measure (beat)
    pub beat: f32,
    /// Duration in beats
    pub duration: f32,
    /// Root note name
    pub root: String,
    /// Chord quality
    pub quality: String,
    /// Bass note for slash chords
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bass: Option<String>,
    /// Scale suggestion for improvisation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<ScaleSuggestion>,
}

/// Scale suggestion for a chord
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScaleSuggestion {
    /// Scale name (e.g., "C Mixolydian")
    pub name: String,
    /// Scale degrees as note names
    pub degrees: Vec<String>,
    /// Chord tones to highlight
    pub chord_tones: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════
// CONVERSION UTILITIES
// ═══════════════════════════════════════════════════════════════════

/// Convert MIDI pitch to musical Pitch
///
/// Uses the key signature to determine whether to use sharps or flats.
#[must_use]
pub fn midi_to_pitch(midi: u8, key_fifths: i8) -> Pitch {
    let octave = midi / 12;
    let octave = if octave > 0 { octave - 1 } else { 0 };
    let pc = midi % 12;
    let use_sharps = key_fifths >= 0;

    let (step, alter) = match pc {
        0 => (NoteStep::C, 0),
        1 => {
            if use_sharps {
                (NoteStep::C, 1)
            } else {
                (NoteStep::D, -1)
            }
        }
        2 => (NoteStep::D, 0),
        3 => {
            if use_sharps {
                (NoteStep::D, 1)
            } else {
                (NoteStep::E, -1)
            }
        }
        4 => (NoteStep::E, 0),
        5 => (NoteStep::F, 0),
        6 => {
            if use_sharps {
                (NoteStep::F, 1)
            } else {
                (NoteStep::G, -1)
            }
        }
        7 => (NoteStep::G, 0),
        8 => {
            if use_sharps {
                (NoteStep::G, 1)
            } else {
                (NoteStep::A, -1)
            }
        }
        9 => (NoteStep::A, 0),
        10 => {
            if use_sharps {
                (NoteStep::A, 1)
            } else {
                (NoteStep::B, -1)
            }
        }
        11 => (NoteStep::B, 0),
        _ => (NoteStep::C, 0),
    };

    Pitch { step, octave, alter }
}

/// Convert step duration to musical Duration
#[must_use]
pub fn steps_to_duration(steps: usize, steps_per_quarter: usize) -> Duration {
    let quarters = steps as f32 / steps_per_quarter as f32;

    let (base, dots) = if quarters >= 4.0 {
        (DurationBase::Whole, 0)
    } else if quarters >= 3.0 {
        (DurationBase::Half, 1)
    } else if quarters >= 2.0 {
        (DurationBase::Half, 0)
    } else if quarters >= 1.5 {
        (DurationBase::Quarter, 1)
    } else if quarters >= 1.0 {
        (DurationBase::Quarter, 0)
    } else if quarters >= 0.75 {
        (DurationBase::Eighth, 1)
    } else if quarters >= 0.5 {
        (DurationBase::Eighth, 0)
    } else if quarters >= 0.375 {
        (DurationBase::Sixteenth, 1)
    } else if quarters >= 0.25 {
        (DurationBase::Sixteenth, 0)
    } else {
        (DurationBase::ThirtySecond, 0)
    };

    Duration { base, dots, tuplet: None }
}

/// Calculate circle of fifths position from key root and mode
#[must_use]
pub const fn fifths_from_key(key_root: u8, is_minor: bool) -> i8 {
    let root = key_root % 12;

    // Major key fifths mapping
    let major_fifths: [i8; 12] = [
        0,  // C  = 0 fifths
        -5, // Db = -5 fifths
        2,  // D  = 2 fifths
        -3, // Eb = -3 fifths
        4,  // E  = 4 fifths
        -1, // F  = -1 fifths
        6,  // F# = 6 fifths
        1,  // G  = 1 fifth
        -4, // Ab = -4 fifths
        3,  // A  = 3 fifths
        -2, // Bb = -2 fifths
        5,  // B  = 5 fifths
    ];

    if is_minor {
        // Minor keys use relative major signature
        let relative_major = (root + 3) % 12;
        major_fifths[relative_major as usize]
    } else {
        major_fifths[root as usize]
    }
}

/// Convert pitch class to note name
#[must_use]
pub fn pitch_class_to_name(pc: u8, use_sharps: bool) -> String {
    match pc % 12 {
        0 => "C".to_string(),
        1 => {
            if use_sharps {
                "C#".to_string()
            } else {
                "Db".to_string()
            }
        }
        2 => "D".to_string(),
        3 => {
            if use_sharps {
                "D#".to_string()
            } else {
                "Eb".to_string()
            }
        }
        4 => "E".to_string(),
        5 => "F".to_string(),
        6 => {
            if use_sharps {
                "F#".to_string()
            } else {
                "Gb".to_string()
            }
        }
        7 => "G".to_string(),
        8 => {
            if use_sharps {
                "G#".to_string()
            } else {
                "Ab".to_string()
            }
        }
        9 => "A".to_string(),
        10 => {
            if use_sharps {
                "A#".to_string()
            } else {
                "Bb".to_string()
            }
        }
        11 => "B".to_string(),
        _ => "C".to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS FOR SERDE
// ═══════════════════════════════════════════════════════════════════

const fn is_zero(n: &i8) -> bool {
    *n == 0
}

const fn is_zero_usize(n: &usize) -> bool {
    *n == 0
}

// ═══════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_id_generation() {
        reset_note_id_counter();
        let id1 = next_note_id();
        let id2 = next_note_id();
        let id3 = next_note_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_midi_to_pitch_c4() {
        let pitch = midi_to_pitch(60, 0);
        assert_eq!(pitch.step, NoteStep::C);
        assert_eq!(pitch.octave, 4);
        assert_eq!(pitch.alter, 0);
    }

    #[test]
    fn test_midi_to_pitch_sharp_key() {
        // G major (1 sharp), C# should be spelled as C#
        let pitch = midi_to_pitch(61, 1);
        assert_eq!(pitch.step, NoteStep::C);
        assert_eq!(pitch.alter, 1); // C#
    }

    #[test]
    fn test_midi_to_pitch_flat_key() {
        // F major (1 flat), C# should be spelled as Db
        let pitch = midi_to_pitch(61, -1);
        assert_eq!(pitch.step, NoteStep::D);
        assert_eq!(pitch.alter, -1); // Db
    }

    #[test]
    fn test_pitch_to_vexflow() {
        let pitch = Pitch::new(NoteStep::C, 4, 0);
        assert_eq!(pitch.to_vexflow(), "c/4");

        let pitch = Pitch::new(NoteStep::F, 5, 1);
        assert_eq!(pitch.to_vexflow(), "f#/5");

        let pitch = Pitch::new(NoteStep::B, 3, -1);
        assert_eq!(pitch.to_vexflow(), "bb/3");
    }

    #[test]
    fn test_pitch_to_midi() {
        let pitch = Pitch::new(NoteStep::C, 4, 0);
        assert_eq!(pitch.to_midi(), 60);

        let pitch = Pitch::new(NoteStep::A, 4, 0);
        assert_eq!(pitch.to_midi(), 69);

        let pitch = Pitch::new(NoteStep::C, 4, 1); // C#4
        assert_eq!(pitch.to_midi(), 61);
    }

    #[test]
    fn test_steps_to_duration() {
        // With steps_per_quarter = 4 (16th note resolution)
        assert_eq!(steps_to_duration(4, 4).base, DurationBase::Quarter);
        assert_eq!(steps_to_duration(8, 4).base, DurationBase::Half);
        assert_eq!(steps_to_duration(16, 4).base, DurationBase::Whole);
        assert_eq!(steps_to_duration(2, 4).base, DurationBase::Eighth);
        assert_eq!(steps_to_duration(1, 4).base, DurationBase::Sixteenth);
    }

    #[test]
    fn test_duration_to_vexflow() {
        let d = Duration::new(DurationBase::Quarter);
        assert_eq!(d.to_vexflow(), "q");

        let d = Duration::new(DurationBase::Half).dotted();
        assert_eq!(d.to_vexflow(), "hd");

        let d = Duration::new(DurationBase::Eighth);
        assert_eq!(d.to_vexflow(), "8");
    }

    #[test]
    fn test_duration_to_beats() {
        assert!((Duration::new(DurationBase::Quarter).to_beats() - 1.0).abs() < f32::EPSILON);
        assert!((Duration::new(DurationBase::Half).to_beats() - 2.0).abs() < f32::EPSILON);
        assert!(
            (Duration::new(DurationBase::Quarter).dotted().to_beats() - 1.5).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_fifths_from_key() {
        assert_eq!(fifths_from_key(0, false), 0); // C major
        assert_eq!(fifths_from_key(7, false), 1); // G major
        assert_eq!(fifths_from_key(5, false), -1); // F major
        assert_eq!(fifths_from_key(9, true), 0); // A minor (relative to C major)
    }

    #[test]
    fn test_key_signature_from_pitch_class() {
        let key = KeySignature::from_pitch_class(0, false);
        assert_eq!(key.root, "C");
        assert_eq!(key.mode, KeyMode::Major);
        assert_eq!(key.fifths, 0);

        let key = KeySignature::from_pitch_class(7, false);
        assert_eq!(key.root, "G");
        assert_eq!(key.fifths, 1);
    }

    #[test]
    fn test_dynamic_from_velocity() {
        assert_eq!(Dynamic::from_velocity(0), Dynamic::Pianississimo);
        assert_eq!(Dynamic::from_velocity(64), Dynamic::MezzoPiano);
        assert_eq!(Dynamic::from_velocity(100), Dynamic::Forte);
        assert_eq!(Dynamic::from_velocity(127), Dynamic::Fortississimo);
    }

    #[test]
    fn test_transposition_semitones() {
        assert_eq!(Transposition::bb().semitone_offset(), 2);
        assert_eq!(Transposition::eb().semitone_offset(), 3);
        assert_eq!(Transposition::f().semitone_offset(), 7);
        assert_eq!(Transposition::tenor_sax().semitone_offset(), -10); // M2 - octave
    }

    #[test]
    fn test_score_serialization() {
        let score = HarmoniumScore {
            version: "1.0".to_string(),
            title: Some("Test".to_string()),
            tempo: 120.0,
            time_signature: (4, 4),
            key_signature: KeySignature::default(),
            parts: vec![],
        };

        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("\"version\":\"1.0\""));

        let parsed: HarmoniumScore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tempo, 120.0);
    }
}
