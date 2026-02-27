//! Monophonic Voicer - Single-note instruments (winds, brass, voice)
//!
//! This voicer simply returns the melody note without any harmony.
//! Used for instruments that cannot play chords, such as:
//! - Wind instruments (saxophone, clarinet, flute, oboe)
//! - Brass instruments (trumpet, trombone, horn)
//! - Voice (vocals)

use super::voicer::{VoicedNote, Voicer, VoicerContext};

/// Monophonic voicer that returns only the melody note
///
/// Used for single-note instruments that cannot play chords.
/// Simply passes through the melody note without any voicing.
#[derive(Clone)]
pub struct MonophonicVoicer {
    /// Note duration in steps
    note_duration: u8,
}

impl Default for MonophonicVoicer {
    fn default() -> Self {
        Self::new()
    }
}

impl MonophonicVoicer {
    /// Creates a new `MonophonicVoicer`
    #[must_use]
    pub fn new() -> Self {
        Self {
            note_duration: 2, // Default: 2 steps (8th note at 4 steps per beat)
        }
    }

    /// Sets the note duration in steps
    pub fn set_note_duration(&mut self, duration: u8) {
        self.note_duration = duration.max(1);
    }
}

impl Voicer for MonophonicVoicer {
    fn clone_box(&self) -> Box<dyn Voicer> {
        Box::new(self.clone())
    }

    fn name(&self) -> &'static str {
        "Monophonic (Single Note)"
    }

    fn process_note(
        &mut self,
        melody_note: u8,
        base_velocity: u8,
        _ctx: &VoicerContext,
    ) -> Vec<VoicedNote> {
        // Simply return the melody note without any voicing
        vec![VoicedNote {
            midi: melody_note,
            velocity: base_velocity,
            duration_steps: self.note_duration,
        }]
    }

    fn on_step(&mut self, _ctx: &VoicerContext) {
        // No state to update for monophonic voicing
    }

    fn should_voice(&self, _ctx: &VoicerContext) -> bool {
        // Always voice (output) melody notes
        true
    }

    fn on_density_change(&mut self, _new_density: f32, _steps: usize) {
        // Density doesn't affect monophonic voicing
    }
}

#[cfg(test)]
mod tests {
    use harmonium_core::harmony::chord::ChordType;

    use super::*;

    #[test]
    fn test_monophonic_voicing() {
        let mut voicer = MonophonicVoicer::new();

        let ctx = VoicerContext {
            chord_root_midi: 60, // C4
            chord_type: ChordType::Major7,
            lcc_scale: vec![0, 2, 4, 5, 7, 9, 11],
            tension: 0.5,
            density: 0.5,
            current_step: 0,
            total_steps: 16,
        };

        // G5 (MIDI 79) as melody note
        let voiced = voicer.process_note(79, 100, &ctx);

        // Should return only 1 note - the melody
        assert_eq!(voiced.len(), 1);
        assert_eq!(voiced[0].midi, 79);
        assert_eq!(voiced[0].velocity, 100);
    }

    #[test]
    fn test_monophonic_preserves_velocity() {
        let mut voicer = MonophonicVoicer::new();
        let ctx = VoicerContext::default();

        let voiced = voicer.process_note(72, 80, &ctx);

        assert_eq!(voiced[0].velocity, 80);
    }

    #[test]
    fn test_monophonic_custom_duration() {
        let mut voicer = MonophonicVoicer::new();
        voicer.set_note_duration(4);

        let ctx = VoicerContext::default();
        let voiced = voicer.process_note(72, 100, &ctx);

        assert_eq!(voiced[0].duration_steps, 4);
    }

    #[test]
    fn test_monophonic_always_voices() {
        let voicer = MonophonicVoicer::new();
        let ctx = VoicerContext::default();

        // Monophonic voicer should always return true
        assert!(voicer.should_voice(&ctx));
    }
}
