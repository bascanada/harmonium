use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::engine::{HarmoniumEngine, SessionConfig, EngineParams, HarmonyState, VisualizationEvent};
use crate::backend::synth_backend::SynthBackend;
use crate::backend::recorder::RecorderBackend;
use crate::events::RecordFormat;
use crate::params::ControlMode;
use crate::log;
use std::sync::{Arc, Mutex};

pub fn create_stream(
    target_state: Arc<Mutex<EngineParams>>,
    control_mode: Arc<Mutex<ControlMode>>,
    sf2_bytes: Option<&[u8]>,
) -> Result<(cpal::Stream, SessionConfig, Arc<Mutex<HarmonyState>>, Arc<Mutex<Vec<VisualizationEvent>>>, Arc<Mutex<Vec<(u32, Vec<u8>)>>>, Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>), String> {
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
    let synth_backend = Box::new(SynthBackend::new(sample_rate, sf2_bytes, &initial_routing));

    let finished_recordings = Arc::new(Mutex::new(Vec::new()));
    let recorder_backend = Box::new(RecorderBackend::new(synth_backend, finished_recordings.clone(), sample_rate as u32));

    let mut engine = HarmoniumEngine::new(sample_rate, target_state, control_mode, recorder_backend);
    let session_config = engine.config.clone();
    let harmony_state = engine.harmony_state.clone(); // Cloner l'Arc pour le retourner
    let event_queue = engine.event_queue.clone(); // Cloner l'Arc pour le retourner
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
