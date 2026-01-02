use crate::events::AudioEvent;

pub mod adapter;
pub mod synth_backend;
pub mod wav_backend;
pub mod midi_backend;
pub mod abc_backend;
pub mod recorder;

pub trait AudioRenderer: Send + Sync {
    /// Appelé à chaque tick logique (ex: changement de step)
    fn handle_event(&mut self, event: AudioEvent);
    
    /// Appelé pour générer l'audio par bloc
    /// output est un buffer entrelacé [L, R, L, R, ...]
    fn process_buffer(&mut self, output: &mut [f32], channels: usize);
}
