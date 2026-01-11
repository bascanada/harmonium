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
    // Add other state as needed
}

impl MusicKernel {
    pub fn new(sequencer: CoreSequencer, params: CoreMusicalParams) -> Self {
        Self { sequencer, params }
    }

    pub fn update(&mut self, dt: f64) -> Vec<CoreAudioEvent> {
        let mut events = Vec::new();
        // Logic will be moved here from legacy engine
        // For now, this is a placeholder
        events
    }
}
