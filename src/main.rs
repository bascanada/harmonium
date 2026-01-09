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
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    log::info("Harmonium - Procedural Music Generator");
    log::info("State Management + Morphing Engine activé");

    // === 0. Parse Arguments ===
    let args: Vec<String> = env::args().collect();
    let mut sf2_path: Option<String> = None;
    let mut record_wav = false;
    let mut record_midi = false;
    let mut record_abc = false;
    let mut use_osc = false;
    let mut duration_secs = 0; // 0 = infini
    let mut harmony_mode = HarmonyMode::Driver; // Default to Driver
    let mut poly_steps: usize = 48; // Default polyrythm steps
    let mut backend_type = AudioBackendType::Odin2;
    let mut fixed_kick = false; // Mode Drum Kit (kick fixe sur C1)

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record-wav" => record_wav = true,
            "--record-midi" => record_midi = true,
            "--record-abc" => record_abc = true,
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
                if i + 1 < args.len() {
                    if let Ok(d) = args[i + 1].parse::<u64>() {
                        duration_secs = d;
                        i += 1;
                    }
                }
            }
            "--poly-steps" | "-p" => {
                if i + 1 < args.len() {
                    if let Ok(s) = args[i + 1].parse::<usize>() {
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
                println!("  --record-wav               Record to WAV file");
                println!("  --record-midi              Record to MIDI file");
                println!("  --record-abc               Record to ABC notation");
                println!("  --osc                      Enable OSC control (UDP 8080)");
                println!("  --duration <SECONDS>       Recording duration (0 = infinite)");
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

    // === 1. État Partagé (Lock-free avec Triple Buffer) ===
    // Phase 3: Create triple buffer for lock-free UI→Audio parameter updates
    let mut initial_params = EngineParams::default();
    initial_params.harmony_mode = harmony_mode;
    initial_params.poly_steps = poly_steps;
    initial_params.fixed_kick = fixed_kick;

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
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                            match packet {
                                OscPacket::Message(msg) => {
                                    #[cfg(feature = "ai")]
                                    if msg.addr == "/harmonium/label" {
                                        let args = msg.args.clone();
                                        if let Some(OscType::String(label)) = args.get(0) {
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
                                                input.write(current);
                                            }
                                        }
                                    }
                                }
                                _ => {}
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
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            thread::sleep(Duration::from_secs(3)); // Attendre le démarrage

            log::info("Simulateur d'IA démarré (changements toutes les 5s)");

            loop {
                thread::sleep(Duration::from_secs(5));
                // Phase 3: Use triple buffer write (lock Input on UI side)
                if let Ok(mut input) = simulator_params_input.lock() {
                    let mut params = input.input_buffer_mut().clone();
                    // Simule un changement d'action/émotio
                    params.arousal = rng.gen_range(0.15..0.95); // Activation/Énergie
                    params.valence = rng.gen_range(-0.8..0.8); // Positif/Négatif
                    params.density = rng.gen_range(0.15..0.95); // Complexité rythmique
                    params.tension = rng.gen_range(0.0..1.0); // Dissonance

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
        std::sync::Arc::new(std::sync::Mutex::new(harmonium::ControlMode::default()));
    let (_stream, config, _harmony_state, _event_queue, _font_queue, finished_recordings) =
        audio::create_stream(
            target_params_output,
            control_mode,
            sf2_data.as_deref(),
            backend_type,
        )
        .expect("Failed to create audio stream");

    // Démarrage de l'enregistrement si demandé
    if record_wav || record_midi || record_abc {
        // Phase 3: Use triple buffer write (lock Input on UI side)
        if let Ok(mut input) = target_params_input.lock() {
            let mut params = input.input_buffer_mut().clone();
            params.record_wav = record_wav;
            params.record_midi = record_midi;
            params.record_abc = record_abc;
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
    let mut recording_stopped = false;

    // Keep the main thread alive
    loop {
        std::thread::sleep(Duration::from_millis(100));

        // Gestion de la durée d'enregistrement
        if duration_secs > 0 && !recording_stopped {
            if start_time.elapsed().as_secs() >= duration_secs {
                log::info("Duration reached. Stopping recording...");
                // Phase 3: Use triple buffer write (lock Input on UI side)
                if let Ok(mut input) = target_params_input.lock() {
                    let mut params = input.input_buffer_mut().clone();
                    params.record_wav = false;
                    params.record_midi = false;
                    params.record_abc = false;
                    input.write(params);
                }
                recording_stopped = true;
                // Attendre un peu que le backend traite l'événement
                std::thread::sleep(Duration::from_millis(500));
            }
        }

        // Vérification des enregistrements terminés
        if let Ok(mut queue) = finished_recordings.lock() {
            while let Some((fmt, data)) = queue.pop() {
                let filename = match fmt {
                    harmonium::events::RecordFormat::Wav => "output.wav",
                    harmonium::events::RecordFormat::Midi => "output.mid",
                    harmonium::events::RecordFormat::Abc => "output.abc",
                };
                log::info(&format!(
                    "Saving recording to {} ({} bytes)",
                    filename,
                    data.len()
                ));
                if let Err(e) = fs::write(filename, data) {
                    log::warn(&format!("Failed to write file: {}", e));
                }
            }
        }

        if recording_stopped && duration_secs > 0 {
            log::info("Exiting after recording.");
            break;
        }
    }
}
