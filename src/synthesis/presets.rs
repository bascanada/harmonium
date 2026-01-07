//! Emotional Preset Bank
//!
//! Defines the 4 corner presets for emotional morphing based on Russell's Circumplex Model.

use super::types::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Holds the 4 corner presets for emotional morphing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalPresetBank {
    /// Joy/Victory (Valence +, Arousal +) - Q1
    pub joy: InstrumentPresets,

    /// Anger/Stress (Valence -, Arousal +) - Q2
    pub anger: InstrumentPresets,

    /// Sadness/Dark (Valence -, Arousal -) - Q3
    pub sadness: InstrumentPresets,

    /// Calm/Serenity (Valence +, Arousal -) - Q4
    pub calm: InstrumentPresets,
}

/// Presets for all instrument types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentPresets {
    pub bass: SynthPreset,
    pub lead: SynthPreset,
    pub snare: SynthPreset,
    pub hat: SynthPreset,
    pub poly: SynthPreset,
}

impl EmotionalPresetBank {
    /// Load default hardcoded presets
    pub fn default_presets() -> Self {
        Self {
            joy: InstrumentPresets::joy_presets(),
            anger: InstrumentPresets::anger_presets(),
            sadness: InstrumentPresets::sadness_presets(),
            calm: InstrumentPresets::calm_presets(),
        }
    }

    /// Load from TOML file (future extensibility)
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let bank: EmotionalPresetBank = toml::from_str(&contents)?;
        Ok(bank)
    }

    /// Save to TOML file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }
}

impl InstrumentPresets {
    /// Joy/Victory presets (Valence +, Arousal +)
    /// Bright, energetic, uplifting sound
    pub fn joy_presets() -> Self {
        Self {
            lead: SynthPreset {
                name: "Joy Lead".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.33,  // Triangle (bright but not harsh)
                    detune: 0.3,          // Moderate chorus effect
                    stereo_width: 0.9,    // Wide stereo
                    pitch_mod: 0.15,      // Subtle vibrato
                    sub_level: 0.2,       // Light sub-bass warmth
                    noise_level: 0.0,     // Clean
                },
                filter: FilterParams {
                    cutoff: 4000.0,       // Open, bright
                    resonance: 0.3,       // Gentle peak
                    env_amount: 0.7,      // Strong modulation
                    drive: 1.2,           // Slight warmth
                    filter_type: 0,       // LP4 (smooth)
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.005,    // Fast attack (energetic)
                        decay: 0.2,
                        sustain: 0.6,
                        release: 0.15,
                    },
                    filter: AdsrValues {
                        attack: 0.01,
                        decay: 0.3,
                        sustain: 0.3,
                        release: 0.2,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams {
                        time: 0.375,      // Dotted 8th note feel
                        feedback: 0.25,
                        mix: 0.20,
                    },
                    chorus: ChorusParams {
                        lfo_freq: 0.8,    // Moderate chorus
                        depth: 0.4,
                        mix: 0.3,
                    },
                    reverb: ReverbParams {
                        room_size: 0.4,   // Medium room
                        damping: 0.3,     // Bright
                        mix: 0.25,
                    },
                },
                output: OutputParams {
                    gain: 1.0,
                    pan: 0.0,
                },
            },
            bass: SynthPreset {
                name: "Joy Bass".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.5,    // Saw (bright bass)
                    detune: 0.1,
                    stereo_width: 0.3,    // Narrow (bass mono-ish)
                    pitch_mod: 0.0,
                    sub_level: 0.6,       // Strong sub
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 1200.0,       // Open bass filter
                    resonance: 0.4,
                    env_amount: 0.6,
                    drive: 1.3,
                    filter_type: 0,       // LP4
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.002,
                        decay: 0.15,
                        sustain: 0.5,
                        release: 0.1,
                    },
                    filter: AdsrValues {
                        attack: 0.005,
                        decay: 0.2,
                        sustain: 0.2,
                        release: 0.15,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.25, feedback: 0.15, mix: 0.05 },
                    chorus: ChorusParams { lfo_freq: 0.3, depth: 0.2, mix: 0.1 },
                    reverb: ReverbParams { room_size: 0.2, damping: 0.6, mix: 0.05 },
                },
                output: OutputParams { gain: 0.8, pan: 0.0 },
            },
            snare: SynthPreset {
                name: "Joy Snare".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.2,
                    detune: 0.0,
                    stereo_width: 0.6,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.7,     // High noise for snare body
                },
                filter: FilterParams {
                    cutoff: 3000.0,
                    resonance: 0.4,
                    env_amount: 0.5,
                    drive: 1.5,
                    filter_type: 3,       // BP (bandpass for snare)
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.08,
                        sustain: 0.1,
                        release: 0.05,
                    },
                    filter: AdsrValues {
                        attack: 0.001,
                        decay: 0.1,
                        sustain: 0.0,
                        release: 0.05,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.1, feedback: 0.0, mix: 0.0 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.3, damping: 0.4, mix: 0.15 },
                },
                output: OutputParams { gain: 1.0, pan: 0.0 },
            },
            hat: SynthPreset {
                name: "Joy Hat".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.0,
                    detune: 0.0,
                    stereo_width: 0.8,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 1.0,     // Pure noise
                },
                filter: FilterParams {
                    cutoff: 8000.0,       // High-pass region
                    resonance: 0.2,
                    env_amount: 0.3,
                    drive: 1.0,
                    filter_type: 2,       // HP4 (highpass)
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.05,
                        sustain: 0.0,
                        release: 0.03,
                    },
                    filter: AdsrValues {
                        attack: 0.001,
                        decay: 0.06,
                        sustain: 0.0,
                        release: 0.03,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.0, feedback: 0.0, mix: 0.0 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.1, damping: 0.8, mix: 0.05 },
                },
                output: OutputParams { gain: 0.7, pan: 0.0 },
            },
            poly: SynthPreset {
                name: "Joy Poly".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.4,
                    detune: 0.25,
                    stereo_width: 0.8,
                    pitch_mod: 0.1,
                    sub_level: 0.2,
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 3500.0,
                    resonance: 0.3,
                    env_amount: 0.6,
                    drive: 1.2,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.01,
                        decay: 0.2,
                        sustain: 0.6,
                        release: 0.2,
                    },
                    filter: AdsrValues {
                        attack: 0.015,
                        decay: 0.25,
                        sustain: 0.3,
                        release: 0.2,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.3, feedback: 0.2, mix: 0.15 },
                    chorus: ChorusParams { lfo_freq: 0.7, depth: 0.35, mix: 0.25 },
                    reverb: ReverbParams { room_size: 0.4, damping: 0.4, mix: 0.2 },
                },
                output: OutputParams { gain: 1.0, pan: 0.0 },
            },
        }
    }

    /// Anger/Stress presets (Valence -, Arousal +)
    /// Aggressive, harsh, intense sound
    pub fn anger_presets() -> Self {
        Self {
            lead: SynthPreset {
                name: "Anger Lead".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.85,   // Mostly square (harsh, aggressive)
                    detune: 0.6,          // Heavy detune (dissonance)
                    stereo_width: 0.7,
                    pitch_mod: 0.0,       // No vibrato (rigid)
                    sub_level: 0.0,       // No warmth
                    noise_level: 0.15,    // Added aggression
                },
                filter: FilterParams {
                    cutoff: 2500.0,       // Mid-range bite
                    resonance: 0.7,       // Harsh resonance
                    env_amount: 0.5,      // Moderate modulation
                    drive: 2.5,           // Heavy distortion
                    filter_type: 0,       // LP4
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,    // Instant attack (aggressive)
                        decay: 0.05,      // Fast decay (punchy)
                        sustain: 0.8,
                        release: 0.05,    // Fast release (staccato)
                    },
                    filter: AdsrValues {
                        attack: 0.005,
                        decay: 0.1,
                        sustain: 0.5,
                        release: 0.1,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams {
                        time: 0.125,      // Short, fast echoes
                        feedback: 0.5,    // More feedback
                        mix: 0.15,
                    },
                    chorus: ChorusParams {
                        lfo_freq: 2.0,    // Fast, chaotic modulation
                        depth: 0.6,
                        mix: 0.2,
                    },
                    reverb: ReverbParams {
                        room_size: 0.2,   // Small, tight space
                        damping: 0.7,     // Dark, aggressive
                        mix: 0.10,
                    },
                },
                output: OutputParams {
                    gain: 1.2,            // Louder
                    pan: 0.0,
                },
            },
            bass: SynthPreset {
                name: "Anger Bass".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.66,   // Saw (aggressive)
                    detune: 0.3,          // More detune
                    stereo_width: 0.4,
                    pitch_mod: 0.0,
                    sub_level: 0.8,       // Heavy sub
                    noise_level: 0.1,
                },
                filter: FilterParams {
                    cutoff: 600.0,        // Darker, growling
                    resonance: 0.6,       // More resonance
                    env_amount: 0.7,
                    drive: 2.0,           // Heavy distortion
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.08,
                        sustain: 0.7,
                        release: 0.05,
                    },
                    filter: AdsrValues {
                        attack: 0.002,
                        decay: 0.1,
                        sustain: 0.4,
                        release: 0.08,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.125, feedback: 0.3, mix: 0.08 },
                    chorus: ChorusParams { lfo_freq: 1.5, depth: 0.4, mix: 0.15 },
                    reverb: ReverbParams { room_size: 0.15, damping: 0.8, mix: 0.03 },
                },
                output: OutputParams { gain: 1.0, pan: 0.0 },
            },
            snare: SynthPreset {
                name: "Anger Snare".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.3,
                    detune: 0.0,
                    stereo_width: 0.5,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.9,
                },
                filter: FilterParams {
                    cutoff: 2800.0,
                    resonance: 0.6,
                    env_amount: 0.6,
                    drive: 2.0,
                    filter_type: 3,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.06,
                        sustain: 0.05,
                        release: 0.04,
                    },
                    filter: AdsrValues {
                        attack: 0.001,
                        decay: 0.08,
                        sustain: 0.0,
                        release: 0.04,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.08, feedback: 0.1, mix: 0.05 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.2, damping: 0.6, mix: 0.08 },
                },
                output: OutputParams { gain: 1.2, pan: 0.0 },
            },
            hat: SynthPreset {
                name: "Anger Hat".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.0,
                    detune: 0.0,
                    stereo_width: 0.7,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 1.0,
                },
                filter: FilterParams {
                    cutoff: 9000.0,
                    resonance: 0.4,
                    env_amount: 0.4,
                    drive: 1.3,
                    filter_type: 2,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.04,
                        sustain: 0.0,
                        release: 0.02,
                    },
                    filter: AdsrValues {
                        attack: 0.001,
                        decay: 0.05,
                        sustain: 0.0,
                        release: 0.02,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.0, feedback: 0.0, mix: 0.0 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.08, damping: 0.9, mix: 0.02 },
                },
                output: OutputParams { gain: 0.8, pan: 0.0 },
            },
            poly: SynthPreset {
                name: "Anger Poly".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.75,
                    detune: 0.5,
                    stereo_width: 0.6,
                    pitch_mod: 0.0,
                    sub_level: 0.1,
                    noise_level: 0.1,
                },
                filter: FilterParams {
                    cutoff: 2200.0,
                    resonance: 0.65,
                    env_amount: 0.5,
                    drive: 2.2,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.002,
                        decay: 0.08,
                        sustain: 0.7,
                        release: 0.08,
                    },
                    filter: AdsrValues {
                        attack: 0.005,
                        decay: 0.12,
                        sustain: 0.4,
                        release: 0.1,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.15, feedback: 0.35, mix: 0.12 },
                    chorus: ChorusParams { lfo_freq: 1.8, depth: 0.5, mix: 0.18 },
                    reverb: ReverbParams { room_size: 0.2, damping: 0.7, mix: 0.08 },
                },
                output: OutputParams { gain: 1.1, pan: 0.0 },
            },
        }
    }

    /// Sadness/Dark presets (Valence -, Arousal -)
    /// Dark, mellow, atmospheric sound
    pub fn sadness_presets() -> Self {
        Self {
            lead: SynthPreset {
                name: "Sadness Lead".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.0,    // Pure sine (mellow, dark)
                    detune: 0.05,         // Minimal detune
                    stereo_width: 0.3,    // Narrow (intimate)
                    pitch_mod: 0.3,       // Heavy vibrato (emotional)
                    sub_level: 0.5,       // Heavy sub (dark warmth)
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 800.0,        // Very dark
                    resonance: 0.1,       // No peaks (smooth)
                    env_amount: 0.3,      // Subtle modulation
                    drive: 1.0,           // Clean
                    filter_type: 0,       // LP4
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.15,     // Slow attack (gentle)
                        decay: 0.5,
                        sustain: 0.7,
                        release: 0.8,     // Long release (lingering)
                    },
                    filter: AdsrValues {
                        attack: 0.2,
                        decay: 0.6,
                        sustain: 0.2,
                        release: 1.0,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams {
                        time: 0.75,       // Long, spacious delay
                        feedback: 0.6,
                        mix: 0.35,
                    },
                    chorus: ChorusParams {
                        lfo_freq: 0.2,    // Very slow modulation
                        depth: 0.2,
                        mix: 0.1,
                    },
                    reverb: ReverbParams {
                        room_size: 0.9,   // Huge space (atmospheric)
                        damping: 0.5,     // Medium damping
                        mix: 0.50,        // Lots of reverb
                    },
                },
                output: OutputParams {
                    gain: 0.8,            // Quieter
                    pan: 0.0,
                },
            },
            bass: SynthPreset {
                name: "Sadness Bass".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.1,    // Mostly sine (dark)
                    detune: 0.05,
                    stereo_width: 0.2,
                    pitch_mod: 0.0,
                    sub_level: 0.7,
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 500.0,        // Very dark
                    resonance: 0.2,
                    env_amount: 0.4,
                    drive: 1.0,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.05,
                        decay: 0.3,
                        sustain: 0.6,
                        release: 0.4,
                    },
                    filter: AdsrValues {
                        attack: 0.08,
                        decay: 0.4,
                        sustain: 0.2,
                        release: 0.5,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.5, feedback: 0.4, mix: 0.15 },
                    chorus: ChorusParams { lfo_freq: 0.15, depth: 0.15, mix: 0.08 },
                    reverb: ReverbParams { room_size: 0.7, damping: 0.6, mix: 0.25 },
                },
                output: OutputParams { gain: 0.7, pan: 0.0 },
            },
            snare: SynthPreset {
                name: "Sadness Snare".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.1,
                    detune: 0.0,
                    stereo_width: 0.4,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.5,
                },
                filter: FilterParams {
                    cutoff: 2000.0,
                    resonance: 0.2,
                    env_amount: 0.3,
                    drive: 1.0,
                    filter_type: 3,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.002,
                        decay: 0.12,
                        sustain: 0.15,
                        release: 0.08,
                    },
                    filter: AdsrValues {
                        attack: 0.003,
                        decay: 0.15,
                        sustain: 0.05,
                        release: 0.1,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.4, feedback: 0.2, mix: 0.1 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.6, damping: 0.5, mix: 0.3 },
                },
                output: OutputParams { gain: 0.8, pan: 0.0 },
            },
            hat: SynthPreset {
                name: "Sadness Hat".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.0,
                    detune: 0.0,
                    stereo_width: 0.5,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.8,
                },
                filter: FilterParams {
                    cutoff: 6000.0,
                    resonance: 0.1,
                    env_amount: 0.2,
                    drive: 1.0,
                    filter_type: 2,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.002,
                        decay: 0.08,
                        sustain: 0.0,
                        release: 0.06,
                    },
                    filter: AdsrValues {
                        attack: 0.003,
                        decay: 0.1,
                        sustain: 0.0,
                        release: 0.06,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.0, feedback: 0.0, mix: 0.0 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.4, damping: 0.6, mix: 0.15 },
                },
                output: OutputParams { gain: 0.6, pan: 0.0 },
            },
            poly: SynthPreset {
                name: "Sadness Poly".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.05,
                    detune: 0.08,
                    stereo_width: 0.4,
                    pitch_mod: 0.25,
                    sub_level: 0.4,
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 900.0,
                    resonance: 0.15,
                    env_amount: 0.35,
                    drive: 1.0,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.12,
                        decay: 0.4,
                        sustain: 0.65,
                        release: 0.7,
                    },
                    filter: AdsrValues {
                        attack: 0.18,
                        decay: 0.5,
                        sustain: 0.25,
                        release: 0.9,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.65, feedback: 0.5, mix: 0.3 },
                    chorus: ChorusParams { lfo_freq: 0.25, depth: 0.25, mix: 0.12 },
                    reverb: ReverbParams { room_size: 0.85, damping: 0.55, mix: 0.45 },
                },
                output: OutputParams { gain: 0.85, pan: 0.0 },
            },
        }
    }

    /// Calm/Serenity presets (Valence +, Arousal -)
    /// Pure, peaceful, gentle sound
    pub fn calm_presets() -> Self {
        Self {
            lead: SynthPreset {
                name: "Calm Lead".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.15,   // Mostly sine (pure, peaceful)
                    detune: 0.1,          // Very subtle detune
                    stereo_width: 0.5,    // Moderate width
                    pitch_mod: 0.1,       // Gentle vibrato
                    sub_level: 0.3,       // Some warmth
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 1500.0,       // Mellow (not too dark)
                    resonance: 0.2,       // Gentle
                    env_amount: 0.4,      // Moderate modulation
                    drive: 1.0,           // Clean
                    filter_type: 0,       // LP4
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.05,     // Gentle attack
                        decay: 0.3,
                        sustain: 0.6,
                        release: 0.5,     // Moderate release
                    },
                    filter: AdsrValues {
                        attack: 0.08,
                        decay: 0.4,
                        sustain: 0.3,
                        release: 0.6,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams {
                        time: 0.5,        // Medium delay
                        feedback: 0.3,
                        mix: 0.15,
                    },
                    chorus: ChorusParams {
                        lfo_freq: 0.4,    // Slow chorus
                        depth: 0.3,
                        mix: 0.15,
                    },
                    reverb: ReverbParams {
                        room_size: 0.6,   // Medium-large room
                        damping: 0.4,     // Balanced
                        mix: 0.30,
                    },
                },
                output: OutputParams {
                    gain: 0.9,
                    pan: 0.0,
                },
            },
            bass: SynthPreset {
                name: "Calm Bass".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.25,   // Mostly sine with some saw
                    detune: 0.08,
                    stereo_width: 0.25,
                    pitch_mod: 0.0,
                    sub_level: 0.5,
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 700.0,
                    resonance: 0.25,
                    env_amount: 0.5,
                    drive: 1.1,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.01,
                        decay: 0.2,
                        sustain: 0.55,
                        release: 0.2,
                    },
                    filter: AdsrValues {
                        attack: 0.02,
                        decay: 0.25,
                        sustain: 0.25,
                        release: 0.3,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.35, feedback: 0.2, mix: 0.08 },
                    chorus: ChorusParams { lfo_freq: 0.25, depth: 0.18, mix: 0.12 },
                    reverb: ReverbParams { room_size: 0.4, damping: 0.5, mix: 0.12 },
                },
                output: OutputParams { gain: 0.75, pan: 0.0 },
            },
            snare: SynthPreset {
                name: "Calm Snare".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.15,
                    detune: 0.0,
                    stereo_width: 0.5,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.6,
                },
                filter: FilterParams {
                    cutoff: 2500.0,
                    resonance: 0.3,
                    env_amount: 0.4,
                    drive: 1.1,
                    filter_type: 3,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.1,
                        sustain: 0.12,
                        release: 0.06,
                    },
                    filter: AdsrValues {
                        attack: 0.002,
                        decay: 0.12,
                        sustain: 0.08,
                        release: 0.08,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.2, feedback: 0.1, mix: 0.05 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.4, damping: 0.45, mix: 0.2 },
                },
                output: OutputParams { gain: 0.9, pan: 0.0 },
            },
            hat: SynthPreset {
                name: "Calm Hat".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.0,
                    detune: 0.0,
                    stereo_width: 0.6,
                    pitch_mod: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.9,
                },
                filter: FilterParams {
                    cutoff: 7000.0,
                    resonance: 0.15,
                    env_amount: 0.25,
                    drive: 1.0,
                    filter_type: 2,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.001,
                        decay: 0.06,
                        sustain: 0.0,
                        release: 0.04,
                    },
                    filter: AdsrValues {
                        attack: 0.002,
                        decay: 0.08,
                        sustain: 0.0,
                        release: 0.05,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.0, feedback: 0.0, mix: 0.0 },
                    chorus: ChorusParams { lfo_freq: 0.0, depth: 0.0, mix: 0.0 },
                    reverb: ReverbParams { room_size: 0.3, damping: 0.7, mix: 0.08 },
                },
                output: OutputParams { gain: 0.65, pan: 0.0 },
            },
            poly: SynthPreset {
                name: "Calm Poly".to_string(),
                osc: OscillatorParams {
                    waveform_mix: 0.2,
                    detune: 0.12,
                    stereo_width: 0.6,
                    pitch_mod: 0.12,
                    sub_level: 0.3,
                    noise_level: 0.0,
                },
                filter: FilterParams {
                    cutoff: 1600.0,
                    resonance: 0.22,
                    env_amount: 0.45,
                    drive: 1.05,
                    filter_type: 0,
                },
                envelopes: EnvelopeParams {
                    amp: AdsrValues {
                        attack: 0.06,
                        decay: 0.28,
                        sustain: 0.58,
                        release: 0.45,
                    },
                    filter: AdsrValues {
                        attack: 0.09,
                        decay: 0.35,
                        sustain: 0.28,
                        release: 0.55,
                    },
                },
                effects: EffectsParams {
                    delay: DelayParams { time: 0.45, feedback: 0.28, mix: 0.18 },
                    chorus: ChorusParams { lfo_freq: 0.35, depth: 0.3, mix: 0.18 },
                    reverb: ReverbParams { room_size: 0.58, damping: 0.42, mix: 0.28 },
                },
                output: OutputParams { gain: 0.92, pan: 0.0 },
            },
        }
    }
}
