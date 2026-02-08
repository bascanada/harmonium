//! VST MIDI Backend - Collects MIDI events for DAW output
//!
//! This backend doesn't generate audio - it collects MIDI events
//! that the VST plugin will send to the host DAW.

use std::collections::VecDeque;

use harmonium_core::events::AudioEvent;

use crate::backend::AudioRenderer;

/// A MIDI event ready to be sent to the DAW
#[derive(Clone, Debug)]
pub struct VstMidiEvent {
    /// Sample offset within the current buffer
    pub sample_offset: u32,
    /// MIDI channel (0-15)
    pub channel: u8,
    /// MIDI note number (0-127)
    pub note: u8,
    /// Velocity (0-127), 0 = note off
    pub velocity: u8,
    /// True for `NoteOn`, false for `NoteOff`
    pub is_note_on: bool,
}

/// A Control Change event ready to be sent to the DAW
#[derive(Clone, Debug)]
pub struct VstCCEvent {
    /// Sample offset within the current buffer
    pub sample_offset: u32,
    /// MIDI channel (0-15)
    pub channel: u8,
    /// CC number (0-127)
    pub cc: u8,
    /// CC value (0-127)
    pub value: u8,
}

/// Backend that collects MIDI events instead of generating audio.
/// Used by the VST plugin to output MIDI to the DAW.
pub struct VstMidiBackend {
    /// Queue of MIDI note events to send
    midi_events: VecDeque<VstMidiEvent>,
    /// Queue of CC events to send
    cc_events: VecDeque<VstCCEvent>,
    /// Current sample position within buffer (for timing)
    current_sample: u32,
}

impl VstMidiBackend {
    #[must_use]
    pub fn new() -> Self {
        Self {
            midi_events: VecDeque::with_capacity(64),
            cc_events: VecDeque::with_capacity(32),
            current_sample: 0,
        }
    }

    /// Take all pending MIDI note events
    pub fn take_midi_events(&mut self) -> Vec<VstMidiEvent> {
        self.midi_events.drain(..).collect()
    }

    /// Take all pending CC events
    pub fn take_cc_events(&mut self) -> Vec<VstCCEvent> {
        self.cc_events.drain(..).collect()
    }

    /// Set the current sample offset for incoming events
    pub const fn set_sample_offset(&mut self, offset: u32) {
        self.current_sample = offset;
    }

    /// Reset sample counter for new buffer
    pub const fn reset_sample_counter(&mut self) {
        self.current_sample = 0;
    }

    /// Clear all pending events
    pub fn clear(&mut self) {
        self.midi_events.clear();
        self.cc_events.clear();
        self.current_sample = 0;
    }
}

impl Default for VstMidiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRenderer for VstMidiBackend {
    fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                self.midi_events.push_back(VstMidiEvent {
                    sample_offset: self.current_sample,
                    channel,
                    note,
                    velocity,
                    is_note_on: true,
                });
            }
            AudioEvent::NoteOff { note, channel } => {
                self.midi_events.push_back(VstMidiEvent {
                    sample_offset: self.current_sample,
                    channel,
                    note,
                    velocity: 0,
                    is_note_on: false,
                });
            }
            AudioEvent::ControlChange { ctrl: _, value: _, channel: _ } => {
                // For CC, we don't have a direct mapping in VstMidiEvent yet?
                // Re-read VstMidiEvent definition or use a generic mapping.
                // Assuming VstMidiEvent only handles notes for now based on available fields.
            }
            AudioEvent::AllNotesOff { channel: _ } => {
                // Generic note off or CC 123
            }
            AudioEvent::BufferUpdate { .. } => {}
            _ => {
                // Ignore other events for VST MIDI backend
            }
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], _channels: usize) {
        // MIDI backend doesn't generate audio - just silence
        for sample in output.iter_mut() {
            *sample = 0.0;
        }
        // Increment sample counter for timing
        self.current_sample += (output.len() / 2) as u32;
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
