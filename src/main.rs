use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::env;
use std::fs;
use harmonium::audio;
use harmonium::engine::EngineParams;
use harmonium::log;
use rand::Rng;

fn main() {
    log::info("Harmonium - Procedural Music Generator");
    log::info("State Management + Morphing Engine activé");

    // === 0. Parse Arguments ===
    let args: Vec<String> = env::args().collect();
    let mut sf2_path: Option<String> = None;
    let mut record_wav = false;
    let mut record_midi = false;
    let mut duration_secs = 0; // 0 = infini

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record-wav" => record_wav = true,
            "--record-midi" => record_midi = true,
            "--duration" => {
                if i + 1 < args.len() {
                    if let Ok(d) = args[i+1].parse::<u64>() {
                        duration_secs = d;
                        i += 1;
                    }
                }
            }
            arg => {
                if !arg.starts_with("-") && sf2_path.is_none() {
                    sf2_path = Some(arg.to_string());
                }
            }
        }
        i += 1;
    }

    let sf2_data = if let Some(path) = sf2_path {
        log::info(&format!("📂 Loading SoundFont: {}", path));
        match fs::read(&path) {
            Ok(bytes) => {
                log::info("SoundFont loaded successfully");
                Some(bytes)
            },
            Err(e) => {
                log::warn(&format!("Failed to read SoundFont: {}", e));
                None
            }
        }
    } else {
        log::info("No SoundFont provided. Using default synthesis.");
        None
    };

    // === 1. État Partagé (Thread-safe) ===
    let target_state = Arc::new(Mutex::new(EngineParams::default()));
    
    // Si on a un SoundFont, on active le routing Oxisynth par défaut pour tester
    if sf2_data.is_some() {
        if let Ok(mut params) = target_state.lock() {
            // Tout sur Oxisynth (Bank 0) sauf peut-être la batterie ?
            // Mettons tout sur Oxisynth pour l'instant pour tester le fichier
            params.channel_routing = vec![0; 16]; 
            log::info("Routing set to Oxisynth (Bank 0) for all channels");
        }
    }

    // === 2. Thread Simulateur d'IA (Changements aléatoires toutes les 5 secondes) ===
    let controller_state = target_state.clone();
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        thread::sleep(Duration::from_secs(3)); // Attendre le démarrage
        
        log::info("Simulateur d'IA démarré (changements toutes les 5s)");
        
        loop {
            thread::sleep(Duration::from_secs(5));
            let mut params = controller_state.lock().unwrap();
            
            // Simule un changement d'action/émotio
            params.arousal = rng.gen_range(0.15..0.95);   // Activation/Énergie
            params.valence = rng.gen_range(-0.8..0.8);    // Positif/Négatif
            params.density = rng.gen_range(0.15..0.95);   // Complexité rythmique
            params.tension = rng.gen_range(0.0..1.0);     // Dissonance
            
            let bpm = params.compute_bpm();
            log::info(&format!(
                "EMOTION CHANGE: Arousal {:.2} (→ {:.0} BPM) | Valence {:.2} | Density {:.2} | Tension {:.2}",
                params.arousal, bpm, params.valence, params.density, params.tension
            ));
        }
    });

    // === 3. Création du Stream Audio avec l'état partagé ===
    let (_stream, config, _harmony_state, _event_queue, _font_queue, finished_recordings) = audio::create_stream(target_state.clone(), sf2_data.as_deref())
        .expect("Failed to create audio stream");

    // Démarrage de l'enregistrement si demandé
    if record_wav || record_midi {
        if let Ok(mut params) = target_state.lock() {
            params.record_wav = record_wav;
            params.record_midi = record_midi;
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
                if let Ok(mut params) = target_state.lock() {
                    params.record_wav = false;
                    params.record_midi = false;
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
                };
                log::info(&format!("Saving recording to {} ({} bytes)", filename, data.len()));
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
