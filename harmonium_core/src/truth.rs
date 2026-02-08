use serde::{Deserialize, Serialize};

use crate::{events::AudioEvent, export::GitVersion, params::MusicalParams};

/// The "Ground Truth" of a recording session.
/// This structure captures everything needed to reconstruct or verify
/// the exported files (WAV, MIDI, MusicXML).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingTruth {
    /// Version of the engine used for this recording
    pub version: String,
    /// Git SHA of the engine
    pub git_sha: String,
    /// The musical parameters active during the recording
    pub params: MusicalParams,
    /// Sequential history of all musical events with step-based timestamps
    pub events: Vec<(f64, AudioEvent)>,
    /// Sample rate used during the recording
    pub sample_rate: u32,
}

impl RecordingTruth {
    /// Create a new recording truth from events and parameters
    pub fn new(events: Vec<(f64, AudioEvent)>, params: MusicalParams, sample_rate: u32) -> Self {
        let git = GitVersion::detect();
        Self { version: git.tag, git_sha: git.sha, params, events, sample_rate }
    }
}
