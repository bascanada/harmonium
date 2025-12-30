use fundsp::hacker32::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::{Duration, Instant};

mod sequencer;
mod harmony;

use sequencer::Sequencer;
use harmony::HarmonyNavigator;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;
use rand::Rng;

fn main() {
    println!("🎵 Harmonium - Procedural Music Generator");

    let mut rng = rand::rng();
    let bpm = rng.random_range(80.0..140.0);
    let steps = 16;
    let pulses = rng.random_range(3..=9);
    let keys = [PitchSymbol::C, PitchSymbol::D, PitchSymbol::E, PitchSymbol::F, PitchSymbol::G, PitchSymbol::A, PitchSymbol::B];
    let scales = [ScaleType::PentatonicMinor, ScaleType::PentatonicMajor];
    let random_key = keys[rng.random_range(0..keys.len())];
    let random_scale = scales[rng.random_range(0..scales.len())];

    println!("Session: {} {:?} | BPM: {:.1} | Pulses: {}/{}", random_key, random_scale, bpm, pulses, steps);

    // 1. Setup Audio Graph
    let frequency = shared(440.0);
    let gate = shared(0.0);

    // Patch: Sawtooth wave * ADSR envelope
    // adsr_live(attack, decay, sustain_level, release)
    // using lowpass to soften the saw wave
    let patch = (var(&frequency) >> saw() >> lowpass_hz(1000.0, 1.0)) * (var(&gate) >> adsr_live(0.05, 0.2, 0.5, 0.1));

    // 2. Setup CPAL (Audio Backend)
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config().expect("No default config");
    let sample_rate = config.sample_rate() as f64;
    let channels = config.channels() as usize;

    println!("Output device: {}", device.name().unwrap_or("unknown".to_string()));
    println!("Sample rate: {}, Channels: {}", sample_rate, channels);

    // We need to move the patch into the audio thread.
    // fundsp provides a nice wrapper to turn a graph into a closure for processing samples.
    // But we need to handle split/stereo if channels > 1.
    // Let's create a Block that we can run.
    let node = patch >> split::<U2>(); // Stereo output (assuming 2 channels for simplicity or duplicating mono)
    
    // If we have more or fewer channels, we might need adjustment, but U2 is safe for headphones/speakers usually.
    // Actually, let's adapt to `channels`.
    // For simplicity in this POC, we'll assume stereo or mono and just output stereo.
    // If the device is mono, we might need to change logic.
    // But let's stick to the user's graph structure which was mono, and upmix it.
    
    // Audio processing callback
    let mut node = BlockRateAdapter::new(Box::new(node), sample_rate);

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // Processing loop
            for frame in data.chunks_mut(channels) {
                let (l, r) = node.get_stereo();
                
                if channels >= 1 { frame[0] = l; }
                if channels >= 2 { frame[1] = r; }
                // fill others with silence or copy
                for sample in frame.iter_mut().skip(2) {
                    *sample = 0.0;
                }
            }
        },
        err_fn,
        None,
    ).expect("Failed to build stream");

    stream.play().expect("Failed to play stream");

    // 3. Setup Logic Components
    let mut sequencer = Sequencer::new(steps, pulses, bpm);
    
    let mut harmony = HarmonyNavigator::new(random_key, random_scale, 4);

    println!("Playing... Press Ctrl+C to stop.");
    println!("Pattern: {:?}", sequencer.pattern);


    // 4. Main Loop
    let step_duration = Duration::from_secs_f64(60.0 / (bpm as f64) / 4.0); // 16th notes
    let mut next_step_time = Instant::now();

    loop {
        let now = Instant::now();
        if now >= next_step_time {
            // Tick
            let trigger = sequencer.tick();
            
            if trigger {
                let freq = harmony.next_note();
                frequency.set_value(freq);
                gate.set_value(1.0);
                print!("♪ ");
            } else {
                gate.set_value(0.0);
                print!(". ");
            }
            use std::io::Write;
            std::io::stdout().flush().unwrap();

            // Schedule next step
            next_step_time += step_duration;
            
            // If we drifted a lot (lag), reset next_step to now + duration to avoid catch-up burst
            if next_step_time < now {
                 next_step_time = now + step_duration;
            }
        }

        // Sleep a tiny bit to not burn CPU
        std::thread::sleep(Duration::from_millis(1));
    }
}

// Helper to adapt Block to sample rate if needed, or just run it.
// fundsp::hacker32::BlockRateAdapter doesn't exist directly like that?
// Actually, `node.set_sample_rate(sample_rate)` is usually how it's done.
// Let's fix the audio callback setup.

struct BlockRateAdapter {
    block: Box<dyn AudioUnit>,
}

impl BlockRateAdapter {
    fn new(mut block: Box<dyn AudioUnit>, sample_rate: f64) -> Self {
        block.set_sample_rate(sample_rate);
        block.allocate();
        Self { block }
    }

    fn get_stereo(&mut self) -> (f32, f32) {
        self.block.get_stereo()
    }
}