use harmonium_core::events::AudioEvent;

pub mod adapter;
pub mod midi_backend;
pub mod recorder;
pub mod synth_backend;
pub mod wav_backend;

#[cfg(feature = "vst")]
pub mod vst_midi_backend;

#[cfg(feature = "odin2")]
pub mod odin2_backend;

pub trait AudioRenderer: Send + Sync {
    /// Appelé à chaque tick logique (ex: changement de step)
    fn handle_event(&mut self, event: AudioEvent);

    /// Appelé pour générer l'audio par bloc
    /// output est un buffer entrelacé [L, R, L, R, ...]
    fn process_buffer(&mut self, output: &mut [f32], channels: usize);

    /// Allow downcasting to concrete types for emotional morphing
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Provides mutable access to `Odin2Backend` if present in chain
    /// Default implementation returns None for backends that don't wrap Odin2
    #[cfg(feature = "odin2")]
    fn odin2_backend_mut(&mut self) -> Option<&mut odin2_backend::Odin2Backend> {
        None
    }
}
