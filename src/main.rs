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
    log::info("🎵 Harmonium - Procedural Music Generator");
    log::info("🧠 State Management + Morphing Engine activé");

    // === 0. Parse Arguments (SoundFont) ===
    let args: Vec<String> = env::args().collect();
    let sf2_data = if args.len() > 1 {
        let path = &args[1];
        log::info(&format!("📂 Loading SoundFont: {}", path));
        match fs::read(path) {
            Ok(bytes) => {
                log::info("✅ SoundFont loaded successfully");
                Some(bytes)
            },
            Err(e) => {
                log::warn(&format!("❌ Failed to read SoundFont: {}", e));
                None
            }
        }
    } else {
        log::info("ℹ️ No SoundFont provided. Using default synthesis.");
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
            log::info("🔀 Routing set to Oxisynth (Bank 0) for all channels");
        }
    }

    // === 2. Thread Simulateur d'IA (Changements aléatoires toutes les 5 secondes) ===
    let controller_state = target_state.clone();
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        thread::sleep(Duration::from_secs(3)); // Attendre le démarrage
        
        log::info("🤖 Simulateur d'IA démarré (changements toutes les 5s)");
        
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
                "🎭 EMOTION CHANGE: Arousal {:.2} (→ {:.0} BPM) | Valence {:.2} | Density {:.2} | Tension {:.2}",
                params.arousal, bpm, params.valence, params.density, params.tension
            ));
        }
    });

    // === 3. Création du Stream Audio avec l'état partagé ===
    let (_stream, config, _harmony_state, _event_queue, _font_queue) = audio::create_stream(target_state, sf2_data.as_deref())
        .expect("Failed to create audio stream");

    log::info(&format!(
        "Session: {} {} | BPM: {:.1} | Pulses: {}/{}",
        config.key, config.scale, config.bpm, config.pulses, config.steps
    ));
    log::info("🎶 Playing... Press Ctrl+C to stop.");
    log::info("🔄 Le moteur va maintenant morpher automatiquement entre les états!");

    // Keep the main thread alive
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
