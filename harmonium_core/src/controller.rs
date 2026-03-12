//! HarmoniumController - Public interface for controlling the engine
//!
//! This is the ONLY way external code should interact with the engine.
//! All control flows through EngineCommand, all state comes back through EngineReport.

use crate::{
    command::EngineCommand,
    events::RecordFormat,
    harmony::HarmonyMode,
    params::{EngineParams, HarmonyStrategy},
    report::{EngineReport, MeasureSnapshot},
    sequencer::RhythmMode,
};

/// Error types for controller operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerError {
    /// Command queue is full
    QueueFull,

    /// No engine report available
    NoReport,

    /// Controller not initialized
    NotInitialized,
}

impl std::fmt::Display for ControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(f, "Command queue is full"),
            Self::NoReport => write!(f, "No engine report available"),
            Self::NotInitialized => write!(f, "Controller not initialized"),
        }
    }
}

impl std::error::Error for ControllerError {}

/// Control mode (emotion vs direct)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlMode {
    /// Emotion mode: EmotionMapper translates arousal/valence/density/tension to MusicalParams
    Emotion,

    /// Direct mode: UI sends MusicalParams directly
    Direct,
}

impl Default for ControlMode {
    fn default() -> Self {
        Self::Emotion
    }
}

/// Public controller interface for Harmonium
/// This is the ONLY way external code should interact with the engine
pub struct HarmoniumController {
    /// Send commands to audio thread
    command_tx: rtrb::Producer<EngineCommand>,

    /// Receive reports from audio thread
    report_rx: rtrb::Consumer<EngineReport>,

    /// Current control mode
    control_mode: ControlMode,

    /// Cached state (last received report)
    cached_state: Option<EngineReport>,

    /// Cached emotional parameters (for emotion mode)
    cached_emotions: Option<EngineParams>,
}

impl HarmoniumController {
    /// Create a new controller
    ///
    /// # Arguments
    ///
    /// * `command_tx` - Producer for sending commands to engine
    /// * `report_rx` - Consumer for receiving reports from engine
    #[must_use]
    pub fn new(
        command_tx: rtrb::Producer<EngineCommand>,
        report_rx: rtrb::Consumer<EngineReport>,
    ) -> Self {
        Self {
            command_tx,
            report_rx,
            control_mode: ControlMode::default(),
            cached_state: None,
            cached_emotions: None,
        }
    }

    // === CORE OPERATIONS ===

    /// Send a command to the engine (non-blocking)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn send(&mut self, cmd: EngineCommand) -> Result<(), ControllerError> {
        self.command_tx
            .push(cmd)
            .map_err(|_| ControllerError::QueueFull)
    }

    /// Poll for new reports from the engine (non-blocking, drains queue)
    ///
    /// Updates cached_state with the latest report
    #[must_use]
    pub fn poll_reports(&mut self) -> Vec<EngineReport> {
        let mut reports = Vec::new();

        while let Ok(report) = self.report_rx.pop() {
            self.cached_state = Some(report.clone());
            reports.push(report);
        }

        reports
    }

    /// Get cached state (last received report)
    #[must_use]
    pub const fn get_state(&self) -> Option<&EngineReport> {
        self.cached_state.as_ref()
    }

    /// Get current control mode
    #[must_use]
    pub const fn get_mode(&self) -> ControlMode {
        self.control_mode
    }

    // === EMOTION MODE API ===

    /// Switch to emotion control mode
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn use_emotion_mode(&mut self) -> Result<(), ControllerError> {
        self.control_mode = ControlMode::Emotion;
        self.send(EngineCommand::UseEmotionMode)
    }

    /// Set emotional parameters (arousal, valence, density, tension)
    ///
    /// If in emotion mode, the EmotionMapper (running in the engine) will translate
    /// these to MusicalParams. If in direct mode, this has no effect.
    ///
    /// # Arguments
    ///
    /// * `arousal` - Energy level (0.0-1.0) → BPM (70-180)
    /// * `valence` - Emotional valence (-1.0 to 1.0) → Major/Minor
    /// * `density` - Rhythmic/harmonic density (0.0-1.0) → Pulses
    /// * `tension` - Overall tension (0.0-1.0) → Harmony strategy, LCC level
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_emotions(
        &mut self,
        arousal: f32,
        valence: f32,
        density: f32,
        tension: f32,
    ) -> Result<(), ControllerError> {
        // Cache emotional parameters
        let emotions = EngineParams {
            arousal,
            valence,
            density,
            tension,
            smoothness: self
                .cached_emotions
                .as_ref()
                .map(|e| e.smoothness)
                .unwrap_or(0.5),
            algorithm: self
                .cached_emotions
                .as_ref()
                .map(|e| e.algorithm)
                .unwrap_or(RhythmMode::default()),
            harmony_mode: self
                .cached_emotions
                .as_ref()
                .map(|e| e.harmony_mode)
                .unwrap_or(HarmonyMode::default()),
            gain_lead: self
                .cached_emotions
                .as_ref()
                .map(|e| e.gain_lead)
                .unwrap_or(1.0),
            gain_bass: self
                .cached_emotions
                .as_ref()
                .map(|e| e.gain_bass)
                .unwrap_or(0.6),
            gain_snare: self
                .cached_emotions
                .as_ref()
                .map(|e| e.gain_snare)
                .unwrap_or(0.5),
            gain_hat: self
                .cached_emotions
                .as_ref()
                .map(|e| e.gain_hat)
                .unwrap_or(0.4),
            vel_base_bass: self
                .cached_emotions
                .as_ref()
                .map(|e| e.vel_base_bass)
                .unwrap_or(80),
            vel_base_snare: self
                .cached_emotions
                .as_ref()
                .map(|e| e.vel_base_snare)
                .unwrap_or(100),
            record_wav: self
                .cached_emotions
                .as_ref()
                .map(|e| e.record_wav)
                .unwrap_or(false),
            record_midi: self
                .cached_emotions
                .as_ref()
                .map(|e| e.record_midi)
                .unwrap_or(false),
            record_musicxml: self
                .cached_emotions
                .as_ref()
                .map(|e| e.record_musicxml)
                .unwrap_or(false),
            enable_synthesis_morphing: self
                .cached_emotions
                .as_ref()
                .map(|e| e.enable_synthesis_morphing)
                .unwrap_or(true),
            poly_steps: self
                .cached_emotions
                .as_ref()
                .map(|e| e.poly_steps)
                .unwrap_or(48),
            fixed_kick: self
                .cached_emotions
                .as_ref()
                .map(|e| e.fixed_kick)
                .unwrap_or(false),
            channel_routing: self
                .cached_emotions
                .as_ref()
                .map(|e| e.channel_routing.clone())
                .unwrap_or_else(|| vec![-1; 16]),
            muted_channels: self
                .cached_emotions
                .as_ref()
                .map(|e| e.muted_channels.clone())
                .unwrap_or_else(|| vec![false; 16]),
        };

        self.cached_emotions = Some(emotions.clone());

        // Send emotion params command
        self.send(EngineCommand::SetEmotionParams {
            arousal,
            valence,
            density,
            tension,
        })
    }

    // === DIRECT MODE API ===

    /// Switch to direct technical control mode
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn use_direct_mode(&mut self) -> Result<(), ControllerError> {
        self.control_mode = ControlMode::Direct;
        self.send(EngineCommand::UseDirectMode)
    }

    // === GLOBAL CONTROLS ===

    /// Set BPM (70-180)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_bpm(&mut self, bpm: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetBpm(bpm.clamp(70.0, 180.0)))
    }

    /// Set master volume (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_master_volume(&mut self, volume: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetMasterVolume(volume.clamp(0.0, 1.0)))
    }

    /// Set time signature
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_time_signature(
        &mut self,
        numerator: usize,
        denominator: usize,
    ) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetTimeSignature {
            numerator,
            denominator,
        })
    }

    // === MODULE TOGGLES ===

    /// Enable/disable rhythm module
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn enable_rhythm(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.send(EngineCommand::EnableRhythm(enabled))
    }

    /// Enable/disable harmony module
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn enable_harmony(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.send(EngineCommand::EnableHarmony(enabled))
    }

    /// Enable/disable melody module
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn enable_melody(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.send(EngineCommand::EnableMelody(enabled))
    }

    /// Enable/disable voicing (harmonized chords)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn enable_voicing(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.send(EngineCommand::EnableVoicing(enabled))
    }

    // === RHYTHM CONTROLS ===

    /// Set rhythm mode
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_mode(&mut self, mode: RhythmMode) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmMode(mode))
    }

    /// Set rhythm density (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_density(&mut self, density: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmDensity(density.clamp(0.0, 1.0)))
    }

    /// Set rhythm tension (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_tension(&mut self, tension: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmTension(tension.clamp(0.0, 1.0)))
    }

    /// Set rhythm steps (16, 48, 96, 192)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_steps(&mut self, steps: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmSteps(steps))
    }

    /// Set rhythm pulses (1-steps)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_pulses(&mut self, pulses: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmPulses(pulses))
    }

    /// Set rhythm rotation (0-steps)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_rhythm_rotation(&mut self, rotation: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetRhythmRotation(rotation))
    }

    // === HARMONY CONTROLS ===

    /// Set harmony mode
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_harmony_mode(&mut self, mode: HarmonyMode) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetHarmonyMode(mode))
    }

    /// Set harmony strategy
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_harmony_strategy(
        &mut self,
        strategy: HarmonyStrategy,
    ) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetHarmonyStrategy(strategy))
    }

    /// Set harmony tension (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_harmony_tension(&mut self, tension: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetHarmonyTension(tension.clamp(0.0, 1.0)))
    }

    /// Set harmony valence (-1.0 to 1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_harmony_valence(&mut self, valence: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetHarmonyValence(valence.clamp(-1.0, 1.0)))
    }

    // === MELODY/VOICING CONTROLS ===

    /// Set melody smoothness (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_melody_smoothness(&mut self, smoothness: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetMelodySmoothness(
            smoothness.clamp(0.0, 1.0),
        ))
    }

    /// Set voicing density (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_voicing_density(&mut self, density: f32) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetVoicingDensity(density.clamp(0.0, 1.0)))
    }

    // === MIXER CONTROLS ===

    /// Set channel gain (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_channel_gain(
        &mut self,
        channel: u8,
        gain: f32,
    ) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetChannelGain {
            channel,
            gain: gain.clamp(0.0, 1.0),
        })
    }

    /// Set channel mute
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_channel_mute(
        &mut self,
        channel: u8,
        muted: bool,
    ) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetChannelMute { channel, muted })
    }

    // === RECORDING CONTROLS ===

    /// Start recording
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn start_recording(&mut self, format: RecordFormat) -> Result<(), ControllerError> {
        self.send(EngineCommand::StartRecording(format))
    }

    /// Stop recording
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn stop_recording(&mut self, format: RecordFormat) -> Result<(), ControllerError> {
        self.send(EngineCommand::StopRecording(format))
    }

    // === TIMELINE CONTROLS ===

    /// Seek to a specific bar (1-based)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn seek(&mut self, bar: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::Seek(bar))
    }

    /// Set loop region (start_bar..=end_bar, 1-based)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_loop(
        &mut self,
        start_bar: usize,
        end_bar: usize,
    ) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetLoop {
            start_bar,
            end_bar,
        })
    }

    /// Clear loop region
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn clear_loop(&mut self) -> Result<(), ControllerError> {
        self.send(EngineCommand::ClearLoop)
    }

    /// Seek playhead to a specific bar without resetting the writehead.
    ///
    /// Re-fills the ring buffer from the writehead's committed timeline
    /// so playback starts with the same measures the frontend displays.
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn seek_playhead(&mut self, bar: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::SeekPlayhead(bar))
    }

    // === UTILITY ===

    /// Request full state report
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn get_state_request(&mut self) -> Result<(), ControllerError> {
        self.send(EngineCommand::GetState)
    }

    /// Reset engine to defaults
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn reset(&mut self) -> Result<(), ControllerError> {
        self.send(EngineCommand::Reset)
    }

    // === WRITEHEAD CONTROLS ===

    /// Set writehead lookahead distance (minimum 4 bars)
    ///
    /// # Errors
    ///
    /// Returns `QueueFull` if the command queue is full
    pub fn set_writehead_lookahead(&mut self, bars: usize) -> Result<(), ControllerError> {
        self.send(EngineCommand::SetWriteheadLookahead(bars))
    }

    // === CONVENIENCE METHODS ===

    /// Get current BPM from cached state
    #[must_use]
    pub fn current_bpm(&self) -> Option<f32> {
        self.cached_state
            .as_ref()
            .map(|state| state.musical_params.bpm)
    }

    /// Get current chord from cached state
    #[must_use]
    pub fn current_chord(&self) -> Option<&str> {
        self.cached_state
            .as_ref()
            .map(|state| state.current_chord.as_str())
    }

    /// Get current bar from cached state
    #[must_use]
    pub fn current_bar(&self) -> Option<usize> {
        self.cached_state.as_ref().map(|state| state.current_bar)
    }

    /// Poll reports and collect all new measure snapshots.
    ///
    /// Drains `new_measures` from every report received since the last poll.
    /// The frontend should append these to its score cache for rendering.
    #[must_use]
    pub fn poll_new_measures(&mut self) -> Vec<MeasureSnapshot> {
        let reports = self.poll_reports();
        let mut measures = Vec::new();
        for report in reports {
            measures.extend(report.new_measures);
        }
        measures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_controller() -> HarmoniumController {
        let (cmd_tx, _cmd_rx) = rtrb::RingBuffer::new(1024);
        let (_report_tx, report_rx) = rtrb::RingBuffer::new(256);

        HarmoniumController::new(cmd_tx, report_rx)
    }

    #[test]
    fn test_controller_creation() {
        let controller = create_test_controller();
        assert_eq!(controller.get_mode(), ControlMode::Emotion);
        assert!(controller.get_state().is_none());
    }

    #[test]
    fn test_send_command() {
        let mut controller = create_test_controller();
        let result = controller.set_bpm(140.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mode_switching() {
        let mut controller = create_test_controller();
        assert_eq!(controller.get_mode(), ControlMode::Emotion);

        controller.use_direct_mode().unwrap();
        assert_eq!(controller.get_mode(), ControlMode::Direct);

        controller.use_emotion_mode().unwrap();
        assert_eq!(controller.get_mode(), ControlMode::Emotion);
    }

    #[test]
    fn test_poll_reports() {
        let (cmd_tx, _cmd_rx) = rtrb::RingBuffer::new(1024);
        let (mut report_tx, report_rx) = rtrb::RingBuffer::new(256);

        let mut controller = HarmoniumController::new(cmd_tx, report_rx);

        // Push a report
        let mut report = EngineReport::default();
        report.current_bar = 42;
        report_tx.push(report).unwrap();

        // Poll reports
        let reports = controller.poll_reports();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].current_bar, 42);

        // Check cached state
        assert_eq!(controller.current_bar(), Some(42));
    }

    #[test]
    fn test_set_emotions() {
        let mut controller = create_test_controller();
        let result = controller.set_emotions(0.8, 0.5, 0.7, 0.6);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bpm_clamping() {
        let mut controller = create_test_controller();

        // BPM should be clamped to 70-180
        controller.set_bpm(50.0).unwrap(); // Too low, should clamp to 70
        controller.set_bpm(200.0).unwrap(); // Too high, should clamp to 180
    }
}
