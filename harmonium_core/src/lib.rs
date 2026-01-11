pub mod harmony;
pub mod sequencer;
pub mod events;
pub mod params;
pub mod fractal;
pub mod log;

// Re-export common types
pub use events::AudioEvent;
pub use params::{MusicalParams, EngineParams};
pub use sequencer::Sequencer;

// Define MusicKernel (skeleton for now, as requested in plan)
use crate::events::AudioEvent as CoreAudioEvent;
use crate::sequencer::Sequencer as CoreSequencer;
use crate::params::MusicalParams as CoreMusicalParams;

pub struct MusicKernel {
    pub sequencer: CoreSequencer,
    pub params: CoreMusicalParams,
    pub accumulator: f64,
}

impl MusicKernel {
    pub fn new(sequencer: CoreSequencer, params: CoreMusicalParams) -> Self {
        Self { sequencer, params, accumulator: 0.0 }
    }

    pub fn update(&mut self, dt: f64) -> Vec<CoreAudioEvent> {
        let mut events = Vec::new();

        // 1. Sync Sequencer Settings
        // In a full implementation, we would compare self.params vs self.sequencer state
        // and regenerate if needed (like engine.rs).
        // For minimal "Manual Control" test, we ensure pulses/steps match the params.
        if self.sequencer.mode != self.params.rhythm_mode {
            self.sequencer.mode = self.params.rhythm_mode;
            self.sequencer.regenerate_pattern();
        }
        // Minimal pulse update for Euclidean
        if self.sequencer.mode == crate::sequencer::RhythmMode::Euclidean {
             let target_pulses = self.params.rhythm_pulses.min(self.sequencer.steps);
             if self.sequencer.pulses != target_pulses {
                 self.sequencer.pulses = target_pulses;
                 // self.sequencer.density = self.params.rhythm_density; // if used
                 self.sequencer.regenerate_pattern();
             }
        }
    
        // 2. Timing
        let bpm = self.params.bpm.max(10.0) as f64;
        let steps_per_beat = 4.0; // 16th notes
        let step_duration = 60.0 / bpm / steps_per_beat;

        self.accumulator += dt;

        while self.accumulator >= step_duration {
            self.accumulator -= step_duration;
            
            // 3. Tick
            let trigger = self.sequencer.tick();
            
            // 4. Generate Events
            let velocity = (trigger.velocity * 127.0) as u8;
            
            if trigger.kick {
                 // Kick on Channel 0
                 events.push(CoreAudioEvent::NoteOn { channel: 0, note: 36, velocity }); // C1
                 // NoteOff later? Or rely on synth envelope. 
                 // Simple synths often need explicit Off, drum samplers might trigger one-shot.
                 // We'll send NoteOff immediately for one-shot behavior if supported, 
                 // or just NoteOn for now. 
            }
            if trigger.snare {
                 // Snare on Channel 2 (per Odin backend mapping)
                 events.push(CoreAudioEvent::NoteOn { channel: 2, note: 38, velocity }); // D1
            }
            if trigger.hat {
                 // Hat on Channel 3
                 events.push(CoreAudioEvent::NoteOn { channel: 3, note: 42, velocity }); // F#1
            }
        }
        
        events
    }
}
