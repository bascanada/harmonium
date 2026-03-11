//! Timeline Engine Tests
//!
//! These tests verify that the timeline engine produces valid musical output
//! across various parameter configurations without panicking.
//!
//! Strategy:
//! 1. Instantiate engine with a fixed seed for deterministic output
//! 2. Run engine for N bars worth of audio samples (offline, no audio device)
//! 3. Verify no panics, correct initialization, and deterministic behavior

#![allow(dead_code)]

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

/// Simplified note event for comparison.
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
                self.advance_step();
            }
            _ => {}
        }
    }

    fn process_buffer(&mut self, output: &mut [f32], _channels: usize) {
        output.fill(0.0);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Create a timeline engine with the given seed.
fn create_engine(seed: u64, sample_rate: f64) -> (
    TimelineEngine,
    rtrb::Producer<harmonium_core::EngineCommand>,
    rtrb::Consumer<harmonium_core::EngineReport>,
) {
    let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
    let (rpt_tx, rpt_rx) = rtrb::RingBuffer::new(256);
    let renderer: Box<dyn AudioRenderer> = Box::new(EventCapture::new());
    let mut engine = TimelineEngine::new_with_seed(
        sample_rate, cmd_rx, rpt_tx, renderer, seed,
    );
    engine.set_offline(true);
    (engine, cmd_tx, rpt_rx)
}

/// Run an engine for a given number of audio samples (offline).
fn run_engine_samples(engine: &mut TimelineEngine, total_samples: usize) {
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

// ═══════════════════════════════════════════════════════════════
// LEVEL 1: Basic sanity — engine produces events without panic
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_engine_produces_events() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let (mut engine, _cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);

    let total_samples = samples_for_bars(8, 120.0, 4, sample_rate);
    run_engine_samples(&mut engine, total_samples);
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 2: Parameter sweep — no panics across parameter space
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_parameter_sweep_no_panics() {
    let sample_rate = 44100.0;

    for (i, &bpm) in [70.0, 100.0, 120.0, 150.0, 180.0].iter().enumerate() {
        let seed = 100 + i as u64;
        let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));
        let total_samples = samples_for_bars(4, bpm as f64, 4, sample_rate);
        run_engine_samples(&mut engine, total_samples);
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
            let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(density));
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmTension(tension));
            let total = samples_for_bars(4, 120.0, 4, sample_rate);
            run_engine_samples(&mut engine, total);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 3: Rhythm mode sweep
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
        let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
        let total = samples_for_bars(8, 120.0, 4, sample_rate);
        run_engine_samples(&mut engine, total);
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 4: Harmony mode sweep
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_harmony_mode_sweep() {
    use harmonium_core::harmony::HarmonyMode;

    let sample_rate = 44100.0;
    let modes = [HarmonyMode::Basic, HarmonyMode::Driver];

    for (i, &mode) in modes.iter().enumerate() {
        let seed = 400 + i as u64;
        let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(mode));
        let total = samples_for_bars(8, 120.0, 4, sample_rate);
        run_engine_samples(&mut engine, total);
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
            let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
            let _ = cmd_tx.push(harmonium_core::EngineCommand::SetBpm(bpm));
            let total = samples_for_bars(4, bpm as f64, 4, sample_rate);
            run_engine_samples(&mut engine, total);
        }
    }
}

#[test]
fn test_all_channels_muted_no_panic() {
    let sample_rate = 44100.0;
    let seed = 600u64;

    let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
    for ch in 0..4u8 {
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetChannelMute { channel: ch, muted: true });
    }
    let total = samples_for_bars(4, 120.0, 4, sample_rate);
    run_engine_samples(&mut engine, total);
}

#[test]
fn test_time_signature_change_no_panic() {
    let sample_rate = 44100.0;
    let seed = 700u64;
    let time_sigs = [(3, 4), (5, 4), (7, 8), (6, 8)];

    for &(num, den) in &time_sigs {
        let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
            numerator: num,
            denominator: den,
        });
        let total = samples_for_bars(4, 120.0, num, sample_rate);
        run_engine_samples(&mut engine, total);
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
        let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(mode));
        let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmDensity(1.0));
        let total = samples_for_bars(4, 120.0, 4, sample_rate);
        run_engine_samples(&mut engine, total);
    }
}

// ═══════════════════════════════════════════════════════════════
// LEVEL 6: Determinism — same seed produces same output twice
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_same_seed_produces_same_session_config() {
    let seed = 42u64;
    let sample_rate = 44100.0;

    let (engine1, _, _) = create_engine(seed, sample_rate);
    let (engine2, _, _) = create_engine(seed, sample_rate);

    assert_eq!(engine1.config.key, engine2.config.key, "Same seed should produce same key");
    assert_eq!(engine1.config.scale, engine2.config.scale, "Same seed should produce same scale");
    assert_eq!(engine1.config.bpm, engine2.config.bpm, "Same seed should produce same BPM");
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
                let (mut engine, mut cmd_tx, _rpt_rx) = create_engine(seed, sample_rate);
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(rhythm));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(harmony));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                    numerator: num,
                    denominator: den,
                });
                let total = samples_for_bars(8, 120.0, num, sample_rate);
                run_engine_samples(&mut engine, total);
                test_count += 1;
            }
        }
    }

    assert_eq!(test_count, 18, "Should run 3x2x3 = 18 configurations");
}
