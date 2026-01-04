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

use serde::{Serialize, Deserialize};
use crate::sequencer::RhythmMode;
use crate::harmony::HarmonyMode;

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

    /// Activer le module mélodique/voicing
    #[serde(default = "default_true")]
    pub enable_melody: bool,

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

    /// Enregistrer en ABC notation
    #[serde(default)]
    pub record_abc: bool,
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
            record_abc: false,
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
