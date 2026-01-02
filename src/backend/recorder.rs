use crate::backend::AudioRenderer;
use crate::events::{AudioEvent, RecordFormat};
use hound::{WavSpec, WavWriter};
use midly::{Header, Smf, Track, TrackEvent, TrackEventKind, MidiMessage, Format, Timing};
use std::sync::{Arc, Mutex};
use std::io::{Cursor, Write, Seek};

#[derive(Clone)]
struct SharedWriter {
    buffer: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl SharedWriter {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Cursor::new(Vec::with_capacity(1024 * 1024)))),
        }
    }
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.lock().unwrap().flush()
    }
}

impl Seek for SharedWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.buffer.lock().unwrap().seek(pos)
    }
}

pub struct RecorderBackend {
    inner: Box<dyn AudioRenderer>,
    // Shared storage for finished recordings
    finished_recordings: Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>,
    
    // WAV State
    wav_writer: Option<WavWriter<SharedWriter>>,
    wav_output: Option<SharedWriter>,
    sample_rate: u32,
    
    // MIDI State
    midi_track: Option<Vec<TrackEvent<'static>>>,
    midi_samples_since_last: u64,
    current_samples_per_step: usize,
}

impl RecorderBackend {
    pub fn new(
        inner: Box<dyn AudioRenderer>, 
        finished_recordings: Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>,
        sample_rate: u32
    ) -> Self {
        Self {
            inner,
            finished_recordings,
            wav_writer: None,
            wav_output: None,
            sample_rate,
            midi_track: None,
            midi_samples_since_last: 0,
            current_samples_per_step: 11025,
        }
    }

    fn start_wav(&mut self) {
        let spec = WavSpec {
            channels: 2,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let shared = SharedWriter::new();
        self.wav_output = Some(shared.clone());
        if let Ok(writer) = WavWriter::new(shared, spec) {
            self.wav_writer = Some(writer);
        }
    }

    fn stop_wav(&mut self) {
        // Drop writer to finalize
        self.wav_writer = None;
        
        if let Some(shared) = self.wav_output.take() {
            if let Ok(mutex) = Arc::try_unwrap(shared.buffer) {
                if let Ok(cursor) = mutex.into_inner() {
                    let data = cursor.into_inner();
                    if let Ok(mut queue) = self.finished_recordings.lock() {
                        queue.push((RecordFormat::Wav, data));
                    }
                }
            }
        }
    }

    fn start_midi(&mut self) {
        self.midi_track = Some(Vec::new());
        self.midi_samples_since_last = 0;
    }

    fn stop_midi(&mut self) {
        if let Some(track_events) = self.midi_track.take() {
            let header = Header::new(Format::SingleTrack, Timing::Metrical(480.into()));
            let mut smf = Smf::new(header);
            smf.tracks.push(track_events);
            
            let mut buffer = Vec::new();
            if smf.write(&mut buffer).is_ok() {
                if let Ok(mut queue) = self.finished_recordings.lock() {
                    queue.push((RecordFormat::Midi, buffer));
                }
            }
        }
    }

    fn samples_to_ticks(&self, samples: u64) -> u32 {
        if self.current_samples_per_step == 0 { return 0; }
        // 1 step = 1/4 beat (16th note)
        // ticks per beat = 480
        // ticks per step = 120
        ((samples as f64 * 120.0) / self.current_samples_per_step as f64) as u32
    }
}

impl AudioRenderer for RecorderBackend {
    fn handle_event(&mut self, event: AudioEvent) {
        // Intercept recording commands
        match &event {
            AudioEvent::StartRecording { format } => {
                match format {
                    RecordFormat::Wav => self.start_wav(),
                    RecordFormat::Midi => self.start_midi(),
                }
            },
            AudioEvent::StopRecording { format } => {
                match format {
                    RecordFormat::Wav => self.stop_wav(),
                    RecordFormat::Midi => self.stop_midi(),
                }
            },
            AudioEvent::TimingUpdate { samples_per_step } => {
                self.current_samples_per_step = *samples_per_step;
            },
            AudioEvent::NoteOn { note, velocity, channel } => {
                // Capture MIDI
                let delta = self.samples_to_ticks(self.midi_samples_since_last);
                if let Some(track) = &mut self.midi_track {
                    self.midi_samples_since_last = 0;
                    track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Midi {
                            channel: (*channel).into(),
                            message: MidiMessage::NoteOn { key: (*note).into(), vel: (*velocity).into() }
                        }
                    });
                }
            },
            AudioEvent::NoteOff { note, channel } => {
                // Capture MIDI
                let delta = self.samples_to_ticks(self.midi_samples_since_last);
                if let Some(track) = &mut self.midi_track {
                    self.midi_samples_since_last = 0;
                    track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Midi {
                            channel: (*channel).into(),
                            message: MidiMessage::NoteOff { key: (*note).into(), vel: 0.into() }
                        }
                    });
                }
            },
            _ => {}
        }
        self.inner.handle_event(event);
    }

    fn next_frame(&mut self) -> Option<(f32, f32)> {
        let frame = self.inner.next_frame();
        
        // Capture WAV
        if let Some((l, r)) = frame {
            if let Some(writer) = &mut self.wav_writer {
                writer.write_sample(l).ok();
                writer.write_sample(r).ok();
            }
        }
        
        // Advance MIDI time
        if self.midi_track.is_some() {
            self.midi_samples_since_last += 1;
        }

        frame
    }
}
