use serde::{Deserialize, Serialize};

use crate::params::MusicalParams;

/// Audio events for playback and recording
///
/// NoteOn and NoteOff events include an optional `id` field that links
/// them to corresponding ScoreNoteEvents for synchronized visualization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioEvent {
    NoteOn {
        /// Unique note identifier for audio/score synchronization
        /// If None, the note is not tracked for visualization
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        note: u8,
        velocity: u8,
        channel: u8,
    },
    NoteOff {
        /// References the corresponding NoteOn id
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        note: u8,
        channel: u8,
    },
    ControlChange {
        ctrl: u8,
        value: u8,
        channel: u8,
    },
    /// Coupe toutes les notes sur un channel (CC 123)
    AllNotesOff {
        channel: u8,
    },
    #[serde(skip)]
    LoadFont {
        id: u32,
        bytes: Vec<u8>,
    },
    /// Load an Odin 2 preset from raw bytes
    #[serde(skip)]
    LoadOdinPreset {
        channel: u8,
        bytes: Vec<u8>,
    },
    SetChannelRoute {
        channel: u8,
        bank: i32,
    }, // -1 = FundSP, >=0 = Oxisynth Bank
    TimingUpdate {
        samples_per_step: usize,
    },
    /// Send musical parameters (key, time signature, etc.) to recorder
    UpdateMusicalParams {
        params: Box<MusicalParams>,
    },
    StartRecording {
        format: RecordFormat,
    },
    StopRecording {
        format: RecordFormat,
    },
    /// Set mixer gains for each instrument (0.0-1.0)
    SetMixerGains {
        lead: f32,
        bass: f32,
        snare: f32,
        hat: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordFormat {
    Wav,
    Midi,
    MusicXml,
    Truth,
}
