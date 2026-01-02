
#[derive(Clone, Debug)]
pub enum AudioEvent {
    NoteOn { note: u8, velocity: u8, channel: u8 },
    NoteOff { note: u8, channel: u8 },
    ControlChange { ctrl: u8, value: u8 },
}
