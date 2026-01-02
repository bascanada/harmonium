use fundsp::hacker32::*;
use crate::events::AudioEvent;

pub struct VoiceManager {
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
        frequency_lead: Shared, gate_lead: Shared,
        frequency_bass: Shared, gate_bass: Shared,
        gate_snare: Shared, gate_hat: Shared,
        cutoff: Shared, resonance: Shared, distortion: Shared,
        fm_ratio: Shared, fm_amount: Shared, timbre_mix: Shared, reverb_mix: Shared,
    ) -> Self {
        Self {
            frequency_lead, gate_lead, gate_timer_lead: 0,
            frequency_bass, gate_bass, gate_timer_bass: 0,
            gate_snare, gate_timer_snare: 0,
            gate_hat, gate_timer_hat: 0,
            cutoff, resonance, distortion,
            fm_ratio, fm_amount, timbre_mix, reverb_mix,
        }
    }

    pub fn process_event(&mut self, event: AudioEvent, samples_per_step: usize) {
        match event {
            AudioEvent::NoteOn { note, velocity, channel } => {
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
            AudioEvent::NoteOff { note: _, channel } => {
                match channel {
                    0 => self.gate_bass.set_value(0.0),
                    1 => self.gate_lead.set_value(0.0),
                    2 => self.gate_snare.set_value(0.0),
                    3 => self.gate_hat.set_value(0.0),
                    _ => {}
                }
            },
            AudioEvent::ControlChange { ctrl, value } => {
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
}
