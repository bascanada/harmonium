use crate::events::AudioEvent;

pub mod adapter;
pub mod synth_backend;
pub mod wav_backend;
pub mod midi_backend;
pub mod recorder;

pub trait AudioRenderer: Send + Sync {
    /// Appelé à chaque tick logique (ex: changement de step)
    fn handle_event(&mut self, event: AudioEvent);
    
    /// Appelé pour générer l'audio (si applicable)
    /// Retourne None si le backend ne gère pas l'audio (ex: export MIDI pur)
    fn next_frame(&mut self) -> Option<(f32, f32)>;
}
