//! Emotional Morphing Engine
//!
//! Implements bilinear interpolation for morphing synthesis parameters
//! across Russell's Circumplex Model quadrants.

use super::presets::EmotionalPresetBank;
use super::types::*;

/// Bilinear interpolation engine for emotional morphing
pub struct EmotionalMorpher {
    /// The 4 corner presets
    preset_bank: EmotionalPresetBank,

    /// Cached morphed results (per instrument)
    cached_bass: SynthPreset,
    cached_lead: SynthPreset,
    cached_snare: SynthPreset,
    cached_hat: SynthPreset,
    cached_poly: SynthPreset,

    /// Last morph position (for change detection)
    last_valence: f32,
    last_arousal: f32,
}

/// Morphed presets for all instruments
#[derive(Debug, Clone)]
pub struct MorphedPresets {
    pub bass: SynthPreset,
    pub lead: SynthPreset,
    pub snare: SynthPreset,
    pub hat: SynthPreset,
    pub poly: SynthPreset,
}

/// Weights for 4 quadrants (sum = 1.0)
#[derive(Debug, Clone, Copy)]
pub struct QuadWeights {
    pub calm: f32,     // Q4: Valence +, Arousal -
    pub joy: f32,      // Q1: Valence +, Arousal +
    pub sadness: f32,  // Q3: Valence -, Arousal -
    pub anger: f32,    // Q2: Valence -, Arousal +
}

impl EmotionalMorpher {
    /// Create a new morpher with the given preset bank
    pub fn new(preset_bank: EmotionalPresetBank) -> Self {
        // Initialize with calm presets as default
        Self {
            cached_bass: preset_bank.calm.bass.clone(),
            cached_lead: preset_bank.calm.lead.clone(),
            cached_snare: preset_bank.calm.snare.clone(),
            cached_hat: preset_bank.calm.hat.clone(),
            cached_poly: preset_bank.calm.poly.clone(),
            preset_bank,
            last_valence: 0.0,
            last_arousal: 0.0,
        }
    }

    /// Perform bilinear interpolation for given emotional position
    /// Returns morphed presets for all instruments
    pub fn morph(&mut self, valence: f32, arousal: f32) -> MorphedPresets {
        // Only recalculate if position changed significantly
        if (valence - self.last_valence).abs() < 0.01
            && (arousal - self.last_arousal).abs() < 0.01
        {
            return self.get_cached_presets();
        }

        self.last_valence = valence;
        self.last_arousal = arousal;

        // Normalize inputs to [0, 1]
        let v = ((valence + 1.0) * 0.5).clamp(0.0, 1.0); // -1..1 → 0..1
        let a = arousal.clamp(0.0, 1.0);

        // Calculate corner weights using bilinear interpolation
        let weights = self.calculate_weights(v, a);

        // Morph each instrument
        self.cached_bass = self.morph_instrument_presets(
            &self.preset_bank.calm.bass,      // Q4 (v=1, a=0)
            &self.preset_bank.joy.bass,       // Q1 (v=1, a=1)
            &self.preset_bank.sadness.bass,   // Q3 (v=0, a=0)
            &self.preset_bank.anger.bass,     // Q2 (v=0, a=1)
            &weights,
        );

        self.cached_lead = self.morph_instrument_presets(
            &self.preset_bank.calm.lead,
            &self.preset_bank.joy.lead,
            &self.preset_bank.sadness.lead,
            &self.preset_bank.anger.lead,
            &weights,
        );

        self.cached_snare = self.morph_instrument_presets(
            &self.preset_bank.calm.snare,
            &self.preset_bank.joy.snare,
            &self.preset_bank.sadness.snare,
            &self.preset_bank.anger.snare,
            &weights,
        );

        self.cached_hat = self.morph_instrument_presets(
            &self.preset_bank.calm.hat,
            &self.preset_bank.joy.hat,
            &self.preset_bank.sadness.hat,
            &self.preset_bank.anger.hat,
            &weights,
        );

        self.cached_poly = self.morph_instrument_presets(
            &self.preset_bank.calm.poly,
            &self.preset_bank.joy.poly,
            &self.preset_bank.sadness.poly,
            &self.preset_bank.anger.poly,
            &weights,
        );

        self.get_cached_presets()
    }

    /// Calculate bilinear interpolation weights for 4 corners
    /// Returns (w_calm, w_joy, w_sadness, w_anger) where sum = 1.0
    ///
    /// Bilinear interpolation formula:
    /// f(x,y) = f(0,0)(1-x)(1-y) + f(1,0)x(1-y) + f(0,1)(1-x)y + f(1,1)xy
    ///
    /// Where:
    ///   x = valence (0..1, 0=negative, 1=positive)
    ///   y = arousal (0..1, 0=low, 1=high)
    ///
    /// Corners:
    ///   (0,0) = Sadness  (v-, a-)
    ///   (1,0) = Calm     (v+, a-)
    ///   (0,1) = Anger    (v-, a+)
    ///   (1,1) = Joy      (v+, a+)
    fn calculate_weights(&self, v: f32, a: f32) -> QuadWeights {
        QuadWeights {
            calm: v * (1.0 - a),               // Q4: (v+, a-)
            joy: v * a,                         // Q1: (v+, a+)
            sadness: (1.0 - v) * (1.0 - a),    // Q3: (v-, a-)
            anger: (1.0 - v) * a,               // Q2: (v-, a+)
        }
    }

    /// Morph a single instrument's preset using bilinear interpolation
    fn morph_instrument_presets(
        &self,
        calm: &SynthPreset,
        joy: &SynthPreset,
        sadness: &SynthPreset,
        anger: &SynthPreset,
        weights: &QuadWeights,
    ) -> SynthPreset {
        SynthPreset {
            name: format!(
                "Morphed (V:{:.2} A:{:.2})",
                self.last_valence, self.last_arousal
            ),
            osc: self.morph_oscillator(&calm.osc, &joy.osc, &sadness.osc, &anger.osc, weights),
            filter: self.morph_filter(
                &calm.filter,
                &joy.filter,
                &sadness.filter,
                &anger.filter,
                weights,
            ),
            envelopes: self.morph_envelopes(
                &calm.envelopes,
                &joy.envelopes,
                &sadness.envelopes,
                &anger.envelopes,
                weights,
            ),
            effects: self.morph_effects(
                &calm.effects,
                &joy.effects,
                &sadness.effects,
                &anger.effects,
                weights,
            ),
            output: self.morph_output(
                &calm.output,
                &joy.output,
                &sadness.output,
                &anger.output,
                weights,
            ),
        }
    }

    fn morph_oscillator(
        &self,
        calm: &OscillatorParams,
        joy: &OscillatorParams,
        sadness: &OscillatorParams,
        anger: &OscillatorParams,
        w: &QuadWeights,
    ) -> OscillatorParams {
        OscillatorParams {
            waveform_mix: self.lerp4(
                calm.waveform_mix,
                joy.waveform_mix,
                sadness.waveform_mix,
                anger.waveform_mix,
                w,
            ),
            detune: self.lerp4(calm.detune, joy.detune, sadness.detune, anger.detune, w),
            stereo_width: self.lerp4(
                calm.stereo_width,
                joy.stereo_width,
                sadness.stereo_width,
                anger.stereo_width,
                w,
            ),
            pitch_mod: self.lerp4(
                calm.pitch_mod,
                joy.pitch_mod,
                sadness.pitch_mod,
                anger.pitch_mod,
                w,
            ),
            sub_level: self.lerp4(
                calm.sub_level,
                joy.sub_level,
                sadness.sub_level,
                anger.sub_level,
                w,
            ),
            noise_level: self.lerp4(
                calm.noise_level,
                joy.noise_level,
                sadness.noise_level,
                anger.noise_level,
                w,
            ),
        }
    }

    fn morph_filter(
        &self,
        calm: &FilterParams,
        joy: &FilterParams,
        sadness: &FilterParams,
        anger: &FilterParams,
        w: &QuadWeights,
    ) -> FilterParams {
        FilterParams {
            cutoff: self.lerp4(calm.cutoff, joy.cutoff, sadness.cutoff, anger.cutoff, w),
            resonance: self.lerp4(
                calm.resonance,
                joy.resonance,
                sadness.resonance,
                anger.resonance,
                w,
            ),
            env_amount: self.lerp4(
                calm.env_amount,
                joy.env_amount,
                sadness.env_amount,
                anger.env_amount,
                w,
            ),
            drive: self.lerp4(calm.drive, joy.drive, sadness.drive, anger.drive, w),
            // For filter_type (discrete), use nearest neighbor
            filter_type: self.discrete4(
                calm.filter_type,
                joy.filter_type,
                sadness.filter_type,
                anger.filter_type,
                w,
            ),
        }
    }

    fn morph_envelopes(
        &self,
        calm: &EnvelopeParams,
        joy: &EnvelopeParams,
        sadness: &EnvelopeParams,
        anger: &EnvelopeParams,
        w: &QuadWeights,
    ) -> EnvelopeParams {
        EnvelopeParams {
            amp: self.morph_adsr(&calm.amp, &joy.amp, &sadness.amp, &anger.amp, w),
            filter: self.morph_adsr(&calm.filter, &joy.filter, &sadness.filter, &anger.filter, w),
        }
    }

    fn morph_adsr(
        &self,
        calm: &AdsrValues,
        joy: &AdsrValues,
        sadness: &AdsrValues,
        anger: &AdsrValues,
        w: &QuadWeights,
    ) -> AdsrValues {
        AdsrValues {
            attack: self.lerp4(calm.attack, joy.attack, sadness.attack, anger.attack, w),
            decay: self.lerp4(calm.decay, joy.decay, sadness.decay, anger.decay, w),
            sustain: self.lerp4(
                calm.sustain,
                joy.sustain,
                sadness.sustain,
                anger.sustain,
                w,
            ),
            release: self.lerp4(
                calm.release,
                joy.release,
                sadness.release,
                anger.release,
                w,
            ),
        }
    }

    fn morph_effects(
        &self,
        calm: &EffectsParams,
        joy: &EffectsParams,
        sadness: &EffectsParams,
        anger: &EffectsParams,
        w: &QuadWeights,
    ) -> EffectsParams {
        EffectsParams {
            delay: self.morph_delay(&calm.delay, &joy.delay, &sadness.delay, &anger.delay, w),
            chorus: self.morph_chorus(
                &calm.chorus,
                &joy.chorus,
                &sadness.chorus,
                &anger.chorus,
                w,
            ),
            reverb: self.morph_reverb(
                &calm.reverb,
                &joy.reverb,
                &sadness.reverb,
                &anger.reverb,
                w,
            ),
        }
    }

    fn morph_delay(
        &self,
        calm: &DelayParams,
        joy: &DelayParams,
        sadness: &DelayParams,
        anger: &DelayParams,
        w: &QuadWeights,
    ) -> DelayParams {
        DelayParams {
            time: self.lerp4(calm.time, joy.time, sadness.time, anger.time, w),
            feedback: self.lerp4(
                calm.feedback,
                joy.feedback,
                sadness.feedback,
                anger.feedback,
                w,
            ),
            mix: self.lerp4(calm.mix, joy.mix, sadness.mix, anger.mix, w),
        }
    }

    fn morph_chorus(
        &self,
        calm: &ChorusParams,
        joy: &ChorusParams,
        sadness: &ChorusParams,
        anger: &ChorusParams,
        w: &QuadWeights,
    ) -> ChorusParams {
        ChorusParams {
            lfo_freq: self.lerp4(
                calm.lfo_freq,
                joy.lfo_freq,
                sadness.lfo_freq,
                anger.lfo_freq,
                w,
            ),
            depth: self.lerp4(calm.depth, joy.depth, sadness.depth, anger.depth, w),
            mix: self.lerp4(calm.mix, joy.mix, sadness.mix, anger.mix, w),
        }
    }

    fn morph_reverb(
        &self,
        calm: &ReverbParams,
        joy: &ReverbParams,
        sadness: &ReverbParams,
        anger: &ReverbParams,
        w: &QuadWeights,
    ) -> ReverbParams {
        ReverbParams {
            room_size: self.lerp4(
                calm.room_size,
                joy.room_size,
                sadness.room_size,
                anger.room_size,
                w,
            ),
            damping: self.lerp4(
                calm.damping,
                joy.damping,
                sadness.damping,
                anger.damping,
                w,
            ),
            mix: self.lerp4(calm.mix, joy.mix, sadness.mix, anger.mix, w),
        }
    }

    fn morph_output(
        &self,
        calm: &OutputParams,
        joy: &OutputParams,
        sadness: &OutputParams,
        anger: &OutputParams,
        w: &QuadWeights,
    ) -> OutputParams {
        OutputParams {
            gain: self.lerp4(calm.gain, joy.gain, sadness.gain, anger.gain, w),
            pan: self.lerp4(calm.pan, joy.pan, sadness.pan, anger.pan, w),
        }
    }

    /// 4-way linear interpolation (bilinear)
    fn lerp4(&self, calm: f32, joy: f32, sadness: f32, anger: f32, w: &QuadWeights) -> f32 {
        calm * w.calm + joy * w.joy + sadness * w.sadness + anger * w.anger
    }

    /// Discrete selection (nearest neighbor) for non-interpolatable values
    fn discrete4(&self, calm: u8, joy: u8, sadness: u8, anger: u8, w: &QuadWeights) -> u8 {
        // Pick the corner with highest weight
        let max_weight = w.calm.max(w.joy).max(w.sadness).max(w.anger);
        if (w.calm - max_weight).abs() < 0.001 {
            calm
        } else if (w.joy - max_weight).abs() < 0.001 {
            joy
        } else if (w.sadness - max_weight).abs() < 0.001 {
            sadness
        } else {
            anger
        }
    }

    fn get_cached_presets(&self) -> MorphedPresets {
        MorphedPresets {
            bass: self.cached_bass.clone(),
            lead: self.cached_lead.clone(),
            snare: self.cached_snare.clone(),
            hat: self.cached_hat.clone(),
            poly: self.cached_poly.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilinear_weights_sum_to_one() {
        let bank = EmotionalPresetBank::default_presets();
        let morpher = EmotionalMorpher::new(bank);

        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for a in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let weights = morpher.calculate_weights(v, a);
                let sum = weights.calm + weights.joy + weights.sadness + weights.anger;
                assert!(
                    (sum - 1.0).abs() < 0.0001,
                    "Weights don't sum to 1.0 at v={}, a={}: sum={}",
                    v,
                    a,
                    sum
                );
            }
        }
    }

    #[test]
    fn test_corner_positions() {
        let bank = EmotionalPresetBank::default_presets();
        let morpher = EmotionalMorpher::new(bank);

        // Joy corner (v=1, a=1)
        let w = morpher.calculate_weights(1.0, 1.0);
        assert!((w.joy - 1.0).abs() < 0.001);
        assert!(w.calm < 0.001 && w.sadness < 0.001 && w.anger < 0.001);

        // Calm corner (v=1, a=0)
        let w = morpher.calculate_weights(1.0, 0.0);
        assert!((w.calm - 1.0).abs() < 0.001);

        // Sadness corner (v=0, a=0)
        let w = morpher.calculate_weights(0.0, 0.0);
        assert!((w.sadness - 1.0).abs() < 0.001);

        // Anger corner (v=0, a=1)
        let w = morpher.calculate_weights(0.0, 1.0);
        assert!((w.anger - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_center_position() {
        let bank = EmotionalPresetBank::default_presets();
        let morpher = EmotionalMorpher::new(bank);
        let w = morpher.calculate_weights(0.5, 0.5);

        // At center, all quadrants should have equal weight (0.25 each)
        assert!((w.calm - 0.25).abs() < 0.001);
        assert!((w.joy - 0.25).abs() < 0.001);
        assert!((w.sadness - 0.25).abs() < 0.001);
        assert!((w.anger - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_morph_caching() {
        let bank = EmotionalPresetBank::default_presets();
        let mut morpher = EmotionalMorpher::new(bank);

        // First call should calculate
        let result1 = morpher.morph(0.5, 0.5);

        // Second call with same position should use cache
        let result2 = morpher.morph(0.5, 0.5);

        // Results should be identical (using same cached values)
        assert_eq!(result1.lead.name, result2.lead.name);
    }

    #[test]
    fn test_morph_interpolation() {
        let bank = EmotionalPresetBank::default_presets();
        let mut morpher = EmotionalMorpher::new(bank);

        // Test that morphed value is between corner values
        let result = morpher.morph(0.5, 0.5);

        // At center, cutoff should be average of all 4 corners
        let avg_cutoff = (800.0 + 4000.0 + 1500.0 + 2500.0) / 4.0;
        assert!(
            (result.lead.filter.cutoff - avg_cutoff).abs() < 100.0,
            "Expected cutoff around {}, got {}",
            avg_cutoff,
            result.lead.filter.cutoff
        );
    }
}
