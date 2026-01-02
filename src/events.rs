
#[derive(Clone, Debug)]
pub enum AudioEvent {
    NoteOn { note: u8, velocity: u8, channel: u8 },
    NoteOff { note: u8, channel: u8 },
    ControlChange { ctrl: u8, value: u8 },
    LoadFont { id: u32, bytes: Vec<u8> },
    SetChannelRoute { channel: u8, bank: i32 }, // -1 = FundSP, >=0 = Oxisynth Bank
    TimingUpdate { samples_per_step: usize },
    StartRecording { format: RecordFormat },
    StopRecording { format: RecordFormat },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecordFormat {
    Wav,
    Midi,
    Abc,
}
