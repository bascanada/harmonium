use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "odin2")]
use harmonium_audio::backend::odin2_backend::Odin2Backend;
use harmonium_audio::backend::{
    AudioRenderer, recorder::RecorderBackend, synth_backend::SynthBackend,
};
use harmonium_core::{log, HarmoniumController};

use crate::timeline_engine::TimelineEngine;
use harmonium_core::params::SessionConfig;

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

/// Create a timeline-based audio stream for real-time playback.
///
/// Uses the Writehead/Playhead separation for seekable, replayable output.
#[allow(clippy::type_complexity)]
pub fn create_timeline_stream(
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
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or_else(|| "No output device found".to_string())?;

    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let sample_rate = config.sample_rate().0 as f64;
    let channels = config.channels() as usize;

    log::info(&format!(
        "Timeline Engine - Sample rate: {}, Channels: {}",
        sample_rate, channels
    ));

    let (command_tx, command_rx) = rtrb::RingBuffer::<harmonium_core::EngineCommand>::new(1024);
    let (report_tx, report_rx) = rtrb::RingBuffer::<harmonium_core::EngineReport>::new(256);

    let default_routing = vec![0, 1, 2, 3];

    let inner_backend: Box<dyn AudioRenderer> = match backend_type {
        AudioBackendType::FundSP => {
            Box::new(SynthBackend::new(sample_rate, sf2_bytes, &default_routing))
        }
        #[cfg(feature = "odin2")]
        AudioBackendType::Odin2 => {
            Box::new(Odin2Backend::new(sample_rate))
        }
    };

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(
        inner_backend,
        finished_recordings.clone(),
        sample_rate as u32,
    ));

    let mut engine = TimelineEngine::new(sample_rate, command_rx, report_tx, recorder_backend);
    let session_config = engine.config.clone();
    let font_queue = engine.font_queue.clone();

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

/// Create a timeline engine for offline (non-realtime) rendering.
///
/// No audio device is opened. The caller drives `engine.process_buffer()`
/// in a tight loop to render as fast as possible.
#[allow(clippy::type_complexity)]
pub fn create_offline_engine(
    sf2_bytes: Option<&[u8]>,
    backend_type: AudioBackendType,
    sample_rate: f64,
) -> Result<
    (
        TimelineEngine,
        harmonium_core::HarmoniumController,
        crate::FinishedRecordings,
    ),
    String,
> {
    let (command_tx, command_rx) = rtrb::RingBuffer::<harmonium_core::EngineCommand>::new(1024);
    let (report_tx, report_rx) = rtrb::RingBuffer::<harmonium_core::EngineReport>::new(256);

    let default_routing = vec![0, 1, 2, 3];

    let inner_backend: Box<dyn AudioRenderer> = match backend_type {
        AudioBackendType::FundSP => {
            Box::new(SynthBackend::new(sample_rate, sf2_bytes, &default_routing))
        }
        #[cfg(feature = "odin2")]
        AudioBackendType::Odin2 => {
            Box::new(Odin2Backend::new(sample_rate))
        }
    };

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(
        inner_backend,
        finished_recordings.clone(),
        sample_rate as u32,
    ));

    let mut engine = TimelineEngine::new(sample_rate, command_rx, report_tx, recorder_backend);
    engine.set_offline(true);
    let controller = HarmoniumController::new(command_tx, report_rx);

    Ok((engine, controller, finished_recordings))
}
