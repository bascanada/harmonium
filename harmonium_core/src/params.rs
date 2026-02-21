//! Paramètres Musicaux Techniques - Découplés des Émotions
//!
//! Ce module contient les paramètres techniques purs qui pilotent le moteur.
//! Le moteur n'a plus à "deviner" quoi faire - il applique ces paramètres directement.
//!
//! ## Architecture
//! ```text
//! [UI/IA] → EngineParams → EmotionMapper → MusicalParams → HarmoniumEngine
//!                   ou
//! [Debug] → MusicalParams → HarmoniumEngine (bypass émotionnel)
//! ```

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

use crate::{harmony::HarmonyMode, sequencer::RhythmMode};

// ═══════════════════════════════════════════════════════════════════
// TIME SIGNATURE
// ═══════════════════════════════════════════════════════════════════

/// Time signature (N/D format) - e.g., 4/4, 3/4, 5/4, 7/8
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    /// Numerator (beats per measure)
    pub numerator: u8,
    /// Denominator (beat unit: 2=half, 4=quarter, 8=eighth, 16=sixteenth)
    pub denominator: u8,
}

impl TimeSignature {
    /// Create a new time signature
    #[must_use]
    pub const fn new(numerator: u8, denominator: u8) -> Self {
        Self { numerator, denominator }
    }

    // Common time signatures
    pub const FOUR_FOUR: Self = Self::new(4, 4);
    pub const THREE_FOUR: Self = Self::new(3, 4);
    pub const SIX_EIGHT: Self = Self::new(6, 8);
    pub const FIVE_FOUR: Self = Self::new(5, 4);
    pub const SEVEN_EIGHT: Self = Self::new(7, 8);

    /// Calculate steps per measure given a subdivision
    ///
    /// # Arguments
    /// * `steps_per_quarter` - Steps per quarter note (e.g., 4, 12, 24, 48)
    ///
    /// # Returns
    /// Total steps per measure
    ///
    /// # Examples
    /// ```
    /// # use harmonium_core::params::TimeSignature;
    /// // 4/4 with 16th note resolution (4 subdivisions per quarter)
    /// assert_eq!(TimeSignature::FOUR_FOUR.steps_per_measure(4), 16);
    ///
    /// // 3/4 with 16th note resolution
    /// assert_eq!(TimeSignature::THREE_FOUR.steps_per_measure(4), 12);
    ///
    /// // 5/4 with 16th note resolution
    /// assert_eq!(TimeSignature::FIVE_FOUR.steps_per_measure(4), 20);
    /// ```
    #[must_use]
    pub const fn steps_per_measure(&self, steps_per_quarter: usize) -> usize {
        // Convert denominator to quarter-note equivalent
        // denominator=4 (quarter) → 1.0x, denominator=8 (eighth) → 0.5x, denominator=2 (half) → 2.0x
        let quarter_equiv = match self.denominator {
            2 => self.numerator as usize * 2,      // half notes
            4 => self.numerator as usize,          // quarter notes
            8 => (self.numerator as usize + 1) / 2, // eighth notes (round up)
            16 => (self.numerator as usize + 3) / 4, // sixteenth notes (round up)
            _ => self.numerator as usize,          // fallback to numerator
        };
        quarter_equiv * steps_per_quarter
    }

    /// Validate the time signature
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.numerator > 0
            && (self.denominator == 2
                || self.denominator == 4
                || self.denominator == 8
                || self.denominator == 16)
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::FOUR_FOUR
    }
}

impl std::fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

// ═══════════════════════════════════════════════════════════════════
// MUSICAL POSITION (for visualization)
// ═══════════════════════════════════════════════════════════════════

/// Musical position within a performance
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MusicalPosition {
    /// Measure number (1-indexed)
    pub measure: usize,
    /// Beat within measure (1-indexed)
    pub beat: usize,
    /// Step within beat (0-indexed)
    pub step_in_beat: usize,
    /// Total step count from start (for absolute positioning)
    pub total_step: usize,
}

// ═══════════════════════════════════════════════════════════════════
// CONTROL MODE & PARAMS
// ═══════════════════════════════════════════════════════════════════

/// État du mode de contrôle (émotion vs direct)
/// Partagé entre VST et standalone builds
#[derive(Clone, Debug)]
pub struct ControlMode {
    /// true = mode émotion (`EngineParams` → `EmotionMapper` → `MusicalParams`)
    /// false = mode direct (`MusicalParams` directement)
    pub use_emotion_mode: bool,
    /// Paramètres musicaux directs (utilisés quand `use_emotion_mode` = false)
    pub direct_params: MusicalParams,

    // === GLOBAL ENABLE OVERRIDES ===
    // Ces flags s'appliquent dans TOUS les modes (émotion ET direct)
    /// Enable rhythm module (global override)
    pub enable_rhythm: bool,
    /// Enable harmony module (global override)
    pub enable_harmony: bool,
    /// Enable melody module (global override)
    pub enable_melody: bool,
    /// Enable voicing (harmonized chords) - global override
    pub enable_voicing: bool,
    /// Mode Drum Kit (kick fixe sur C1/36)
    pub fixed_kick: bool,

    // === WEBVIEW CONTROL FLAGS ===
    /// When true, the webview is the source of truth for emotional params
    /// and `sync_params_to_engine` should NOT overwrite `target_state`
    pub webview_controls_emotions: bool,
    /// When true, the webview controls the emotion/direct mode switch
    pub webview_controls_mode: bool,
    /// When true, the webview controls direct/technical params
    pub webview_controls_direct: bool,

    // === LIVE STATE FROM ENGINE ===
    // Updated by the engine during processing for UI visualization
    /// Current step in the sequencer (updated by engine)
    pub current_step: u32,
    /// Current measure number
    pub current_measure: u32,
    /// Primary pattern (for visualization)
    pub primary_pattern: Vec<bool>,
    /// Secondary pattern (for visualization)
    pub secondary_pattern: Vec<bool>,
    /// Current chord name
    pub current_chord: String,
    /// Whether current chord is minor
    pub is_minor_chord: bool,
    /// Progression name
    pub progression_name: String,
    /// Session key (e.g., "C")
    pub session_key: String,
    /// Session scale (e.g., "major")
    pub session_scale: String,
}

impl Default for ControlMode {
    fn default() -> Self {
        Self {
            use_emotion_mode: true,
            direct_params: MusicalParams::default(),
            // All modules enabled by default
            enable_rhythm: true,
            enable_harmony: true,
            enable_melody: true,
            enable_voicing: false,
            fixed_kick: false,
            // Start with DAW params as source of truth
            webview_controls_emotions: false,
            webview_controls_mode: false,
            webview_controls_direct: false,
            // Live state
            current_step: 0,
            current_measure: 1,
            primary_pattern: vec![],
            secondary_pattern: vec![],
            current_chord: "I".to_string(),
            is_minor_chord: false,
            progression_name: String::new(),
            session_key: "C".to_string(),
            session_scale: "major".to_string(),
        }
    }
}

/// Stratégie harmonique explicite (pour le Driver)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HarmonyStrategy {
    /// Grammaire de Steedman (harmonie fonctionnelle: ii-V-I, etc.)
    #[default]
    Steedman,
    /// Transformations Neo-Riemannian (P, L, R - triades uniquement)
    NeoRiemannian,
    /// Voice-leading parsimonieux (tous types d'accords)
    Parsimonious,
    /// Laisser le Driver décider automatiquement selon la tension
    Auto,
}

/// Paramètres purement techniques. Le moteur obéit à ça directement.
/// Aucune interprétation émotionnelle - juste des valeurs musicales concrètes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicalParams {
    // ═══════════════════════════════════════════════════════════════════
    // GLOBAL
    // ═══════════════════════════════════════════════════════════════════
    /// BPM direct (pas calculé depuis arousal)
    pub bpm: f32,

    /// Volume master (0.0 - 1.0)
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,

    // ═══════════════════════════════════════════════════════════════════
    // MODULES ON/OFF (pour debug/tests)
    // ═══════════════════════════════════════════════════════════════════
    /// Activer le module rythmique
    #[serde(default = "default_true")]
    pub enable_rhythm: bool,

    /// Activer le module harmonique
    #[serde(default = "default_true")]
    pub enable_harmony: bool,

    /// Activer le module mélodique
    #[serde(default = "default_true")]
    pub enable_melody: bool,

    /// Activer le voicing (accords harmonisés sur la mélodie)
    /// Quand désactivé, seule la note mélodique joue
    #[serde(default)]
    pub enable_voicing: bool,

    // ═══════════════════════════════════════════════════════════════════
    // TIME & RYTHME
    // ═══════════════════════════════════════════════════════════════════
    /// Time signature (replaces inference from rhythm_steps)
    #[serde(default)]
    pub time_signature: TimeSignature,

    /// Steps per quarter note (subdivision resolution)
    /// 4 = sixteenth notes, 12 = triplet sixteenths, 24 = thirty-seconds, etc.
    /// This is DECOUPLED from time signature
    #[serde(default = "default_steps_per_quarter")]
    pub steps_per_quarter: usize,

    /// Mode rythmique (Euclidean classique ou `PerfectBalance` polyrythmique)
    #[serde(default)]
    pub rhythm_mode: RhythmMode,

    /// DEPRECATED: Nombre de steps (maintenant calculé depuis time_signature)
    /// Use steps_per_measure() method instead
    #[serde(default = "default_rhythm_steps")]
    pub rhythm_steps: usize,

    /// Nombre de pulses primaires (kicks/accents principaux)
    #[serde(default = "default_rhythm_pulses")]
    pub rhythm_pulses: usize,

    /// Rotation du pattern primaire (décalage de phase)
    #[serde(default)]
    pub rhythm_rotation: usize,

    /// Densité rythmique pour `PerfectBalance` (0.0-1.0)
    /// Contrôle la complexité du pattern de kicks
    #[serde(default = "default_density")]
    pub rhythm_density: f32,

    /// Tension rythmique (0.0-1.0)
    /// Contrôle les ghost notes et syncopation
    #[serde(default = "default_tension")]
    pub rhythm_tension: f32,

    /// Steps du séquenceur secondaire (polyrythme 4:3)
    #[serde(default = "default_secondary_steps")]
    pub rhythm_secondary_steps: usize,

    /// Pulses du séquenceur secondaire
    #[serde(default = "default_secondary_pulses")]
    pub rhythm_secondary_pulses: usize,

    /// Rotation du séquenceur secondaire
    #[serde(default)]
    pub rhythm_secondary_rotation: usize,

    /// Mode du Kick : true = Note fixe 36 (Drum Kit), false = Harmonisé (Synth Bass)
    /// - true: Idéal pour VST Drums, Samplers, Percussions
    /// - false: Idéal pour Synthwave, Basses mélodiques (Odin2)
    #[serde(default)]
    pub fixed_kick: bool,

    // ═══════════════════════════════════════════════════════════════════
    // HARMONIE
    // ═══════════════════════════════════════════════════════════════════
    /// Mode harmonique (Basic quadrants ou Driver avancé)
    #[serde(default)]
    pub harmony_mode: HarmonyMode,

    /// Stratégie harmonique explicite (pour Driver)
    #[serde(default)]
    pub harmony_strategy: HarmonyStrategy,

    /// Tension harmonique (0.0-1.0)
    /// Contrôle le niveau LCC (consonance → dissonance)
    /// et la sélection de stratégie si Auto
    #[serde(default = "default_tension")]
    pub harmony_tension: f32,

    /// Valence harmonique (-1.0 à 1.0)
    /// Contrôle la sélection majeur/mineur et les progressions
    #[serde(default)]
    pub harmony_valence: f32,

    /// Mesures par accord (1 = changement rapide, 2 = normal)
    #[serde(default = "default_measures_per_chord")]
    pub harmony_measures_per_chord: usize,

    /// Tonique globale (pitch class 0-11, 0 = C)
    #[serde(default)]
    pub key_root: u8,

    // ═══════════════════════════════════════════════════════════════════
    // MÉLODIE / VOICING
    // ═══════════════════════════════════════════════════════════════════
    /// Lissage mélodique (facteur de Hurst, 0.0-1.0)
    /// 0.0 = erratique, 1.0 = très lisse
    #[serde(default = "default_smoothness")]
    pub melody_smoothness: f32,

    /// Densité de voicing (0.0-1.0)
    /// Contrôle la probabilité de jouer des accords vs notes seules
    #[serde(default = "default_density")]
    pub voicing_density: f32,

    /// Tension de voicing (0.0-1.0)
    /// Affecte le nombre de voix et les extensions
    #[serde(default = "default_tension")]
    pub voicing_tension: f32,

    /// Octave de base pour la mélodie (3-6)
    #[serde(default = "default_octave")]
    pub melody_octave: i32,

    // ═══════════════════════════════════════════════════════════════════
    // MIXER
    // ═══════════════════════════════════════════════════════════════════
    /// Gain du lead/mélodie (0.0-1.0)
    #[serde(default = "default_gain_lead")]
    pub gain_lead: f32,

    /// Gain de la basse/kick (0.0-1.0)
    #[serde(default = "default_gain_bass")]
    pub gain_bass: f32,

    /// Gain du snare (0.0-1.0)
    #[serde(default = "default_gain_snare")]
    pub gain_snare: f32,

    /// Gain du hi-hat (0.0-1.0)
    #[serde(default = "default_gain_hat")]
    pub gain_hat: f32,

    /// Vélocité de base pour la basse (0-127)
    #[serde(default = "default_vel_bass")]
    pub vel_base_bass: u8,

    /// Vélocité de base pour le snare (0-127)
    #[serde(default = "default_vel_snare")]
    pub vel_base_snare: u8,

    // ═══════════════════════════════════════════════════════════════════
    // ROUTAGE & MUTING
    // ═══════════════════════════════════════════════════════════════════
    /// Routage des canaux MIDI (-1 = `FundSP`, >=0 = Oxisynth Bank ID)
    #[serde(default = "default_channel_routing")]
    pub channel_routing: Vec<i32>,

    /// Canaux mutés (true = silencieux)
    #[serde(default = "default_muted_channels")]
    pub muted_channels: Vec<bool>,

    // ═══════════════════════════════════════════════════════════════════
    // ENREGISTREMENT
    // ═══════════════════════════════════════════════════════════════════
    /// Enregistrer en WAV
    #[serde(default)]
    pub record_wav: bool,

    /// Enregistrer en MIDI
    #[serde(default)]
    pub record_midi: bool,

    /// Enregistrer en `MusicXML` (pour validation dans `MuseScore`)
    #[serde(default)]
    pub record_musicxml: bool,

    /// Enregistrer la "Ground Truth" (JSON)
    #[serde(default)]
    pub record_truth: bool,
}

// === Fonctions par défaut ===

const fn default_master_volume() -> f32 {
    1.0
}
const fn default_true() -> bool {
    true
}
const fn default_steps_per_quarter() -> usize {
    4  // 16th note resolution by default
}

const fn default_rhythm_steps() -> usize {
    16  // Deprecated, kept for backward compat
}
const fn default_rhythm_pulses() -> usize {
    4
}
const fn default_secondary_steps() -> usize {
    12
}
const fn default_secondary_pulses() -> usize {
    3
}
const fn default_density() -> f32 {
    0.5
}
const fn default_tension() -> f32 {
    0.3
}
const fn default_smoothness() -> f32 {
    0.7
}
const fn default_measures_per_chord() -> usize {
    2
}
const fn default_octave() -> i32 {
    4
}
const fn default_gain_lead() -> f32 {
    1.0
}
const fn default_gain_bass() -> f32 {
    0.6
}
const fn default_gain_snare() -> f32 {
    0.5
}
const fn default_gain_hat() -> f32 {
    0.4
}
const fn default_vel_bass() -> u8 {
    85
}
const fn default_vel_snare() -> u8 {
    70
}
fn default_channel_routing() -> Vec<i32> {
    vec![-1; 16]
}
fn default_muted_channels() -> Vec<bool> {
    vec![false; 16]
}

impl Default for MusicalParams {
    fn default() -> Self {
        Self {
            // Global
            bpm: 120.0,
            master_volume: default_master_volume(),

            // Modules
            enable_rhythm: true,
            enable_harmony: true,
            enable_melody: true,
            enable_voicing: false,

            // Time & Rythme
            time_signature: TimeSignature::default(),
            steps_per_quarter: default_steps_per_quarter(),
            rhythm_mode: RhythmMode::Euclidean,
            rhythm_steps: default_rhythm_steps(),  // Deprecated
            rhythm_pulses: default_rhythm_pulses(),
            rhythm_rotation: 0,
            rhythm_density: default_density(),
            rhythm_tension: default_tension(),
            rhythm_secondary_steps: default_secondary_steps(),
            rhythm_secondary_pulses: default_secondary_pulses(),
            rhythm_secondary_rotation: 0,
            fixed_kick: false,

            // Harmonie
            harmony_mode: HarmonyMode::Driver,
            harmony_strategy: HarmonyStrategy::Auto,
            harmony_tension: default_tension(),
            harmony_valence: 0.3,
            harmony_measures_per_chord: default_measures_per_chord(),
            key_root: 0, // C

            // Mélodie / Voicing
            melody_smoothness: default_smoothness(),
            voicing_density: default_density(),
            voicing_tension: default_tension(),
            melody_octave: default_octave(),

            // Mixer
            gain_lead: default_gain_lead(),
            gain_bass: default_gain_bass(),
            gain_snare: default_gain_snare(),
            gain_hat: default_gain_hat(),
            vel_base_bass: default_vel_bass(),
            vel_base_snare: default_vel_snare(),

            // Routage
            channel_routing: default_channel_routing(),
            muted_channels: default_muted_channels(),

            // Recording
            record_wav: false,
            record_midi: false,
            record_musicxml: false,
            record_truth: false,
        }
    }
}

impl MusicalParams {
    /// Calculate total steps per measure (derived property)
    /// This replaces the old rhythm_steps direct access
    #[must_use]
    pub fn steps_per_measure(&self) -> usize {
        self.time_signature.steps_per_measure(self.steps_per_quarter)
    }

    /// Backward compatibility: rhythm_steps getter
    /// DEPRECATED: Use steps_per_measure() instead
    #[deprecated(since = "0.2.0", note = "Use steps_per_measure() instead")]
    #[must_use]
    pub fn rhythm_steps(&self) -> usize {
        self.steps_per_measure()
    }

    /// Créer des paramètres pour un test/debug avec rythme désactivé
    #[must_use]
    pub fn melody_only() -> Self {
        Self {
            enable_rhythm: false,
            enable_harmony: true,
            enable_melody: true,
            voicing_density: 1.0, // Force 100% de notes
            ..Default::default()
        }
    }

    /// Créer des paramètres pour un test/debug de rythme seul
    #[must_use]
    pub fn rhythm_only() -> Self {
        Self {
            enable_rhythm: true,
            enable_harmony: false,
            enable_melody: false,
            ..Default::default()
        }
    }

    /// Créer des paramètres pour un tempo spécifique
    #[must_use]
    pub fn with_bpm(bpm: f32) -> Self {
        Self { bpm, ..Default::default() }
    }

    /// Builder pattern: set BPM
    #[must_use]
    pub const fn bpm(mut self, bpm: f32) -> Self {
        self.bpm = bpm;
        self
    }

    /// Builder pattern: set harmony mode
    #[must_use]
    pub const fn harmony_mode(mut self, mode: HarmonyMode) -> Self {
        self.harmony_mode = mode;
        self
    }

    /// Builder pattern: set rhythm mode
    #[must_use]
    pub const fn rhythm_mode(mut self, mode: RhythmMode) -> Self {
        self.rhythm_mode = mode;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params() {
        let params = MusicalParams::default();
        assert!((params.bpm - 120.0).abs() < f32::EPSILON);
        assert!(params.enable_rhythm);
        assert!(params.enable_harmony);
        assert!(params.enable_melody);
    }

    #[test]
    fn test_melody_only() {
        let params = MusicalParams::melody_only();
        assert!(!params.enable_rhythm);
        assert!(params.enable_melody);
        assert!((params.voicing_density - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_pattern() {
        let params = MusicalParams::default().bpm(140.0).harmony_mode(HarmonyMode::Basic);

        assert!((params.bpm - 140.0).abs() < f32::EPSILON);
        assert_eq!(params.harmony_mode, HarmonyMode::Basic);
    }

    // ═══════════════════════════════════════════════════════════════════
    // TIME SIGNATURE TESTS
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_time_signature_steps_calculation() {
        // 4/4 tests
        let ts_4_4 = TimeSignature::FOUR_FOUR;
        assert_eq!(ts_4_4.steps_per_measure(4), 16);  // 4 beats * 4 sixteenths
        assert_eq!(ts_4_4.steps_per_measure(12), 48); // triplet subdivision
        assert_eq!(ts_4_4.steps_per_measure(24), 96); // 32nd notes

        // 3/4 tests
        let ts_3_4 = TimeSignature::THREE_FOUR;
        assert_eq!(ts_3_4.steps_per_measure(4), 12);  // 3 beats * 4 sixteenths

        // 6/8 tests (compound meter)
        let ts_6_8 = TimeSignature::SIX_EIGHT;
        assert_eq!(ts_6_8.steps_per_measure(4), 12);  // 6 eighths = 3 quarters

        // 5/4 tests
        let ts_5_4 = TimeSignature::FIVE_FOUR;
        assert_eq!(ts_5_4.steps_per_measure(4), 20);  // 5 beats * 4 sixteenths

        // 7/8 tests
        let ts_7_8 = TimeSignature::SEVEN_EIGHT;
        assert_eq!(ts_7_8.steps_per_measure(4), 16);  // 7 eighths ≈ 3.5 quarters → 4
    }

    #[test]
    fn test_time_signature_validation() {
        // Valid signatures
        assert!(TimeSignature::new(4, 4).is_valid());
        assert!(TimeSignature::new(3, 4).is_valid());
        assert!(TimeSignature::new(6, 8).is_valid());
        assert!(TimeSignature::new(5, 4).is_valid());
        assert!(TimeSignature::new(7, 8).is_valid());
        assert!(TimeSignature::new(2, 2).is_valid());  // cut time
        assert!(TimeSignature::new(9, 16).is_valid()); // compound irregular

        // Invalid signatures
        assert!(!TimeSignature::new(0, 4).is_valid());  // zero numerator
        assert!(!TimeSignature::new(4, 3).is_valid());  // invalid denominator
        assert!(!TimeSignature::new(4, 7).is_valid());  // invalid denominator
    }

    #[test]
    fn test_time_signature_display() {
        assert_eq!(format!("{}", TimeSignature::FOUR_FOUR), "4/4");
        assert_eq!(format!("{}", TimeSignature::THREE_FOUR), "3/4");
        assert_eq!(format!("{}", TimeSignature::FIVE_FOUR), "5/4");
        assert_eq!(format!("{}", TimeSignature::SIX_EIGHT), "6/8");
    }

    #[test]
    fn test_time_signature_default() {
        let ts = TimeSignature::default();
        assert_eq!(ts, TimeSignature::FOUR_FOUR);
        assert_eq!(ts.numerator, 4);
        assert_eq!(ts.denominator, 4);
    }

    #[test]
    fn test_time_signature_equality() {
        let ts1 = TimeSignature::new(4, 4);
        let ts2 = TimeSignature::FOUR_FOUR;
        assert_eq!(ts1, ts2);

        let ts3 = TimeSignature::new(3, 4);
        assert_ne!(ts1, ts3);
    }

    #[test]
    fn test_backward_compatible_rhythm_steps() {
        let mut params = MusicalParams::default();
        params.time_signature = TimeSignature::FOUR_FOUR;
        params.steps_per_quarter = 4;

        // New method: steps_per_measure()
        assert_eq!(params.steps_per_measure(), 16);

        // Legacy method should still work (though deprecated)
        #[allow(deprecated)]
        {
            assert_eq!(params.rhythm_steps(), 16);
        }

        // Test with different time signatures
        params.time_signature = TimeSignature::THREE_FOUR;
        assert_eq!(params.steps_per_measure(), 12);

        params.time_signature = TimeSignature::FIVE_FOUR;
        assert_eq!(params.steps_per_measure(), 20);

        // Test with different subdivisions
        params.time_signature = TimeSignature::FOUR_FOUR;
        params.steps_per_quarter = 12;  // triplets
        assert_eq!(params.steps_per_measure(), 48);
    }

    #[test]
    fn test_musical_params_with_time_signature() {
        let params = MusicalParams::default();

        // Verify defaults
        assert_eq!(params.time_signature, TimeSignature::FOUR_FOUR);
        assert_eq!(params.steps_per_quarter, 4);
        assert_eq!(params.steps_per_measure(), 16);

        // Verify rhythm_steps field is also set correctly for backward compat
        assert_eq!(params.rhythm_steps, 16);
    }
}

// =========================================================================================
//  Structures moved from Engine (Data Contracts)
// =========================================================================================

#[derive(Clone, Debug)]
pub struct VisualizationEvent {
    pub note_midi: u8,
    pub instrument: u8, // 0=Bass, 1=Lead, 2=Snare, 3=Hat
    pub step: usize,
    pub duration_samples: usize,
}

/// Enhanced visualization event with musical position structure
/// Used by get_lookahead_truth_v2 for sight reading and structured notation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualizationEventV2 {
    pub note_midi: u8,
    pub instrument: u8, // 0=Bass, 1=Lead, 2=Snare, 3=Hat
    pub velocity: u8,
    pub duration_steps: usize,
    pub position: MusicalPosition,
}

#[derive(Clone, Debug)]
pub struct HarmonyState {
    pub current_chord_index: usize,
    pub chord_root_offset: i32,
    pub chord_is_minor: bool,
    pub chord_name: ArrayString<64>,
    pub measure_number: usize,
    pub cycle_number: usize,
    pub current_step: usize,
    pub progression_name: ArrayString<64>,
    pub progression_length: usize,
    pub harmony_mode: HarmonyMode,
    pub primary_steps: usize,
    pub primary_pulses: usize,
    pub secondary_steps: usize,
    pub secondary_pulses: usize,
    pub primary_rotation: usize,
    pub secondary_rotation: usize,
    pub primary_pattern: [bool; 192],
    pub secondary_pattern: [bool; 192],
}

impl Default for HarmonyState {
    fn default() -> Self {
        Self {
            current_chord_index: 0,
            chord_root_offset: 0,
            chord_is_minor: false,
            chord_name: ArrayString::from("I").unwrap_or_default(),
            measure_number: 1,
            cycle_number: 1,
            current_step: 0,
            progression_name: ArrayString::from("Folk Peaceful (I-IV-I-V)").unwrap_or_default(),
            progression_length: 4,
            harmony_mode: HarmonyMode::Driver,
            primary_steps: 16,
            primary_pulses: 4,
            secondary_steps: 12,
            secondary_pulses: 3,
            primary_rotation: 0,
            secondary_rotation: 0,
            primary_pattern: [false; 192],
            secondary_pattern: [false; 192],
        }
    }
}

// Helpers for Serde defaults
const fn default_poly_steps() -> usize {
    48
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineParams {
    pub arousal: f32,
    pub valence: f32,
    pub density: f32,
    pub tension: f32,
    pub smoothness: f32,
    #[serde(default)]
    pub algorithm: RhythmMode,
    #[serde(default)]
    pub channel_routing: Vec<i32>,
    #[serde(default)]
    pub muted_channels: Vec<bool>,
    #[serde(default)]
    pub harmony_mode: HarmonyMode,

    // Recording Control
    #[serde(default)]
    pub record_wav: bool,
    #[serde(default)]
    pub record_midi: bool,
    #[serde(default)]
    pub record_musicxml: bool,
    #[serde(default)]
    pub record_truth: bool,

    // Synthesis Morphing Control
    #[serde(default = "default_true")]
    pub enable_synthesis_morphing: bool,

    // Mixer Gains (0.0 - 1.0)
    #[serde(default = "default_gain_lead")]
    pub gain_lead: f32,
    #[serde(default = "default_gain_bass")]
    pub gain_bass: f32,
    #[serde(default = "default_gain_snare")]
    pub gain_snare: f32,
    #[serde(default = "default_gain_hat")]
    pub gain_hat: f32,

    // Velocity Base (MIDI 0-127)
    #[serde(default = "default_vel_bass")]
    pub vel_base_bass: u8,
    #[serde(default = "default_vel_snare")]
    pub vel_base_snare: u8,

    // Polyrythm Steps (48, 96, 192...)
    #[serde(default = "default_poly_steps")]
    pub poly_steps: usize,

    // Mode Drum Kit
    #[serde(default)]
    pub fixed_kick: bool,
}

impl Default for EngineParams {
    fn default() -> Self {
        Self {
            arousal: 0.5,
            valence: 0.3,
            density: 0.2,
            tension: 0.4,
            smoothness: 0.7,
            algorithm: RhythmMode::Euclidean,
            channel_routing: vec![-1; 16],
            muted_channels: vec![false; 16],
            harmony_mode: HarmonyMode::Driver,
            record_wav: false,
            record_midi: false,
            record_musicxml: false,
            record_truth: false,
            enable_synthesis_morphing: true,
            gain_lead: default_gain_lead(),
            gain_bass: default_gain_bass(),
            gain_snare: default_gain_snare(),
            gain_hat: default_gain_hat(),
            vel_base_bass: default_vel_bass(),
            vel_base_snare: default_vel_snare(),
            poly_steps: default_poly_steps(),
            fixed_kick: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub bpm: f32,
    pub key: String,
    pub scale: String,
    pub pulses: usize,
    pub steps: usize,
}

#[derive(Clone, Debug, Default)]
pub struct CurrentState {
    pub bpm: f32,
    pub density: f32,
    pub tension: f32,
    pub smoothness: f32,
    pub valence: f32,
    pub arousal: f32,
}

impl EngineParams {
    #[must_use]
    pub fn compute_bpm(&self) -> f32 {
        self.arousal.mul_add(110.0, 70.0)
    }
}
