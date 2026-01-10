//! EmotionMapper - Traducteur Émotions → Paramètres Musicaux
//!
//! Ce module contient toute la logique de mapping qui était auparavant
//! dispersée dans `update_controls()`. C'est un composant pur, sans état,
//! facile à tester unitairement.
//!
//! ## Principe de Russell's Circumplex Model
//! ```text
//!              High Arousal
//!                   ↑
//!                   |
//!     Angry    ←────┼────→    Excited
//!  (-, high)        |        (+, high)
//!                   |
//!  Low Valence ←────┼────→ High Valence
//!                   |
//!     Sad      ←────┼────→    Relaxed
//!  (-, low)         |        (+, low)
//!                   |
//!                   ↓
//!              Low Arousal
//! ```

use harmonium_core::params::EngineParams;
use harmonium_core::params::{MusicalParams, HarmonyStrategy};
use harmonium_core::sequencer::RhythmMode;

/// Configuration du mapper (seuils, courbes, etc.)
#[derive(Clone, Debug)]
pub struct MapperConfig {
    /// BPM minimum (arousal = 0.0)
    pub bpm_min: f32,
    /// BPM maximum (arousal = 1.0)
    pub bpm_max: f32,
    /// Seuil de tension pour passer en Neo-Riemannian/Parsimonious
    pub tension_threshold_high: f32,
    /// Seuil de tension pour revenir en Steedman
    pub tension_threshold_low: f32,
    /// Seuil de tension pour changer d'accord plus vite
    pub fast_chord_change_threshold: f32,
}

impl Default for MapperConfig {
    fn default() -> Self {
        Self {
            bpm_min: 70.0,
            bpm_max: 180.0,
            tension_threshold_high: 0.7,
            tension_threshold_low: 0.5,
            fast_chord_change_threshold: 0.6,
        }
    }
}

/// Traducteur d'émotions vers paramètres musicaux
pub struct EmotionMapper {
    config: MapperConfig,
}

impl Default for EmotionMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionMapper {
    pub fn new() -> Self {
        Self {
            config: MapperConfig::default(),
        }
    }

    pub fn with_config(config: MapperConfig) -> Self {
        Self { config }
    }

    /// Traduit les paramètres émotionnels en paramètres musicaux concrets
    ///
    /// C'est la fonction principale qui contient toute la logique de mapping.
    /// Elle est pure (pas d'effets de bord) et peut être testée unitairement.
    pub fn map(&self, emotions: &EngineParams) -> MusicalParams {
        let mut params = MusicalParams::default();

        // ═══════════════════════════════════════════════════════════════════
        // BPM: Arousal → Tempo
        // ═══════════════════════════════════════════════════════════════════
        // Faible arousal = calme = tempo lent
        // Haute arousal = excité = tempo rapide
        params.bpm = self.config.bpm_min + (emotions.arousal * (self.config.bpm_max - self.config.bpm_min));

        // ═══════════════════════════════════════════════════════════════════
        // RYTHME: Density + Tension → Pattern
        // ═══════════════════════════════════════════════════════════════════
        params.rhythm_mode = emotions.algorithm;
        params.rhythm_density = emotions.density;
        params.rhythm_tension = emotions.tension;

        // Pulses primaires: density contrôle la "densité" d'accents
        // density 0.0 → 1 pulse, density 1.0 → 12 pulses (sur 16 steps)
        if emotions.algorithm == RhythmMode::Euclidean {
            params.rhythm_steps = 16;
            params.rhythm_pulses = ((emotions.density * 11.0) as usize + 1).min(16);
        } else {
            params.rhythm_steps = emotions.poly_steps;
            // PerfectBalance garde les pulses internes
        }

        // Rotation: tension contrôle le décalage de phase (syncopation)
        let max_rotation = if emotions.algorithm == RhythmMode::PerfectBalance {
            params.rhythm_steps / 2
        } else {
            8
        };
        params.rhythm_rotation = (emotions.tension * max_rotation as f32) as usize;

        // Séquenceur secondaire (polyrythme 4:3)
        params.rhythm_secondary_steps = 12;
        params.rhythm_secondary_pulses = ((emotions.density * 8.0) as usize + 1).min(12);
        params.rhythm_secondary_rotation = 8_usize.saturating_sub(params.rhythm_rotation % 8);

        // ═══════════════════════════════════════════════════════════════════
        // HARMONIE: Tension + Valence → Stratégie + Palette
        // ═══════════════════════════════════════════════════════════════════
        params.harmony_mode = emotions.harmony_mode;
        params.harmony_tension = emotions.tension;
        params.harmony_valence = emotions.valence;

        // Sélection automatique de stratégie selon tension
        params.harmony_strategy = if emotions.tension > self.config.tension_threshold_high {
            // Haute tension → transformations géométriques (plus imprévisible)
            HarmonyStrategy::NeoRiemannian
        } else if emotions.tension < self.config.tension_threshold_low {
            // Basse tension → harmonie fonctionnelle (prévisible, stable)
            HarmonyStrategy::Steedman
        } else {
            // Zone intermédiaire → parsimonieux (transition douce)
            HarmonyStrategy::Parsimonious
        };

        // Vitesse de changement d'accords
        params.harmony_measures_per_chord = if emotions.tension > self.config.fast_chord_change_threshold {
            1 // Changement rapide
        } else {
            2 // Changement normal
        };

        // ═══════════════════════════════════════════════════════════════════
        // MÉLODIE: Smoothness → Hurst factor
        // ═══════════════════════════════════════════════════════════════════
        params.melody_smoothness = emotions.smoothness;

        // Voicing: density + tension
        params.voicing_density = emotions.density;
        params.voicing_tension = emotions.tension;

        // Octave: arousal influence légèrement (plus aigu quand excité)
        params.melody_octave = 4 + (emotions.arousal * 0.5) as i32;

        // ═══════════════════════════════════════════════════════════════════
        // MIXER: Passthrough des gains
        // ═══════════════════════════════════════════════════════════════════
        params.gain_lead = emotions.gain_lead;
        params.gain_bass = emotions.gain_bass;
        params.gain_snare = emotions.gain_snare;
        params.gain_hat = emotions.gain_hat;
        params.vel_base_bass = emotions.vel_base_bass;
        params.vel_base_snare = emotions.vel_base_snare;

        // ═══════════════════════════════════════════════════════════════════
        // ROUTAGE & RECORDING: Passthrough
        // ═══════════════════════════════════════════════════════════════════
        params.channel_routing = emotions.channel_routing.clone();
        params.muted_channels = emotions.muted_channels.clone();
        params.record_wav = emotions.record_wav;
        params.record_midi = emotions.record_midi;
        params.record_abc = emotions.record_abc;

        // Mode Drum Kit (passthrough)
        params.fixed_kick = emotions.fixed_kick;

        params
    }

    /// Fonction statique pour usage rapide sans instancier le mapper
    pub fn quick_map(emotions: &EngineParams) -> MusicalParams {
        EmotionMapper::new().map(emotions)
    }
}

/// Fonctions utilitaires pour le mapping
impl EmotionMapper {
    /// Calcule le BPM depuis l'arousal (extraction de la logique)
    pub fn arousal_to_bpm(&self, arousal: f32) -> f32 {
        self.config.bpm_min + (arousal.clamp(0.0, 1.0) * (self.config.bpm_max - self.config.bpm_min))
    }

    /// Détermine la stratégie harmonique depuis la tension
    pub fn tension_to_strategy(&self, tension: f32) -> HarmonyStrategy {
        if tension > self.config.tension_threshold_high {
            HarmonyStrategy::NeoRiemannian
        } else if tension < self.config.tension_threshold_low {
            HarmonyStrategy::Steedman
        } else {
            HarmonyStrategy::Parsimonious
        }
    }

    /// Calcule les pulses depuis la density pour un nombre de steps donné
    pub fn density_to_pulses(density: f32, max_steps: usize) -> usize {
        let scale = (max_steps as f32 * 0.75).max(1.0);
        ((density * scale) as usize + 1).min(max_steps)
    }

    /// Calcule la rotation depuis la tension pour un nombre de steps donné
    pub fn tension_to_rotation(tension: f32, max_steps: usize) -> usize {
        let max_rotation = max_steps / 2;
        (tension * max_rotation as f32) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arousal_to_bpm() {
        let mapper = EmotionMapper::new();

        // Arousal 0.0 = BPM minimum
        assert_eq!(mapper.arousal_to_bpm(0.0), 70.0);

        // Arousal 1.0 = BPM maximum
        assert_eq!(mapper.arousal_to_bpm(1.0), 180.0);

        // Arousal 0.5 = BPM moyen
        assert_eq!(mapper.arousal_to_bpm(0.5), 125.0);
    }

    #[test]
    fn test_tension_to_strategy() {
        let mapper = EmotionMapper::new();

        // Basse tension → Steedman
        assert_eq!(mapper.tension_to_strategy(0.3), HarmonyStrategy::Steedman);

        // Haute tension → NeoRiemannian
        assert_eq!(mapper.tension_to_strategy(0.8), HarmonyStrategy::NeoRiemannian);

        // Tension moyenne → Parsimonious
        assert_eq!(mapper.tension_to_strategy(0.6), HarmonyStrategy::Parsimonious);
    }

    #[test]
    fn test_full_mapping() {
        let emotions = EngineParams {
            arousal: 0.7,        // → ~147 BPM
            valence: 0.5,       // Major
            density: 0.4,       // Medium density
            tension: 0.3,       // Low tension → Steedman
            smoothness: 0.7,
            ..Default::default()
        };

        let params = EmotionMapper::quick_map(&emotions);

        // Vérifier le BPM
        assert!((params.bpm - 147.0).abs() < 1.0);

        // Vérifier la stratégie (basse tension)
        assert_eq!(params.harmony_strategy, HarmonyStrategy::Steedman);

        // Vérifier que les modules sont actifs par défaut
        assert!(params.enable_rhythm);
        assert!(params.enable_harmony);
        assert!(params.enable_melody);
    }

    #[test]
    fn test_density_to_pulses() {
        // 16 steps, density 0 → 1 pulse
        assert_eq!(EmotionMapper::density_to_pulses(0.0, 16), 1);

        // 16 steps, density 1 → 12 pulses (75% max)
        assert_eq!(EmotionMapper::density_to_pulses(1.0, 16), 13);

        // 48 steps, density 0.5 → ~18 pulses
        let pulses = EmotionMapper::density_to_pulses(0.5, 48);
        assert!(pulses > 10 && pulses < 30);
    }

    #[test]
    fn test_custom_config() {
        let config = MapperConfig {
            bpm_min: 60.0,
            bpm_max: 200.0,
            ..Default::default()
        };
        let mapper = EmotionMapper::with_config(config);

        assert_eq!(mapper.arousal_to_bpm(0.0), 60.0);
        assert_eq!(mapper.arousal_to_bpm(1.0), 200.0);
    }
}
