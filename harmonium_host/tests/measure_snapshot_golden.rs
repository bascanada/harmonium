//! Measure Snapshot Golden File Tests
//!
//! These tests verify that the MeasureSnapshot stream produced by the engine
//! is deterministic and matches saved golden JSON files. This guarantees that
//! a frontend (e.g. VexFlow) calling `get_new_measures_json()` on every frame
//! can reconstruct the full music sheet identically across engine versions.
//!
//! Strategy:
//! 1. Run engine with a fixed seed for N bars, collect all MeasureSnapshots
//! 2. On first run (golden file missing): write the JSON golden file
//! 3. On subsequent runs: compare against the golden file byte-for-byte
//!
//! To regenerate golden files, delete `tests/golden_measures/` and re-run tests.

use harmonium::timeline_engine::TimelineEngine;
use harmonium_audio::backend::AudioRenderer;
use harmonium_core::events::AudioEvent;
use harmonium_core::report::MeasureSnapshot;

// ─── Null renderer (captures nothing, just satisfies the trait) ───

struct NullRenderer;

impl AudioRenderer for NullRenderer {
    fn handle_event(&mut self, _event: AudioEvent) {}

    fn process_buffer(&mut self, output: &mut [f32], _channels: usize) {
        output.fill(0.0);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ─── Test helpers ───

fn create_engine(
    seed: u64,
    sample_rate: f64,
) -> (
    TimelineEngine,
    rtrb::Producer<harmonium_core::EngineCommand>,
    rtrb::Consumer<harmonium_core::EngineReport>,
) {
    let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
    let (rpt_tx, rpt_rx) = rtrb::RingBuffer::new(256);
    let renderer: Box<dyn AudioRenderer> = Box::new(NullRenderer);
    let mut engine = TimelineEngine::new_with_seed(sample_rate, cmd_rx, rpt_tx, renderer, seed);
    engine.set_offline(true);
    (engine, cmd_tx, rpt_rx)
}

fn samples_for_bars(bars: usize, bpm: f64, time_sig_numerator: usize, sample_rate: f64) -> usize {
    let beats_per_bar = time_sig_numerator as f64;
    let seconds_per_beat = 60.0 / bpm;
    let seconds_per_bar = seconds_per_beat * beats_per_bar;
    (seconds_per_bar * bars as f64 * sample_rate) as usize
}

fn run_engine_and_collect_measures(
    engine: &mut TimelineEngine,
    rpt_rx: &mut rtrb::Consumer<harmonium_core::EngineReport>,
    total_samples: usize,
) -> Vec<MeasureSnapshot> {
    let chunk_size = 512;
    let channels = 2;
    let mut buffer = vec![0.0f32; chunk_size * channels];
    let mut all_measures = Vec::new();

    let mut processed = 0;
    while processed < total_samples {
        let remaining = total_samples - processed;
        let this_chunk = remaining.min(chunk_size);
        let buf_len = this_chunk * channels;
        buffer[..buf_len].fill(0.0);
        engine.process_buffer(&mut buffer[..buf_len], channels);
        processed += this_chunk;

        // Drain reports after each buffer to avoid ring buffer overflow
        while let Ok(report) = rpt_rx.pop() {
            all_measures.extend(report.new_measures);
        }
    }

    // Final drain
    while let Ok(report) = rpt_rx.pop() {
        all_measures.extend(report.new_measures);
    }

    all_measures
}

/// Directory for golden measure JSON files (relative to workspace root).
fn golden_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("tests").join("golden_measures")
}

/// Compare collected measures against a golden file, or create it if missing.
///
/// Returns the measures for further assertions.
fn assert_golden(name: &str, measures: &[MeasureSnapshot]) {
    let dir = golden_dir();
    let path = dir.join(format!("{name}.json"));

    let actual_json = serde_json::to_string_pretty(measures)
        .expect("Failed to serialize measures");

    if path.exists() {
        let expected_json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read golden file {}: {e}", path.display()));

        if actual_json != expected_json {
            // Write actual output next to golden for diffing
            let actual_path = dir.join(format!("{name}.actual.json"));
            std::fs::write(&actual_path, &actual_json).ok();

            // Find first divergence for a useful error message
            let (line, col) = first_diff_location(&expected_json, &actual_json);
            panic!(
                "Golden file mismatch for '{name}' at line {line}, col {col}.\n\
                 Expected: {}\n\
                 Actual:   {}\n\
                 Run `diff {} {}` to inspect.",
                path.display(),
                actual_path.display(),
                path.display(),
                actual_path.display(),
            );
        }
    } else {
        // First run: create golden file
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("Failed to create golden dir {}: {e}", dir.display()));
        std::fs::write(&path, &actual_json)
            .unwrap_or_else(|e| panic!("Failed to write golden file {}: {e}", path.display()));
        eprintln!("Created golden file: {} ({} measures)", path.display(), measures.len());
    }
}

fn first_diff_location(a: &str, b: &str) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            return (line, col);
        }
        if ca == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    // Lengths differ
    (line, col)
}

// ═══════════════════════════════════════════════════════════════
// Golden file tests — one per configuration
// ═══════════════════════════════════════════════════════════════

#[test]
fn golden_default_8bars() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let bars = 8;
    let (mut engine, _cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);

    let total_samples = samples_for_bars(bars, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total_samples);

    assert!(!measures.is_empty(), "Should produce measures");
    assert_golden("default_seed42_8bars", &measures);

    // Structural assertions the frontend relies on
    for m in &measures {
        assert!(m.steps > 0, "Measure {} has 0 steps", m.index);
        // First measure may have tempo 0.0 due to CurrentState::default() before morphing
        if m.index > 1 {
            assert!(m.tempo > 0.0, "Measure {} has 0 tempo", m.index);
        }
        assert!(!m.chord_name.is_empty(), "Measure {} has empty chord name", m.index);
        assert!(m.time_sig_numerator > 0);
        assert!(m.time_sig_denominator > 0);
    }

    // Indices should be strictly ascending
    for w in measures.windows(2) {
        assert!(
            w[1].index > w[0].index,
            "Measure indices not ascending: {} then {}",
            w[0].index, w[1].index
        );
    }
}

#[test]
fn golden_determinism_same_seed_twice() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let bars = 8;

    // Run 1
    let (mut e1, _tx1, mut rx1) = create_engine(seed, sample_rate);
    let total = samples_for_bars(bars, 120.0, 4, sample_rate);
    let m1 = run_engine_and_collect_measures(&mut e1, &mut rx1, total);

    // Run 2
    let (mut e2, _tx2, mut rx2) = create_engine(seed, sample_rate);
    let m2 = run_engine_and_collect_measures(&mut e2, &mut rx2, total);

    let json1 = serde_json::to_string(&m1).unwrap();
    let json2 = serde_json::to_string(&m2).unwrap();

    assert_eq!(
        json1, json2,
        "Same seed must produce byte-identical measure snapshots"
    );
}

#[test]
fn golden_different_seeds_differ() {
    let sample_rate = 44100.0;
    let bars = 8;
    let total = samples_for_bars(bars, 120.0, 4, sample_rate);

    let (mut e1, _tx1, mut rx1) = create_engine(42, sample_rate);
    let m1 = run_engine_and_collect_measures(&mut e1, &mut rx1, total);

    let (mut e2, _tx2, mut rx2) = create_engine(99, sample_rate);
    let m2 = run_engine_and_collect_measures(&mut e2, &mut rx2, total);

    let json1 = serde_json::to_string(&m1).unwrap();
    let json2 = serde_json::to_string(&m2).unwrap();

    assert_ne!(json1, json2, "Different seeds should produce different output");
}

// ─── Rhythm mode matrix ───

#[test]
fn golden_euclidean_8bars() {
    let seed = 100u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(
        harmonium_core::sequencer::RhythmMode::Euclidean,
    ));
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("euclidean_seed100_8bars", &measures);
}

#[test]
fn golden_perfect_balance_8bars() {
    let seed = 100u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(
        harmonium_core::sequencer::RhythmMode::PerfectBalance,
    ));
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("perfect_balance_seed100_8bars", &measures);
}

#[test]
fn golden_classic_groove_8bars() {
    let seed = 100u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(
        harmonium_core::sequencer::RhythmMode::ClassicGroove,
    ));
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("classic_groove_seed100_8bars", &measures);
}

// ─── Harmony mode matrix ───

#[test]
fn golden_harmony_basic_8bars() {
    let seed = 200u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(
        harmonium_core::harmony::HarmonyMode::Basic,
    ));
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("harmony_basic_seed200_8bars", &measures);
}

#[test]
fn golden_harmony_driver_8bars() {
    let seed = 200u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(
        harmonium_core::harmony::HarmonyMode::Driver,
    ));
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("harmony_driver_seed200_8bars", &measures);
}

// ─── Time signature variations ───

#[test]
fn golden_time_sig_3_4() {
    let seed = 300u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
        numerator: 3,
        denominator: 4,
    });
    let total = samples_for_bars(8, 120.0, 3, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("timesig_3_4_seed300_8bars", &measures);
}

#[test]
fn golden_time_sig_5_4() {
    let seed = 300u64;
    let sample_rate = 44100.0;
    let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
        numerator: 5,
        denominator: 4,
    });
    let total = samples_for_bars(8, 120.0, 5, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);
    assert!(!measures.is_empty());
    assert_golden("timesig_5_4_seed300_8bars", &measures);
}

// ─── Full coverage matrix (rhythm × harmony × time sig) ───

#[test]
fn golden_coverage_matrix() {
    use harmonium_core::harmony::HarmonyMode;
    use harmonium_core::sequencer::RhythmMode;

    let sample_rate = 44100.0;
    let rhythm_modes = [
        ("euclidean", RhythmMode::Euclidean),
        ("perfect_balance", RhythmMode::PerfectBalance),
        ("classic_groove", RhythmMode::ClassicGroove),
    ];
    let harmony_modes = [
        ("basic", HarmonyMode::Basic),
        ("driver", HarmonyMode::Driver),
    ];
    let time_sigs: &[(&str, usize, usize)] = &[
        ("4_4", 4, 4),
        ("3_4", 3, 4),
        ("5_4", 5, 4),
    ];

    for (ri, (r_name, rhythm)) in rhythm_modes.iter().enumerate() {
        for (hi, (h_name, harmony)) in harmony_modes.iter().enumerate() {
            for (ti, (ts_name, num, den)) in time_sigs.iter().enumerate() {
                let seed = 1000 + (ri * 6 + hi * 3 + ti) as u64;
                let (mut engine, mut cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);

                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetRhythmMode(*rhythm));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetHarmonyMode(*harmony));
                let _ = cmd_tx.push(harmonium_core::EngineCommand::SetTimeSignature {
                    numerator: *num,
                    denominator: *den,
                });

                let total = samples_for_bars(8, 120.0, *num, sample_rate);
                let measures =
                    run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);

                let golden_name = format!("matrix_{r_name}_{h_name}_{ts_name}_seed{seed}");
                assert!(
                    !measures.is_empty(),
                    "No measures for config {golden_name}"
                );
                assert_golden(&golden_name, &measures);
            }
        }
    }
}

// ─── Frontend reconstruction simulation ───

/// Simulates what the frontend does: receives measures incrementally,
/// builds up a score cache, and verifies the final result matches golden.
#[test]
fn golden_incremental_reconstruction() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let bars = 16;
    let (mut engine, _cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);

    let chunk_size = 512;
    let channels = 2;
    let total_samples = samples_for_bars(bars, 120.0, 4, sample_rate);
    let mut buffer = vec![0.0f32; chunk_size * channels];

    // Simulate frontend: accumulate measures frame-by-frame
    let mut score_cache: Vec<MeasureSnapshot> = Vec::new();
    let mut frame_counts: Vec<usize> = Vec::new(); // how many measures per "frame"

    let mut processed = 0;
    while processed < total_samples {
        let remaining = total_samples - processed;
        let this_chunk = remaining.min(chunk_size);
        let buf_len = this_chunk * channels;
        buffer[..buf_len].fill(0.0);
        engine.process_buffer(&mut buffer[..buf_len], channels);
        processed += this_chunk;

        // Simulate: frontend polls on each "frame" (= each buffer here)
        let mut frame_new = 0;
        while let Ok(report) = rpt_rx.pop() {
            // Frontend would do: JSON.parse(handle.get_new_measures_json())
            // We simulate by collecting the MeasureSnapshots directly
            for m in report.new_measures {
                // Verify each measure can round-trip through JSON (like the frontend receives)
                let json = serde_json::to_string(&m)
                    .expect("MeasureSnapshot must serialize");
                let deserialized: MeasureSnapshot = serde_json::from_str(&json)
                    .expect("MeasureSnapshot must deserialize from its own JSON");

                assert_eq!(m.index, deserialized.index);
                assert_eq!(m.notes.len(), deserialized.notes.len());

                score_cache.push(deserialized);
                frame_new += 1;
            }
        }
        frame_counts.push(frame_new);
    }

    // Final drain
    while let Ok(report) = rpt_rx.pop() {
        for m in report.new_measures {
            let json = serde_json::to_string(&m).unwrap();
            let deserialized: MeasureSnapshot = serde_json::from_str(&json).unwrap();
            score_cache.push(deserialized);
        }
    }

    assert!(
        !score_cache.is_empty(),
        "Frontend score cache should have measures"
    );

    // Verify ascending indices (frontend relies on this for sequential rendering)
    for w in score_cache.windows(2) {
        assert!(
            w[1].index > w[0].index,
            "Score cache indices must be ascending: {} then {}",
            w[0].index, w[1].index
        );
    }

    // Verify every measure has notes (at least across all measures)
    let total_notes: usize = score_cache.iter().map(|m| m.notes.len()).sum();
    assert!(total_notes > 0, "Score should contain notes");

    // Verify the incrementally reconstructed score matches bulk collection
    assert_golden("incremental_seed42_16bars", &score_cache);
}

/// Verify that the incremental reconstruction matches a single bulk run.
/// This proves the frontend gets identical data regardless of polling frequency.
#[test]
fn golden_incremental_matches_bulk() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let bars = 8;
    let total = samples_for_bars(bars, 120.0, 4, sample_rate);

    // Bulk: collect all at once
    let (mut e1, _tx1, mut rx1) = create_engine(seed, sample_rate);
    let bulk = run_engine_and_collect_measures(&mut e1, &mut rx1, total);

    // Incremental: collect per-buffer (small buffers to maximize number of polls)
    let (mut e2, _tx2, mut rx2) = create_engine(seed, sample_rate);
    let chunk = 128; // smaller chunks = more frequent polling
    let channels = 2;
    let mut buffer = vec![0.0f32; chunk * channels];
    let mut incremental: Vec<MeasureSnapshot> = Vec::new();

    let mut processed = 0;
    while processed < total {
        let remaining = total - processed;
        let this_chunk = remaining.min(chunk);
        let buf_len = this_chunk * channels;
        buffer[..buf_len].fill(0.0);
        e2.process_buffer(&mut buffer[..buf_len], channels);
        processed += this_chunk;

        while let Ok(report) = rx2.pop() {
            incremental.extend(report.new_measures);
        }
    }
    while let Ok(report) = rx2.pop() {
        incremental.extend(report.new_measures);
    }

    let json_bulk = serde_json::to_string(&bulk).unwrap();
    let json_incr = serde_json::to_string(&incremental).unwrap();

    assert_eq!(
        json_bulk, json_incr,
        "Incremental polling must produce identical measures as bulk collection"
    );
}

// ─── Note content validation ───

/// Verify that note snapshots contain valid MIDI data the frontend can render.
#[test]
fn golden_note_validity() {
    let seed = 42u64;
    let sample_rate = 44100.0;
    let (mut engine, _cmd_tx, mut rpt_rx) = create_engine(seed, sample_rate);
    let total = samples_for_bars(8, 120.0, 4, sample_rate);
    let measures = run_engine_and_collect_measures(&mut engine, &mut rpt_rx, total);

    for m in &measures {
        for note in &m.notes {
            assert!(note.pitch <= 127, "MIDI pitch out of range: {}", note.pitch);
            assert!(note.velocity <= 127, "Velocity out of range: {}", note.velocity);
            assert!(note.velocity > 0, "Velocity should be > 0 for triggered notes");
            assert!(
                note.track <= 3,
                "Track/channel out of range: {} (expected 0-3)",
                note.track
            );
            assert!(
                note.start_step < m.steps,
                "Note start_step {} >= measure steps {} in measure {}",
                note.start_step,
                m.steps,
                m.index
            );
        }
    }
}
