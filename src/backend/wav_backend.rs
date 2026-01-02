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

    fn next_frame(&mut self) -> Option<(f32, f32)> {
        let frame = self.inner.next_frame();
        if let Some((l, r)) = frame {
            self.writer.write_sample(l).ok();
            self.writer.write_sample(r).ok();
        }
        frame
    }
}
