//! Emotional Morphing Synthesis Module
//!
//! Implements bilinear interpolation for morphing synthesis parameters
//! across Russell's Circumplex Model (Valence × Arousal).

pub mod modulation;
pub mod morph;
pub mod presets;
pub mod types;

// Re-export commonly used types
pub use modulation::apply_tension_density_modulation;
pub use morph::{EmotionalMorpher, MorphedPresets, QuadWeights};
pub use presets::{EmotionalPresetBank, InstrumentPresets};
pub use types::*;
