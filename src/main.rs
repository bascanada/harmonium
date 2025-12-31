use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use harmonium::audio;
use harmonium::engine::EngineParams;
use harmonium::log;
use rand::Rng;

fn main() {
    log::info("🎵 Harmonium - Procedural Music Generator");
    log::info("🧠 State Management + Morphing Engine activé");

    // === 1. État Partagé (Thread-safe) ===
    let target_state = Arc::new(Mutex::new(EngineParams::default()));

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
    let (_stream, config) = audio::create_stream(target_state)
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
