use std::time::Duration;
use harmonium::audio;
use harmonium::log;

fn main() {
    log::info("🎵 Harmonium - Procedural Music Generator");

    let (_stream, config) = audio::create_stream().expect("Failed to create audio stream");

    log::info(&format!("Session: {} {} | BPM: {:.1} | Pulses: {}/{}", config.key, config.scale, config.bpm, config.pulses, config.steps));
    log::info("Playing... Press Ctrl+C to stop.");

    // Keep the main thread alive
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
