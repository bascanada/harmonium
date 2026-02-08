pub mod events;
pub mod export;
pub mod fractal;
pub mod harmony;
pub mod log;
pub mod params;
pub mod sequencer;

// Re-export common types
pub use events::AudioEvent;
pub use export::{
    ChordSymbol, GitVersion, to_musicxml, to_musicxml_with_chords, write_musicxml,
    write_musicxml_with_chords,
};
pub use params::{EngineParams, MusicalParams};
pub use sequencer::Sequencer;

// Define MusicKernel (skeleton for now, as requested in plan)
use crate::events::AudioEvent as CoreAudioEvent;
use crate::{params::MusicalParams as CoreMusicalParams, sequencer::Sequencer as CoreSequencer};

pub struct MusicKernel {
    pub sequencer: CoreSequencer,
    pub params: CoreMusicalParams,
    pub accumulator: f64,
    // Track active notes to send NoteOffs: (channel, note, duration_remaining)
    pub active_notes: Vec<(u8, u8, f64)>,
    pub look_ahead: crate::sequencer::LookAheadBuffer,
}

impl MusicKernel {
    #[must_use]
    pub fn new(sequencer: CoreSequencer, params: CoreMusicalParams) -> Self {
        let mut kernel = Self {
            sequencer,
            params,
            accumulator: 0.0,
            active_notes: Vec::with_capacity(16),
            look_ahead: crate::sequencer::LookAheadBuffer::new(48), // Default horizon: 48 steps (1 bar @ 16th)
        };
        kernel.fill_buffer();
        kernel
    }

    /// Fills the look-ahead buffer with upcoming steps from the sequencer.
    pub fn fill_buffer(&mut self) -> Option<CoreAudioEvent> {
        let mut events_emitted = false;
        let start_index = self.look_ahead.last_generated_step;

        while self.look_ahead.queue.len() < self.look_ahead.horizon_steps {
            let next_step = self.look_ahead.last_generated_step;
            let trigger = self.sequencer.peek_at_step(next_step);

            // Pre-calculate pitches
            let mut pitches = vec![None; 5];

            // Bass (Channel 0)
            if trigger.bass {
                pitches[0] = Some(if next_step % 8 == 0 { 36 } else { 48 });
            }

            // Lead (Channel 1)
            if trigger.kick || trigger.snare {
                let chord_tones = [60, 63, 67, 70];
                let note_idx = (next_step / 2) % chord_tones.len();
                pitches[1] = Some(chord_tones[note_idx]);
            }

            // Snare (Channel 2)
            if trigger.snare {
                pitches[2] = Some(38);
            }

            // Hat (Channel 3)
            if trigger.hat {
                pitches[3] = Some(42);
            }

            // Kick (Channel 0)
            if trigger.kick {
                pitches[4] = Some(36);
            }

            self.look_ahead.queue.push_back(crate::sequencer::ScheduledStep {
                absolute_step: next_step,
                trigger,
                pitches,
            });

            self.look_ahead.last_generated_step += 1;
            events_emitted = true;
        }

        if events_emitted {
            let upcoming_steps: Vec<_> = self.look_ahead.queue.iter().cloned().collect();

            Some(CoreAudioEvent::BufferUpdate {
                upcoming_steps,
                start_step_index: start_index,
            })
        } else {
            None
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
        let mut needs_regen = false;
        if self.sequencer.mode != self.params.rhythm_mode {
            self.sequencer.mode = self.params.rhythm_mode;
            needs_regen = true;
        }

        if (self.sequencer.tension - self.params.rhythm_tension).abs() > f32::EPSILON {
            self.sequencer.tension = self.params.rhythm_tension;
            needs_regen = true;
        }

        if (self.sequencer.density - self.params.rhythm_density).abs() > f32::EPSILON {
            self.sequencer.density = self.params.rhythm_density;
            needs_regen = true;
        }

        // Minimal pulse update for Euclidean
        if self.sequencer.mode == crate::sequencer::RhythmMode::Euclidean {
            let target_pulses = self.params.rhythm_pulses.min(self.sequencer.steps);
            if self.sequencer.pulses != target_pulses {
                self.sequencer.pulses = target_pulses;
                needs_regen = true;
            }
        }

        if needs_regen {
            self.sequencer.regenerate_pattern();
        }

        // 2. Timing
        let bpm = f64::from(self.params.bpm.max(10.0));
        let steps_per_beat = 4.0; // 16th notes
        let step_duration = 60.0 / bpm / steps_per_beat;

        self.accumulator += dt;

        #[allow(clippy::while_float)]
        while self.accumulator >= step_duration {
            self.accumulator -= step_duration;

            // 3. Tick from Buffer
            let step = self.look_ahead.queue.pop_front().unwrap_or_default();
            let trigger = step.trigger;
            let _absolute_step = step.absolute_step;
            let pitches = step.pitches;

            crate::log::info(&format!(
                "MusicKernel: Pop step {}, remaining queue: {}",
                _absolute_step,
                self.look_ahead.queue.len()
            ));

            // 4. Generate Events
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let velocity = (trigger.velocity * 127.0) as u8;

            // Duration for notes (e.g. 50% gate)
            let note_duration = step_duration * 0.5;

            if trigger.kick {
                if let Some(note) = pitches[4] {
                    // Kick on Channel 0
                    events.push(CoreAudioEvent::NoteOff { channel: 0, note });
                    events.push(CoreAudioEvent::NoteOn { channel: 0, note, velocity });
                    self.active_notes.push((0, note, note_duration));
                }
            }
            if trigger.bass {
                if let Some(note) = pitches[0] {
                    // Bass line on Channel 0
                    events.push(CoreAudioEvent::NoteOff { channel: 0, note });
                    events.push(CoreAudioEvent::NoteOn { channel: 0, note, velocity });
                    self.active_notes.push((0, note, note_duration * 1.5));
                }
            }
            // For now, matching HarmoniumEngine behavior: lead plays on kick/snare regardless of trigger.lead
            if trigger.kick || trigger.snare {
                if let Some(note) = pitches[1] {
                    // Lead on Channel 1
                    events.push(CoreAudioEvent::NoteOff { channel: 1, note });
                    events.push(CoreAudioEvent::NoteOn { channel: 1, note, velocity });
                    self.active_notes.push((1, note, note_duration));
                }
            }
            if trigger.snare {
                if let Some(note) = pitches[2] {
                    // Snare on Channel 2
                    events.push(CoreAudioEvent::NoteOff { channel: 2, note });
                    events.push(CoreAudioEvent::NoteOn { channel: 2, note, velocity });
                    self.active_notes.push((2, note, note_duration));
                }
            }
            if trigger.hat {
                if let Some(note) = pitches[3] {
                    // Hat on Channel 3
                    events.push(CoreAudioEvent::NoteOff { channel: 3, note });
                    events.push(CoreAudioEvent::NoteOn { channel: 3, note, velocity });
                    self.active_notes.push((3, note, note_duration * 0.5)); // shorter hats
                }
            }
        }

        // 5. Replenish Buffer and Emit Update
        if let Some(buf_event) = self.fill_buffer() {
            events.push(buf_event);
        }

        events
    }
}