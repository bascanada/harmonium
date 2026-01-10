use fundsp::hacker32::*;

pub struct BlockRateAdapter {
    block: Box<dyn AudioUnit>,
    sample_rate: f64,
}

impl BlockRateAdapter {
    pub fn new(mut block: Box<dyn AudioUnit>, sample_rate: f64) -> Self {
        block.set_sample_rate(sample_rate);
        block.allocate();
        Self { block, sample_rate }
    }

    pub fn get_stereo(&mut self) -> (f32, f32) {
        self.block.get_stereo()
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
}
