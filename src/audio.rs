use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::engine::{HarmoniumEngine, SessionConfig, EngineParams, HarmonyState, VisualizationEvent};
use crate::backend::synth_backend::SynthBackend;
use crate::backend::recorder::RecorderBackend;
use crate::backend::AudioRenderer;
use crate::events::RecordFormat;
use crate::params::ControlMode;
use crate::log;
use std::sync::{Arc, Mutex};

#[cfg(feature = "odin2")]
use crate::backend::odin2_backend::Odin2Backend;

/// Available audio backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioBackendType {
    /// FundSP + Oxisynth backend (default)
    #[default]
    FundSP,
    /// Odin2 synthesizer backend
    #[cfg(feature = "odin2")]
    Odin2,
}

pub fn create_stream(
    target_state: Arc<Mutex<EngineParams>>,
    control_mode: Arc<Mutex<ControlMode>>,
    sf2_bytes: Option<&[u8]>,
    backend_type: AudioBackendType,
) -> Result<(cpal::Stream, SessionConfig, Arc<Mutex<rtrb::Consumer<HarmonyState>>>, Arc<Mutex<rtrb::Consumer<VisualizationEvent>>>, Arc<Mutex<Vec<(u32, Vec<u8>)>>>, Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>), String> {
    // 1. Setup CPAL
    let host = cpal::default_host();

    log::info(&format!("CPAL Host: {:?}", host.id()));

    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            log::warn("default_output_device() returned None. Trying to find any device...");
            let mut devices = host.output_devices().map_err(|e| format!("Failed to list devices: {:?}", e))?;
            if let Some(d) = devices.next() {
                log::info(&format!("Found fallback device: {}", d.name().unwrap_or("unknown".to_string())));
                d
            } else {
                return Err("No output devices found at all".into());
            }
        }
    };

    log::info(&format!("Output device: {}", device.name().unwrap_or("unknown".to_string())));

    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let sample_rate = config.sample_rate().0 as f64;
    let channels = config.channels() as usize;

    log::info(&format!("Sample rate: {}, Channels: {}", sample_rate, channels));

    let initial_routing = target_state.lock().unwrap().channel_routing.clone();

    // Create the appropriate backend based on backend_type
    let inner_backend: Box<dyn AudioRenderer> = match backend_type {
        AudioBackendType::FundSP => {
            log::info("Using FundSP/Oxisynth backend");
            Box::new(SynthBackend::new(sample_rate, sf2_bytes, &initial_routing))
        }
        #[cfg(feature = "odin2")]
        AudioBackendType::Odin2 => {
            log::info("Using Odin2 backend");
            Box::new(Odin2Backend::new(sample_rate))
        }
    };

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(inner_backend, finished_recordings.clone(), sample_rate as u32));

    // Phase 2: Engine now returns consumers for lock-free queues
    let (mut engine, harmony_state_rx, event_queue_rx) = HarmoniumEngine::new(sample_rate, target_state, control_mode, recorder_backend);
    let session_config = engine.config.clone();

    // Wrap consumers in Arc<Mutex<>> for backwards compatibility with existing API
    // (Phase 2: temporary until we update callers to use consumers directly)
    let harmony_state = Arc::new(Mutex::new(harmony_state_rx));
    let event_queue = Arc::new(Mutex::new(event_queue_rx));
    let font_queue = engine.font_queue.clone();

    let err_fn = |err| log::error(&format!("an error occurred on stream: {}", err));

    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            engine.process_buffer(data, channels);
        },
        err_fn,
        None,
    ).map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    Ok((stream, session_config, harmony_state, event_queue, font_queue, finished_recordings))
}
