//! DNA Comparator - Similarity and Divergence Metrics
//!
//! Compares Musical DNA profiles against reference style profiles
//! and generates reports for LLM-assisted tuning.

use harmonium_core::exporters::dna::{GlobalMetrics, MusicalDNA};
use serde::{Deserialize, Serialize};

use crate::benchmark::StyleProfile;

/// Comparison report between generated DNA and reference profile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// Overall similarity score (0.0 - 1.0)
    pub overall_similarity: f32,

    /// Voice leading divergence (lower = more similar)
    pub voice_leading_divergence: f32,

    /// Tension variance divergence
    pub tension_divergence: f32,

    /// Harmonic rhythm divergence
    pub harmonic_rhythm_divergence: f32,

    /// Diatonic percentage divergence
    pub diatonic_divergence: f32,

    /// Tension/release balance divergence
    pub balance_divergence: f32,

    /// Suggestions for parameter adjustments
    pub suggestions: Vec<String>,

    /// Detailed metrics comparison
    pub generated_metrics: GlobalMetrics,

    /// Reference metrics
    pub reference_metrics: GlobalMetrics,
}

/// DNA Comparator for similarity analysis
#[derive(Clone, Debug, Default)]
pub struct DNAComparator {
    /// Weights for different metrics in overall similarity
    weights: MetricWeights,
}

/// Weights for combining metrics into overall similarity
#[derive(Clone, Debug)]
pub struct MetricWeights {
    pub voice_leading: f32,
    pub tension_variance: f32,
    pub harmonic_rhythm: f32,
    pub diatonic: f32,
    pub balance: f32,
}

impl Default for MetricWeights {
    fn default() -> Self {
        Self {
            voice_leading: 0.25,
            tension_variance: 0.20,
            harmonic_rhythm: 0.20,
            diatonic: 0.15,
            balance: 0.20,
        }
    }
}

impl DNAComparator {
    /// Create a new DNA comparator with default weights
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom weights
    #[must_use]
    pub fn with_weights(weights: MetricWeights) -> Self {
        Self { weights }
    }

    /// Compare generated DNA against a reference profile
    #[must_use]
    pub fn compare(&self, generated: &MusicalDNA, reference: &StyleProfile) -> ComparisonReport {
        let gen_metrics = &generated.global_metrics;
        let ref_metrics = &reference.metrics;

        // Calculate individual divergences
        let voice_leading_divergence = self.calculate_divergence(
            gen_metrics.average_voice_leading_effort,
            ref_metrics.average_voice_leading_effort,
            10.0, // Max expected value
        );

        let tension_divergence = self.calculate_divergence(
            gen_metrics.tension_variance,
            ref_metrics.tension_variance,
            0.5, // Max expected variance
        );

        let harmonic_rhythm_divergence = self.calculate_divergence(
            gen_metrics.harmonic_rhythm,
            ref_metrics.harmonic_rhythm,
            4.0, // Max chords per measure
        );

        let diatonic_divergence = self.calculate_divergence(
            gen_metrics.diatonic_percentage,
            ref_metrics.diatonic_percentage,
            100.0,
        );

        let balance_divergence = self.calculate_divergence(
            gen_metrics.tension_release_balance,
            ref_metrics.tension_release_balance,
            2.0, // Range: -1 to 1
        );

        // Calculate overall similarity (1.0 - weighted divergence)
        let weighted_divergence = voice_leading_divergence * self.weights.voice_leading
            + tension_divergence * self.weights.tension_variance
            + harmonic_rhythm_divergence * self.weights.harmonic_rhythm
            + diatonic_divergence * self.weights.diatonic
            + balance_divergence * self.weights.balance;

        let overall_similarity = (1.0 - weighted_divergence).clamp(0.0, 1.0);

        // Generate suggestions
        let suggestions = self.generate_suggestions(gen_metrics, ref_metrics);

        ComparisonReport {
            overall_similarity,
            voice_leading_divergence,
            tension_divergence,
            harmonic_rhythm_divergence,
            diatonic_divergence,
            balance_divergence,
            suggestions,
            generated_metrics: gen_metrics.clone(),
            reference_metrics: ref_metrics.clone(),
        }
    }

    /// Compare two DNA profiles directly
    #[must_use]
    pub fn compare_dna(&self, a: &MusicalDNA, b: &MusicalDNA) -> f32 {
        let gen_metrics = &a.global_metrics;
        let ref_metrics = &b.global_metrics;

        let voice_leading_divergence = self.calculate_divergence(
            gen_metrics.average_voice_leading_effort,
            ref_metrics.average_voice_leading_effort,
            10.0,
        );

        let tension_divergence = self.calculate_divergence(
            gen_metrics.tension_variance,
            ref_metrics.tension_variance,
            0.5,
        );

        let harmonic_rhythm_divergence = self.calculate_divergence(
            gen_metrics.harmonic_rhythm,
            ref_metrics.harmonic_rhythm,
            4.0,
        );

        let weighted_divergence = voice_leading_divergence * self.weights.voice_leading
            + tension_divergence * self.weights.tension_variance
            + harmonic_rhythm_divergence * self.weights.harmonic_rhythm;

        (1.0 - weighted_divergence).clamp(0.0, 1.0)
    }

    /// Calculate normalized divergence between two values
    fn calculate_divergence(&self, generated: f32, reference: f32, max_value: f32) -> f32 {
        let diff = (generated - reference).abs();
        (diff / max_value).clamp(0.0, 1.0)
    }

    /// Generate human-readable suggestions based on divergence
    fn generate_suggestions(
        &self,
        gen_metrics: &GlobalMetrics,
        reference: &GlobalMetrics,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Voice leading suggestions
        let vl_diff =
            gen_metrics.average_voice_leading_effort - reference.average_voice_leading_effort;
        if vl_diff.abs() > 0.5 {
            if vl_diff > 0.0 {
                suggestions.push(format!(
                    "Voice leading is rougher than target ({:.2} vs {:.2}). Consider reducing max_semitone_movement.",
                    gen_metrics.average_voice_leading_effort, reference.average_voice_leading_effort
                ));
            } else {
                suggestions.push(format!(
                    "Voice leading is smoother than target ({:.2} vs {:.2}). Consider increasing max_semitone_movement for more contrast.",
                    gen_metrics.average_voice_leading_effort, reference.average_voice_leading_effort
                ));
            }
        }

        // Tension variance suggestions
        let tension_diff = gen_metrics.tension_variance - reference.tension_variance;
        if tension_diff.abs() > 0.05 {
            if tension_diff > 0.0 {
                suggestions.push(format!(
                    "Tension variance is higher than target ({:.4} vs {:.4}). Consider narrowing hysteresis thresholds.",
                    gen_metrics.tension_variance, reference.tension_variance
                ));
            } else {
                suggestions.push(format!(
                    "Tension variance is lower than target ({:.4} vs {:.4}). Consider widening hysteresis thresholds for more dynamic range.",
                    gen_metrics.tension_variance, reference.tension_variance
                ));
            }
        }

        // Harmonic rhythm suggestions
        let hr_diff = gen_metrics.harmonic_rhythm - reference.harmonic_rhythm;
        if hr_diff.abs() > 0.25 {
            if hr_diff > 0.0 {
                suggestions.push(format!(
                    "Harmonic rhythm is faster than target ({:.2} vs {:.2} chords/measure). Consider longer chord durations.",
                    gen_metrics.harmonic_rhythm, reference.harmonic_rhythm
                ));
            } else {
                suggestions.push(format!(
                    "Harmonic rhythm is slower than target ({:.2} vs {:.2} chords/measure). Consider shorter chord durations.",
                    gen_metrics.harmonic_rhythm, reference.harmonic_rhythm
                ));
            }
        }

        // Tension/release balance suggestions
        let balance_diff = gen_metrics.tension_release_balance - reference.tension_release_balance;
        if balance_diff.abs() > 0.1 {
            if balance_diff > 0.0 {
                suggestions.push(format!(
                    "Music is more tense than target ({:.2} vs {:.2}). Consider lowering trq_threshold.",
                    gen_metrics.tension_release_balance, reference.tension_release_balance
                ));
            } else {
                suggestions.push(format!(
                    "Music is more relaxed than target ({:.2} vs {:.2}). Consider raising trq_threshold.",
                    gen_metrics.tension_release_balance, reference.tension_release_balance
                ));
            }
        }

        if suggestions.is_empty() {
            suggestions.push("Generated music closely matches the target style!".to_string());
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metrics(vl: f32, variance: f32, hr: f32) -> GlobalMetrics {
        GlobalMetrics {
            average_voice_leading_effort: vl,
            tension_variance: variance,
            tension_release_balance: 0.0,
            diatonic_percentage: 90.0,
            harmonic_rhythm: hr,
            total_duration_beats: 32.0,
            chord_change_count: 8,
        }
    }

    #[test]
    fn test_identical_metrics_high_similarity() {
        let comparator = DNAComparator::new();

        let dna = MusicalDNA {
            truth: None,
            harmonic_profile: vec![],
            rhythmic_profile: Default::default(),
            global_metrics: make_metrics(2.0, 0.1, 1.0),
        };

        let profile = StyleProfile {
            name: "test".to_string(),
            metrics: make_metrics(2.0, 0.1, 1.0),
            sample_count: 1,
        };

        let report = comparator.compare(&dna, &profile);

        assert!(
            report.overall_similarity > 0.95,
            "Identical metrics should have >95% similarity, got {}",
            report.overall_similarity
        );
    }

    #[test]
    fn test_divergent_metrics_lower_similarity() {
        let comparator = DNAComparator::new();

        let dna = MusicalDNA {
            truth: None,
            harmonic_profile: vec![],
            rhythmic_profile: Default::default(),
            global_metrics: make_metrics(5.0, 0.3, 2.0), // Very different
        };

        let profile = StyleProfile {
            name: "test".to_string(),
            metrics: make_metrics(1.0, 0.05, 0.5), // Smooth, stable, slow
            sample_count: 1,
        };

        let report = comparator.compare(&dna, &profile);

        assert!(
            report.overall_similarity < 0.75,
            "Divergent metrics should have <75% similarity, got {}",
            report.overall_similarity
        );
    }

    #[test]
    fn test_suggestions_generated() {
        let comparator = DNAComparator::new();

        let dna = MusicalDNA {
            truth: None,
            harmonic_profile: vec![],
            rhythmic_profile: Default::default(),
            global_metrics: make_metrics(5.0, 0.3, 2.0),
        };

        let profile = StyleProfile {
            name: "test".to_string(),
            metrics: make_metrics(1.0, 0.05, 0.5),
            sample_count: 1,
        };

        let report = comparator.compare(&dna, &profile);

        assert!(!report.suggestions.is_empty(), "Should generate suggestions");
        assert!(
            report.suggestions.iter().any(|s| s.to_lowercase().contains("voice leading")),
            "Should suggest voice leading adjustment"
        );
    }

    #[test]
    fn test_divergence_calculation() {
        let comparator = DNAComparator::new();

        // Same value = 0 divergence
        assert!((comparator.calculate_divergence(2.0, 2.0, 10.0) - 0.0).abs() < 0.001);

        // Max difference = 1.0 divergence (clamped)
        assert!((comparator.calculate_divergence(0.0, 10.0, 10.0) - 1.0).abs() < 0.001);

        // Partial difference
        assert!((comparator.calculate_divergence(2.0, 4.0, 10.0) - 0.2).abs() < 0.001);
    }
}
