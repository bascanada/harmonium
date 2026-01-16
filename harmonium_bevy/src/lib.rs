use bevy::prelude::*;
use kira::{AudioManager, AudioManagerSettings, backend::DefaultBackend};
use rtrb::RingBuffer;
use cpal::traits::{DeviceTrait, HostTrait};
use harmonium_core::MusicKernel;
use harmonium_core::sequencer::Sequencer;
use harmonium_core::params::MusicalParams;
use harmonium_ai::EmotionMapper;
use harmonium_core::EngineParams;
use harmonium_core::events::AudioEvent;

// Re-export core for easy access
pub use harmonium_core;

pub mod components;
pub mod assets;
mod sound;
mod systems;

use sound::HarmoniumSoundData;
use assets::{OdinAsset, OdinAssetLoader};

/// Main Resource accessible in Bevy systems
#[derive(Resource)]
pub struct Harmonium {
    pub kernel: MusicKernel,
    pub mapper: EmotionMapper,
    /// The "Manual" parameters (e.g. from UI) that serve as a base for mixing
    pub params: EngineParams, 
    
    // Channel to audio thread
    pub event_producer: std::sync::Mutex<rtrb::Producer<AudioEvent>>,
}

/// Gets the system's default audio sample rate using cpal.
/// Falls back to 48000 Hz if unable to query the system.
fn get_system_sample_rate() -> u32 {
    match cpal::default_host().default_output_device() {
        Some(device) => {
            match device.default_output_config() {
                Ok(config) => config.sample_rate().0,
                Err(e) => {
                    warn!("Failed to get default audio config: {}. Using 48000 Hz.", e);
                    48000
                }
            }
        }
        None => {
            warn!("No default output device found. Using 48000 Hz.");
            48000
        }
    }
}

pub struct HarmoniumPlugin;

impl Plugin for HarmoniumPlugin {
    fn build(&self, app: &mut App) {
        // 1. Initialize Kira
        let mut audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("Failed to init Kira");

        // 2. Create communication channel (RingBuffer)
        let (producer, consumer) = RingBuffer::new(1024);

        // 3. Launch Harmonium sound in Kira (runs indefinitely on audio thread)
        // Query the system's actual sample rate using cpal
        let sample_rate = get_system_sample_rate();
        let sound_data = HarmoniumSoundData {
            sample_rate,
            event_consumer: consumer,
            settings: Default::default(),
        };
        
        audio_manager.play(sound_data).expect("Failed to play Harmonium sound");

        // 4. Initialize Logic (Kernel + AI)
        // Sequencer: 16 steps, Euclidean default
        let sequencer = Sequencer::new(16, 4, 120.0);
        let params = MusicalParams::default(); // Ensure MusicalParams derives Default in core
        let kernel = MusicKernel::new(sequencer, params);
        
        let mapper = EmotionMapper::new();

        // 5. Register types and systems
        app
            .init_asset::<OdinAsset>()
            .init_asset_loader::<OdinAssetLoader>()
            .register_type::<components::HarmoniumSource>()
            .register_type::<components::GenerativeConfig>()
            .register_type::<components::OdinConfig>()
            .register_type::<harmonium_core::sequencer::RhythmMode>() 
            .register_type::<components::HarmoniumTag>()
            .register_type::<components::AiDriver>()

            .insert_resource(Harmonium {
                kernel,
                mapper,
                params: EngineParams::default(), // This is the "Emotional" params
                event_producer: std::sync::Mutex::new(producer),
            })
            // IMPORTANT: Keep Kira manager alive
            .insert_resource(HarmoniumAudioKeepAlive(audio_manager))
            
            .add_systems(Update, (
                update_harmonium_main_system,
                systems::sync_harmonium_params.after(update_harmonium_main_system),
                systems::scan_environment_system
            ));
    }
}

// Wrapper to keep Kira alive
#[derive(Resource)]
struct HarmoniumAudioKeepAlive(#[allow(dead_code)] AudioManager);

/// The system that advances music logic synced with game time
fn update_harmonium_main_system(
    time: Res<Time>, 
    mut harmonium: ResMut<Harmonium>,
    query: Query<&components::HarmoniumSource>,
) {
    // Check enable state from source of truth
    if let Some(source) = query.iter().next() {
        if !source.is_enabled {
            return;
        }
    }

    // 1. AI: Map Emotion -> Musical Parameters
    // DISABLED for Manual Control Test
    // Map the CURRENT emotional params (harmonium.params) to musical params
    // let mapped_musical_params = harmonium.mapper.map(&harmonium.params);
    // harmonium.kernel.params = mapped_musical_params; 
    
    // 2. Core: Advance sequencer
    let dt = time.delta_secs_f64();
    let events = harmonium.kernel.update(dt);

    // 3. Send events to Kira
    if let Ok(mut producer) = harmonium.event_producer.lock() {
        for event in events {
            if producer.push(event).is_err() {
                warn!("Harmonium event buffer full!");
            }
        }
    }
}
