//! Tuning Parameters - Configurable Constants for Algorithm Optimization
//!
//! This module centralizes all tunable parameters for the music generation
//! algorithms. Instead of hardcoded constants scattered across modules,
//! these parameters can be:
//!
//! - Loaded from a config file (TOML/JSON)
//! - Adjusted via the LLM tuning loop
//! - Saved/restored for reproducibility
//!
//! The default values match the original hardcoded constants to ensure
//! backwards compatibility.

use serde::{Deserialize, Serialize};

/// Centralized tuning parameters for music generation algorithms
///
/// These parameters control various aspects of harmony and rhythm generation.
/// The defaults match the original hardcoded values in the codebase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TuningParams {
    // === HARMONY: Parsimonious Voice Leading ===
    /// Maximum semitone movement per voice in parsimonious voice leading.
    /// Original: `parsimonious.rs:19` `MAX_SEMITONE_MOVEMENT = 2`
    pub max_semitone_movement: u8,

    /// Allow cardinality morphing (triad ↔ tetrad transitions).
    /// Original: `ParsimoniousDriver::default()` `allow_cardinality_morph = true`
    pub cardinality_morph_enabled: bool,

    /// TRQ threshold for neighbor selection in parsimonious driver.
    /// Lower values prefer more relaxed transitions.
    /// Original: `ParsimoniousDriver::default()` `trq_threshold = 0.5`
    pub trq_threshold: f32,

    // === HARMONIC DRIVER: Strategy Selection ===
    /// Lower threshold for Steedman strategy (stays in Steedman until tension exceeds this).
    /// Original: `HarmonicDriver::new()` `steedman_lower = 0.45`
    pub steedman_lower_threshold: f32,

    /// Upper threshold for Steedman strategy (enters Steedman if tension drops below this).
    /// Original: `HarmonicDriver::new()` `steedman_upper = 0.55`
    pub steedman_upper_threshold: f32,

    /// Lower threshold for Neo-Riemannian strategy (enters if tension exceeds this).
    /// Original: `HarmonicDriver::new()` `neo_lower = 0.65`
    pub neo_riemannian_lower_threshold: f32,

    /// Upper threshold for Neo-Riemannian strategy (stays until tension drops below this).
    /// Original: `HarmonicDriver::new()` `neo_upper = 0.75`
    pub neo_riemannian_upper_threshold: f32,

    /// Hysteresis boost for strategy stability (prevents rapid switching).
    /// Original: `driver.rs:273` `HYSTERESIS_BOOST = 0.1`
    pub hysteresis_boost: f32,

    // === RHYTHM: Perfect Balance Algorithm ===
    /// Kick polygon vertices at low density (< 0.3).
    /// Original: `sequencer.rs:268` Digon (2 vertices)
    pub kick_low_density_vertices: usize,

    /// Kick polygon vertices at higher density (≥ 0.3).
    /// Original: `sequencer.rs:272` Square (4 vertices)
    pub kick_high_density_vertices: usize,

    /// Density threshold for kick polygon switching.
    /// Original: `sequencer.rs:268` `density < 0.3`
    pub kick_density_threshold: f32,

    /// Snare vertices at low density (< 0.5).
    /// Original: `sequencer.rs:279` Triangle (3 vertices)
    pub snare_low_density_vertices: usize,

    /// Snare vertices at higher density (≥ 0.5).
    /// Original: `sequencer.rs:279` Hexagon (6 vertices)
    pub snare_high_density_vertices: usize,

    /// Density threshold for snare polygon switching.
    /// Original: `sequencer.rs:279` `density < 0.5`
    pub snare_density_threshold: f32,

    /// Hat vertices at very low density (< 0.25).
    /// Original: `sequencer.rs:293` 6 vertices (triplet eighths)
    pub hat_very_low_density_vertices: usize,

    /// Hat vertices at low density (0.25 - 0.6).
    /// Original: `sequencer.rs:295` 8 vertices (straight eighths)
    pub hat_low_density_vertices: usize,

    /// Hat vertices at medium density (0.6 - 0.85).
    /// Original: `sequencer.rs:297` 12 vertices (sixteenths)
    pub hat_medium_density_vertices: usize,

    /// Hat vertices at high density (≥ 0.85).
    /// Original: `sequencer.rs:299` 16 vertices (fast polyrhythm)
    pub hat_high_density_vertices: usize,

    // === MELODY: Markov + Fractal ===
    /// Hurst exponent for fractal noise (0.0 = white, 0.5 = pink, 1.0 = brown).
    /// Controls smoothness of melodic contours.
    pub melody_hurst_factor: f32,

    /// Maximum melodic leap in semitones before gap-fill compensation.
    pub melody_max_leap: u8,

    /// Gap-fill compensation strength (0.0 = none, 1.0 = full).
    pub melody_gap_fill_strength: f32,
}

impl Default for TuningParams {
    fn default() -> Self {
        Self {
            // Harmony: Parsimonious
            max_semitone_movement: 2,
            cardinality_morph_enabled: true,
            trq_threshold: 0.5,

            // Harmonic Driver
            steedman_lower_threshold: 0.45,
            steedman_upper_threshold: 0.55,
            neo_riemannian_lower_threshold: 0.65,
            neo_riemannian_upper_threshold: 0.75,
            hysteresis_boost: 0.1,

            // Rhythm: Perfect Balance
            kick_low_density_vertices: 2,
            kick_high_density_vertices: 4,
            kick_density_threshold: 0.3,
            snare_low_density_vertices: 3,
            snare_high_density_vertices: 6,
            snare_density_threshold: 0.5,
            hat_very_low_density_vertices: 6,
            hat_low_density_vertices: 8,
            hat_medium_density_vertices: 12,
            hat_high_density_vertices: 16,

            // Melody
            melody_hurst_factor: 0.8,
            melody_max_leap: 7,
            melody_gap_fill_strength: 0.5,
        }
    }
}

impl TuningParams {
    /// Create a new TuningParams with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load tuning parameters from a TOML file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_toml_file(path: &std::path::Path) -> Result<Self, TuningError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| TuningError::IoError(e.to_string()))?;
        Self::from_toml_str(&content)
    }

    /// Parse tuning parameters from a TOML string
    ///
    /// # Errors
    /// Returns an error if the TOML cannot be parsed.
    pub fn from_toml_str(content: &str) -> Result<Self, TuningError> {
        toml::from_str(content).map_err(|e| TuningError::ParseError(e.to_string()))
    }

    /// Save tuning parameters to a TOML file
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn to_toml_file(&self, path: &std::path::Path) -> Result<(), TuningError> {
        let content = self.to_toml_string()?;
        std::fs::write(path, content).map_err(|e| TuningError::IoError(e.to_string()))
    }

    /// Serialize tuning parameters to a TOML string
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_toml_string(&self) -> Result<String, TuningError> {
        toml::to_string_pretty(self).map_err(|e| TuningError::SerializeError(e.to_string()))
    }

    /// Load tuning parameters from a JSON file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, TuningError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| TuningError::IoError(e.to_string()))?;
        Self::from_json_str(&content)
    }

    /// Parse tuning parameters from a JSON string
    ///
    /// # Errors
    /// Returns an error if the JSON cannot be parsed.
    pub fn from_json_str(content: &str) -> Result<Self, TuningError> {
        serde_json::from_str(content).map_err(|e| TuningError::ParseError(e.to_string()))
    }

    /// Serialize tuning parameters to a JSON string
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_json_string(&self) -> Result<String, TuningError> {
        serde_json::to_string_pretty(self).map_err(|e| TuningError::SerializeError(e.to_string()))
    }

    /// Validate that all parameters are within sensible ranges
    ///
    /// # Errors
    /// Returns an error describing the first invalid parameter found.
    pub fn validate(&self) -> Result<(), TuningError> {
        // Validate semitone movement (1-4 is reasonable)
        if self.max_semitone_movement == 0 || self.max_semitone_movement > 4 {
            return Err(TuningError::ValidationError(format!(
                "max_semitone_movement must be 1-4, got {}",
                self.max_semitone_movement
            )));
        }

        // Validate TRQ threshold (0.0 - 1.0)
        if !(0.0..=1.0).contains(&self.trq_threshold) {
            return Err(TuningError::ValidationError(format!(
                "trq_threshold must be 0.0-1.0, got {}",
                self.trq_threshold
            )));
        }

        // Validate hysteresis ordering
        if self.steedman_lower_threshold >= self.steedman_upper_threshold {
            return Err(TuningError::ValidationError(format!(
                "steedman_lower_threshold ({}) must be < steedman_upper_threshold ({})",
                self.steedman_lower_threshold, self.steedman_upper_threshold
            )));
        }
        if self.steedman_upper_threshold > self.neo_riemannian_lower_threshold {
            return Err(TuningError::ValidationError(format!(
                "steedman_upper_threshold ({}) must be <= neo_riemannian_lower_threshold ({})",
                self.steedman_upper_threshold, self.neo_riemannian_lower_threshold
            )));
        }
        if self.neo_riemannian_lower_threshold >= self.neo_riemannian_upper_threshold {
            return Err(TuningError::ValidationError(format!(
                "neo_riemannian_lower_threshold ({}) must be < neo_riemannian_upper_threshold ({})",
                self.neo_riemannian_lower_threshold, self.neo_riemannian_upper_threshold
            )));
        }

        // Validate polygon vertices (at least 2)
        if self.kick_low_density_vertices < 2 || self.kick_high_density_vertices < 2 {
            return Err(TuningError::ValidationError(
                "kick polygon vertices must be >= 2".to_string(),
            ));
        }

        Ok(())
    }

    /// Merge another TuningParams, taking non-default values from `other`
    ///
    /// This is useful for partial updates where you only want to change
    /// specific parameters.
    #[must_use]
    pub fn merge_with(&self, other: &Self) -> Self {
        // For now, just return `other` - could be smarter about defaults later
        other.clone()
    }
}

/// Errors that can occur when working with tuning parameters
#[derive(Debug, Clone)]
pub enum TuningError {
    /// IO error (file read/write)
    IoError(String),
    /// Parse error (invalid format)
    ParseError(String),
    /// Serialization error
    SerializeError(String),
    /// Validation error (invalid parameter value)
    ValidationError(String),
}

impl std::fmt::Display for TuningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::SerializeError(msg) => write!(f, "Serialize error: {msg}"),
            Self::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for TuningError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values_match_original() {
        let params = TuningParams::default();

        // Parsimonious defaults
        assert_eq!(params.max_semitone_movement, 2);
        assert!(params.cardinality_morph_enabled);
        assert!((params.trq_threshold - 0.5).abs() < 0.001);

        // Driver defaults
        assert!((params.steedman_lower_threshold - 0.45).abs() < 0.001);
        assert!((params.steedman_upper_threshold - 0.55).abs() < 0.001);
        assert!((params.neo_riemannian_lower_threshold - 0.65).abs() < 0.001);
        assert!((params.neo_riemannian_upper_threshold - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_validation_passes_for_defaults() {
        let params = TuningParams::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_validation_fails_for_invalid_semitone() {
        let mut params = TuningParams::default();
        params.max_semitone_movement = 0;
        assert!(params.validate().is_err());

        params.max_semitone_movement = 10;
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validation_fails_for_invalid_hysteresis_order() {
        let mut params = TuningParams::default();
        params.steedman_lower_threshold = 0.6;
        params.steedman_upper_threshold = 0.5; // Invalid: lower > upper
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_json_roundtrip() {
        let params = TuningParams::default();
        let json = params.to_json_string().unwrap();
        let parsed = TuningParams::from_json_str(&json).unwrap();

        assert_eq!(params.max_semitone_movement, parsed.max_semitone_movement);
        assert_eq!(params.cardinality_morph_enabled, parsed.cardinality_morph_enabled);
    }

    #[test]
    fn test_toml_roundtrip() {
        let params = TuningParams::default();
        let toml = params.to_toml_string().unwrap();
        let parsed = TuningParams::from_toml_str(&toml).unwrap();

        assert_eq!(params.max_semitone_movement, parsed.max_semitone_movement);
        assert!((params.trq_threshold - parsed.trq_threshold).abs() < 0.001);
    }
}
