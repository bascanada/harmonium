//! Synthesis Parameter Data Types
//!
//! Defines the data structures for Odin2 synthesis parameters that can be morphed.

use serde::{Deserialize, Serialize};

/// Represents a complete set of Odin2 synthesis parameters for one instrument
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthPreset {
    pub name: String,

    /// Oscillator parameters
    pub osc: OscillatorParams,

    /// Filter parameters
    pub filter: FilterParams,

    /// Envelope parameters
    pub envelopes: EnvelopeParams,

    /// Effects parameters
    pub effects: EffectsParams,

    /// Mixer/Output parameters
    pub output: OutputParams,
}

/// Oscillator parameters (for MultiOscillator or AnalogOscillator)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OscillatorParams {
    /// Waveform mix: 0.0 = Sine, 0.33 = Triangle, 0.66 = Saw, 1.0 = Square
    pub waveform_mix: f32,

    /// Detune amount (0.0 - 1.0) for stereo width
    pub detune: f32,

    /// Stereo width (0.0 = mono, 1.0 = full stereo)
    pub stereo_width: f32,

    /// Pitch modulation amount (for LFO/vibrato)
    pub pitch_mod: f32,

    /// Sub-oscillator level (0.0 - 1.0)
    pub sub_level: f32,

    /// Noise level mixed with oscillator (0.0 - 1.0)
    pub noise_level: f32,
}

impl Default for OscillatorParams {
    fn default() -> Self {
        Self {
            waveform_mix: 0.0,  // Sine
            detune: 0.0,
            stereo_width: 0.5,
            pitch_mod: 0.0,
            sub_level: 0.0,
            noise_level: 0.0,
        }
    }
}

/// Filter parameters (LadderFilter)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterParams {
    /// Cutoff frequency (20.0 - 20000.0 Hz)
    pub cutoff: f32,

    /// Resonance (0.0 - 1.0)
    pub resonance: f32,

    /// Filter envelope modulation amount (-1.0 to 1.0)
    /// Negative = inverse envelope
    pub env_amount: f32,

    /// Filter drive/saturation (1.0 - 4.0)
    pub drive: f32,

    /// Filter type: 0 = LP4, 1 = LP2, 2 = HP4, 3 = BP
    pub filter_type: u8,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            cutoff: 1000.0,
            resonance: 0.2,
            env_amount: 0.0,
            drive: 1.0,
            filter_type: 0,  // LP4
        }
    }
}

/// ADSR Envelope parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvelopeParams {
    /// Amplitude envelope
    pub amp: AdsrValues,

    /// Filter envelope
    pub filter: AdsrValues,
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        Self {
            amp: AdsrValues::default(),
            filter: AdsrValues::default(),
        }
    }
}

/// ADSR values for a single envelope
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdsrValues {
    /// Attack time in seconds (0.001 - 5.0)
    pub attack: f32,

    /// Decay time in seconds (0.001 - 5.0)
    pub decay: f32,

    /// Sustain level (0.0 - 1.0)
    pub sustain: f32,

    /// Release time in seconds (0.001 - 10.0)
    pub release: f32,
}

impl Default for AdsrValues {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

/// Effects parameters (global effects chain)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectsParams {
    /// Delay settings
    pub delay: DelayParams,

    /// Chorus settings
    pub chorus: ChorusParams,

    /// Reverb settings
    pub reverb: ReverbParams,
}

impl Default for EffectsParams {
    fn default() -> Self {
        Self {
            delay: DelayParams::default(),
            chorus: ChorusParams::default(),
            reverb: ReverbParams::default(),
        }
    }
}

/// Delay effect parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelayParams {
    /// Delay time in seconds (0.0 - 2.0)
    pub time: f32,

    /// Feedback amount (0.0 - 0.95)
    pub feedback: f32,

    /// Wet/dry mix (0.0 - 1.0)
    pub mix: f32,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            time: 0.25,
            feedback: 0.2,
            mix: 0.15,
        }
    }
}

/// Chorus effect parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChorusParams {
    /// LFO frequency in Hz (0.1 - 10.0)
    pub lfo_freq: f32,

    /// Modulation depth (0.0 - 1.0)
    pub depth: f32,

    /// Wet/dry mix (0.0 - 1.0)
    pub mix: f32,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            lfo_freq: 0.5,
            depth: 0.3,
            mix: 0.15,
        }
    }
}

/// Reverb effect parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReverbParams {
    /// Room size (0.0 - 1.0)
    pub room_size: f32,

    /// Damping/high frequency absorption (0.0 - 1.0)
    pub damping: f32,

    /// Wet/dry mix (0.0 - 1.0)
    pub mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            mix: 0.2,
        }
    }
}

/// Output/mixer parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputParams {
    /// Output gain (0.0 - 2.0)
    pub gain: f32,

    /// Pan position (-1.0 = left, 0.0 = center, 1.0 = right)
    pub pan: f32,
}

impl Default for OutputParams {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pan: 0.0,
        }
    }
}

impl Default for SynthPreset {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            osc: OscillatorParams::default(),
            filter: FilterParams::default(),
            envelopes: EnvelopeParams::default(),
            effects: EffectsParams::default(),
            output: OutputParams::default(),
        }
    }
}
