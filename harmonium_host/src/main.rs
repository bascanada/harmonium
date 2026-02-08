use std::{
    env, fs,
    net::UdpSocket,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use harmonium::{audio, audio::AudioBackendType, engine::EngineParams, harmony::HarmonyMode, log};
use rosc::{OscPacket, OscType};

fn save_finished_recordings(
    finished_recordings: &harmonium::FinishedRecordings,
    record_wav: &Option<String>,
    record_midi: &Option<String>,
    record_musicxml: &Option<String>,
    record_truth: &Option<String>,
) -> usize {
    let mut saved_count = 0;
    if let Ok(mut queue) = finished_recordings.lock() {
        while let Some((fmt, data)) = queue.pop() {
            let filename = match fmt {
                harmonium::events::RecordFormat::Wav => {
                    record_wav.as_deref().unwrap_or("output.wav")
                }
                harmonium::events::RecordFormat::Midi => {
                    record_midi.as_deref().unwrap_or("output.mid")
                }
                harmonium::events::RecordFormat::MusicXml => {
                    record_musicxml.as_deref().unwrap_or("output.musicxml")
                }
                harmonium::events::RecordFormat::Truth => {
                    record_truth.as_deref().unwrap_or("output.truth.json")
                }
            };

            log::info(&format!(
                "Saving {} recording to {} ({} bytes)",
                match fmt {
                    harmonium::events::RecordFormat::Wav => "WAV",
                    harmonium::events::RecordFormat::Midi => "MIDI",
                    harmonium::events::RecordFormat::MusicXml => "MusicXML",
                    harmonium::events::RecordFormat::Truth => "Truth",
                },
                filename,
                data.len()
            ));

            if let Err(e) = fs::write(filename, &data) {
                log::warn(&format!("Failed to write {filename}: {e}"));
            } else {
                saved_count += 1;
            }
        }
    }
    saved_count
}

fn perform_graceful_shutdown(
    target_params_input: &Arc<Mutex<triple_buffer::Input<EngineParams>>>,
    finished_recordings: &harmonium::FinishedRecordings,
    record_wav: &Option<String>,
    record_midi: &Option<String>,
    record_musicxml: &Option<String>,
    record_truth: &Option<String>,
) -> bool {
    // Step 1: Calculate expected number of recordings
    let expected_recordings = i32::from(record_wav.is_some())
        + i32::from(record_midi.is_some())
        + i32::from(record_musicxml.is_some())
        + i32::from(record_truth.is_some());

    if expected_recordings == 0 {
        return true; // Nothing to save
    }

    log::info(&format!("Stopping {expected_recordings} recording(s)..."));

    // Step 2: Mute all channels to stop new note generation
    if let Ok(mut input) = target_params_input.lock() {
        let mut params = input.input_buffer_mut().clone();
        params.muted_channels = vec![true; 16];
        params.record_wav = record_wav.is_some();
        params.record_midi = record_midi.is_some();
        params.record_musicxml = record_musicxml.is_some();
        params.record_truth = record_truth.is_some();
        input.write(params);
        log::info("Muted all channels to stop new note generation");
    }

    std::thread::sleep(Duration::from_millis(200));

    log::info("Waiting for playing notes to finish...");
    std::thread::sleep(Duration::from_millis(3000));

    // Step 4: Now stop recordings
    if let Ok(mut input) = target_params_input.lock() {
        let mut params = input.input_buffer_mut().clone();
        params.record_wav = false;
        params.record_midi = false;
        params.record_musicxml = false;
        params.record_truth = false;
        input.write(params);
        log::info("Recording stop signal sent to audio thread");
    }

    log::info("Waiting for recordings to finalize...");
    std::thread::sleep(Duration::from_millis(1500));

    let mut saved_count = 0;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 50; // 5 seconds total

    log::info("Polling for finalized recordings...");
    while iterations < MAX_ITERATIONS && saved_count < expected_recordings as usize {
        saved_count += save_finished_recordings(
            finished_recordings,
            record_wav,
            record_midi,
            record_musicxml,
            record_truth,
        );

        if saved_count >= expected_recordings as usize {
            log::info("All recordings saved successfully");
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
        iterations += 1;
    }

    if saved_count < expected_recordings as usize {
        log::warn(&format!("Timeout: Only saved {saved_count}/{expected_recordings} recordings"));
    }

    saved_count >= expected_recordings as usize
}

fn main() {
    log::info("Harmonium - Procedural Music Generator");

    // === 0. Parse Arguments ===
    let args: Vec<String> = env::args().collect();
    let mut sf2_path: Option<String> = None;
    let mut record_wav: Option<String> = None;
    let mut record_midi: Option<String> = None;
    let mut record_musicxml: Option<String> = None;
    let mut record_truth: Option<String> = None;
    let mut use_osc = false;
    let mut use_export = false;
    let mut duration_secs = 0; // 0 = infini
    let mut harmony_mode = HarmonyMode::Driver;
    let mut poly_steps: usize = 48;
    #[cfg(feature = "odin2")]
    let mut backend_type = AudioBackendType::Odin2;
    #[cfg(not(feature = "odin2"))]
    let mut backend_type = AudioBackendType::FundSP;
    let mut fixed_kick = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record-wav" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    record_wav = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    record_wav = Some("output.wav".to_string());
                }
            }
            "--record-midi" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    record_midi = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    record_midi = Some("output.mid".to_string());
                }
            }
            "--record-musicxml" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    record_musicxml = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    record_musicxml = Some("output.musicxml".to_string());
                }
            }
            "--record-truth" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    record_truth = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    record_truth = Some("output.truth.json".to_string());
                }
            }
            "--osc" => use_osc = true,
            "--export" => use_export = true,
            "--drum-kit" | "--fixed-kick" => fixed_kick = true,
            "--harmony-mode" | "-m" => {
                if i + 1 < args.len() {
                    harmony_mode = match args[i + 1].to_lowercase().as_str() {
                        "basic" => HarmonyMode::Basic,
                        "driver" => HarmonyMode::Driver,
                        _ => {
                            log::warn(&format!(
                                "Unknown harmony mode '{}', using Driver",
                                args[i + 1]
                            ));
                            HarmonyMode::Driver
                        }
                    };
                    i += 1;
                }
            }
            "--duration" => {
                if i + 1 < args.len()
                    && let Ok(d) = args[i + 1].parse::<u64>()
                {
                    duration_secs = d;
                    i += 1;
                }
            }
            "--poly-steps" | "-p" => {
                if i + 1 < args.len()
                    && let Ok(s) = args[i + 1].parse::<usize>()
                {
                    let valid = (s / 4) * 4;
                    poly_steps = valid.clamp(16, 384);
                    i += 1;
                }
            }
            "--backend" | "-b" => {
                if i + 1 < args.len() {
                    backend_type = match args[i + 1].to_lowercase().as_str() {
                        "fundsp" | "synth" | "default" => AudioBackendType::FundSP,
                        #[cfg(feature = "odin2")]
                        "odin2" | "odin" => AudioBackendType::Odin2,
                        _ => AudioBackendType::default(),
                    };
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Usage: harmonium [OPTIONS] [SOUNDFONT.sf2]");
                println!();
                println!("Options:");
                println!(
                    "  --harmony-mode, -m <MODE>  Harmony engine: 'basic' or 'driver' (default: driver)"
                );
                println!(
                    "  --backend, -b <BACKEND>    Audio backend: 'fundsp' or 'odin2' (default: fundsp)"
                );
                println!("  --record-wav [PATH]        Record to WAV file (default: output.wav)");
                println!("  --record-midi [PATH]       Record to MIDI file (default: output.mid)");
                println!(
                    "  --record-musicxml [PATH]   Record to MusicXML file (default: output.musicxml)"
                );
                println!(
                    "  --record-truth [PATH]      Record Ground Truth JSON (default: output.truth.json)"
                );
                println!("  --osc                      Enable OSC control (UDP 8080)");
                println!(
                    "  --export                   Faster-than-real-time offline export (requires --duration)"
                );
                println!(
                    "  --duration <SECONDS>       Recording duration (0 = infinite, Ctrl+C to stop)"
                );
                println!(
                    "  --poly-steps, -p <STEPS>   Polyrythm resolution: 48, 96, 192... (default: 48)"
                );
                println!("  --drum-kit                 Fixed kick on C1 (for VST drums/samplers)");
                println!("  --help, -h                 Show this help");
                return;
            }
            arg => {
                if !arg.starts_with('-') && sf2_path.is_none() {
                    sf2_path = Some(arg.to_string());
                }
            }
        }
        i += 1;
    }

    let sf2_data = if let Some(path) = sf2_path {
        match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                log::warn(&format!("Failed to read SoundFont: {e}"));
                None
            }
        }
    } else {
        None
    };

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_handler = shutdown_flag.clone();
    ctrlc::set_handler(move || {
        shutdown_flag_handler.store(true, Ordering::Relaxed);
    })
    .ok();

    let initial_params =
        EngineParams { harmony_mode, poly_steps, fixed_kick, ..Default::default() };

    let (target_params_input, target_params_output) = triple_buffer::triple_buffer(&initial_params);
    let target_params_input = Arc::new(Mutex::new(target_params_input));

    // === 2. Offline Export ===
    if use_export {
        if duration_secs == 0 {
            log::error("Error: --export requires --duration <SECONDS>");
            return;
        }

        log::info("🚀 Starting Offline Export...");

        let control_mode =
            std::sync::Arc::new(std::sync::Mutex::new(harmonium::params::ControlMode::default()));

        let finished_recordings = match audio::export_offline(
            target_params_output,
            control_mode,
            sf2_data.as_deref(),
            backend_type,
            duration_secs as f64,
            44100.0,
            record_wav.is_some(),
            record_midi.is_some(),
            record_musicxml.is_some(),
            record_truth.is_some(),
        ) {
            Ok(fr) => fr,
            Err(e) => {
                log::error(&format!("Export failed: {e}"));
                return;
            }
        };

        save_finished_recordings(
            &finished_recordings,
            &record_wav,
            &record_midi,
            &record_musicxml,
            &record_truth,
        );

        log::info("✨ Offline Export complete!");
        return;
    }

    if sf2_data.is_some() {
        if let Ok(mut input) = target_params_input.lock() {
            let mut params = input.input_buffer_mut().clone();
            params.channel_routing = vec![0; 16];
            input.write(params);
        }
    }

    if use_osc {
        let osc_params_input = target_params_input.clone();
        let osc_record_wav = record_wav.clone();
        let osc_record_midi = record_midi.clone();
        let osc_record_musicxml = record_musicxml.clone();
        let osc_record_truth = record_truth.clone();
        let osc_muted_channels = initial_params.muted_channels.clone();

        thread::spawn(move || {
            let addr = "127.0.0.1:8080";
            let socket = match UdpSocket::bind(addr) {
                Ok(s) => s,
                Err(_) => return,
            };

            let mut buf = [0u8; 4096];
            loop {
                if let Ok((size, _)) = socket.recv_from(&mut buf) {
                    if let Ok((_, OscPacket::Message(msg))) =
                        rosc::decoder::decode_udp(&buf[..size])
                    {
                        if msg.addr == "/harmonium/params" {
                            let args = msg.args;
                            if args.len() >= 4 {
                                let get_float = |arg: &OscType| match arg {
                                    OscType::Float(f) => *f,
                                    OscType::Double(d) => *d as f32,
                                    _ => 0.0,
                                };

                                if let Ok(mut input) = osc_params_input.lock() {
                                    let mut current = input.input_buffer_mut().clone();
                                    current.arousal = get_float(&args[0]);
                                    current.valence = get_float(&args[1]);
                                    current.density = get_float(&args[2]);
                                    current.tension = get_float(&args[3]);
                                    current.record_wav = osc_record_wav.is_some();
                                    current.record_midi = osc_record_midi.is_some();
                                    current.record_musicxml = osc_record_musicxml.is_some();
                                    current.record_truth = osc_record_truth.is_some();
                                    current.muted_channels = osc_muted_channels.clone();
                                    input.write(current);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    let control_mode =
        std::sync::Arc::new(std::sync::Mutex::new(harmonium::params::ControlMode::default()));
    let (_stream, config, _, _, _, finished_recordings) =
        audio::create_stream(target_params_output, control_mode, sf2_data.as_deref(), backend_type)
            .expect("Failed to create audio stream");

    if record_wav.is_some()
        || record_midi.is_some()
        || record_musicxml.is_some()
        || record_truth.is_some()
    {
        if let Ok(mut input) = target_params_input.lock() {
            let mut params = input.input_buffer_mut().clone();
            params.record_wav = record_wav.is_some();
            params.record_midi = record_midi.is_some();
            params.record_musicxml = record_musicxml.is_some();
            params.record_truth = record_truth.is_some();
            input.write(params);
            log::info("Recording started...");
        }
    }

    log::info(&format!(
        "Session: {} {} | BPM: {:.1} | Pulses: {}/{}",
        config.key, config.scale, config.bpm, config.pulses, config.steps
    ));

    let start_time = std::time::Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(100));

        if shutdown_flag.load(Ordering::Relaxed) {
            log::info("Received interrupt signal, saving recordings...");
            perform_graceful_shutdown(
                &target_params_input,
                &finished_recordings,
                &record_wav,
                &record_midi,
                &record_musicxml,
                &record_truth,
            );
            break;
        }

        if duration_secs > 0 && start_time.elapsed().as_secs() >= duration_secs {
            log::info("Duration reached. Stopping recording...");
            perform_graceful_shutdown(
                &target_params_input,
                &finished_recordings,
                &record_wav,
                &record_midi,
                &record_musicxml,
                &record_truth,
            );
            break;
        }

        save_finished_recordings(
            &finished_recordings,
            &record_wav,
            &record_midi,
            &record_musicxml,
            &record_truth,
        );
    }
}
