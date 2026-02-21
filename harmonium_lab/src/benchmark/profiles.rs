//! Style Profile Management
//!
//! Style profiles are aggregated statistics from collections of Musical DNA,
//! representing the "average" characteristics of a musical style.

use std::path::Path;

use anyhow::{Context, Result};
use harmonium_core::dna::{GlobalMetrics, MusicalDNA};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// A style profile representing averaged metrics from a corpus
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StyleProfile {
    /// Name of the style (e.g., "baroque", "romantic", "jazz")
    pub name: String,

    /// Aggregated metrics (averaged from all samples)
    pub metrics: GlobalMetrics,

    /// Number of samples used to build this profile
    pub sample_count: usize,
}

impl StyleProfile {
    /// Create a new empty style profile
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), metrics: GlobalMetrics::default(), sample_count: 0 }
    }

    /// Build a style profile from a directory of DNA JSON files
    ///
    /// # Errors
    /// Returns error if directory cannot be read or files cannot be parsed
    pub fn from_directory(name: &str, dir: &Path) -> Result<Self> {
        let mut all_metrics: Vec<GlobalMetrics> = Vec::new();

        // Walk directory and collect DNA files
        for entry in
            WalkDir::new(dir).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "json" && path.to_string_lossy().contains(".dna.") {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(dna) = serde_json::from_str::<MusicalDNA>(&content) {
                            all_metrics.push(dna.global_metrics);
                        }
                    }
                }
            }
        }

        if all_metrics.is_empty() {
            return Ok(Self::new(name));
        }

        // Calculate averaged metrics
        let metrics = Self::average_metrics(&all_metrics);

        Ok(Self { name: name.to_string(), metrics, sample_count: all_metrics.len() })
    }

    /// Build a style profile from a collection of MusicalDNA
    #[must_use]
    pub fn from_dna_collection(name: &str, dnas: &[MusicalDNA]) -> Self {
        if dnas.is_empty() {
            return Self::new(name);
        }

        let all_metrics: Vec<GlobalMetrics> =
            dnas.iter().map(|d| d.global_metrics.clone()).collect();

        Self {
            name: name.to_string(),
            metrics: Self::average_metrics(&all_metrics),
            sample_count: dnas.len(),
        }
    }

    /// Calculate averaged metrics from a collection
    fn average_metrics(metrics: &[GlobalMetrics]) -> GlobalMetrics {
        if metrics.is_empty() {
            return GlobalMetrics::default();
        }

        let n = metrics.len() as f32;

        let average_voice_leading_effort =
            metrics.iter().map(|m| m.average_voice_leading_effort).sum::<f32>() / n;

        let tension_variance = metrics.iter().map(|m| m.tension_variance).sum::<f32>() / n;

        let tension_release_balance =
            metrics.iter().map(|m| m.tension_release_balance).sum::<f32>() / n;

        let diatonic_percentage = metrics.iter().map(|m| m.diatonic_percentage).sum::<f32>() / n;

        let harmonic_rhythm = metrics.iter().map(|m| m.harmonic_rhythm).sum::<f32>() / n;

        let total_duration_beats = metrics.iter().map(|m| m.total_duration_beats).sum::<f32>() / n;

        let chord_change_count =
            (metrics.iter().map(|m| m.chord_change_count).sum::<usize>() as f32 / n) as usize;

        GlobalMetrics {
            average_voice_leading_effort,
            tension_variance,
            tension_release_balance,
            diatonic_percentage,
            harmonic_rhythm,
            total_duration_beats,
            chord_change_count,
        }
    }

    /// Save profile to a JSON file
    ///
    /// # Errors
    /// Returns error if file cannot be written
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to serialize profile")?;
        std::fs::write(path, json).context("Failed to write profile file")?;
        Ok(())
    }

    /// Load profile from a JSON file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read profile file")?;
        let profile: Self =
            serde_json::from_str(&content).context("Failed to parse profile JSON")?;
        Ok(profile)
    }

    /// Merge another profile into this one (weighted average)
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let total_samples = self.sample_count + other.sample_count;

        if total_samples == 0 {
            return Self::new(&self.name);
        }

        let self_weight = self.sample_count as f32 / total_samples as f32;
        let other_weight = other.sample_count as f32 / total_samples as f32;

        let metrics = GlobalMetrics {
            average_voice_leading_effort: self.metrics.average_voice_leading_effort * self_weight
                + other.metrics.average_voice_leading_effort * other_weight,
            tension_variance: self.metrics.tension_variance * self_weight
                + other.metrics.tension_variance * other_weight,
            tension_release_balance: self.metrics.tension_release_balance * self_weight
                + other.metrics.tension_release_balance * other_weight,
            diatonic_percentage: self.metrics.diatonic_percentage * self_weight
                + other.metrics.diatonic_percentage * other_weight,
            harmonic_rhythm: self.metrics.harmonic_rhythm * self_weight
                + other.metrics.harmonic_rhythm * other_weight,
            total_duration_beats: self.metrics.total_duration_beats * self_weight
                + other.metrics.total_duration_beats * other_weight,
            chord_change_count: ((self.metrics.chord_change_count as f32 * self_weight)
                + (other.metrics.chord_change_count as f32 * other_weight))
                as usize,
        };

        Self { name: self.name.clone(), metrics, sample_count: total_samples }
    }

    /// Check if this profile is similar to another (within thresholds)
    #[must_use]
    pub fn is_similar_to(&self, other: &Self, threshold: f32) -> bool {
        let vl_diff = (self.metrics.average_voice_leading_effort
            - other.metrics.average_voice_leading_effort)
            .abs();
        let tension_diff = (self.metrics.tension_variance - other.metrics.tension_variance).abs();
        let hr_diff = (self.metrics.harmonic_rhythm - other.metrics.harmonic_rhythm).abs();

        vl_diff < threshold * 2.0 && tension_diff < threshold * 0.1 && hr_diff < threshold
    }
}

/// Predefined style profiles based on music theory literature
impl StyleProfile {
    /// Baroque style profile (e.g., Bach)
    /// Characteristics: Smooth voice leading, moderate tension variance, steady harmonic rhythm
    #[must_use]
    pub fn baroque() -> Self {
        Self {
            name: "baroque".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 1.5,
                tension_variance: 0.08,
                tension_release_balance: -0.1, // Slightly release-biased
                diatonic_percentage: 92.0,
                harmonic_rhythm: 1.0, // 1 chord per measure
                total_duration_beats: 64.0,
                chord_change_count: 16,
            },
            sample_count: 0, // Theoretical, not from samples
        }
    }

    /// Classical style profile (e.g., Mozart, Haydn)
    /// Characteristics: Clear phrases, balanced tension, periodic structure
    #[must_use]
    pub fn classical() -> Self {
        Self {
            name: "classical".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 1.8,
                tension_variance: 0.12,
                tension_release_balance: 0.0, // Balanced
                diatonic_percentage: 88.0,
                harmonic_rhythm: 1.5,
                total_duration_beats: 64.0,
                chord_change_count: 24,
            },
            sample_count: 0,
        }
    }

    /// Romantic style profile (e.g., Chopin, Brahms)
    /// Characteristics: More chromaticism, higher tension variance, expressive
    #[must_use]
    pub fn romantic() -> Self {
        Self {
            name: "romantic".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 2.2,
                tension_variance: 0.18,
                tension_release_balance: 0.1, // Slightly tension-biased
                diatonic_percentage: 78.0,
                harmonic_rhythm: 2.0,
                total_duration_beats: 64.0,
                chord_change_count: 32,
            },
            sample_count: 0,
        }
    }

    /// Jazz style profile
    /// Characteristics: Complex harmony, high tension variance, extended chords
    #[must_use]
    pub fn jazz() -> Self {
        Self {
            name: "jazz".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 2.5,
                tension_variance: 0.22,
                tension_release_balance: 0.15,
                diatonic_percentage: 65.0,
                harmonic_rhythm: 2.5,
                total_duration_beats: 32.0,
                chord_change_count: 20,
            },
            sample_count: 0,
        }
    }

    /// Ambient/minimal style profile
    /// Characteristics: Very smooth, low tension, slow harmonic rhythm
    #[must_use]
    pub fn ambient() -> Self {
        Self {
            name: "ambient".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 0.8,
                tension_variance: 0.03,
                tension_release_balance: -0.2,
                diatonic_percentage: 95.0,
                harmonic_rhythm: 0.25, // 1 chord every 4 measures
                total_duration_beats: 64.0,
                chord_change_count: 4,
            },
            sample_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let profile = StyleProfile::new("test");
        assert_eq!(profile.name, "test");
        assert_eq!(profile.sample_count, 0);
    }

    #[test]
    fn test_from_dna_collection() {
        let dnas = vec![
            MusicalDNA {
                truth: None,
                harmonic_profile: vec![],
                rhythmic_profile: Default::default(),
                global_metrics: GlobalMetrics {
                    average_voice_leading_effort: 2.0,
                    tension_variance: 0.1,
                    tension_release_balance: 0.0,
                    diatonic_percentage: 90.0,
                    harmonic_rhythm: 1.0,
                    total_duration_beats: 32.0,
                    chord_change_count: 8,
                },
            },
            MusicalDNA {
                truth: None,
                harmonic_profile: vec![],
                rhythmic_profile: Default::default(),
                global_metrics: GlobalMetrics {
                    average_voice_leading_effort: 4.0,
                    tension_variance: 0.2,
                    tension_release_balance: 0.2,
                    diatonic_percentage: 80.0,
                    harmonic_rhythm: 2.0,
                    total_duration_beats: 64.0,
                    chord_change_count: 16,
                },
            },
        ];

        let profile = StyleProfile::from_dna_collection("test", &dnas);

        assert_eq!(profile.sample_count, 2);
        assert!((profile.metrics.average_voice_leading_effort - 3.0).abs() < 0.01);
        assert!((profile.metrics.tension_variance - 0.15).abs() < 0.01);
        assert!((profile.metrics.harmonic_rhythm - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_merge_profiles() {
        let a = StyleProfile {
            name: "a".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 2.0,
                tension_variance: 0.1,
                tension_release_balance: 0.0,
                diatonic_percentage: 90.0,
                harmonic_rhythm: 1.0,
                total_duration_beats: 32.0,
                chord_change_count: 8,
            },
            sample_count: 10,
        };

        let b = StyleProfile {
            name: "b".to_string(),
            metrics: GlobalMetrics {
                average_voice_leading_effort: 4.0,
                tension_variance: 0.2,
                tension_release_balance: 0.2,
                diatonic_percentage: 80.0,
                harmonic_rhythm: 2.0,
                total_duration_beats: 64.0,
                chord_change_count: 16,
            },
            sample_count: 10,
        };

        let merged = a.merge(&b);

        assert_eq!(merged.sample_count, 20);
        // Equal weights, so should be average
        assert!((merged.metrics.average_voice_leading_effort - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_predefined_profiles() {
        let baroque = StyleProfile::baroque();
        let jazz = StyleProfile::jazz();

        // Baroque should have smoother voice leading than jazz
        assert!(
            baroque.metrics.average_voice_leading_effort
                < jazz.metrics.average_voice_leading_effort
        );

        // Jazz should have lower diatonic percentage
        assert!(baroque.metrics.diatonic_percentage > jazz.metrics.diatonic_percentage);
    }

    #[test]
    fn test_is_similar() {
        let a = StyleProfile::baroque();
        let b = StyleProfile::baroque();
        let c = StyleProfile::jazz();

        assert!(a.is_similar_to(&b, 0.5));
        assert!(!a.is_similar_to(&c, 0.5));
    }
}
