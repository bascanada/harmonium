use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "odin2")]
use harmonium_audio::backend::odin2_backend::Odin2Backend;
use harmonium_audio::backend::{
    AudioRenderer, recorder::RecorderBackend, synth_backend::SynthBackend,
};
use harmonium_core::{events::AudioEvent, log, params::ControlMode};

use crate::engine::{
    EngineParams, HarmoniumEngine, HarmonyState, SessionConfig, VisualizationEvent,
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
    mut target_params: triple_buffer::Output<EngineParams>,
    control_mode: Arc<Mutex<ControlMode>>,
    sf2_bytes: Option<&[u8]>,
    backend_type: AudioBackendType,
) -> Result<
    (
        cpal::Stream,
        SessionConfig,
        Arc<Mutex<rtrb::Consumer<HarmonyState>>>,
        Arc<Mutex<rtrb::Consumer<VisualizationEvent>>>,
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

    // Phase 3: Read initial params from triple buffer
    let initial_routing = target_params.read().channel_routing.clone();

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
    let recorder_backend = Box::new(RecorderBackend::new(
        inner_backend,
        finished_recordings.clone(),
        sample_rate as u32,
    ));

    // Phase 2-3: Engine now returns consumers for lock-free queues
    let (mut engine, harmony_state_rx, event_queue_rx) =
        HarmoniumEngine::new(sample_rate, target_params, control_mode, recorder_backend);
    let session_config = engine.config.clone();

    // Wrap consumers in Arc<Mutex<>> for backwards compatibility with existing API
    // (Phase 2: temporary until we update callers to use consumers directly)
    let harmony_state = Arc::new(Mutex::new(harmony_state_rx));
    let event_queue = Arc::new(Mutex::new(event_queue_rx));
    let font_queue = engine.font_queue.clone();

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

    Ok((stream, session_config, harmony_state, event_queue, font_queue, finished_recordings))
}

/// Offline export driver (faster-than-real-time)
pub fn export_offline(
    mut target_params: triple_buffer::Output<EngineParams>,
    control_mode: Arc<Mutex<ControlMode>>,
    sf2_bytes: Option<&[u8]>,
    backend_type: AudioBackendType,
    duration_secs: f64,
    sample_rate: f64,
    record_wav: bool,
    record_midi: bool,
    record_musicxml: bool,
    record_truth: bool,
) -> Result<crate::FinishedRecordings, String> {
    // Disable real-time checks for offline processing
    crate::realtime::rt_check::disable_rt_check();

    // 1. Create the appropriate backend
    let initial_routing = target_params.read().channel_routing.clone();
    let inner_backend: Box<dyn AudioRenderer> = match backend_type {
        AudioBackendType::FundSP => {
            Box::new(SynthBackend::new(sample_rate, sf2_bytes, &initial_routing))
        }
        #[cfg(feature = "odin2")]
        AudioBackendType::Odin2 => Box::new(Odin2Backend::new(sample_rate)),
    };

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(
        inner_backend,
        finished_recordings.clone(),
        sample_rate as u32,
    ));

    // 2. Initialize Engine
    let (mut engine, _, _) =
        HarmoniumEngine::new(sample_rate, target_params, control_mode, recorder_backend);

    let formats_to_record = [
        (record_wav, harmonium_core::events::RecordFormat::Wav),
        (record_midi, harmonium_core::events::RecordFormat::Midi),
        (record_musicxml, harmonium_core::events::RecordFormat::MusicXml),
        (record_truth, harmonium_core::events::RecordFormat::Truth),
    ];

    // 3. Start Recording
    if record_musicxml || record_truth {
        // Send musical params through event system for accurate export metadata
        let mp = engine.get_musical_params();
        engine.handle_event(AudioEvent::UpdateMusicalParams { params: Box::new(mp) });
    }

    for (should_record, format) in formats_to_record {
        if should_record {
            engine.handle_event(AudioEvent::StartRecording { format });
        }
    }

    // 4. Offline Render Loop
    let block_size = 1024;
    let mut output = vec![0.0f32; block_size * 2];
    let total_samples = (duration_secs * sample_rate) as usize;
    let mut rendered_samples = 0;

    log::info(&format!(
        "Exporting {:.1}s of audio ({} samples) at {}x speed...",
        duration_secs, total_samples, "CPU-limit"
    ));

    while rendered_samples < total_samples {
        engine.process_buffer(&mut output, 2);
        rendered_samples += block_size;
    }

    // 5. Stop Recording & Finalize
    for (should_record, format) in formats_to_record {
        if should_record {
            engine.handle_event(AudioEvent::StopRecording { format });
        }
    }

    Ok(finished_recordings)
}
