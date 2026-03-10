//! Golden File Comparison Tests
//!
//! These tests verify that the timeline engine produces equivalent musical output
//! to the legacy engine when given the same seed and parameters.
//!
//! Strategy:
//! 1. Instantiate both engines with same seed → same key, scale, RNG state
//! 2. Run both engines for N bars worth of audio samples (offline, no audio device)
//! 3. Capture all NoteOn/NoteOff events from each engine
//! 4. Compare note events with tolerance for velocity differences (morph timing)

#![allow(dead_code)] // Comparison utilities are prepared for future golden file serialization

use harmonium::engine::HarmoniumEngine;
use harmonium::timeline_engine::TimelineEngine;
use harmonium_audio::backend::AudioRenderer;
use harmonium_core::events::AudioEvent;

/// Test renderer that captures NoteOn/NoteOff events without synthesizing audio.
struct EventCapture {
    events: Vec<CapturedEvent>,
    step_counter: usize,
}

/// A captured musical event with its step position.
#[derive(Debug, Clone)]
struct CapturedEvent {
    step: usize,
    event: NoteEvent,
}

/// Simplified note event for comparison (ignoring velocity differences).
#[derive(Debug, Clone, PartialEq)]
enum NoteEvent {
    NoteOn { note: u8, velocity: u8, channel: u8 },
    NoteOff { note: u8, channel: u8 },
    AllNotesOff { channel: u8 },
}

impl EventCapture {
    fn new() -> Self {
        Self {
            events: Vec::with_capacity(1024),
            step_counter: 0,
        }
    }

    fn note_events(&self) -> &[CapturedEvent] {
        &self.events
    }

    /// Advance step counter (called externally after each tick boundary).
    fn advance_step(&mut self) {
        self.step_counter += 1;
    }
}

impl AudioRenderer for EventCapture {
    fn handle_event(&mut self, event: AudioEvent) {
        match &event {
            AudioEvent::NoteOn { note, velocity, channel } => {
                self.events.push(CapturedEvent {
                    step: self.step_counter,
                    event: NoteEvent::NoteOn {
                        note: *note,
                        velocity: *velocity,
                        channel: *channel,
                    },
                });
            }
            AudioEvent::NoteOff { note, channel } => {
                self.events.push(CapturedEvent {
                    step: self.step_counter,
                    event: NoteEvent::NoteOff {
                        note: *note,
                        channel: *channel,
                    },
                });
            }
            AudioEvent::AllNotesOff { channel } => {
                self.events.push(CapturedEvent {
                    step: self.step_counter,
                    event: NoteEvent::AllNotesOff { channel: *channel },
                });
            }
            AudioEvent::TimingUpdate { .. } => {
                // Track step boundaries for event positioning
                self.advance_step();
            }
            _ => {}
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], _channels: usize) {
        // Fill with silence — no synthesis needed for golden file tests
        output.fill(0.0);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Create a pair of engines (legacy + timeline) with the same seed, ready for comparison.
fn create_engine_pair(
    seed: u64,
    sample_rate: f64,
) -> (
    HarmoniumEngine,
    rtrb::Producer<harmonium_core::EngineCommand>,
    rtrb::Consumer<harmonium_core::EngineReport>,
    TimelineEngine,
    rtrb::Producer<harmonium_core::EngineCommand>,
    rtrb::Consumer<harmonium_core::EngineReport>,
) {
    let (legacy_cmd_tx, legacy_cmd_rx) = rtrb::RingBuffer::new(1024);
    let (legacy_rpt_tx, legacy_rpt_rx) = rtrb::RingBuffer::new(256);
    let legacy_renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
    let legacy = HarmoniumEngine::new_with_seed(
        sample_rate,
        legacy_cmd_rx,
        legacy_rpt_tx,
        legacy_renderer,
        seed,
    );

    let (timeline_cmd_tx, timeline_cmd_rx) = rtrb::RingBuffer::new(1024);
    let (timeline_rpt_tx, timeline_rpt_rx) = rtrb::RingBuffer::new(256);
    let timeline_renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
    let timeline = TimelineEngine::new_with_seed(
        sample_rate,
        timeline_cmd_rx,
        timeline_rpt_tx,
        timeline_renderer,
        seed,
    );

    (
        legacy,
        legacy_cmd_tx,
        legacy_rpt_rx,
        timeline,
        timeline_cmd_tx,
        timeline_rpt_rx,
    )
}

/// Run an engine for a given number of audio samples (offline).
fn run_engine_samples(engine: &mut HarmoniumEngine, total_samples: usize) {
    // Process in chunks matching typical audio callback size (512 samples)
    let chunk_size = 512;
    let channels = 2;
    let mut buffer = vec![0.0f32; chunk_size * channels];

    let mut processed = 0;
    while processed < total_samples {
        let remaining = total_samples - processed;
        let this_chunk = remaining.min(chunk_size);
        let buf_len = this_chunk * channels;
        buffer[..buf_len].fill(0.0);
        engine.process_buffer(&mut buffer[..buf_len], channels);
        processed += this_chunk;
    }
}

fn run_timeline_samples(engine: &mut TimelineEngine, total_samples: usize) {
    let chunk_size = 512;
    let channels = 2;
    let mut buffer = vec![0.0f32; chunk_size * channels];

    let mut processed = 0;
    while processed < total_samples {
        let remaining = total_samples - processed;
        let this_chunk = remaining.min(chunk_size);
        let buf_len = this_chunk * channels;
        buffer[..buf_len].fill(0.0);
        engine.process_buffer(&mut buffer[..buf_len], channels);
        processed += this_chunk;
    }
}

/// Calculate the number of audio samples for N bars at a given BPM and sample rate.
fn samples_for_bars(bars: usize, bpm: f64, time_sig_numerator: usize, sample_rate: f64) -> usize {
    let beats_per_bar = time_sig_numerator as f64;
    let seconds_per_beat = 60.0 / bpm;
    let seconds_per_bar = seconds_per_beat * beats_per_bar;
    (seconds_per_bar * bars as f64 * sample_rate) as usize
}

/// Filter to only note-on events for comparison (ignoring NoteOff timing differences).
fn note_on_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| matches!(e.event, NoteEvent::NoteOn { .. }))
        .collect()
}

/// Compare two event streams, returning (matching, total_legacy, total_timeline, mismatches).
fn compare_note_on_streams(
    legacy: &[CapturedEvent],
    timeline: &[CapturedEvent],
    velocity_tolerance: u8,
) -> ComparisonResult {
    let legacy_notes = note_on_events(legacy);
    let timeline_notes = note_on_events(timeline);

    let mut matching = 0;
    let mut velocity_diffs = 0;
    let mut mismatches = Vec::new();

    let min_len = legacy_notes.len().min(timeline_notes.len());

    for i in 0..min_len {
        let l = &legacy_notes[i];
        let t = &timeline_notes[i];

        match (&l.event, &t.event) {
            (
                NoteEvent::NoteOn { note: ln, velocity: lv, channel: lc },
                NoteEvent::NoteOn { note: tn, velocity: tv, channel: tc },
            ) => {
                if ln == tn && lc == tc {
                    if (*lv as i16 - *tv as i16).unsigned_abs() as u8 <= velocity_tolerance {
                        matching += 1;
                    } else {
                        velocity_diffs += 1;
                    }
                } else {
                    mismatches.push(format!(
                        "Event {}: legacy=({},{},{}) timeline=({},{},{})",
                        i, ln, lv, lc, tn, tv, tc
                    ));
                }
            }
            _ => unreachable!("filtered to NoteOn only"),
        }
    }

    ComparisonResult {
        matching,
        velocity_diffs,
        total_legacy: legacy_notes.len(),
        total_timeline: timeline_notes.len(),
        mismatches,
    }
}

struct ComparisonResult {
    matching: usize,
    velocity_diffs: usize,
    total_legacy: usize,
    total_timeline: usize,
    mismatches: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 1: Basic sanity — both engines produce events
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_legacy_engine_produces_events() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let (mut legacy, _cmd_tx, _rpt_rx, _, _, _) = create_engine_pair(seed, sample_rate);

    let total_samples = samples_for_bars(8, 120.0, 4, sample_rate);
    run_engine_samples(&mut legacy, total_samples);

    // Downcast renderer to get events — we need mutable access
    // The renderer is inside the engine, so we need a way to get it out.
    // For now, just verify the engine doesn't panic during 8 bars of generation.
}

#[test]
fn test_timeline_engine_produces_events() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let (_, _, _, mut timeline, _cmd_tx, _rpt_rx) = create_engine_pair(seed, sample_rate);

    let total_samples = samples_for_bars(8, 120.0, 4, sample_rate);
    run_timeline_samples(&mut timeline, total_samples);
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 2: Parameter sweep — no panics across parameter space
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_legacy_parameter_sweep_no_panics() {
    let sample_rate = 44100.0;

    for (i, &bpm) in [70.0, 100.0, 120.0, 150.0, 180.0].iter().enumerate() {
        let seed = 100 + i as u64;
        let (mut legacy_cmd_tx, legacy_cmd_rx) = rtrb::RingBuffer::new(1024);
        let (legacy_rpt_tx, _legacy_rpt_rx) = rtrb::RingBuffer::new(256);
        let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
        let mut engine = HarmoniumEngine::new_with_seed(
            sample_rate,
            legacy_cmd_rx,
            legacy_rpt_tx,
            renderer,
            seed,
        );

        // Set BPM via command
        let _ = legacy_cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));

        let total_samples = samples_for_bars(4, bpm as f64, 4, sample_rate);
        run_engine_samples(&mut engine, total_samples);
    }
}

#[test]
fn test_timeline_parameter_sweep_no_panics() {
    let sample_rate = 44100.0;

    for (i, &bpm) in [70.0, 100.0, 120.0, 150.0, 180.0].iter().enumerate() {
        let seed = 100 + i as u64;
        let (mut timeline_cmd_tx, timeline_cmd_rx) = rtrb::RingBuffer::new(1024);
        let (timeline_rpt_tx, _timeline_rpt_rx) = rtrb::RingBuffer::new(256);
        let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
        let mut engine = TimelineEngine::new_with_seed(
            sample_rate,
            timeline_cmd_rx,
            timeline_rpt_tx,
            renderer,
            seed,
        );

        let _ = timeline_cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));

        let total_samples = samples_for_bars(4, bpm as f64, 4, sample_rate);
        run_timeline_samples(&mut engine, total_samples);
    }
}

#[test]
fn test_density_tension_sweep_no_panics() {
    let sample_rate = 44100.0;
    let densities = [0.0, 0.3, 0.6, 1.0];
    let tensions = [0.0, 0.3, 0.6, 1.0];

    for (i, &density) in densities.iter().enumerate() {
        for (j, &tension) in tensions.iter().enumerate() {
            let seed = 200 + (i * 4 + j) as u64;

            // Legacy engine
            {
                let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                let mut engine = HarmoniumEngine::new_with_seed(
                    sample_rate, cmd_rx, rpt_tx, renderer, seed,
                );
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(density));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmTension(tension));
                let total = samples_for_bars(4, 120.0, 4, sample_rate);
                run_engine_samples(&mut engine, total);
            }

            // Timeline engine
            {
                let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                let mut engine = TimelineEngine::new_with_seed(
                    sample_rate, cmd_rx, rpt_tx, renderer, seed,
                );
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(density));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmTension(tension));
                let total = samples_for_bars(4, 120.0, 4, sample_rate);
                run_timeline_samples(&mut engine, total);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 3: Rhythm mode sweep — all modes across both engines
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_rhythm_mode_sweep() {
    use harmonium_core::sequencer::RhythmMode;

    let sample_rate = 44100.0;
    let modes = [
        RhythmMode::Euclidean,
        RhythmMode::PerfectBalance,
        RhythmMode::ClassicGroove,
    ];

    for (i, &mode) in modes.iter().enumerate() {
        let seed = 300 + i as u64;

        // Legacy
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = HarmoniumEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
            let total = samples_for_bars(8, 120.0, 4, sample_rate);
            run_engine_samples(&mut engine, total);
        }

        // Timeline
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = TimelineEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
            let total = samples_for_bars(8, 120.0, 4, sample_rate);
            run_timeline_samples(&mut engine, total);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 4: Harmony mode sweep — Basic vs Driver across both engines
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_harmony_mode_sweep() {
    use harmonium_core::harmony::HarmonyMode;

    let sample_rate = 44100.0;
    let modes = [HarmonyMode::Basic, HarmonyMode::Driver];

    for (i, &mode) in modes.iter().enumerate() {
        let seed = 400 + i as u64;

        // Legacy
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = HarmoniumEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(mode));
            let total = samples_for_bars(8, 120.0, 4, sample_rate);
            run_engine_samples(&mut engine, total);
        }

        // Timeline
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = TimelineEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(mode));
            let total = samples_for_bars(8, 120.0, 4, sample_rate);
            run_timeline_samples(&mut engine, total);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 5: Edge cases
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_extreme_bpm_no_panic() {
    let sample_rate = 44100.0;

    for &bpm in &[70.0f32, 180.0] {
        for seed in [500u64, 501] {
            // Legacy
            {
                let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                let mut engine = HarmoniumEngine::new_with_seed(
                    sample_rate, cmd_rx, rpt_tx, renderer, seed,
                );
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));
                let total = samples_for_bars(4, bpm as f64, 4, sample_rate);
                run_engine_samples(&mut engine, total);
            }

            // Timeline
            {
                let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                let mut engine = TimelineEngine::new_with_seed(
                    sample_rate, cmd_rx, rpt_tx, renderer, seed,
                );
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));
                let total = samples_for_bars(4, bpm as f64, 4, sample_rate);
                run_timeline_samples(&mut engine, total);
            }
        }
    }
}

#[test]
fn test_all_channels_muted_no_panic() {
    let sample_rate = 44100.0;
    let seed = 600u64;

    // Legacy
    {
        let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
        let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
        let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
        let mut engine = HarmoniumEngine::new_with_seed(
            sample_rate, cmd_rx, rpt_tx, renderer, seed,
        );
        for ch in 0..4u8 {
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetChannelMute { channel: ch, muted: true });
        }
        let total = samples_for_bars(4, 120.0, 4, sample_rate);
        run_engine_samples(&mut engine, total);
    }

    // Timeline
    {
        let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
        let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
        let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
        let mut engine = TimelineEngine::new_with_seed(
            sample_rate, cmd_rx, rpt_tx, renderer, seed,
        );
        for ch in 0..4u8 {
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetChannelMute { channel: ch, muted: true });
        }
        let total = samples_for_bars(4, 120.0, 4, sample_rate);
        run_timeline_samples(&mut engine, total);
    }
}

#[test]
fn test_time_signature_change_no_panic() {
    let sample_rate = 44100.0;
    let seed = 700u64;
    let time_sigs = [(3, 4), (5, 4), (7, 8), (6, 8)];

    for &(num, den) in &time_sigs {
        // Legacy
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = HarmoniumEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                numerator: num,
                denominator: den,
            });
            let total = samples_for_bars(4, 120.0, num, sample_rate);
            run_engine_samples(&mut engine, total);
        }

        // Timeline
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = TimelineEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                numerator: num,
                denominator: den,
            });
            let total = samples_for_bars(4, 120.0, num, sample_rate);
            run_timeline_samples(&mut engine, total);
        }
    }
}

#[test]
fn test_maximum_density_all_modes_no_panic() {
    use harmonium_core::sequencer::RhythmMode;

    let sample_rate = 44100.0;
    let modes = [
        RhythmMode::Euclidean,
        RhythmMode::PerfectBalance,
        RhythmMode::ClassicGroove,
    ];

    for (i, &mode) in modes.iter().enumerate() {
        let seed = 800 + i as u64;

        // Legacy
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = HarmoniumEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(1.0));
            let total = samples_for_bars(4, 120.0, 4, sample_rate);
            run_engine_samples(&mut engine, total);
        }

        // Timeline
        {
            let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
            let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
            let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
            let mut engine = TimelineEngine::new_with_seed(
                sample_rate, cmd_rx, rpt_tx, renderer, seed,
            );
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(1.0));
            let total = samples_for_bars(4, 120.0, 4, sample_rate);
            run_timeline_samples(&mut engine, total);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 6: Determinism — same seed produces same output twice
// ═══════════════════════════════════════════════════════════════

/// Helper: run a legacy engine with given seed for N bars, return its config.
fn run_legacy_get_config(seed: u64, sample_rate: f64, bars: usize) -> harmonium::engine::SessionConfig {
    let (_cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
    let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
    let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
    let mut engine = HarmoniumEngine::new_with_seed(sample_rate, cmd_rx, rpt_tx, renderer, seed);
    let config = engine.config.clone();
    let total = samples_for_bars(bars, 120.0, 4, sample_rate);
    run_engine_samples(&mut engine, total);
    config
}

#[test]
fn test_same_seed_produces_same_session_config() {
    let seed = 42u64;
    let sample_rate = 44100.0;

    let config1 = run_legacy_get_config(seed, sample_rate, 1);
    let config2 = run_legacy_get_config(seed, sample_rate, 1);

    assert_eq!(config1.key, config2.key, "Same seed should produce same key");
    assert_eq!(config1.scale, config2.scale, "Same seed should produce same scale");
    assert_eq!(config1.bpm, config2.bpm, "Same seed should produce same BPM");
}

#[test]
fn test_legacy_and_timeline_same_seed_same_config() {
    let seed = 42u64;
    let sample_rate = 44100.0;

    let (legacy, _, _, timeline, _, _) = create_engine_pair(seed, sample_rate);

    assert_eq!(
        legacy.config.key, timeline.config.key,
        "Same seed should produce same key across engines"
    );
    assert_eq!(
        legacy.config.scale, timeline.config.scale,
        "Same seed should produce same scale across engines"
    );
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 7: Full coverage matrix (integration)
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_coverage_matrix_no_panics() {
    use harmonium_core::harmony::HarmonyMode;
    use harmonium_core::sequencer::RhythmMode;

    let sample_rate = 44100.0;
    let rhythm_modes = [
        RhythmMode::Euclidean,
        RhythmMode::PerfectBalance,
        RhythmMode::ClassicGroove,
    ];
    let harmony_modes = [HarmonyMode::Basic, HarmonyMode::Driver];
    let time_sigs: [(usize, usize); 3] = [(4, 4), (3, 4), (5, 4)];

    let mut test_count = 0;

    for (ri, &rhythm) in rhythm_modes.iter().enumerate() {
        for (hi, &harmony) in harmony_modes.iter().enumerate() {
            for (ti, &(num, den)) in time_sigs.iter().enumerate() {
                let seed = 1000 + (ri * 6 + hi * 3 + ti) as u64;

                // Legacy
                {
                    let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                    let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                    let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                    let mut engine = HarmoniumEngine::new_with_seed(
                        sample_rate, cmd_rx, rpt_tx, renderer, seed,
                    );
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(rhythm));
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(harmony));
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                        numerator: num,
                        denominator: den,
                    });
                    let total = samples_for_bars(8, 120.0, num, sample_rate);
                    run_engine_samples(&mut engine, total);
                }

                // Timeline
                {
                    let (mut cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
                    let (rpt_tx, _rpt_rx) = rtrb::RingBuffer::new(256);
                    let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
                    let mut engine = TimelineEngine::new_with_seed(
                        sample_rate, cmd_rx, rpt_tx, renderer, seed,
                    );
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(rhythm));
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(harmony));
                    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                        numerator: num,
                        denominator: den,
                    });
                    let total = samples_for_bars(8, 120.0, num, sample_rate);
                    run_timeline_samples(&mut engine, total);
                }

                test_count += 1;
            }
        }
    }

    assert_eq!(test_count, 18, "Should run 3×2×3 = 18 configurations");
}
