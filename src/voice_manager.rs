use fundsp::hacker32::*;
use crate::events::AudioEvent;
use std::io::Cursor;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChannelType {
    FundSP,
    Oxisynth { bank: u32 },
}

pub struct VoiceManager {
    // === HYBRID ENGINE ===
    pub synth: oxisynth::Synth,
    pub channel_routing: [ChannelType; 16],
    pub current_banks: [u32; 16], // Track current bank to avoid redundant CCs

    // === LEAD ===
    pub frequency_lead: Shared,
    pub gate_lead: Shared,
    pub gate_timer_lead: usize,

    // === BASS ===
    pub frequency_bass: Shared,
    pub gate_bass: Shared,
    pub gate_timer_bass: usize,

    // === DRUMS ===
    pub gate_snare: Shared,
    pub gate_timer_snare: usize,
    pub gate_hat: Shared,
    pub gate_timer_hat: usize,

    // === EFFECTS ===
    pub cutoff: Shared,
    pub resonance: Shared,
    pub distortion: Shared,
    pub fm_ratio: Shared,
    pub fm_amount: Shared,
    pub timbre_mix: Shared,
    pub reverb_mix: Shared,
}

impl VoiceManager {
    pub fn new(
        sf2_bytes: Option<&[u8]>,
        sample_rate: f32,
        frequency_lead: Shared, gate_lead: Shared,
        frequency_bass: Shared, gate_bass: Shared,
        gate_snare: Shared, gate_hat: Shared,
        cutoff: Shared, resonance: Shared, distortion: Shared,
        fm_ratio: Shared, fm_amount: Shared, timbre_mix: Shared, reverb_mix: Shared,
    ) -> Self {
        let mut synth = oxisynth::Synth::default();
        synth.set_sample_rate(sample_rate);
        
        if let Some(_bytes) = sf2_bytes {
            // Don't load here, use add_font below
            // let mut cursor = Cursor::new(bytes);
            // if let Ok(font) = oxisynth::SoundFont::load(&mut cursor) {
            //    synth.add_font(font, true);
            // }
        }

        let channel_routing = [ChannelType::FundSP; 16];
        let current_banks = [0; 16];

        let mut vm = Self {
            synth,
            channel_routing,
            current_banks,
            frequency_lead, gate_lead, gate_timer_lead: 0,
            frequency_bass, gate_bass, gate_timer_bass: 0,
            gate_snare, gate_timer_snare: 0,
            gate_hat, gate_timer_hat: 0,
            cutoff, resonance, distortion,
            fm_ratio, fm_amount, timbre_mix, reverb_mix,
        };

        if let Some(bytes) = sf2_bytes {
            vm.add_font(0, bytes);
        }

        vm
    }

    pub fn add_font(&mut self, bank_id: u32, bytes: &[u8]) {
        let mut cursor = Cursor::new(bytes);
        if let Ok(font) = oxisynth::SoundFont::load(&mut cursor) {
            let font_id = self.synth.add_font(font, true);
            self.synth.set_bank_offset(font_id, bank_id);
        }
    }

    pub fn set_channel_route(&mut self, channel: usize, mode: ChannelType) {
        if channel < 16 {
            self.channel_routing[channel] = mode;
            
            // If switching to Oxisynth, ensure bank is selected
            if let ChannelType::Oxisynth { bank } = mode {
                if self.current_banks[channel] != bank {
                    // Send Bank Select (CC 0)
                    let _ = self.synth.send_event(oxisynth::MidiEvent::ControlChange { 
                        channel: channel as u8, 
                        ctrl: 0, 
                        value: bank as u8 
                    });
                    // Send Program Change (default to 0 for now)
                    let _ = self.synth.send_event(oxisynth::MidiEvent::ProgramChange { 
                        channel: channel as u8, 
                        program_id: 0 
                    });
                    self.current_banks[channel] = bank;
                }
            }
        }
    }

    pub fn process_event(&mut self, event: AudioEvent, samples_per_step: usize) {
        match event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                if let ChannelType::Oxisynth { .. } = self.channel_routing[channel as usize] {
                    let _ = self.synth.send_event(oxisynth::MidiEvent::NoteOn { 
                        channel: channel, 
                        key: note, 
                        vel: velocity 
                    });
                    return;
                }

                let freq = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
                let vel = velocity as f32 / 127.0;

                match channel {
                    0 => { // Bass
                        self.frequency_bass.set_value(freq);
                        self.gate_bass.set_value(vel);
                        self.gate_timer_bass = (samples_per_step as f32 * 0.6) as usize;
                    },
                    1 => { // Lead
                        self.frequency_lead.set_value(freq);
                        self.gate_lead.set_value(vel);
                        // Duration logic was: if kick 0.8 else 0.4. 
                        // We might need to pass duration in event or handle it differently.
                        // For now, let's assume a default or pass it via velocity/channel logic?
                        // The original code had logic based on "is_strong" (kick).
                        // Let's use a standard duration for now, or maybe encode duration in velocity?
                        // Or maybe the caller handles NoteOff?
                        // The user said "NoteOn / NoteOff resolves this implicitly".
                        // But here we are using gate timers for "one-shot" style triggering in the original code.
                        // If we switch to full NoteOn/NoteOff, we need to handle NoteOff events.
                        
                        // For now, let's keep the timer logic for compatibility, but maybe NoteOff can clear it?
                        self.gate_timer_lead = (samples_per_step as f32 * 0.5) as usize; 
                    },
                    2 => { // Snare
                        self.gate_snare.set_value(vel);
                        self.gate_timer_snare = (samples_per_step as f32 * 0.3) as usize;
                    },
                    3 => { // Hat
                        self.gate_hat.set_value(vel);
                        self.gate_timer_hat = (samples_per_step as f32 * 0.1) as usize;
                    },
                    _ => {}
                }
            },
            AudioEvent::NoteOff { note, channel } => {
                if let ChannelType::Oxisynth { .. } = self.channel_routing[channel as usize] {
                    let _ = self.synth.send_event(oxisynth::MidiEvent::NoteOff { 
                        channel: channel, 
                        key: note 
                    });
                    return;
                }

                match channel {
                    0 => self.gate_bass.set_value(0.0),
                    1 => self.gate_lead.set_value(0.0),
                    2 => self.gate_snare.set_value(0.0),
                    3 => self.gate_hat.set_value(0.0),
                    _ => {}
                }
            },
            AudioEvent::ControlChange { ctrl, value } => {
                // Send global CCs to Oxisynth on channel 0 (or broadcast if needed)
                let _ = self.synth.send_event(oxisynth::MidiEvent::ControlChange { 
                    channel: 0, 
                    ctrl: ctrl, 
                    value: value 
                });

                let val_norm = value as f32 / 127.0;
                match ctrl {
                    1 => { // Modulation / Tension
                        // Original: fm_ratio = 1.0 + (tension * 4.0)
                        // We can map CC1 to tension-like effects
                        self.fm_ratio.set_value(1.0 + (val_norm * 4.0));
                        self.fm_amount.set_value(val_norm * 0.8);
                        self.timbre_mix.set_value(val_norm);
                        self.cutoff.set_value(500.0 + (val_norm * 3500.0));
                        self.resonance.set_value(1.0 + (val_norm * 4.0));

                        // === SOUNDFONT TENSION EFFECTS ===
                        // Apply to all channels (0-3 are used)
                        for ch in 0..4 {
                            if let ChannelType::Oxisynth { .. } = self.channel_routing[ch] {
                                // 1. Standard: Filter & Resonance
                                // Cutoff (CC 74): 30 (dull) -> 127 (bright)
                                let cutoff_val = (30.0 + val_norm * 97.0) as i32;
                                // Resonance (CC 71): 0 -> 90 (resonant)
                                let res_val = (val_norm * 90.0) as i32;

                                let _ = self.synth.send_event(oxisynth::MidiEvent::ControlChange { 
                                    channel: ch as u8, 
                                    ctrl: 74, 
                                    value: cutoff_val.clamp(0, 127) as u8
                                });
                                let _ = self.synth.send_event(oxisynth::MidiEvent::ControlChange { 
                                    channel: ch as u8, 
                                    ctrl: 71, 
                                    value: res_val.clamp(0, 127) as u8
                                });

                                // 2. Horror: Pitch Instability (Detune)
                                let bend_val = if val_norm > 0.6 {
                                    let mut rng = rand::thread_rng();
                                    let wobble_amount = (val_norm - 0.6) * 2000.0;
                                    let random_bend = rng.gen_range(-0.5..0.5) * wobble_amount;
                                    (8192.0 + random_bend) as i32
                                } else {
                                    8192
                                };

                                let _ = self.synth.send_event(oxisynth::MidiEvent::PitchBend { 
                                    channel: ch as u8, 
                                    value: bend_val.clamp(0, 16383) as u16
                                });
                            }
                        }
                    },
                    11 => { // Expression / Arousal
                         self.distortion.set_value(val_norm * 0.8);
                    },
                    91 => { // Reverb
                        self.reverb_mix.set_value(0.1 + (val_norm * 0.4));
                    },
                    _ => {}
                }
            }
            AudioEvent::TimingUpdate { .. } | AudioEvent::LoadFont { .. } | AudioEvent::SetChannelRoute { .. } => {},
        }
    }

    pub fn update_timers(&mut self) {
        if self.gate_timer_lead > 0 { 
            self.gate_timer_lead -= 1; 
            if self.gate_timer_lead == 0 { self.gate_lead.set_value(0.0); } 
        }
        if self.gate_timer_bass > 0 { 
            self.gate_timer_bass -= 1; 
            if self.gate_timer_bass == 0 { self.gate_bass.set_value(0.0); } 
        }
        if self.gate_timer_snare > 0 { 
            self.gate_timer_snare -= 1; 
            if self.gate_timer_snare == 0 { self.gate_snare.set_value(0.0); } 
        }
        if self.gate_timer_hat > 0 { 
            self.gate_timer_hat -= 1; 
            if self.gate_timer_hat == 0 { self.gate_hat.set_value(0.0); } 
        }
    }

    pub fn process_audio(&mut self) -> (f32, f32) {
        let mut buffer = [0.0; 2]; 
        self.synth.write(&mut buffer[..]);
        (buffer[0], buffer[1])
    }
}
