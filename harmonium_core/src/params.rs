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
use serde::{Serialize, Deserialize};
use crate::sequencer::RhythmMode;
use crate::harmony::HarmonyMode;

/// État du mode de contrôle (émotion vs direct)
/// Partagé entre VST et standalone builds
#[derive(Clone, Debug)]
pub struct ControlMode {
    /// true = mode émotion (EngineParams → EmotionMapper → MusicalParams)
    /// false = mode direct (MusicalParams directement)
    pub use_emotion_mode: bool,
    /// Paramètres musicaux directs (utilisés quand use_emotion_mode = false)
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
    /// and sync_params_to_engine should NOT overwrite target_state
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
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
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
    // RYTHME
    // ═══════════════════════════════════════════════════════════════════

    /// Mode rythmique (Euclidean classique ou PerfectBalance polyrythmique)
    #[serde(default)]
    pub rhythm_mode: RhythmMode,

    /// Nombre de steps (16 pour Euclidean, 48/96/192 pour PerfectBalance)
    #[serde(default = "default_rhythm_steps")]
    pub rhythm_steps: usize,

    /// Nombre de pulses primaires (kicks/accents principaux)
    #[serde(default = "default_rhythm_pulses")]
    pub rhythm_pulses: usize,

    /// Rotation du pattern primaire (décalage de phase)
    #[serde(default)]
    pub rhythm_rotation: usize,

    /// Densité rythmique pour PerfectBalance (0.0-1.0)
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

    /// Routage des canaux MIDI (-1 = FundSP, >=0 = Oxisynth Bank ID)
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

    /// Enregistrer en MusicXML (pour validation dans MuseScore)
    #[serde(default)]
    pub record_musicxml: bool,
}

// === Fonctions par défaut ===

fn default_master_volume() -> f32 { 1.0 }
fn default_true() -> bool { true }
fn default_rhythm_steps() -> usize { 16 }
fn default_rhythm_pulses() -> usize { 4 }
fn default_secondary_steps() -> usize { 12 }
fn default_secondary_pulses() -> usize { 3 }
fn default_density() -> f32 { 0.5 }
fn default_tension() -> f32 { 0.3 }
fn default_smoothness() -> f32 { 0.7 }
fn default_measures_per_chord() -> usize { 2 }
fn default_octave() -> i32 { 4 }
fn default_gain_lead() -> f32 { 1.0 }
fn default_gain_bass() -> f32 { 0.6 }
fn default_gain_snare() -> f32 { 0.5 }
fn default_gain_hat() -> f32 { 0.4 }
fn default_vel_bass() -> u8 { 85 }
fn default_vel_snare() -> u8 { 70 }
fn default_channel_routing() -> Vec<i32> { vec![-1; 16] }
fn default_muted_channels() -> Vec<bool> { vec![false; 16] }

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

            // Rythme
            rhythm_mode: RhythmMode::Euclidean,
            rhythm_steps: default_rhythm_steps(),
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
        }
    }
}

impl MusicalParams {
    /// Créer des paramètres pour un test/debug avec rythme désactivé
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
    pub fn rhythm_only() -> Self {
        Self {
            enable_rhythm: true,
            enable_harmony: false,
            enable_melody: false,
            ..Default::default()
        }
    }

    /// Créer des paramètres pour un tempo spécifique
    pub fn with_bpm(bpm: f32) -> Self {
        Self {
            bpm,
            ..Default::default()
        }
    }

    /// Builder pattern: set BPM
    pub fn bpm(mut self, bpm: f32) -> Self {
        self.bpm = bpm;
        self
    }

    /// Builder pattern: set harmony mode
    pub fn harmony_mode(mut self, mode: HarmonyMode) -> Self {
        self.harmony_mode = mode;
        self
    }

    /// Builder pattern: set rhythm mode
    pub fn rhythm_mode(mut self, mode: RhythmMode) -> Self {
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
        assert_eq!(params.bpm, 120.0);
        assert!(params.enable_rhythm);
        assert!(params.enable_harmony);
        assert!(params.enable_melody);
    }

    #[test]
    fn test_melody_only() {
        let params = MusicalParams::melody_only();
        assert!(!params.enable_rhythm);
        assert!(params.enable_melody);
        assert_eq!(params.voicing_density, 1.0);
    }

    #[test]
    fn test_builder_pattern() {
        let params = MusicalParams::default()
            .bpm(140.0)
            .harmony_mode(HarmonyMode::Basic);

        assert_eq!(params.bpm, 140.0);
        assert_eq!(params.harmony_mode, HarmonyMode::Basic);
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
        HarmonyState {
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
fn default_poly_steps() -> usize { 48 }

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
        EngineParams {
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
    pub fn compute_bpm(&self) -> f32 {
        70.0 + (self.arousal * 110.0)
    }
}
