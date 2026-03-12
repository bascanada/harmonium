use crate::{FinishedRecordings, FontQueue, audio::AudioBackendType};
use harmonium_core::{
    EngineCommand, EngineReport, HarmoniumController, MeasureSnapshot,
    params::SessionConfig,
};

/// Wrapper that makes `cpal::Stream` `Send + Sync`.
///
/// `cpal::Stream` is marked `!Send` conservatively, but on desktop platforms
/// (CoreAudio, WASAPI, ALSA) the underlying stream handle is thread-safe.
/// We only call `play()` / `pause()` / `drop()` on it, all of which are safe
/// from any thread.
struct SendStream(cpal::Stream);

// SAFETY: cpal::Stream on desktop platforms (CoreAudio, WASAPI, ALSA) uses
// thread-safe OS handles. We only interact via play()/pause()/drop().
#[allow(unsafe_code)]
unsafe impl Send for SendStream {}
#[allow(unsafe_code)]
unsafe impl Sync for SendStream {}

/// Native (non-WASM) handle for driving the Harmonium engine.
///
/// Unlike `Handle` (which requires `wasm` feature), this is a lightweight
/// wrapper around `HarmoniumController` + `cpal::Stream` for desktop apps.
pub struct NativeHandle {
    stream: SendStream,
    controller: HarmoniumController,
    session_config: SessionConfig,
    font_queue: FontQueue,
    #[allow(dead_code)]
    finished_recordings: FinishedRecordings,
    /// Accumulated measures from the timeline engine.
    measures_buffer: Vec<MeasureSnapshot>,
}

impl NativeHandle {
    /// Start the engine and immediately begin playback.
    pub fn start(
        sf2_bytes: Option<&[u8]>,
        backend: AudioBackendType,
    ) -> Result<Self, String> {
        let (stream, session_config, controller, font_queue, finished_recordings) =
            crate::audio::create_timeline_stream(sf2_bytes, backend)?;

        Ok(Self {
            stream: SendStream(stream),
            controller,
            session_config,
            font_queue,
            finished_recordings,
            measures_buffer: Vec::new(),
        })
    }

    /// Start the engine in a paused state (stream created but paused immediately).
    pub fn start_paused(
        sf2_bytes: Option<&[u8]>,
        backend: AudioBackendType,
    ) -> Result<Self, String> {
        let handle = Self::start(sf2_bytes, backend)?;
        // Pause the stream right after creation so no audio plays until resume()
        handle.pause()?;
        Ok(handle)
    }

    // === Playback Controls ===

    /// Resume (unpause) audio playback.
    pub fn resume(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.0.play().map_err(|e| e.to_string())
    }

    /// Pause audio playback.
    pub fn pause(&self) -> Result<(), String> {
        use cpal::traits::StreamTrait;
        self.stream.0.pause().map_err(|e| e.to_string())
    }

    // === Session Info ===

    pub fn session_config(&self) -> &SessionConfig {
        &self.session_config
    }

    // === Engine Communication ===

    /// Send a command to the audio-thread engine.
    pub fn send_command(&mut self, cmd: EngineCommand) {
        let _ = self.controller.send(cmd);
    }

    /// Poll reports and return the latest cached state.
    pub fn poll_state(&mut self) -> Option<&EngineReport> {
        let _ = self.controller.poll_reports();
        self.controller.get_state()
    }

    // === Timeline / Measure API ===

    /// Drain newly-generated measures from the controller, append to internal buffer,
    /// and return only the new ones. Deduplicates by bar index (keeps latest).
    pub fn poll_measures(&mut self) -> Vec<MeasureSnapshot> {
        let new = self.controller.poll_new_measures();
        for m in &new {
            // Replace existing measure at the same bar index (dedup)
            if let Some(existing) = self.measures_buffer.iter_mut().find(|e| e.index == m.index) {
                *existing = m.clone();
            } else {
                self.measures_buffer.push(m.clone());
            }
        }
        new
    }

    /// Get measures from the accumulated buffer for a given range.
    ///
    /// `from_bar` is 1-based. Returns up to `count` measures starting at that bar.
    pub fn get_buffered_measures(&self, from_bar: usize, count: usize) -> Vec<MeasureSnapshot> {
        self.measures_buffer
            .iter()
            .filter(|m| m.index >= from_bar && m.index < from_bar + count)
            .cloned()
            .collect()
    }

    /// Total number of buffered measures.
    pub fn buffered_measure_count(&self) -> usize {
        self.measures_buffer.len()
    }

    /// Clear the measure buffer (e.g. on seek / restart).
    pub fn clear_measures(&mut self) {
        self.measures_buffer.clear();
    }

    // === Convenience Setters (delegate to controller) ===

    pub fn use_direct_mode(&mut self) {
        let _ = self.controller.use_direct_mode();
    }

    pub fn use_emotion_mode(&mut self) {
        let _ = self.controller.use_emotion_mode();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        let _ = self.controller.set_bpm(bpm);
    }

    pub fn set_density(&mut self, density: f32) {
        let _ = self.controller.set_rhythm_density(density);
    }

    pub fn enable_melody(&mut self, enabled: bool) {
        let _ = self.controller.enable_melody(enabled);
    }

    pub fn enable_harmony(&mut self, enabled: bool) {
        let _ = self.controller.enable_harmony(enabled);
    }

    pub fn enable_rhythm(&mut self, enabled: bool) {
        let _ = self.controller.enable_rhythm(enabled);
    }

    pub fn enable_voicing(&mut self, enabled: bool) {
        let _ = self.controller.enable_voicing(enabled);
    }

    pub fn set_melody_smoothness(&mut self, smoothness: f32) {
        let _ = self.controller.set_melody_smoothness(smoothness);
    }

    pub fn set_rhythm_steps(&mut self, steps: usize) {
        let _ = self.controller.set_rhythm_steps(steps);
    }

    pub fn set_rhythm_pulses(&mut self, pulses: usize) {
        let _ = self.controller.set_rhythm_pulses(pulses);
    }

    pub fn set_rhythm_rotation(&mut self, rotation: usize) {
        let _ = self.controller.set_rhythm_rotation(rotation);
    }

    pub fn set_harmony_tension(&mut self, tension: f32) {
        let _ = self.controller.set_harmony_tension(tension);
    }

    pub fn set_harmony_valence(&mut self, valence: f32) {
        let _ = self.controller.set_harmony_valence(valence);
    }

    pub fn set_writehead_lookahead(&mut self, bars: usize) {
        let _ = self.controller.set_writehead_lookahead(bars);
    }

    pub fn seek_playhead(&mut self, bar: usize) {
        let _ = self.controller.seek_playhead(bar);
    }

    /// Ensure a minimum number of measures have been generated.
    ///
    /// Resumes the stream, polls until at least 16 measures are buffered
    /// (or timeout expires), then pauses.
    pub fn ensure_measures_generated(&mut self, timeout_ms: u64) -> Result<(), String> {
        let min_measures: usize = 16;
        if self.measures_buffer.len() >= min_measures {
            return Ok(());
        }

        // Mute output so pre-generation is silent
        self.send_command(EngineCommand::SetOutputMute(true));
        self.resume()?;

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout && self.measures_buffer.len() < min_measures {
            self.poll_measures();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        self.pause()?;
        // Unmute + seek playhead back to bar 1 for when user presses play
        self.send_command(EngineCommand::SetOutputMute(false));
        self.send_command(EngineCommand::SeekPlayhead(1));
        Ok(())
    }

    /// Reset and regenerate measures.
    ///
    /// Clears the buffer, sends a Reset command, and re-generates measures.
    pub fn regenerate(&mut self) -> Result<(), String> {
        self.clear_measures();
        self.send_command(EngineCommand::Reset);
        // ensure_measures_generated already queues SeekPlayhead(1) after pausing
        self.ensure_measures_generated(500)
    }

    /// Add a SoundFont to a specific bank.
    pub fn add_soundfont(&self, bank_id: u32, sf2_bytes: Vec<u8>) {
        if let Ok(mut queue) = self.font_queue.lock() {
            queue.push((bank_id, sf2_bytes));
        }
    }
}
