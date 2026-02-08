use crate::params::MusicalParams;

#[derive(Clone, Debug)]
pub enum AudioEvent {
    NoteOn {
        note: u8,
        velocity: u8,
        channel: u8,
    },
    NoteOff {
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
    LoadFont {
        id: u32,
        bytes: Vec<u8>,
    },
    /// Load an Odin 2 preset from raw bytes
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordFormat {
    Wav,
    Midi,
    MusicXml,
}
