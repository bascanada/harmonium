//! Emotional Morphing Synthesis Module
//!
//! Implements bilinear interpolation for morphing synthesis parameters
//! across Russell's Circumplex Model (Valence × Arousal).

pub mod types;
pub mod presets;
pub mod morph;
pub mod modulation;

// Re-export commonly used types
pub use types::*;
pub use presets::{EmotionalPresetBank, InstrumentPresets};
pub use morph::{EmotionalMorpher, MorphedPresets, QuadWeights};
pub use modulation::apply_tension_density_modulation;
