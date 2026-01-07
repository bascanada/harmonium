//! Tension and Density Modulation
//!
//! Applies independent modulation on top of morphed presets.
//! - Tension → dissonance/detune/distortion
//! - Density → envelope timing adjustments

use super::types::SynthPreset;

/// Apply tension and density as independent modulators ON TOP of morphed preset
///
/// # Parameters
/// - `base`: The morphed preset from bilinear interpolation
/// - `tension`: 0.0 - 1.0, controls dissonance/distortion
/// - `density`: 0.0 - 1.0, controls rhythmic density/attack speed
///
/// # Tension Effects
/// - Increases oscillator detune (more dissonance)
/// - Increases filter drive (more distortion)
/// - Increases filter resonance (harsher sound)
/// - Adds noise (grit/texture)
///
/// # Density Effects
/// - Reduces attack time (faster note onsets for dense passages)
/// - Reduces release time (shorter notes for clarity)
/// - Increases chorus depth slightly (more texture)
pub fn apply_tension_density_modulation(
    base: &SynthPreset,
    tension: f32,
    density: f32,
) -> SynthPreset {
    let mut result = base.clone();

    // === TENSION → Dissonance/Distortion ===

    // 1. Increase oscillator detune (more dissonance)
    result.osc.detune = (base.osc.detune + tension * 0.3).min(1.0);

    // 2. Increase filter drive (more distortion)
    result.filter.drive = base.filter.drive + tension * 1.5; // Up to +1.5x drive

    // 3. Increase filter resonance (harsher sound)
    result.filter.resonance = (base.filter.resonance + tension * 0.3).min(0.95);

    // 4. Add noise (grit/texture)
    result.osc.noise_level = (base.osc.noise_level + tension * 0.2).min(1.0);

    // === DENSITY → Envelope Timing ===
    // Denser = faster attacks for more notes, shorter releases for clarity

    // 1. Attack modulation (denser = faster attacks for more notes)
    let attack_mod = 1.0 - (density * 0.3); // Reduce attack by up to 30%
    result.envelopes.amp.attack = (base.envelopes.amp.attack * attack_mod).max(0.001);
    result.envelopes.filter.attack = (base.envelopes.filter.attack * attack_mod).max(0.001);

    // 2. Release modulation (denser = shorter releases for clarity)
    let release_mod = 1.0 - (density * 0.4); // Reduce release by up to 40%
    result.envelopes.amp.release = (base.envelopes.amp.release * release_mod).max(0.001);
    result.envelopes.filter.release = (base.envelopes.filter.release * release_mod).max(0.001);

    // 3. Chorus depth (denser = slightly more chorus for texture)
    result.effects.chorus.depth = (base.effects.chorus.depth + density * 0.2).min(1.0);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesis::types::*;

    fn create_test_preset() -> SynthPreset {
        SynthPreset {
            name: "Test".to_string(),
            osc: OscillatorParams {
                waveform_mix: 0.5,
                detune: 0.2,
                stereo_width: 0.5,
                pitch_mod: 0.1,
                sub_level: 0.3,
                noise_level: 0.0,
            },
            filter: FilterParams {
                cutoff: 1000.0,
                resonance: 0.3,
                env_amount: 0.5,
                drive: 1.0,
                filter_type: 0,
            },
            envelopes: EnvelopeParams {
                amp: AdsrValues {
                    attack: 0.01,
                    decay: 0.1,
                    sustain: 0.7,
                    release: 0.2,
                },
                filter: AdsrValues {
                    attack: 0.015,
                    decay: 0.15,
                    sustain: 0.5,
                    release: 0.25,
                },
            },
            effects: EffectsParams {
                delay: DelayParams {
                    time: 0.25,
                    feedback: 0.2,
                    mix: 0.15,
                },
                chorus: ChorusParams {
                    lfo_freq: 0.5,
                    depth: 0.3,
                    mix: 0.15,
                },
                reverb: ReverbParams {
                    room_size: 0.5,
                    damping: 0.5,
                    mix: 0.2,
                },
            },
            output: OutputParams {
                gain: 1.0,
                pan: 0.0,
            },
        }
    }

    #[test]
    fn test_tension_increases_detune() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.5, 0.0);
        assert!(
            modulated.osc.detune > base.osc.detune,
            "Tension should increase detune"
        );

        let modulated_max = apply_tension_density_modulation(&base, 1.0, 0.0);
        assert!(
            modulated_max.osc.detune > modulated.osc.detune,
            "Higher tension should increase detune more"
        );
    }

    #[test]
    fn test_tension_increases_drive() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.5, 0.0);
        assert!(
            modulated.filter.drive > base.filter.drive,
            "Tension should increase drive"
        );

        let modulated_max = apply_tension_density_modulation(&base, 1.0, 0.0);
        assert!(
            modulated_max.filter.drive > modulated.filter.drive,
            "Higher tension should increase drive more"
        );
    }

    #[test]
    fn test_tension_increases_resonance() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.5, 0.0);
        assert!(
            modulated.filter.resonance > base.filter.resonance,
            "Tension should increase resonance"
        );
    }

    #[test]
    fn test_tension_adds_noise() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.5, 0.0);
        assert!(
            modulated.osc.noise_level > base.osc.noise_level,
            "Tension should add noise"
        );
    }

    #[test]
    fn test_density_reduces_attack() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.0, 0.5);
        assert!(
            modulated.envelopes.amp.attack < base.envelopes.amp.attack,
            "Density should reduce attack"
        );
        assert!(
            modulated.envelopes.filter.attack < base.envelopes.filter.attack,
            "Density should reduce filter attack"
        );
    }

    #[test]
    fn test_density_reduces_release() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.0, 0.5);
        assert!(
            modulated.envelopes.amp.release < base.envelopes.amp.release,
            "Density should reduce release"
        );
        assert!(
            modulated.envelopes.filter.release < base.envelopes.filter.release,
            "Density should reduce filter release"
        );
    }

    #[test]
    fn test_density_increases_chorus_depth() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.0, 0.5);
        assert!(
            modulated.effects.chorus.depth > base.effects.chorus.depth,
            "Density should increase chorus depth"
        );
    }

    #[test]
    fn test_no_modulation_returns_identical() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.0, 0.0);

        // With zero modulation, values should be identical
        assert_eq!(modulated.osc.detune, base.osc.detune);
        assert_eq!(modulated.filter.drive, base.filter.drive);
        assert_eq!(modulated.filter.resonance, base.filter.resonance);
        assert_eq!(modulated.osc.noise_level, base.osc.noise_level);
    }

    #[test]
    fn test_combined_modulation() {
        let base = create_test_preset();

        let modulated = apply_tension_density_modulation(&base, 0.5, 0.5);

        // Both tension and density effects should be present
        assert!(modulated.osc.detune > base.osc.detune);
        assert!(modulated.filter.drive > base.filter.drive);
        assert!(modulated.envelopes.amp.attack < base.envelopes.amp.attack);
        assert!(modulated.envelopes.amp.release < base.envelopes.amp.release);
    }

    #[test]
    fn test_tension_clamps_at_max() {
        let mut base = create_test_preset();
        base.osc.detune = 0.95;
        base.filter.resonance = 0.9;

        let modulated = apply_tension_density_modulation(&base, 1.0, 0.0);

        // Values should clamp at maximum
        assert!(modulated.osc.detune <= 1.0);
        assert!(modulated.filter.resonance <= 0.95);
    }

    #[test]
    fn test_density_respects_minimum_envelope_times() {
        let mut base = create_test_preset();
        base.envelopes.amp.attack = 0.002; // Very short already

        let modulated = apply_tension_density_modulation(&base, 0.0, 1.0);

        // Attack should not go below 0.001 (minimum)
        assert!(modulated.envelopes.amp.attack >= 0.001);
        assert!(modulated.envelopes.filter.attack >= 0.001);
    }
}
