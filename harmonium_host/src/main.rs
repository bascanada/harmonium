#[cfg(feature = "ai")]
use harmonium::ai::EmotionEngine;
use harmonium::audio;
use harmonium::audio::AudioBackendType;
use harmonium::engine::EngineParams;
use harmonium::harmony::HarmonyMode;
use harmonium::log;
use rand::Rng;
use rosc::{OscPacket, OscType};
use std::env;
use std::fs;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn perform_graceful_shutdown(
    target_params_input: &Arc<Mutex<triple_buffer::Input<EngineParams>>>,
    finished_recordings: &harmonium::FinishedRecordings,
    record_wav: &Option<String>,
    record_midi: &Option<String>,
    record_musicxml: &Option<String>,
) -> bool {
    // Step 1: Calculate expected number of recordings
    let expected_recordings =
        (if record_wav.is_some() { 1 } else { 0 }) +
        (if record_midi.is_some() { 1 } else { 0 }) +
        (if record_musicxml.is_some() { 1 } else { 0 });

    if expected_recordings == 0 {
        return true; // Nothing to save
    }

    log::info(&format!("Stopping {} recording(s)...", expected_recordings));

    // Step 2: Mute all channels to stop new note generation (but let existing notes finish)
    // This ensures we capture all NoteOff events for notes that are still playing
    if let Ok(mut input) = target_params_input.lock() {
        let mut params = input.input_buffer_mut().clone();
        // Mute all channels to stop generating new notes
        params.muted_channels = vec![true; 16];
        // IMPORTANT: Preserve recording flags while muting
        params.record_wav = record_wav.is_some();
        params.record_midi = record_midi.is_some();
        params.record_musicxml = record_musicxml.is_some();
        input.write(params);
        log::info("Muted all channels to stop new note generation");
    }

    // Step 2.5: Wait briefly for triple buffer to propagate and AllNotesOff to be sent
    std::thread::sleep(Duration::from_millis(200));

    // Step 3: Wait for notes to finish playing and NoteOff events to be sent
    // Use a longer wait time to account for synth release/decay envelopes
    log::info("Waiting for playing notes to finish...");
    std::thread::sleep(Duration::from_millis(3000));

    // Step 4: Now stop recordings
    if let Ok(mut input) = target_params_input.lock() {
        let mut params = input.input_buffer_mut().clone();
        params.record_wav = false;
        params.record_midi = false;
        params.record_musicxml = false;
        input.write(params);
        log::info("Recording stop signal sent to audio thread");
    }

    // Step 5: Wait for backend to process stop events and finalize recordings
    log::info("Waiting for recordings to finalize...");
    std::thread::sleep(Duration::from_millis(1500));

    // Step 6: Poll finished_recordings queue with timeout (up to 5 seconds)
    let mut saved_count = 0;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 50; // 5 seconds total

    log::info("Polling for finalized recordings...");
    while iterations < MAX_ITERATIONS && saved_count < expected_recordings {
        if let Ok(mut queue) = finished_recordings.lock() {
            let queue_size = queue.len();
            if queue_size > 0 {
                log::info(&format!("Found {} recording(s) in queue", queue_size));
            }

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
                };

                log::info(&format!(
                    "Saved {} to {} ({} bytes)",
                    match fmt {
                        harmonium::events::RecordFormat::Wav => "WAV",
                        harmonium::events::RecordFormat::Midi => "MIDI",
                        harmonium::events::RecordFormat::MusicXml => "MusicXML",
                    },
                    filename,
                    data.len()
                ));

                if let Err(e) = fs::write(filename, &data) {
                    log::warn(&format!("Failed to write {}: {}", filename, e));
                } else {
                    saved_count += 1;
                }
            }
        }

        // Exit early if we've saved all expected recordings
        if saved_count >= expected_recordings {
            log::info("All recordings saved successfully");
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
        iterations += 1;
    }

    if saved_count < expected_recordings {
        log::warn(&format!(
            "Timeout: Only saved {}/{} recordings",
            saved_count, expected_recordings
        ));
    }

    // Success if we saved all expected recordings
    saved_count >= expected_recordings
}

fn main() {
    log::info("Harmonium - Procedural Music Generator");
    log::info("State Management + Morphing Engine activé");

    // === 0. Parse Arguments ===
    let args: Vec<String> = env::args().collect();
    let mut sf2_path: Option<String> = None;
    let mut record_wav: Option<String> = None;
    let mut record_midi: Option<String> = None;
    let mut record_musicxml: Option<String> = None;
    let mut use_osc = false;
    let mut duration_secs = 0; // 0 = infini
    let mut harmony_mode = HarmonyMode::Driver; // Default to Driver
    let mut poly_steps: usize = 48; // Default polyrythm steps
    #[cfg(feature = "odin2")]
    let mut backend_type = AudioBackendType::Odin2;
    #[cfg(not(feature = "odin2"))]
    let mut backend_type = AudioBackendType::FundSP;
    let mut fixed_kick = false; // Mode Drum Kit (kick fixe sur C1)

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
            "--osc" => use_osc = true,
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
                    && let Ok(d) = args[i + 1].parse::<u64>() {
                        duration_secs = d;
                        i += 1;
                    }
            }
            "--poly-steps" | "-p" => {
                if i + 1 < args.len()
                    && let Ok(s) = args[i + 1].parse::<usize>() {
                        // Valider: multiple de 4, entre 16 et 384
                        let valid = (s / 4) * 4;
                        poly_steps = valid.clamp(16, 384);
                        if valid != s {
                            log::warn(&format!(
                                "Poly steps adjusted to {} (must be multiple of 4)",
                                poly_steps
                            ));
                        }
                        i += 1;
                    }
            }
            "--backend" | "-b" => {
                if i + 1 < args.len() {
                    backend_type = match args[i + 1].to_lowercase().as_str() {
                        "fundsp" | "synth" | "default" => AudioBackendType::FundSP,
                        #[cfg(feature = "odin2")]
                        "odin2" | "odin" => AudioBackendType::Odin2,
                        _ => {
                            log::warn(&format!("Unknown backend '{}', using default", args[i + 1]));
                            AudioBackendType::default()
                        }
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
                println!("  --record-musicxml [PATH]   Record to MusicXML file (default: output.musicxml)");
                println!("  --osc                      Enable OSC control (UDP 8080)");
                println!("  --duration <SECONDS>       Recording duration (0 = infinite, Ctrl+C to stop)");
                println!(
                    "  --poly-steps, -p <STEPS>   Polyrythm resolution: 48, 96, 192... (default: 48)"
                );
                println!("  --drum-kit                 Fixed kick on C1 (for VST drums/samplers)");
                println!("  --help, -h                 Show this help");
                println!();
                println!("Harmony Modes:");
                println!("  basic   - Russell Circumplex quadrants (I-IV-vi-V progressions)");
                println!("  driver  - Steedman Grammar + Neo-Riemannian PLR + LCC");
                return;
            }
            arg => {
                if !arg.starts_with("-") && sf2_path.is_none() {
                    sf2_path = Some(arg.to_string());
                }
            }
        }
        i += 1;
    }

    log::info(&format!("🎹 Harmony Mode: {:?}", harmony_mode));
    log::info(&format!("🎛️ Audio Backend: {:?}", backend_type));
    if fixed_kick {
        log::info("🥁 Drum Kit Mode: ON (Kick fixed on C1)");
    }

    let sf2_data = if let Some(path) = sf2_path {
        log::info(&format!("📂 Loading SoundFont: {}", path));
        match fs::read(&path) {
            Ok(bytes) => {
                log::info("SoundFont loaded successfully");
                Some(bytes)
            }
            Err(e) => {
                log::warn(&format!("Failed to read SoundFont: {}", e));
                None
            }
        }
    } else {
        log::info("No SoundFont provided. Using default synthesis.");
        None
    };

    // === 0.5. Graceful Shutdown Handler ===
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_handler = shutdown_flag.clone();
    ctrlc::set_handler(move || {
        // Only set the atomic flag - no logging here to avoid allocations in signal handler
        shutdown_flag_handler.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl+C handler");

    // === 1. État Partagé (Lock-free avec Triple Buffer) ===
    // Phase 3: Create triple buffer for lock-free UI→Audio parameter updates
    let initial_params = EngineParams {
        harmony_mode,
        poly_steps,
        fixed_kick,
        ..Default::default()
    };

    let (target_params_input, target_params_output) = triple_buffer::triple_buffer(&initial_params);
    // Wrap Input in Arc<Mutex> for sharing across UI threads (OSC, simulator, main)
    // This is OK since Input is on UI side, not real-time audio side
    let target_params_input = Arc::new(Mutex::new(target_params_input));

    log::info(&format!("🎵 Poly Steps: {}", poly_steps));

    // Si on a un SoundFont, on active le routing Oxisynth par défaut pour tester
    if sf2_data.is_some() {
        // Phase 3: Use triple buffer write (lock Input on UI side)
        if let Ok(mut input) = target_params_input.lock() {
            let mut params = input.input_buffer_mut().clone();
            // Tout sur Oxisynth (Bank 0) sauf peut-être la batterie ?
            // Mettons tout sur Oxisynth pour l'instant pour tester le fichier
            params.channel_routing = vec![0; 16];
            input.write(params);
            log::info("Routing set to Oxisynth (Bank 0) for all channels");
        }
    }

    // === 2. OSC Listener (UDP 8080) ===
    if use_osc {
        // Phase 3: Clone Arc<Mutex<Input>> for OSC thread to write parameters
        let osc_params_input = target_params_input.clone();
        // Clone the recording flags to preserve them across OSC updates
        let osc_record_wav = record_wav.clone();
        let osc_record_midi = record_midi.clone();
        let osc_record_musicxml = record_musicxml.clone();
        // Clone muted channels to preserve across updates
        let osc_muted_channels = initial_params.muted_channels.clone();
        thread::spawn(move || {
            let addr = "127.0.0.1:8080";
            let socket = match UdpSocket::bind(addr) {
                Ok(s) => {
                    log::info(&format!("OSC Listener bound to {}", addr));
                    s
                }
                Err(e) => {
                    log::error(&format!("Failed to bind OSC socket: {}", e));
                    return;
                }
            };

            // Initialize AI Engine (only when ai feature is enabled)
            #[cfg(feature = "ai")]
            let emotion_engine: Option<EmotionEngine> = {
                let config_path = "web/static/models/config.json";
                let weights_path = "web/static/models/model.safetensors";
                let tokenizer_path = "web/static/models/tokenizer.json";

                if fs::metadata(config_path).is_ok()
                    && fs::metadata(weights_path).is_ok()
                    && fs::metadata(tokenizer_path).is_ok()
                {
                    log::info("Loading AI Model for OSC...");
                    match (
                        fs::read(config_path),
                        fs::read(weights_path),
                        fs::read(tokenizer_path),
                    ) {
                        (Ok(c), Ok(w), Ok(t)) => match EmotionEngine::new(&c, &w, &t) {
                            Ok(engine) => {
                                log::info("AI Model loaded successfully!");
                                Some(engine)
                            }
                            Err(e) => {
                                log::error(&format!("Failed to init AI engine: {:?}", e));
                                None
                            }
                        },
                        _ => {
                            log::error("Failed to read model files");
                            None
                        }
                    }
                } else {
                    log::warn(
                        "AI Model files not found in web/static/models. OSC will only accept raw params.",
                    );
                    log::warn("Run 'make models/download' to enable AI features.");
                    None
                }
            };

            #[cfg(not(feature = "ai"))]
            let _emotion_engine: Option<()> = None;

            let mut buf = [0u8; 4096];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if let Ok((_, OscPacket::Message(msg))) = rosc::decoder::decode_udp(&buf[..size]) {
                            #[cfg(feature = "ai")]
                            if msg.addr == "/harmonium/label" {
                                    let args = msg.args.clone();
                                    if let Some(OscType::String(label)) = args.first() {
                                        log::info(&format!("OSC LABEL RECEIVED: {}", label));

                                        if let Some(engine) = &emotion_engine {
                                            match engine.predict_native(label) {
                                                Ok(predicted_params) => {
                                                    // Phase 3: Use triple buffer write (lock Input on UI side)
                                                    if let Ok(mut input) =
                                                        osc_params_input.lock()
                                                    {
                                                        let mut current =
                                                            input.input_buffer_mut().clone();
                                                        current.arousal =
                                                            predicted_params.arousal;
                                                        current.valence =
                                                            predicted_params.valence;
                                                        current.density =
                                                            predicted_params.density;
                                                        current.tension =
                                                            predicted_params.tension;
                                                        // IMPORTANT: Preserve recording flags and muted channels
                                                        current.record_wav = osc_record_wav.is_some();
                                                        current.record_midi = osc_record_midi.is_some();
                                                        current.record_musicxml = osc_record_musicxml.is_some();
                                                        current.muted_channels = osc_muted_channels.clone();
                                                        input.write(current);
                                                        log::info(&format!(
                                                            "AI UPDATE: Arousal {:.2} | Valence {:.2} | Density {:.2} | Tension {:.2}",
                                                            predicted_params.arousal,
                                                            predicted_params.valence,
                                                            predicted_params.density,
                                                            predicted_params.tension
                                                        ));
                                                    }
                                                }
                                                Err(e) => log::error(&format!(
                                                    "AI Prediction failed: {}",
                                                    e
                                                )),
                                            }
                                        } else {
                                            log::warn("AI Engine not loaded. Ignoring label.");
                                        }
                                    }
                                }

                                if msg.addr == "/harmonium/params" {
                                    // Fallback for manual control
                                    let args = msg.args;
                                    if args.len() >= 4 {
                                        let get_float = |arg: &OscType| -> f32 {
                                            match arg {
                                                OscType::Float(f) => *f,
                                                OscType::Double(d) => *d as f32,
                                                _ => 0.0,
                                            }
                                        };

                                        let arousal = get_float(&args[0]);
                                        let valence = get_float(&args[1]);
                                        let density = get_float(&args[2]);
                                        let tension = get_float(&args[3]);

                                        // Phase 3: Use triple buffer write (lock Input on UI side)
                                        if let Ok(mut input) = osc_params_input.lock() {
                                            let mut current = input.input_buffer_mut().clone();
                                            current.arousal = arousal;
                                            current.valence = valence;
                                            current.density = density;
                                            current.tension = tension;
                                            // IMPORTANT: Preserve recording flags and muted channels
                                            current.record_wav = osc_record_wav.is_some();
                                            current.record_midi = osc_record_midi.is_some();
                                            current.record_musicxml = osc_record_musicxml.is_some();
                                            current.muted_channels = osc_muted_channels.clone();
                                            input.write(current);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error(&format!("Error receiving UDP packet: {}", e));
                    }
                }
            }
        });
    } else {
        log::info("OSC disabled. Use --osc to enable external control.");

        // === 2b. Thread Simulateur d'IA (Changements aléatoires toutes les 5 secondes) ===
        // Phase 3: Clone Arc<Mutex<Input>> for simulator thread to write parameters
        let simulator_params_input = target_params_input.clone();
        // Clone the recording flags to preserve them across emotion changes
        let simulator_record_wav = record_wav.clone();
        let simulator_record_midi = record_midi.clone();
        let simulator_record_musicxml = record_musicxml.clone();
        // Clone muted channels to preserve across updates
        let simulator_muted_channels = initial_params.muted_channels.clone();
        let simulator_shutdown_flag = shutdown_flag.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            thread::sleep(Duration::from_secs(3)); // Attendre le démarrage

            log::info("Simulateur d'IA démarré (changements toutes les 5s)");

            loop {
                // Check shutdown flag before sleeping
                if simulator_shutdown_flag.load(Ordering::Relaxed) {
                    log::info("Simulator thread stopping due to shutdown signal");
                    break;
                }

                // Sleep in small chunks to react to shutdown faster
                for _ in 0..50 {
                    if simulator_shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                if simulator_shutdown_flag.load(Ordering::Relaxed) {
                    log::info("Simulator thread stopping due to shutdown signal");
                    break;
                }

                // Phase 3: Use triple buffer write (lock Input on UI side)
                if let Ok(mut input) = simulator_params_input.lock() {
                    // Check one last time before acquiring lock/writing
                    if simulator_shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    let mut params = input.input_buffer_mut().clone();

                    // Simule un changement d'action/émotio
                    params.arousal = rng.gen_range(0.15..0.95); // Activation/Énergie
                    params.valence = rng.gen_range(-0.8..0.8); // Positif/Négatif
                    params.density = rng.gen_range(0.15..0.95); // Complexité rythmique
                    params.tension = rng.gen_range(0.0..1.0); // Dissonance

                    // IMPORTANT: Preserve recording flags and muted channels across emotion changes
                    params.record_wav = simulator_record_wav.is_some();
                    params.record_midi = simulator_record_midi.is_some();
                    params.record_musicxml = simulator_record_musicxml.is_some();
                    params.muted_channels = simulator_muted_channels.clone();

                    // Extract values for logging before moving params
                    let arousal = params.arousal;
                    let valence = params.valence;
                    let density = params.density;
                    let tension = params.tension;
                    let bpm = params.compute_bpm();

                    input.write(params);
                    log::info(&format!(
                        "EMOTION CHANGE: Arousal {:.2} (→ {:.0} BPM) | Valence {:.2} | Density {:.2} | Tension {:.2}",
                        arousal, bpm, valence, density, tension
                    ));
                }
            }
        });
    }

    // === 3. Création du Stream Audio avec l'état partagé ===
    // Phase 3: Pass Output side of triple buffer to audio thread
    let control_mode =
        std::sync::Arc::new(std::sync::Mutex::new(harmonium::params::ControlMode::default()));
    let (_stream, config, _harmony_state, _event_queue, _font_queue, finished_recordings) =
        audio::create_stream(
            target_params_output,
            control_mode,
            sf2_data.as_deref(),
            backend_type,
        )
        .expect("Failed to create audio stream");

    // Démarrage de l'enregistrement si demandé
    if record_wav.is_some() || record_midi.is_some() || record_musicxml.is_some() {
        // Phase 3: Use triple buffer write (lock Input on UI side)
        if let Ok(mut input) = target_params_input.lock() {
            let mut params = input.input_buffer_mut().clone();
            params.record_wav = record_wav.is_some();
            params.record_midi = record_midi.is_some();
            params.record_musicxml = record_musicxml.is_some();
            log::info(&format!(
                "DEBUG: Setting recording flags - WAV={}, MIDI={}, MusicXML={}",
                params.record_wav, params.record_midi, params.record_musicxml
            ));
            input.write(params);
            log::info("Recording started...");
        }
    }

    log::info(&format!(
        "Session: {} {} | BPM: {:.1} | Pulses: {}/{}",
        config.key, config.scale, config.bpm, config.pulses, config.steps
    ));
    log::info("Playing... Press Ctrl+C to stop.");
    log::info("Le moteur va maintenant morpher automatiquement entre les états!");

    let start_time = std::time::Instant::now();

    // Keep the main thread alive
    loop {
        std::thread::sleep(Duration::from_millis(100));

        // Check for Ctrl+C signal
        if shutdown_flag.load(Ordering::Relaxed) {
            log::info("Received interrupt signal, saving recordings...");
            let success = perform_graceful_shutdown(
                &target_params_input,
                &finished_recordings,
                &record_wav,
                &record_midi,
                &record_musicxml,
            );
            if !success {
                log::warn("Some recordings may not have been saved");
            }
            log::info("Exited gracefully.");
            break;
        }

        // Gestion de la durée d'enregistrement
        if duration_secs > 0 && start_time.elapsed().as_secs() >= duration_secs {
            log::info("Duration reached. Stopping recording...");
            let success = perform_graceful_shutdown(
                &target_params_input,
                &finished_recordings,
                &record_wav,
                &record_midi,
                &record_musicxml,
            );
            if !success {
                log::warn("Some recordings may not have been saved");
            }
            log::info("Exiting after recording.");
            break;
        }

        // Vérification des enregistrements terminés (only poll when NOT shutting down)
        if !shutdown_flag.load(Ordering::Relaxed)
            && let Ok(mut queue) = finished_recordings.lock() {
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
                    };
                    log::info(&format!(
                        "Saving recording to {} ({} bytes)",
                        filename,
                        data.len()
                    ));
                    if let Err(e) = fs::write(filename, &data) {
                        log::warn(&format!("Failed to write file: {}", e));
                    }
                }
            }
    }
}
