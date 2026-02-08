use std::{
    io::{Cursor, Seek, Write},
    sync::{Arc, Mutex},
};

use harmonium_core::{
    events::{AudioEvent, RecordFormat},
    params::MusicalParams,
    truth::RecordingTruth,
};
use hound::{WavSpec, WavWriter};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::backend::AudioRenderer;

type FinishedRecordings = Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>;

#[derive(Clone)]
struct SharedWriter {
    buffer: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl SharedWriter {
    fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(Cursor::new(Vec::with_capacity(1024 * 1024)))) }
    }
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.flush()
    }
}

impl Seek for SharedWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let mut guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.seek(pos)
    }
}

pub struct RecorderBackend {
    inner: Box<dyn AudioRenderer>,
    // Shared storage for finished recordings
    finished_recordings: FinishedRecordings,

    // WAV State
    wav_writer: Option<WavWriter<SharedWriter>>,
    wav_output: Option<SharedWriter>,
    sample_rate: u32,

    // MIDI State
    midi_track: Option<Vec<TrackEvent<'static>>>,
    midi_steps_since_last: f64,
    current_samples_per_step: usize,

    // Truth State (centralizes events and params)
    truth: Option<RecordingTruth>,
    steps_elapsed: f64,
}

impl RecorderBackend {
    pub fn new(
        inner: Box<dyn AudioRenderer>,
        finished_recordings: FinishedRecordings,
        sample_rate: u32,
    ) -> Self {
        Self {
            inner,
            finished_recordings,
            wav_writer: None,
            wav_output: None,
            sample_rate,
            midi_track: None,
            midi_steps_since_last: 0.0,
            current_samples_per_step: 11025,
            truth: None,
            steps_elapsed: 0.0,
        }
    }

    /// Access the inner backend for downcasting
    pub fn inner_mut(&mut self) -> &mut dyn AudioRenderer {
        self.inner.as_mut()
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

        if let Some(shared) = self.wav_output.take()
            && let Ok(mutex) = Arc::try_unwrap(shared.buffer)
            && let Ok(cursor) = mutex.into_inner()
        {
            let data = cursor.into_inner();
            if let Ok(mut queue) = self.finished_recordings.lock() {
                queue.push((RecordFormat::Wav, data));
            }
        }
    }

    fn start_midi(&mut self) {
        // Add tempo meta event at the start (default 120 BPM = 500000 microseconds per quarter note)
        // This will be overridden if the engine sends a tempo change
        let track = vec![TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(500000.into())),
        }];

        self.midi_track = Some(track);
        self.midi_steps_since_last = 0.0;
    }

    fn stop_midi(&mut self) {
        if let Some(mut track_events) = self.midi_track.take() {
            // Add End of Track meta event (required by MIDI spec)
            track_events.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });

            let header = Header::new(Format::SingleTrack, Timing::Metrical(480.into()));
            let mut smf = Smf::new(header);
            smf.tracks.push(track_events);

            let mut buffer = Vec::new();
            if smf.write(&mut buffer).is_ok()
                && let Ok(mut queue) = self.finished_recordings.lock()
            {
                queue.push((RecordFormat::Midi, buffer));
            }
        }
    }

    fn ensure_truth_active(&mut self) {
        if self.truth.is_none() {
            self.truth =
                Some(RecordingTruth::new(Vec::new(), MusicalParams::default(), self.sample_rate));
            self.steps_elapsed = 0.0;
        }
    }

    fn start_musicxml(&mut self) {
        self.ensure_truth_active();
    }

    fn stop_musicxml(&mut self) {
        if let Some(truth) = &self.truth {
            let xml = harmonium_core::export::to_musicxml(
                &truth.events,
                &truth.params,
                1, // Dummy value
            );
            if let Ok(mut queue) = self.finished_recordings.lock() {
                queue.push((RecordFormat::MusicXml, xml.into_bytes()));
            }
        }
    }

    fn start_truth(&mut self) {
        self.ensure_truth_active();
    }

    fn stop_truth(&mut self) {
        if let Some(truth) = self.truth.take() {
            if let Ok(json) = serde_json::to_vec(&truth) {
                if let Ok(mut queue) = self.finished_recordings.lock() {
                    queue.push((RecordFormat::Truth, json));
                }
            }
        }
    }

    fn steps_to_ticks(&self, steps: f64) -> u32 {
        // 1 step = 1/4 beat (16th note)
        // Standard MIDI: 480 ticks per quarter note
        // Therefore: 120 ticks per step
        (steps * 120.0).round() as u32
    }
}

impl AudioRenderer for RecorderBackend {
    fn handle_event(&mut self, event: AudioEvent) {
        // Intercept recording commands
        match &event {
            AudioEvent::StartRecording { format } => match format {
                RecordFormat::Wav => self.start_wav(),
                RecordFormat::Midi => self.start_midi(),
                RecordFormat::MusicXml => self.start_musicxml(),
                RecordFormat::Truth => self.start_truth(),
            },
            AudioEvent::StopRecording { format } => match format {
                RecordFormat::Wav => self.stop_wav(),
                RecordFormat::Midi => self.stop_midi(),
                RecordFormat::MusicXml => self.stop_musicxml(),
                RecordFormat::Truth => self.stop_truth(),
            },
            AudioEvent::TimingUpdate { samples_per_step } => {
                self.current_samples_per_step = *samples_per_step;
            }
            AudioEvent::UpdateMusicalParams { params } => {
                if let Some(truth) = &mut self.truth {
                    truth.params = *params.clone();
                }
            }
            AudioEvent::NoteOn { .. } | AudioEvent::NoteOff { .. } => {
                // Capture Truth with step timestamp
                if let Some(truth) = &mut self.truth {
                    truth.events.push((self.steps_elapsed, event.clone()));
                }
            }
            _ => {}
        }

        // MIDI recording logic - convert step delta to ticks
        match &event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                // Compute delta before mutable borrow
                let delta = self.steps_to_ticks(self.midi_steps_since_last);
                if let Some(track) = &mut self.midi_track {
                    self.midi_steps_since_last = 0.0; // Reset to 0.0
                    track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Midi {
                            channel: (*channel).into(),
                            message: MidiMessage::NoteOn {
                                key: (*note).into(),
                                vel: (*velocity).into(),
                            },
                        },
                    });
                }
            }
            AudioEvent::NoteOff { note, channel } => {
                // Compute delta before mutable borrow
                let delta = self.steps_to_ticks(self.midi_steps_since_last);
                if let Some(track) = &mut self.midi_track {
                    self.midi_steps_since_last = 0.0; // Reset to 0.0
                    track.push(TrackEvent {
                        delta: delta.into(),
                        kind: TrackEventKind::Midi {
                            channel: (*channel).into(),
                            message: MidiMessage::NoteOff { key: (*note).into(), vel: 0.into() },
                        },
                    });
                }
            }
            _ => {}
        }

        self.inner.handle_event(event);
    }

    fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
        self.inner.process_buffer(output, channels);

        // Capture WAV
        if let Some(writer) = &mut self.wav_writer {
            for sample in output.iter() {
                writer.write_sample(*sample).ok();
            }
        }

        // Integrate samples → steps continuously
        if self.current_samples_per_step > 0 {
            let frames = (output.len() / channels) as f64;
            let steps_in_buffer = frames / self.current_samples_per_step as f64;

            // Advance MIDI time (steps)
            if self.midi_track.is_some() {
                self.midi_steps_since_last += steps_in_buffer;
            }

            // Advance Central Truth time (steps)
            if self.truth.is_some() {
                self.steps_elapsed += steps_in_buffer;
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[cfg(feature = "odin2")]
    fn odin2_backend_mut(&mut self) -> Option<&mut crate::backend::odin2_backend::Odin2Backend> {
        self.inner.odin2_backend_mut()
    }
}