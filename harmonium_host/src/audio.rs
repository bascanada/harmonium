use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "odin2")]
use harmonium_audio::backend::odin2_backend::Odin2Backend;
use harmonium_audio::backend::{
    AudioRenderer, recorder::RecorderBackend, synth_backend::SynthBackend,
};
use harmonium_core::{log, EngineReport, HarmoniumController};

use crate::engine::{
    HarmoniumEngine, HarmonyState, SessionConfig, VisualizationEvent,
};

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

#[allow(clippy::type_complexity)]
pub fn create_stream(
    sf2_bytes: Option<&[u8]>,
    backend_type: AudioBackendType,
) -> Result<
    (
        cpal::Stream,
        SessionConfig,
        harmonium_core::HarmoniumController,
        crate::FontQueue,
        crate::FinishedRecordings,
    ),
    String,
> {
    // 1. Setup CPAL
    let host = cpal::default_host();

    log::info(&format!("CPAL Host: {:?}", host.id()));

    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            log::warn("default_output_device() returned None. Trying to find any device...");
            let mut devices =
                host.output_devices().map_err(|e| format!("Failed to list devices: {:?}", e))?;
            if let Some(d) = devices.next() {
                log::info(&format!(
                    "Found fallback device: {}",
                    d.name().unwrap_or("unknown".to_string())
                ));
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

    // Create command/report queues for engine communication
    let (command_tx, command_rx) = rtrb::RingBuffer::<harmonium_core::EngineCommand>::new(1024);
    let (report_tx, report_rx) = rtrb::RingBuffer::<harmonium_core::EngineReport>::new(256);

    // Default channel routing (will be updated via commands)
    let default_routing = vec![0, 1, 2, 3];

    // Create the appropriate backend based on backend_type
    let inner_backend: Box<dyn AudioRenderer> = match backend_type {
        AudioBackendType::FundSP => {
            log::info("Using FundSP/Oxisynth backend");
            Box::new(SynthBackend::new(sample_rate, sf2_bytes, &default_routing))
        }
        #[cfg(feature = "odin2")]
        AudioBackendType::Odin2 => {
            log::info("Using Odin2 backend");
            Box::new(Odin2Backend::new(sample_rate))
        }
    };

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(
        inner_backend,
        finished_recordings.clone(),
        sample_rate as u32,
    ));

    // Create engine with new command/report queue architecture
    let mut engine = HarmoniumEngine::new(sample_rate, command_rx, report_tx, recorder_backend);
    let session_config = engine.config.clone();
    let font_queue = engine.font_queue.clone();

    // Create controller for external use
    let controller = HarmoniumController::new(command_tx, report_rx);

    let err_fn = |err| log::error(&format!("an error occurred on stream: {}", err));

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                engine.process_buffer(data, channels);
            },
            err_fn,
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    Ok((stream, session_config, controller, font_queue, finished_recordings))
}
