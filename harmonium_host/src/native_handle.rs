use std::sync::Mutex;

use crate::{FinishedRecordings, FontQueue, audio::AudioBackendType};
use crate::composer::MusicComposer;
use crate::playback::PlaybackCommand;
use harmonium_core::{
    EngineReport, MeasureSnapshot,
    params::SessionConfig,
};

/// Wrapper that makes `cpal::Stream` `Send + Sync`.
struct SendStream(cpal::Stream);

// SAFETY: cpal::Stream on desktop platforms (CoreAudio, WASAPI, ALSA) uses
// thread-safe OS handles. We only interact via play()/pause()/drop().
#[allow(unsafe_code)]
unsafe impl Send for SendStream {}
#[allow(unsafe_code)]
unsafe impl Sync for SendStream {}

/// Native (non-WASM) handle for driving the Harmonium engine.
///
/// Owns a `MusicComposer` (direct calls via Mutex) and a `PlaybackCommand`
/// producer for sending commands to the audio-thread PlaybackEngine.
pub struct NativeHandle {
    stream: SendStream,
    composer: Mutex<MusicComposer>,
    playback_cmd_tx: rtrb::Producer<PlaybackCommand>,
    report_rx: rtrb::Consumer<EngineReport>,
    session_config: SessionConfig,
    font_queue: FontQueue,
    #[allow(dead_code)]
    finished_recordings: FinishedRecordings,
    /// Accumulated measures from the composer.
    measures_buffer: Vec<MeasureSnapshot>,
    /// Cached state (last received report).
    cached_state: Option<EngineReport>,
}

impl NativeHandle {
    /// Start the engine and immediately begin playback.
    pub fn start(
        sf2_bytes: Option<&[u8]>,
        backend: AudioBackendType,
    ) -> Result<Self, String> {
        let (stream, session_config, composer, playback_cmd_tx, report_rx, font_queue, finished_recordings) =
            crate::audio::create_timeline_stream(sf2_bytes, backend)?;

        Ok(Self {
            stream: SendStream(stream),
            composer,
            playback_cmd_tx,
            report_rx,
            session_config,
            font_queue,
            finished_recordings,
            measures_buffer: Vec::new(),
            cached_state: None,
        })
    }

    /// Start the engine in a paused state.
    pub fn start_paused(
        sf2_bytes: Option<&[u8]>,
        backend: AudioBackendType,
    ) -> Result<Self, String> {
        let handle = Self::start(sf2_bytes, backend)?;
        handle.pause()?;
        Ok(handle)
    }

    // === Playback Controls ===

    pub fn resume(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.0.play().map_err(|e| e.to_string())
    }

    pub fn pause(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.0.pause().map_err(|e| e.to_string())
    }

    // === Session Info ===

    pub fn session_config(&self) -> &SessionConfig {
        &self.session_config
    }

    // === Generation (direct calls to MusicComposer) ===

    /// Generate bars synchronously. No audio stream needed.
    pub fn generate_bars(&self, count: usize) {
        if let Ok(mut composer) = self.composer.lock() {
            composer.generate_bars(count);
        }
    }

    /// Reset composer and clear timeline.
    pub fn reset_composer(&mut self) {
        if let Ok(mut composer) = self.composer.lock() {
            composer.reset();
        }
        let _ = self.playback_cmd_tx.push(PlaybackCommand::InvalidateBuffer);
    }

    /// Invalidate future measures and regenerate with current params.
    pub fn invalidate_and_regenerate(&mut self, bars: usize) {
        if let Ok(mut composer) = self.composer.lock() {
            composer.invalidate_future();
        }
        let _ = self.playback_cmd_tx.push(PlaybackCommand::InvalidateBuffer);
        if let Ok(mut composer) = self.composer.lock() {
            composer.generate_bars(bars);
        }
    }

    // === Timeline / Measure API ===

    /// Drain newly-generated measures from the composer, append to buffer.
    pub fn poll_measures(&mut self) -> Vec<MeasureSnapshot> {
        self.poll_reports();

        let new = if let Ok(mut composer) = self.composer.lock() {
            composer.take_snapshots()
        } else {
            Vec::new()
        };

        for m in &new {
            if let Some(existing) = self.measures_buffer.iter_mut().find(|e| e.index == m.index) {
                *existing = m.clone();
            } else {
                self.measures_buffer.push(m.clone());
            }
        }
        new
    }

    /// Get measures from the accumulated buffer for a given range.
    pub fn get_buffered_measures(&self, from_bar: usize, count: usize) -> Vec<MeasureSnapshot> {
        self.measures_buffer
            .iter()
            .filter(|m| m.index >= from_bar && m.index < from_bar + count)
            .cloned()
            .collect()
    }

    pub fn buffered_measure_count(&self) -> usize {
        self.measures_buffer.len()
    }

    pub fn clear_measures(&mut self) {
        self.measures_buffer.clear();
    }

    // === Report polling ===

    fn poll_reports(&mut self) {
        while let Ok(report) = self.report_rx.pop() {
            self.cached_state = Some(report);
        }
    }

    pub fn poll_state(&mut self) -> Option<&EngineReport> {
        self.poll_reports();
        self.cached_state.as_ref()
    }

    // === Composer setters (generation params — direct calls) ===

    pub fn use_emotion_mode(&self) {
        if let Ok(mut c) = self.composer.lock() { c.use_emotion_mode(); }
    }

    pub fn use_direct_mode(&self) {
        if let Ok(mut c) = self.composer.lock() { c.use_direct_mode(); }
    }

    pub fn set_bpm(&self, bpm: f32) {
        if let Ok(mut c) = self.composer.lock() { c.set_bpm(bpm); }
    }

    pub fn set_emotions(&self, arousal: f32, valence: f32, density: f32, tension: f32) {
        if let Ok(mut c) = self.composer.lock() {
            c.set_emotions(arousal, valence, density, tension);
        }
    }

    pub fn set_time_signature(&self, numerator: usize, denominator: usize) {
        if let Ok(mut c) = self.composer.lock() { c.set_time_signature(numerator, denominator); }
    }

    pub fn set_density(&self, density: f32) {
        if let Ok(mut c) = self.composer.lock() { c.set_rhythm_density(density); }
    }

    pub fn enable_melody(&self, enabled: bool) {
        if let Ok(mut c) = self.composer.lock() { c.enable_melody(enabled); }
    }

    pub fn enable_harmony(&self, enabled: bool) {
        if let Ok(mut c) = self.composer.lock() { c.enable_harmony(enabled); }
    }

    pub fn enable_rhythm(&self, enabled: bool) {
        if let Ok(mut c) = self.composer.lock() { c.enable_rhythm(enabled); }
    }

    pub fn enable_voicing(&self, enabled: bool) {
        if let Ok(mut c) = self.composer.lock() { c.enable_voicing(enabled); }
    }

    pub fn set_melody_smoothness(&self, smoothness: f32) {
        if let Ok(mut c) = self.composer.lock() { c.set_melody_smoothness(smoothness); }
    }

    pub fn set_rhythm_steps(&self, steps: usize) {
        if let Ok(mut c) = self.composer.lock() { c.set_rhythm_steps(steps); }
    }

    pub fn set_rhythm_pulses(&self, pulses: usize) {
        if let Ok(mut c) = self.composer.lock() { c.set_rhythm_pulses(pulses); }
    }

    pub fn set_rhythm_rotation(&self, rotation: usize) {
        if let Ok(mut c) = self.composer.lock() { c.set_rhythm_rotation(rotation); }
    }

    pub fn set_harmony_tension(&self, tension: f32) {
        if let Ok(mut c) = self.composer.lock() { c.set_harmony_tension(tension); }
    }

    pub fn set_harmony_valence(&self, valence: f32) {
        if let Ok(mut c) = self.composer.lock() { c.set_harmony_valence(valence); }
    }

    pub fn set_rhythm_mode(&self, mode: harmonium_core::sequencer::RhythmMode) {
        if let Ok(mut c) = self.composer.lock() { c.set_rhythm_mode(mode); }
    }

    pub fn set_writehead_lookahead(&self, bars: usize) {
        if let Ok(mut c) = self.composer.lock() { c.set_writehead_lookahead(bars); }
    }

    // === Playback commands (sent to audio thread) ===

    pub fn set_channel_gain(&mut self, channel: u8, gain: f32) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SetChannelGain { channel, gain });
    }

    pub fn set_channel_mute(&mut self, channel: u8, muted: bool) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SetChannelMute { channel, muted });
    }

    pub fn set_channel_route(&mut self, channel: u8, bank_id: i32) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SetChannelRoute { channel, bank_id });
    }

    pub fn set_output_mute(&mut self, muted: bool) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SetOutputMute(muted));
    }

    /// Seek: reset both composer writehead and playback playhead.
    pub fn seek(&mut self, bar: usize) {
        let target_bar = bar.max(1);
        if let Ok(mut composer) = self.composer.lock() {
            composer.seek_writehead(target_bar);
        }
        let _ = self.playback_cmd_tx.push(PlaybackCommand::Seek(target_bar));
    }

    /// Seek playhead without resetting writehead; refill from timeline.
    pub fn seek_playhead(&mut self, bar: usize) {
        let target_bar = bar.max(1);
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SeekPlayhead(target_bar));
        if let Ok(mut composer) = self.composer.lock() {
            composer.refill_from_timeline(target_bar, 8);
        }
    }

    pub fn set_loop(&mut self, start_bar: usize, end_bar: usize) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::SetLoop { start_bar, end_bar });
    }

    pub fn clear_loop(&mut self) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::ClearLoop);
    }

    pub fn start_recording(&mut self, format: harmonium_core::events::RecordFormat) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::StartRecording(format));
    }

    pub fn stop_recording(&mut self, format: harmonium_core::events::RecordFormat) {
        let _ = self.playback_cmd_tx.push(PlaybackCommand::StopRecording(format));
    }

    /// Add a SoundFont to a specific bank.
    pub fn add_soundfont(&self, bank_id: u32, sf2_bytes: Vec<u8>) {
        if let Ok(mut queue) = self.font_queue.lock() {
            queue.push((bank_id, sf2_bytes));
        }
    }

    /// Load queued fonts into playback engine.
    pub fn flush_fonts(&mut self) {
        if let Ok(mut queue) = self.font_queue.try_lock() {
            while let Some((id, bytes)) = queue.pop() {
                let _ = self.playback_cmd_tx.push(PlaybackCommand::LoadFont { id, bytes });
            }
        }
    }
}
