//! Export module containing multiple file format exporters
//!
//! Provides clean APIs for exporting musical data to various formats:
//! - MusicXML: Notation software format
//! - RecordingTruth: Serialized session metadata
//! - Musical DNA: Intermediate representation for analysis and tuning

pub mod dna;
mod musicxml;
mod truth;
mod version;

// Re-export public API
pub use dna::{
    DNAExtractor, GlobalMetrics, HarmonicFrame, MusicalDNA, PolygonSignature, RhythmicDNA,
    SerializableTRQ,
};
pub use musicxml::{
    ChordSymbol, ClefType, KeyMode, ScoreNote, score_to_musicxml, score_to_musicxml_with_version,
    to_musicxml, to_musicxml_with_chords, write_musicxml, write_musicxml_with_chords,
    write_score_musicxml,
};
pub use truth::RecordingTruth;
pub use version::GitVersion;
