//! Score Generation Module
//!
//! Generates HarmoniumScore from engine state, synchronized with AudioEvents
//! via shared note IDs. This enables real-time score visualization that
//! highlights notes as they play.
//!
//! # Architecture
//! ```text
//! SymbolicState::tick()
//!        │
//!        │ (note_id shared)
//!        ├──────────────────┐
//!        ▼                  ▼
//!   AudioEvent         ScoreNoteEvent
//!   {id: 42, ...}      {id: 42, ...}
//!        │                  │
//!        ▼                  ▼
//!   Audio Buffer       Score Buffer
//!        │                  │
//!        ▼                  ▼
//!   Playback           VexFlow Render
//!   (note 42 plays)    (note 42 highlights)
//! ```

use harmonium_core::notation::{HarmoniumScore, ScoreNoteEvent};
// Re-export ScoreBuffer from harmonium_core
pub use harmonium_core::score_buffer::ScoreBuffer;

use crate::engine::SymbolicState;

/// Result of a single tick: both audio and score events with shared IDs
#[derive(Clone, Debug)]
pub struct TickResult {
    /// Audio events for playback
    pub audio_events: Vec<harmonium_core::events::AudioEvent>,
    /// Score events for visualization (same IDs as audio)
    pub score_events: Vec<ScoreNoteEvent>,
    /// Current step index
    pub step_idx: usize,
}

/// Score generator that wraps SymbolicState and generates synchronized events
pub struct ScoreGenerator {
    /// Score buffer
    pub buffer: ScoreBuffer,
    /// Default note duration in steps
    pub default_duration_steps: usize,
}

impl ScoreGenerator {
    /// Create a new score generator
    #[must_use]
    pub fn new(tempo: f32, time_signature: (u8, u8), key_root: u8, is_minor: bool) -> Self {
        Self {
            buffer: ScoreBuffer::new(tempo, time_signature, key_root, is_minor),
            default_duration_steps: 2, // Default to 1/8 note (2 steps at 16th resolution)
        }
    }

    /// Process a tick from SymbolicState and generate synchronized events
    pub fn process_tick(
        &mut self,
        state: &mut SymbolicState,
        samples_per_step: usize,
    ) -> TickResult {
        let (step_idx, mut audio_events) = state.tick(samples_per_step);

        // Generate score events with shared IDs
        let score_events =
            self.buffer.process_audio_events(&mut audio_events, self.default_duration_steps);

        // Advance the score buffer
        self.buffer.advance_step();

        TickResult { audio_events, score_events, step_idx }
    }

    /// Get the current score
    #[must_use]
    pub fn get_score(&self) -> &HarmoniumScore {
        self.buffer.get_score()
    }

    /// Get score as JSON
    #[must_use]
    pub fn to_json(&self) -> String {
        self.buffer.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_buffer_creation() {
        let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
        let score = buffer.get_score();
        assert_eq!(score.tempo, 120.0);
        assert_eq!(score.time_signature, (4, 4));
        assert_eq!(score.key_signature.root, "C");
        assert_eq!(score.parts.len(), 3);
    }

    #[test]
    fn test_audio_score_sync() {
        let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

        // Create audio events
        let mut events = vec![
            harmonium_core::events::AudioEvent::NoteOn {
                id: None,
                note: 60,
                velocity: 100,
                channel: 1,
            },
            harmonium_core::events::AudioEvent::NoteOn {
                id: None,
                note: 36,
                velocity: 90,
                channel: 0,
            },
        ];

        // Process and get score events
        let score_events = buffer.process_audio_events(&mut events, 2);

        // Verify IDs are assigned and match
        assert_eq!(score_events.len(), 2);

        // Check first event (lead note)
        if let harmonium_core::events::AudioEvent::NoteOn { id, .. } = &events[0] {
            assert_eq!(*id, Some(score_events[0].id));
        }

        // Check second event (bass note)
        if let harmonium_core::events::AudioEvent::NoteOn { id, .. } = &events[1] {
            assert_eq!(*id, Some(score_events[1].id));
        }
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
}
