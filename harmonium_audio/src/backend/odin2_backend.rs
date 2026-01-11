//! Odin2 Audio Backend
//!
//! Uses odin2-core as the synthesis engine instead of fundsp/oxisynth.
//! Implements 4 dedicated instrument channels like SynthBackend:
//! - Channel 0: Bass (sine + saw through lowpass)
//! - Channel 1: Lead (supersaw with filter envelope)
//! - Channel 2: Snare (noise bandpass + tone)
//! - Channel 3: Hat (noise highpass)

use crate::backend::AudioRenderer;
use harmonium_core::events::AudioEvent;
use odin2_core::engine::{OdinEngine, PresetConfig};
use odin2_core::SynthEngine;
use odin2_core::preset::OdinPreset;

#[allow(dead_code)]
const NUM_CHANNELS: usize = 16;

/// Instrument type for channel routing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InstrumentType {
    Bass,
    Lead,
    Snare,
    Hat,
    /// Generic polyphonic synth for channels > 3
    Poly,
}

impl InstrumentType {
    fn index(&self) -> usize {
        match self {
            InstrumentType::Bass => 0,
            InstrumentType::Lead => 1,
            InstrumentType::Snare => 2,
            InstrumentType::Hat => 3,
            InstrumentType::Poly => 4,
        }
    }

    fn from_channel(channel: u8) -> Self {
        match channel {
            0 => InstrumentType::Bass,
            1 => InstrumentType::Lead,
            2 => InstrumentType::Snare,
            3 => InstrumentType::Hat,
            _ => InstrumentType::Poly,
        }
    }
}

/// Holds the 4 preset configs for the quadrants of the emotional plane for a single instrument
#[derive(Clone, Debug)]
pub struct QuadrantConfigs {
    pub top_left: PresetConfig,     // (-1, 1)  Angry
    pub top_right: PresetConfig,    // (1, 1)   Joy
    pub bottom_left: PresetConfig,  // (-1, -1) Sad
    pub bottom_right: PresetConfig, // (1, -1)  Calm
}

impl QuadrantConfigs {
    /// Load presets from relative paths (relative to workspace root usually, or assets)
    /// order: Top-Left, Top-Right, Bottom-Left, Bottom-Right
    pub fn load_defaults(category: &str, names: [&str; 4]) -> Self {
        // Try to load from embedded bytes first (for WASM/Standalone distribution)
        let load_embedded = |name: &str| -> Option<PresetConfig> {
             let bytes: Option<&[u8]> = match (category, name) {
                 // Keys
                 ("Keys", "Synth Piano") => Some(include_bytes!("../../../assets/odin2/Keys/Synth Piano.odin")),
                 ("Keys", "Toy Piano") => Some(include_bytes!("../../../assets/odin2/Keys/Toy Piano.odin")),
                 ("Keys", "Pianet") => Some(include_bytes!("../../../assets/odin2/Keys/Pianet.odin")),
                 ("Keys", "Piano Ballad 3") => Some(include_bytes!("../../../assets/odin2/Keys/Piano Ballad 3.odin")),
                 
                 // Drums
                 ("Drums", "Kick-1 [Photonic]") => Some(include_bytes!("../../../assets/odin2/Drums/Kick-1 [Photonic].odin")),
                 ("Drums", "Snare-1 [Photonic]") => Some(include_bytes!("../../../assets/odin2/Drums/Snare-1 [Photonic].odin")),
                 ("Drums", "Drum Machine") => Some(include_bytes!("../../../assets/odin2/Drums/Drum Machine.odin")),
                 ("Drums", "HiHat-closed [Photonic]") => Some(include_bytes!("../../../assets/odin2/Drums/HiHat-closed [Photonic].odin")),
                 
                 // Bass
                 ("Bass", "Bass Crusher [RM]") => Some(include_bytes!("../../../assets/odin2/Bass/Bass Crusher [RM].odin")),
                 ("Bass", "Bass Simple PM [RS]") => Some(include_bytes!("../../../assets/odin2/Bass/Bass Simple PM [RS].odin")),
                 ("Bass", "DeepBass [Photonic]") => Some(include_bytes!("../../../assets/odin2/Bass/DeepBass [Photonic].odin")),
                 ("Bass", "Analog Bass [tx]") => Some(include_bytes!("../../../assets/odin2/Bass/Analog Bass [tx].odin")),

                 _ => None,
             };

             if let Some(data) = bytes {
                 if let Ok(preset) = OdinPreset::from_bytes(data) {
                     harmonium_core::log::info(&format!("Loaded embedded preset: {}/{}", category, name));
                     return Some(PresetConfig::from_preset(&preset));
                 } else {
                     harmonium_core::log::error(&format!("Failed to parse embedded preset: {}/{}", category, name));
                 }
             }
             None
        };

        // Fallback to filesystem (Dev environment)
        let base_paths = [
            "../odin2-rs/odin2/assets/Soundbanks/Factory Presets", // When running from harmonium/
            "odin2/assets/Soundbanks/Factory Presets" // When running from root?
        ];
        
        let load = |name: &str| -> PresetConfig {
             // 1. Try embedded
             if let Some(config) = load_embedded(name) {
                 return config;
             }

             // 2. Try filesystem
             for base in &base_paths {
                 let path = format!("{}/{}/{}.odin", base, category, name);
                 if let Ok(preset) = OdinPreset::load(&path) {
                     harmonium_core::log::info(&format!("Loaded preset from disk: {}", path));
                     return PresetConfig::from_preset(&preset);
                 }
             }
             
             harmonium_core::log::error(&format!("Failed to load preset: {}/{}.odin - Using Fallback", category, name));
             
             // Manual Fallback: Simple Saw Wave
             let mut config = PresetConfig::from_preset(&OdinPreset::default());
             config.osc_volumes[0] = 0.8; 
             config.master_volume = 0.5;
             config
        };

        Self {
            top_left: load(names[0]),
            top_right: load(names[1]),
            bottom_left: load(names[2]),
            bottom_right: load(names[3]),
        }
    }
    
    pub fn morph(&self, x: f32, y: f32) -> PresetConfig {
        // Bilinear interpolation of PresetConfig fields
        
        // Calculate weights
        let u = (x + 1.0) * 0.5;
        let v = (y + 1.0) * 0.5;
        
        // Weights:
        let w_tl = (1.0 - u) * v;          // Angry
        let w_tr = u * v;                  // Joy
        let w_bl = (1.0 - u) * (1.0 - v);  // Sad
        let w_br = u * (1.0 - v);          // Calm
        
        // Helper for linear mixing 4 values
        let mix = |a: f32, b: f32, c: f32, d: f32| -> f32 {
            a * w_tl + b * w_tr + c * w_bl + d * w_br
        };

        // Helper for discrete selection (Nearest Neighbor) to avoid out-of-tune slides
        let select_i32 = |a: i32, b: i32, c: i32, d: i32| -> i32 {
            if w_tl >= w_tr && w_tl >= w_bl && w_tl >= w_br { a }
            else if w_tr >= w_bl && w_tr >= w_br { b }
            else if w_bl >= w_br { c }
            else { d }
        };
        
        let p1 = &self.top_left;
        let p2 = &self.top_right;
        let p3 = &self.bottom_left;
        let p4 = &self.bottom_right;
        
        PresetConfig {
            osc_volumes: [
                mix(p1.osc_volumes[0], p2.osc_volumes[0], p3.osc_volumes[0], p4.osc_volumes[0]),
                mix(p1.osc_volumes[1], p2.osc_volumes[1], p3.osc_volumes[1], p4.osc_volumes[1]),
                mix(p1.osc_volumes[2], p2.osc_volumes[2], p3.osc_volumes[2], p4.osc_volumes[2]),
            ],
            // Use Discrete selection for Octaves and Semitones to prevent dissonance
            osc_octaves: [
                select_i32(p1.osc_octaves[0], p2.osc_octaves[0], p3.osc_octaves[0], p4.osc_octaves[0]),
                select_i32(p1.osc_octaves[1], p2.osc_octaves[1], p3.osc_octaves[1], p4.osc_octaves[1]),
                select_i32(p1.osc_octaves[2], p2.osc_octaves[2], p3.osc_octaves[2], p4.osc_octaves[2]),
            ],
            osc_semitones: [
                select_i32(p1.osc_semitones[0], p2.osc_semitones[0], p3.osc_semitones[0], p4.osc_semitones[0]),
                select_i32(p1.osc_semitones[1], p2.osc_semitones[1], p3.osc_semitones[1], p4.osc_semitones[1]),
                select_i32(p1.osc_semitones[2], p2.osc_semitones[2], p3.osc_semitones[2], p4.osc_semitones[2]),
            ],
            osc_detune: mix(p1.osc_detune, p2.osc_detune, p3.osc_detune, p4.osc_detune),
            
            filter_frequency: mix(p1.filter_frequency, p2.filter_frequency, p3.filter_frequency, p4.filter_frequency),
            filter_resonance: mix(p1.filter_resonance, p2.filter_resonance, p3.filter_resonance, p4.filter_resonance),
            filter_env_amount: mix(p1.filter_env_amount, p2.filter_env_amount, p3.filter_env_amount, p4.filter_env_amount),
            
            amp_attack: mix(p1.amp_attack, p2.amp_attack, p3.amp_attack, p4.amp_attack),
            amp_decay: mix(p1.amp_decay, p2.amp_decay, p3.amp_decay, p4.amp_decay),
            amp_sustain: mix(p1.amp_sustain, p2.amp_sustain, p3.amp_sustain, p4.amp_sustain),
            amp_release: mix(p1.amp_release, p2.amp_release, p3.amp_release, p4.amp_release),
            
            filter_attack: mix(p1.filter_attack, p2.filter_attack, p3.filter_attack, p4.filter_attack),
            filter_decay: mix(p1.filter_decay, p2.filter_decay, p3.filter_decay, p4.filter_decay),
            filter_sustain: mix(p1.filter_sustain, p2.filter_sustain, p3.filter_sustain, p4.filter_sustain),
            filter_release: mix(p1.filter_release, p2.filter_release, p3.filter_release, p4.filter_release),
            
            master_volume: mix(p1.master_volume, p2.master_volume, p3.master_volume, p4.master_volume),
        }
    }
}

pub struct Odin2Backend {
    engines: [Option<OdinEngine>; 5],
    presets: [Option<QuadrantConfigs>; 5],
    gains: [f32; 5],
    scratch_buffer: Vec<f32>,
    #[allow(dead_code)]
    sample_rate: f32,
    samples_per_step: usize,
}

impl Odin2Backend {
    pub fn new(sample_rate: f64) -> Self {
        let sr = sample_rate as f32;
        
        let mut engines: [Option<OdinEngine>; 5] = [None, None, None, None, None];
        let mut presets: [Option<QuadrantConfigs>; 5] = [None, None, None, None, None];
        let mut gains: [f32; 5] = [1.0; 5];
        
        // Define preset quadrants
        let keys_names = ["Synth Piano", "Toy Piano", "Pianet", "Piano Ballad 3"];
        // Separated lists for stability:
        let snare_names = ["Snare-1 [Photonic]", "Snare-1 [Photonic]", "Snare-1 [Photonic]", "Snare-1 [Photonic]"];
        let hat_names = ["HiHat-closed [Photonic]", "HiHat-closed [Photonic]", "HiHat-closed [Photonic]", "HiHat-closed [Photonic]"];
        let bass_names = ["Bass Crusher [RM]", "Bass Simple PM [RS]", "DeepBass [Photonic]", "Analog Bass [tx]"];
        
        // Map instruments
        let instruments = [
            (InstrumentType::Bass, "Bass", bass_names, 0.6),
            (InstrumentType::Lead, "Keys", keys_names, 0.8),
            (InstrumentType::Snare, "Drums", snare_names, 0.5),
            (InstrumentType::Hat, "Drums", hat_names, 0.3),
            (InstrumentType::Poly, "Keys", keys_names, 0.7),
        ];
        
        for (inst, category, names, gain) in instruments {
            let idx = inst.index();
            let mut engine = OdinEngine::new(sr);
            
            println!("Loading presets for {:?}", inst);
            let quadrant = QuadrantConfigs::load_defaults(category, names);
            
            let initial = quadrant.morph(0.0, 0.0);
            engine.load_config(initial);
            
            engines[idx] = Some(engine);
            presets[idx] = Some(quadrant);
            gains[idx] = gain;
        }

        Self {
            engines,
            presets,
            gains,
            scratch_buffer: vec![0.0f32; 2048],
            sample_rate: sr,
            samples_per_step: 0,
        }
    }

    pub fn set_gains(&mut self, lead: f32, bass: f32, snare: f32, hat: f32) {
        self.gains[InstrumentType::Lead.index()] = lead;
        self.gains[InstrumentType::Bass.index()] = bass;
        self.gains[InstrumentType::Snare.index()] = snare;
        self.gains[InstrumentType::Hat.index()] = hat;
    }

    pub fn apply_emotional_morphing(&mut self, valence: f32, arousal: f32, _tension: f32, _density: f32) {
        let y = arousal * 2.0 - 1.0;
        let x = valence;

        for i in 0..5 {
            if let Some(quadrant) = &self.presets[i] {
                if let Some(engine) = &mut self.engines[i] {
                    let morphed_config = quadrant.morph(x, y);
                    engine.load_config(morphed_config);
                }
            }
        }
    }
}

impl AudioRenderer for Odin2Backend {
    fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                let inst = InstrumentType::from_channel(channel);
                if let Some(engine) = &mut self.engines[inst.index()] {
                    engine.note_on(note, velocity);
                }
            }

            AudioEvent::NoteOff { note, channel } => {
                let inst = InstrumentType::from_channel(channel);
                if let Some(engine) = &mut self.engines[inst.index()] {
                    engine.note_off(note);
                }
            }
            
            AudioEvent::LoadOdinPreset { bytes } => {
                if let Ok(preset) = OdinPreset::from_bytes(&bytes) {
                    let config = PresetConfig::from_preset(&preset);
                    // Apply this preset to the Bass channel (0)
                    // To persist against morphing, we replace the QuadrantConfig with a static one
                    let static_quadrant = QuadrantConfigs {
                        top_left: config.clone(),
                        top_right: config.clone(),
                        bottom_left: config.clone(),
                        bottom_right: config.clone(),
                    };

                    // Update the persistent storage used by morphing
                    self.presets[0] = Some(static_quadrant);

                    // Apply immediately
                    if let Some(engine) = &mut self.engines[0] {
                        engine.load_config(config);
                    }
                }
            }

            AudioEvent::AllNotesOff { channel } => {
                let inst = InstrumentType::from_channel(channel);
                if let Some(engine) = &mut self.engines[inst.index()] {
                    for n in 0..128 {
                        engine.note_off(n);
                    }
                }
            }
            
            AudioEvent::SetMixerGains { lead, bass, snare, hat } => {
                self.set_gains(lead, bass, snare, hat);
            }
            
            AudioEvent::TimingUpdate { samples_per_step } => {
                self.samples_per_step = samples_per_step;
            }

            AudioEvent::ControlChange { ctrl: _, value: _, channel } => {
                let inst = InstrumentType::from_channel(channel);
                 if let Some(_engine) = &mut self.engines[inst.index()] {
                 }
            }

            _ => {}
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);
        
        let buf_len = output.len();
        
        if self.scratch_buffer.len() < buf_len {
             self.scratch_buffer.resize(buf_len, 0.0);
        }
        
        for i in 0..5 {
             if let Some(engine) = &mut self.engines[i] {
                 let gain = self.gains[i];
                 if gain < 0.001 { continue; }
                 
                 let mix_slice = &mut self.scratch_buffer[0..buf_len];
                 mix_slice.fill(0.0);
                 
                 engine.process(mix_slice, channels);
                 
                 for k in 0..buf_len {
                     output[k] += mix_slice[k] * gain;
                 }
             }
        }
        
        for s in output.iter_mut() {
            *s = s.tanh();
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn odin2_backend_mut(&mut self) -> Option<&mut Odin2Backend> {
        Some(self)
    }
}

