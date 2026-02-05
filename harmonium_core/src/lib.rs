pub mod harmony;
pub mod sequencer;
pub mod events;
pub mod params;
pub mod fractal;
pub mod log;
pub mod export;

// Re-export common types
pub use events::AudioEvent;
pub use params::{MusicalParams, EngineParams};
pub use sequencer::Sequencer;
pub use export::{to_musicxml, write_musicxml, to_musicxml_with_chords, write_musicxml_with_chords, ChordSymbol, GitVersion};

// Define MusicKernel (skeleton for now, as requested in plan)
use crate::events::AudioEvent as CoreAudioEvent;
use crate::sequencer::Sequencer as CoreSequencer;
use crate::params::MusicalParams as CoreMusicalParams;

pub struct MusicKernel {
    pub sequencer: CoreSequencer,
    pub params: CoreMusicalParams,
    pub accumulator: f64,
    // Track active notes to send NoteOffs: (channel, note, duration_remaining)
    pub active_notes: Vec<(u8, u8, f64)>,
}

impl MusicKernel {
    pub fn new(sequencer: CoreSequencer, params: CoreMusicalParams) -> Self {
        Self { 
            sequencer, 
            params, 
            accumulator: 0.0,
            active_notes: Vec::with_capacity(16),
        }
    }

    pub fn update(&mut self, dt: f64) -> Vec<CoreAudioEvent> {
        let mut events = Vec::new();

        // 0. Manage Note Offs
        // We decrement duration and remove expired notes
        let mut kept_notes = Vec::new();
        for (channel, note, rem_time) in self.active_notes.drain(..) {
             let new_rem = rem_time - dt;
             if new_rem <= 0.0 {
                 events.push(CoreAudioEvent::NoteOff { channel, note });
             } else {
                 kept_notes.push((channel, note, new_rem));
             }
        }
        self.active_notes = kept_notes;

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
            
            // Duration for notes (e.g. 50% gate)
            let note_duration = step_duration * 0.5;

            if trigger.kick {
                 // Kick on Channel 0
                 // Kill previous note (monophonic kick)
                 events.push(CoreAudioEvent::NoteOff { channel: 0, note: 36 });
                 events.push(CoreAudioEvent::NoteOn { channel: 0, note: 36, velocity });
                 // Schedule Off
                 self.active_notes.push((0, 36, note_duration));
            }
            if trigger.bass {
                 // Bass line on Channel 0 (Bass instrument per Odin2Backend mapping)
                 // TODO: Future refactoring - unify kick/bass or separate into dedicated percussion channel
                 // Simple Octave pattern: Root (C2) or Octave (C3)
                 let note = if self.sequencer.current_step.is_multiple_of(8) { 36 } else { 48 };
                 events.push(CoreAudioEvent::NoteOff { channel: 0, note });
                 events.push(CoreAudioEvent::NoteOn { channel: 0, note, velocity });
                 self.active_notes.push((0, note, note_duration * 1.5));
            }
            // TODO: Future refactoring - unify lead triggering logic across MusicKernel and HarmoniumEngine
            // For now, matching HarmoniumEngine behavior: lead plays on kick/snare regardless of trigger.lead
            if trigger.kick || trigger.snare {
                 // Lead on Channel 1
                 // Simple Arpeggiator: Cm7 (C Eb G Bb)
                 let chord_tones = [60, 63, 67, 70];
                 // Use step index to walk through chord tones
                 let note_idx = (self.sequencer.current_step / 2) % chord_tones.len();
                 let note = chord_tones[note_idx];

                 events.push(CoreAudioEvent::NoteOff { channel: 1, note });
                 events.push(CoreAudioEvent::NoteOn { channel: 1, note, velocity });
                 self.active_notes.push((1, note, note_duration));
            }
            if trigger.snare {
                 // Snare on Channel 2 (per Odin backend mapping)
                 events.push(CoreAudioEvent::NoteOff { channel: 2, note: 38 });
                 events.push(CoreAudioEvent::NoteOn { channel: 2, note: 38, velocity });
                 self.active_notes.push((2, 38, note_duration));
            }
            if trigger.hat {
                 // Hat on Channel 3
                 events.push(CoreAudioEvent::NoteOff { channel: 3, note: 42 });
                 events.push(CoreAudioEvent::NoteOn { channel: 3, note: 42, velocity });
                 self.active_notes.push((3, 42, note_duration * 0.5)); // shorter hats
            }
        }
        
        events
    }
}
