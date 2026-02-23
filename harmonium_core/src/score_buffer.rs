//! Score Buffer Module
//!
//! Accumulates ScoreNoteEvents during recording/playback for export
//! to HarmoniumScore format. This module lives in harmonium_core to
//! be accessible from both harmonium_host and harmonium_audio.

use crate::events::AudioEvent;
use crate::notation::{
    ChordSymbol, Clef, Dynamic, HarmoniumScore, KeySignature, Measure, NoteEventType, Part,
    ScoreNoteEvent, midi_to_pitch, next_note_id, steps_to_duration,
};

/// Score buffer for accumulating generated measures
#[derive(Clone, Debug)]
pub struct ScoreBuffer {
    /// The full score being built
    score: HarmoniumScore,
    /// Current measure being filled
    current_measure_number: usize,
    /// Step counter within current measure
    step_in_measure: usize,
    /// Steps per measure (based on time signature)
    steps_per_measure: usize,
    /// Steps per quarter note (for duration calculation)
    steps_per_quarter: usize,
    /// Key fifths for pitch spelling
    key_fifths: i8,
    /// Current beat position in measure
    current_beat: f32,
    /// Temporary storage for measure events by part
    measure_events: Vec<Vec<ScoreNoteEvent>>,
    /// Temporary storage for chord symbols
    measure_chords: Vec<ChordSymbol>,
}

impl ScoreBuffer {
    /// Create a new score buffer with initial configuration
    #[must_use]
    pub fn new(tempo: f32, time_signature: (u8, u8), key_root: u8, is_minor: bool) -> Self {
        let key_signature = KeySignature::from_pitch_class(key_root, is_minor);
        let key_fifths = key_signature.fifths;

        // Calculate steps per measure (assuming 4 steps per beat/quarter)
        let beats_per_measure = time_signature.0 as f32;
        let steps_per_quarter = 4; // 16th note resolution
        let steps_per_measure = (beats_per_measure * steps_per_quarter as f32) as usize;

        let mut score = HarmoniumScore::default();
        score.tempo = tempo;
        score.time_signature = time_signature;
        score.key_signature = key_signature;

        // Initialize parts (lead, bass, drums)
        score.parts = vec![
            Part {
                id: "lead".to_string(),
                name: "Lead".to_string(),
                clef: Clef::Treble,
                transposition: None,
                measures: Vec::new(),
            },
            Part {
                id: "bass".to_string(),
                name: "Bass".to_string(),
                clef: Clef::Bass,
                transposition: None,
                measures: Vec::new(),
            },
            Part {
                id: "drums".to_string(),
                name: "Drums".to_string(),
                clef: Clef::Percussion,
                transposition: None,
                measures: Vec::new(),
            },
        ];

        // Initialize measure event storage for each part
        let measure_events = vec![Vec::new(); 3];

        Self {
            score,
            current_measure_number: 1,
            step_in_measure: 0,
            steps_per_measure,
            steps_per_quarter,
            key_fifths,
            current_beat: 1.0,
            measure_events,
            measure_chords: Vec::new(),
        }
    }

    /// Update tempo
    pub fn set_tempo(&mut self, tempo: f32) {
        self.score.tempo = tempo;
    }

    /// Update key signature
    pub fn set_key(&mut self, key_root: u8, is_minor: bool) {
        let key_signature = KeySignature::from_pitch_class(key_root, is_minor);
        self.key_fifths = key_signature.fifths;
        self.score.key_signature = key_signature;
    }

    /// Add a chord symbol at the current position
    pub fn add_chord(&mut self, root: String, quality: String, duration_beats: f32) {
        self.measure_chords.push(ChordSymbol {
            beat: self.current_beat,
            duration: duration_beats,
            root,
            quality,
            bass: None,
            scale: None,
        });
    }

    /// Add a note event to the appropriate part
    fn add_event(&mut self, event: ScoreNoteEvent, channel: u8) {
        let part_idx = match channel {
            0 => 1, // Bass
            1 => 0, // Lead
            2 | 3 => 2, // Drums (snare, hat)
            _ => 0,
        };

        if part_idx < self.measure_events.len() {
            self.measure_events[part_idx].push(event);
        }
    }

    /// Advance by one step and finalize measure if needed
    pub fn advance_step(&mut self) {
        self.step_in_measure += 1;
        self.current_beat = 1.0 + (self.step_in_measure as f32 / self.steps_per_quarter as f32);

        // Check if we've completed a measure
        if self.step_in_measure >= self.steps_per_measure {
            self.finalize_measure();
            self.step_in_measure = 0;
            self.current_beat = 1.0;
            self.current_measure_number += 1;
        }
    }

    /// Finalize the current measure and add it to all parts
    fn finalize_measure(&mut self) {
        for (part_idx, events) in self.measure_events.iter_mut().enumerate() {
            let measure = Measure {
                number: self.current_measure_number,
                time_signature: None,
                key_signature: None,
                events: std::mem::take(events),
                chords: if part_idx == 0 {
                    std::mem::take(&mut self.measure_chords)
                } else {
                    Vec::new()
                },
            };
            if part_idx < self.score.parts.len() {
                self.score.parts[part_idx].measures.push(measure);
            }
        }
        // Reset chord storage (already taken for lead part)
        self.measure_chords.clear();
    }

    /// Finalize any remaining events (call at end of recording)
    pub fn finalize(&mut self) {
        // If there are any events in the current measure, finalize it
        let has_events = self.measure_events.iter().any(|e| !e.is_empty())
            || !self.measure_chords.is_empty();
        if has_events || self.step_in_measure > 0 {
            self.finalize_measure();
        }
    }

    /// Get the current score
    #[must_use]
    pub fn get_score(&self) -> &HarmoniumScore {
        &self.score
    }

    /// Get a clone of the current score (for serialization)
    #[must_use]
    pub fn clone_score(&self) -> HarmoniumScore {
        self.score.clone()
    }

    /// Get score as JSON
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.score).unwrap_or_default()
    }

    /// Get score as pretty JSON
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.score).unwrap_or_default()
    }

    /// Process audio events and generate corresponding score events with shared IDs
    pub fn process_audio_events(
        &mut self,
        events: &mut Vec<AudioEvent>,
        duration_steps: usize,
    ) -> Vec<ScoreNoteEvent> {
        let mut score_events = Vec::new();

        for event in events.iter_mut() {
            if let AudioEvent::NoteOn {
                id,
                note,
                velocity,
                channel,
            } = event
            {
                // Generate shared note ID
                let note_id = next_note_id();
                *id = Some(note_id);

                // Create corresponding score event
                let duration = steps_to_duration(duration_steps, self.steps_per_quarter);
                let dynamic = Some(Dynamic::from_velocity(*velocity));

                let (event_type, pitches) = if *channel == 2 || *channel == 3 {
                    // Drum note
                    (NoteEventType::Drum, Vec::new())
                } else {
                    // Melodic note
                    let pitch = midi_to_pitch(*note, self.key_fifths);
                    (NoteEventType::Note, vec![pitch])
                };

                let score_event = ScoreNoteEvent {
                    id: note_id,
                    beat: self.current_beat,
                    event_type,
                    pitches,
                    duration,
                    dynamic,
                    articulation: None,
                };

                self.add_event(score_event.clone(), *channel);
                score_events.push(score_event);
            }
        }

        score_events
    }

    /// Process a single audio event and return the corresponding score event if applicable
    pub fn process_single_event(
        &mut self,
        event: &mut AudioEvent,
        duration_steps: usize,
    ) -> Option<ScoreNoteEvent> {
        if let AudioEvent::NoteOn {
            id,
            note,
            velocity,
            channel,
        } = event
        {
            // Generate shared note ID
            let note_id = next_note_id();
            *id = Some(note_id);

            // Create corresponding score event
            let duration = steps_to_duration(duration_steps, self.steps_per_quarter);
            let dynamic = Some(Dynamic::from_velocity(*velocity));

            let (event_type, pitches) = if *channel == 2 || *channel == 3 {
                // Drum note
                (NoteEventType::Drum, Vec::new())
            } else {
                // Melodic note
                let pitch = midi_to_pitch(*note, self.key_fifths);
                (NoteEventType::Note, vec![pitch])
            };

            let score_event = ScoreNoteEvent {
                id: note_id,
                beat: self.current_beat,
                event_type,
                pitches,
                duration,
                dynamic,
                articulation: None,
            };

            self.add_event(score_event.clone(), *channel);
            Some(score_event)
        } else {
            None
        }
    }

    /// Get current measure number
    #[must_use]
    pub fn current_measure(&self) -> usize {
        self.current_measure_number
    }

    /// Get total number of completed measures
    #[must_use]
    pub fn completed_measures(&self) -> usize {
        if self.score.parts.is_empty() {
            0
        } else {
            self.score.parts[0].measures.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_buffer_creation() {
        let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
        assert_eq!(buffer.score.tempo, 120.0);
        assert_eq!(buffer.score.time_signature, (4, 4));
        assert_eq!(buffer.score.key_signature.root, "C");
        assert_eq!(buffer.score.parts.len(), 3);
    }

    #[test]
    fn test_measure_finalization() {
        let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

        // Advance through a full measure (16 steps for 4/4 time)
        for _ in 0..16 {
            buffer.advance_step();
        }

        // Should have one completed measure
        assert_eq!(buffer.completed_measures(), 1);
        assert_eq!(buffer.current_measure(), 2);
    }

    #[test]
    fn test_score_json_output() {
        let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
        let json = buffer.to_json();

        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"tempo\":120.0"));
        assert!(json.contains("\"lead\""));
        assert!(json.contains("\"bass\""));
        assert!(json.contains("\"drums\""));
    }

    #[test]
    fn test_audio_event_processing() {
        let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

        let mut events = vec![AudioEvent::NoteOn {
            id: None,
            note: 60,
            velocity: 100,
            channel: 1,
        }];

        let score_events = buffer.process_audio_events(&mut events, 2);

        assert_eq!(score_events.len(), 1);
        // Verify ID was assigned
        if let AudioEvent::NoteOn { id, .. } = &events[0] {
            assert!(id.is_some());
            assert_eq!(*id, Some(score_events[0].id));
        }
    }
}
