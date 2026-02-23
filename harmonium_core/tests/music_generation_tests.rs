//! Music Generation Integration Tests
//!
//! These tests run the ACTUAL music generation (Sequencer + `HarmonicDriver`)
//! and export the results to `MusicXML` for visual review in `MuseScore`.
//!
//! Run with: `cargo test -p harmonium_core --test music_generation_tests -- --ignored --nocapture`
//!
//! Output files: `target/generated_music/`

use std::path::Path;

use harmonium_core::{
    events::AudioEvent,
    exporters::{ChordSymbol, write_musicxml_with_chords},
    harmony::driver::HarmonicDriver,
    params::MusicalParams,
    sequencer::{RhythmMode, Sequencer},
};

/// Output directory for generated music
const OUTPUT_DIR: &str = "target/generated_music";

/// Standard samples per step at 120 BPM, 44100 Hz
const SAMPLES_PER_STEP: usize = 11025;

/// Simple deterministic RNG for reproducible tests
struct TestRng(u64);

impl TestRng {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.0 >> 16) & 0x7fff) as f32 / 32768.0
    }

    fn next_range(&mut self, range: std::ops::Range<usize>) -> usize {
        let f = self.next_f32();
        range.start + (f * (range.end - range.start) as f32) as usize
    }
}

impl harmonium_core::harmony::RngCore for TestRng {
    fn next_f32(&mut self) -> f32 {
        Self::next_f32(self)
    }

    fn next_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        self.next_range(range)
    }
}

fn setup_output_dir() {
    std::fs::create_dir_all(OUTPUT_DIR).expect("Failed to create output directory");
}

/// Generate music and export to `MusicXML`
fn generate_and_export(name: &str, params: &MusicalParams, measures: usize, seed: u64) {
    setup_output_dir();

    let mut rng = TestRng::new(seed);
    let steps_per_measure = params.rhythm_steps;
    let total_steps = measures * steps_per_measure;

    // Setup sequencer
    let mut sequencer = Sequencer::new_with_mode(
        params.rhythm_steps,
        params.rhythm_pulses,
        params.bpm,
        params.rhythm_mode,
    );
    // Set density and tension, then regenerate pattern
    sequencer.density = params.rhythm_density;
    sequencer.tension = params.rhythm_tension;
    sequencer.regenerate_pattern();

    // Setup harmony driver
    let mut driver = HarmonicDriver::new(params.key_root);

    // Collect events and chord symbols
    let mut events: Vec<(f64, AudioEvent)> = Vec::new(); // Changed to f64 for step-based timestamps
    let mut chord_symbols: Vec<ChordSymbol> = Vec::new();
    let mut current_chord = driver.current_chord().clone();
    let mut steps_in_chord = 0;
    let steps_per_chord = params.harmony_measures_per_chord * steps_per_measure;

    // Record initial chord at step 0
    chord_symbols.push(ChordSymbol::new(0, current_chord.root, current_chord.chord_type.suffix()));

    // Active notes for NoteOff tracking
    let mut active_bass: Option<u8> = None;
    let mut active_lead: Option<u8> = None;

    for step in 0..total_steps {
        let timestamp = step as f64; // Use step directly as timestamp

        // === HARMONY: Change chord every N measures ===
        if steps_in_chord >= steps_per_chord {
            let decision =
                driver.next_chord(params.harmony_tension, params.harmony_valence, &mut rng);
            current_chord = decision.next_chord;
            steps_in_chord = 0;

            // Record chord change
            chord_symbols.push(ChordSymbol::new(
                step,
                current_chord.root,
                current_chord.chord_type.suffix(),
            ));
        }
        steps_in_chord += 1;

        // === RHYTHM: Get triggers from sequencer ===
        let trigger = sequencer.tick();

        // === BASS (Channel 0) ===
        if trigger.kick {
            // Release previous bass note
            if let Some(prev_note) = active_bass.take() {
                events.push((
                    timestamp,
                    AudioEvent::NoteOff { id: None, note: prev_note, channel: 0 },
                ));
            }

            // Play new bass note (harmonized to current chord root)
            let bass_note = if params.fixed_kick {
                36 // Fixed C1 for drum kit mode
            } else {
                // Harmonized bass: chord root in octave 2
                36 + current_chord.root
            };
            let velocity = (trigger.velocity * 100.0) as u8;
            events.push((
                timestamp,
                AudioEvent::NoteOn { id: None, note: bass_note, velocity, channel: 0 },
            ));
            active_bass = Some(bass_note);
        }

        // === SNARE (Channel 2) ===
        if trigger.snare {
            let velocity = (trigger.velocity * 90.0) as u8;
            events
                .push((timestamp, AudioEvent::NoteOn { id: None, note: 38, velocity, channel: 2 }));
            // Short duration for drums (half step)
            let off_time = timestamp + 0.5;
            events.push((off_time, AudioEvent::NoteOff { id: None, note: 38, channel: 2 }));
        }

        // === HI-HAT (Channel 3) ===
        if trigger.hat {
            let velocity = (trigger.velocity * 70.0) as u8;
            events
                .push((timestamp, AudioEvent::NoteOn { id: None, note: 42, velocity, channel: 3 }));
            let off_time = timestamp + 0.25; // Quarter step
            events.push((off_time, AudioEvent::NoteOff { id: None, note: 42, channel: 3 }));
        }

        // === LEAD/MELODY (Channel 1) ===
        // Play melody on every other step for a simple pattern
        let is_melody_step = step % 2 == 0;
        if is_melody_step {
            // Release previous lead note
            if let Some(prev_note) = active_lead.take() {
                events.push((
                    timestamp,
                    AudioEvent::NoteOff { id: None, note: prev_note, channel: 1 },
                ));
            }

            // Generate melody note from chord tones
            // melody_octave=4 means standard notation octave 4 (middle C = C4 = MIDI 60)
            // MIDI formula: 12 * (octave + 1) + pitch_class
            let chord_pitches = current_chord.pitch_classes();
            let melody_octave = params.melody_octave as u8;
            let base_pitch = chord_pitches[rng.next_range(0..chord_pitches.len())];
            let melody_note = 12 * (melody_octave + 1) + base_pitch; // +1 to convert to MIDI octave

            events.push((
                timestamp,
                AudioEvent::NoteOn { id: None, note: melody_note, velocity: 80, channel: 1 },
            ));
            active_lead = Some(melody_note);
        }
    }

    // Final NoteOffs
    let end_time = total_steps as f64;
    if let Some(note) = active_bass {
        events.push((end_time, AudioEvent::NoteOff { id: None, note, channel: 0 }));
    }
    if let Some(note) = active_lead {
        events.push((end_time, AudioEvent::NoteOff { id: None, note, channel: 1 }));
    }

    // Sort events by timestamp
    events.sort_by(|(t1, _), (t2, _)| t1.partial_cmp(t2).unwrap());

    // Export to MusicXML with chord symbols
    let path = Path::new(OUTPUT_DIR).join(format!("{name}.musicxml"));
    write_musicxml_with_chords(&events, &chord_symbols, params, SAMPLES_PER_STEP, &path)
        .unwrap_or_else(|_| panic!("Failed to write {name}"));

    println!(
        "Generated: {} ({} measures, {} events, {} chords)",
        path.display(),
        measures,
        events.len(),
        chord_symbols.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// RHYTHM MODE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_euclidean_4_pulses() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::Euclidean;
    params.rhythm_steps = 16;
    params.rhythm_pulses = 4;
    params.key_root = 0; // C

    generate_and_export("rhythm_euclidean_4_pulses", &params, 4, 42);
}

#[test]
#[ignore]
fn test_euclidean_5_pulses() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::Euclidean;
    params.rhythm_steps = 16;
    params.rhythm_pulses = 5; // Creates interesting syncopation

    generate_and_export("rhythm_euclidean_5_pulses", &params, 4, 42);
}

#[test]
#[ignore]
fn test_euclidean_7_pulses() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::Euclidean;
    params.rhythm_steps = 16;
    params.rhythm_pulses = 7; // Afro-Cuban feel

    generate_and_export("rhythm_euclidean_7_pulses", &params, 4, 42);
}

#[test]
#[ignore]
fn test_perfect_balance_low_density() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::PerfectBalance;
    params.rhythm_steps = 48;
    params.rhythm_density = 0.3;
    params.rhythm_tension = 0.2;

    generate_and_export("rhythm_perfect_balance_low_density", &params, 4, 42);
}

#[test]
#[ignore]
fn test_perfect_balance_high_density() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::PerfectBalance;
    params.rhythm_steps = 48;
    params.rhythm_density = 0.8;
    params.rhythm_tension = 0.6;

    generate_and_export("rhythm_perfect_balance_high_density", &params, 4, 42);
}

#[test]
#[ignore]
fn test_classic_groove_half_time() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::ClassicGroove;
    params.rhythm_steps = 16;
    params.rhythm_density = 0.2; // Half-time

    generate_and_export("rhythm_classic_half_time", &params, 4, 42);
}

#[test]
#[ignore]
fn test_classic_groove_four_on_floor() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::ClassicGroove;
    params.rhythm_steps = 16;
    params.rhythm_density = 0.5; // Four-on-the-floor

    generate_and_export("rhythm_classic_four_on_floor", &params, 4, 42);
}

#[test]
#[ignore]
fn test_classic_groove_breakbeat() {
    let mut params = MusicalParams::default();
    params.rhythm_mode = RhythmMode::ClassicGroove;
    params.rhythm_steps = 16;
    params.rhythm_density = 0.9; // Breakbeat

    generate_and_export("rhythm_classic_breakbeat", &params, 4, 42);
}

// ═══════════════════════════════════════════════════════════════════════════
// HARMONY TENSION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_harmony_low_tension_steedman() {
    let mut params = MusicalParams::default();
    params.harmony_tension = 0.2; // Should use Steedman (functional harmony)
    params.harmony_valence = 0.5; // Major
    params.harmony_measures_per_chord = 1;

    generate_and_export("harmony_low_tension_steedman", &params, 8, 42);
}

#[test]
#[ignore]
fn test_harmony_medium_tension_parsimonious() {
    let mut params = MusicalParams::default();
    params.harmony_tension = 0.55; // Should use Parsimonious
    params.harmony_valence = 0.3;
    params.harmony_measures_per_chord = 1;

    generate_and_export("harmony_medium_tension_parsimonious", &params, 8, 42);
}

#[test]
#[ignore]
fn test_harmony_high_tension_neo_riemannian() {
    let mut params = MusicalParams::default();
    params.harmony_tension = 0.8; // Should use Neo-Riemannian
    params.harmony_valence = 0.0; // Neutral
    params.harmony_measures_per_chord = 1;

    generate_and_export("harmony_high_tension_neo_riemannian", &params, 8, 42);
}

#[test]
#[ignore]
fn test_harmony_minor_key() {
    let mut params = MusicalParams::default();
    params.harmony_tension = 0.3;
    params.harmony_valence = -0.7; // Minor
    params.key_root = 9; // A minor
    params.harmony_measures_per_chord = 2;

    generate_and_export("harmony_a_minor", &params, 8, 42);
}

#[test]
#[ignore]
fn test_harmony_major_key() {
    let mut params = MusicalParams::default();
    params.harmony_tension = 0.3;
    params.harmony_valence = 0.7; // Major
    params.key_root = 7; // G major
    params.harmony_measures_per_chord = 2;

    generate_and_export("harmony_g_major", &params, 8, 42);
}

// ═══════════════════════════════════════════════════════════════════════════
// KEY SIGNATURE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_key_c_major() {
    let mut params = MusicalParams::default();
    params.key_root = 0;
    params.harmony_valence = 0.5;
    generate_and_export("key_c_major", &params, 4, 42);
}

#[test]
#[ignore]
fn test_key_d_major() {
    let mut params = MusicalParams::default();
    params.key_root = 2;
    params.harmony_valence = 0.5;
    generate_and_export("key_d_major", &params, 4, 42);
}

#[test]
#[ignore]
fn test_key_f_major() {
    let mut params = MusicalParams::default();
    params.key_root = 5;
    params.harmony_valence = 0.5;
    generate_and_export("key_f_major", &params, 4, 42);
}

#[test]
#[ignore]
fn test_key_bb_major() {
    let mut params = MusicalParams::default();
    params.key_root = 10;
    params.harmony_valence = 0.5;
    generate_and_export("key_bb_major", &params, 4, 42);
}

#[test]
#[ignore]
fn test_key_e_minor() {
    let mut params = MusicalParams::default();
    params.key_root = 4;
    params.harmony_valence = -0.5;
    generate_and_export("key_e_minor", &params, 4, 42);
}

// ═══════════════════════════════════════════════════════════════════════════
// COMBINED SCENARIO TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_scenario_calm_ambient() {
    let mut params = MusicalParams::default();
    params.bpm = 80.0;
    params.rhythm_mode = RhythmMode::Euclidean;
    params.rhythm_steps = 16;
    params.rhythm_pulses = 3;
    params.rhythm_density = 0.3;
    params.harmony_tension = 0.2;
    params.harmony_valence = 0.4;
    params.key_root = 0; // C major

    generate_and_export("scenario_calm_ambient", &params, 8, 123);
}

#[test]
#[ignore]
fn test_scenario_energetic_dance() {
    let mut params = MusicalParams::default();
    params.bpm = 128.0;
    params.rhythm_mode = RhythmMode::ClassicGroove;
    params.rhythm_steps = 16;
    params.rhythm_density = 0.5; // Four-on-the-floor
    params.harmony_tension = 0.5;
    params.harmony_valence = 0.6;
    params.key_root = 7; // G major

    generate_and_export("scenario_energetic_dance", &params, 8, 456);
}

#[test]
#[ignore]
fn test_scenario_dark_tense() {
    let mut params = MusicalParams::default();
    params.bpm = 100.0;
    params.rhythm_mode = RhythmMode::PerfectBalance;
    params.rhythm_steps = 48;
    params.rhythm_density = 0.6;
    params.rhythm_tension = 0.7;
    params.harmony_tension = 0.8;
    params.harmony_valence = -0.8; // Minor
    params.key_root = 2; // D minor

    generate_and_export("scenario_dark_tense", &params, 8, 789);
}

#[test]
#[ignore]
fn test_scenario_jazz_swing() {
    let mut params = MusicalParams::default();
    params.bpm = 140.0;
    params.rhythm_mode = RhythmMode::Euclidean;
    params.rhythm_steps = 12; // 3/4 or swing feel
    params.rhythm_pulses = 4;
    params.harmony_tension = 0.5; // Parsimonious for 7th chords
    params.harmony_valence = 0.2;
    params.harmony_measures_per_chord = 1;
    params.key_root = 5; // F major

    generate_and_export("scenario_jazz_swing", &params, 8, 999);
}

// ═══════════════════════════════════════════════════════════════════════════
// GENERATE ALL
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generate_all_music_tests() {
    println!("\n========================================");
    println!("Generating music with REAL generation...");
    println!("Output: {OUTPUT_DIR}/");
    println!("========================================\n");

    // Rhythm tests
    test_euclidean_4_pulses();
    test_euclidean_5_pulses();
    test_euclidean_7_pulses();
    test_perfect_balance_low_density();
    test_perfect_balance_high_density();
    test_classic_groove_half_time();
    test_classic_groove_four_on_floor();
    test_classic_groove_breakbeat();

    // Harmony tests
    test_harmony_low_tension_steedman();
    test_harmony_medium_tension_parsimonious();
    test_harmony_high_tension_neo_riemannian();
    test_harmony_minor_key();
    test_harmony_major_key();

    // Key tests
    test_key_c_major();
    test_key_d_major();
    test_key_f_major();
    test_key_bb_major();
    test_key_e_minor();

    // Scenarios
    test_scenario_calm_ambient();
    test_scenario_energetic_dance();
    test_scenario_dark_tense();
    test_scenario_jazz_swing();

    println!("\n========================================");
    println!("Done! Open files in MuseScore to review.");
    println!("========================================\n");
}
