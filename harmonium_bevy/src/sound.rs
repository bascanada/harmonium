use harmonium_audio::{backend::odin2_backend::Odin2Backend, AudioProcessor};
use harmonium_core::events::AudioEvent;
use kira::{
    sound::{static_sound::StaticSoundSettings, Sound, SoundData},
    Frame,
};
use rtrb::Consumer;

/// Static data to initialize the Harmonium sound.
pub struct HarmoniumSoundData {
    pub sample_rate: u32,
    pub event_consumer: Consumer<AudioEvent>,
    #[allow(dead_code)]
    pub settings: StaticSoundSettings,
}

impl SoundData for HarmoniumSoundData {
    type Error = ();
    type Handle = ();

    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        // Initialize the actual audio engine
        let backend = Box::new(Odin2Backend::new(f64::from(self.sample_rate)));
        let processor = AudioProcessor::new(backend);

        Ok((
            Box::new(HarmoniumSound {
                processor,
                event_consumer: self.event_consumer,
                // Internal buffer for block processing (e.g. 512 frames)
                buffer: vec![0.0; 512 * 2],
                buffer_idx: 512 * 2, // Force generate on first call
            }),
            (),
        ))
    }
}

/// The instance running in the audio thread.
pub struct HarmoniumSound {
    processor: AudioProcessor,
    event_consumer: Consumer<AudioEvent>,
    buffer: Vec<f32>,
    buffer_idx: usize,
}

impl Sound for HarmoniumSound {
    fn process(&mut self, frames: &mut [Frame], _dt: f64, _info: &kira::info::Info) {
        for frame in frames {
            // 1. If buffer is exhausted, generate a new block
            if self.buffer_idx >= self.buffer.len() {
                self.buffer_idx = 0;

                // a. Consume events from Bevy
                let mut events = Vec::new();
                // We drain the ring buffer.
                // Valid because this runs in the audio thread (consumer).
                while let Ok(event) = self.event_consumer.pop() {
                    events.push(event);
                }

                self.processor.process_events(&events);

                // b. Generate audio (stereo interleaved)
                self.processor.process_audio(&mut self.buffer, 2);
            }

            // 2. Serve current sample
            let left = self.buffer[self.buffer_idx];
            let right = self.buffer[self.buffer_idx + 1];
            self.buffer_idx += 2;

            *frame = Frame { left, right };
        }
    }

    fn on_start_processing(&mut self) {
        // Optional: Reset state if needed
    }

    fn finished(&self) -> bool {
        false
    }
}
