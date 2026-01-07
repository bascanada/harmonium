use crate::backend::AudioRenderer;
use crate::events::AudioEvent;
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;

pub struct WavBackend {
    inner: Box<dyn AudioRenderer>,
    writer: WavWriter<BufWriter<File>>,
}

impl WavBackend {
    pub fn new(inner: Box<dyn AudioRenderer>, path: &str, sample_rate: u32) -> Result<Self, hound::Error> {
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let writer = WavWriter::create(path, spec)?;
        Ok(Self { inner, writer })
    }
}

impl AudioRenderer for WavBackend {
    fn handle_event(&mut self, event: AudioEvent) {
        self.inner.handle_event(event);
    }

    fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        self.inner.process_buffer(output, channels);
        for sample in output.iter() {
            self.writer.write_sample(*sample).ok();
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
