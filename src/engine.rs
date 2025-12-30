use fundsp::hacker32::*;
use crate::sequencer::Sequencer;
use crate::harmony::HarmonyNavigator;
use crate::log;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub bpm: f32,
    pub key: String,
    pub scale: String,
    pub pulses: usize,
    pub steps: usize,
}

pub struct BlockRateAdapter {
    block: Box<dyn AudioUnit>,
}

impl BlockRateAdapter {
    pub fn new(mut block: Box<dyn AudioUnit>, sample_rate: f64) -> Self {
        block.set_sample_rate(sample_rate);
        block.allocate();
        Self { block }
    }

    pub fn get_stereo(&mut self) -> (f32, f32) {
        self.block.get_stereo()
    }
}

pub struct HarmoniumEngine {
    pub config: SessionConfig,
    sequencer: Sequencer,
    harmony: HarmonyNavigator,
    node: BlockRateAdapter,
    frequency: Shared,
    gate: Shared,
    sample_counter: usize,
    samples_per_step: usize,
}

impl HarmoniumEngine {
    pub fn new(sample_rate: f64) -> Self {
        let mut rng = rand::thread_rng();
        let bpm = rng.gen_range(80.0..140.0);
        let steps = 16;
        let pulses = rng.gen_range(3..=9);
        let keys = [PitchSymbol::C, PitchSymbol::D, PitchSymbol::E, PitchSymbol::F, PitchSymbol::G, PitchSymbol::A, PitchSymbol::B];
        let scales = [ScaleType::PentatonicMinor, ScaleType::PentatonicMajor];
        let random_key = keys[rng.gen_range(0..keys.len())];
        let random_scale = scales[rng.gen_range(0..scales.len())];

        let config = SessionConfig {
            bpm,
            key: format!("{}", random_key),
            scale: format!("{:?}", random_scale),
            pulses,
            steps,
        };

        log::info(&format!("Session: {} {} | BPM: {:.1} | Pulses: {}/{}", config.key, config.scale, bpm, pulses, steps));

        // 1. Setup Audio Graph
        let frequency = shared(440.0);
        let gate = shared(0.0);

        // Patch: Sawtooth wave * ADSR envelope
        let patch = (var(&frequency) >> saw() >> lowpass_hz(1000.0, 1.0)) * (var(&gate) >> adsr_live(0.05, 0.2, 0.5, 0.1));
        
        let node = patch >> split::<U2>();
        let node = BlockRateAdapter::new(Box::new(node), sample_rate);

        // 2. Setup Logic Components
        let sequencer = Sequencer::new(steps, pulses, bpm);
        let harmony = HarmonyNavigator::new(random_key, random_scale, 4);

        let samples_per_step = (sample_rate * 60.0 / (bpm as f64) / 4.0) as usize;

        Self {
            config,
            sequencer,
            harmony,
            node,
            frequency,
            gate,
            sample_counter: 0,
            samples_per_step,
        }
    }

    pub fn process(&mut self) -> (f32, f32) {
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            let trigger = self.sequencer.tick();
            if trigger {
                let freq = self.harmony.next_note();
                self.frequency.set_value(freq);
                self.gate.set_value(1.0);
            } else {
                self.gate.set_value(0.0);
            }
        }
        self.sample_counter += 1;

        self.node.get_stereo()
    }
}
