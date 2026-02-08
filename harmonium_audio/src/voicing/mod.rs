//! Module de Voicing - Stratégies d'harmonisation pour Jazz
//!
//! Ce module fournit une abstraction modulaire pour transformer
//! des notes mélodiques simples en voicings riches (accords).
//!
//! # Architecture
//!
//! Le trait `Voicer` définit l'interface commune. Plusieurs implémentations
//! sont disponibles et peuvent être swappées dynamiquement:
//!
//! - `BlockChordVoicer`: Style George Shearing (locked hands)
//! - `ShellVoicer`: Style Be-Bop (guide tones: tierce + septième)
//!
//! # Exemple
//!
//! ```ignore
//! use harmonium::voicing::{Voicer, BlockChordVoicer, VoicerContext};
//!
//! let mut voicer = BlockChordVoicer::new(4);
//! let ctx = VoicerContext::default();
//!
//! if voicer.should_voice(&ctx) {
//!     let notes = voicer.process_note(72, 100, &ctx);
//!     // notes contient maintenant 4 VoicedNote
//! }
//! ```

mod block_chord;
mod comping;
mod shell;
mod voicer;

// Re-exports publics
pub use block_chord::BlockChordVoicer;
pub use comping::CompingPattern;
pub use shell::ShellVoicer;
pub use voicer::{
    VoicedNote, Voicer, VoicerContext, apply_drop_two, find_scale_notes_below, get_guide_tones,
};
