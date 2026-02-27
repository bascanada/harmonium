//! VexFlow Format Validation Tests
//!
//! Comprehensive tests to validate that harmonium_core's notation format
//! provides all required data for VexFlow rendering.
//!
//! Run with: `cargo test -p harmonium_core --test vexflow_format_tests`

use harmonium_core::notation::{
    Duration, DurationBase, NoteStep, Pitch,
};

// ═══════════════════════════════════════════════════════════════════
// PITCH CONVERSION TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_pitch_to_vexflow_all_steps_natural() {
    // Test all note steps without alterations
    let test_cases = vec![
        (NoteStep::C, 4, 0, "c/4"),
        (NoteStep::D, 4, 0, "d/4"),
        (NoteStep::E, 4, 0, "e/4"),
        (NoteStep::F, 4, 0, "f/4"),
        (NoteStep::G, 4, 0, "g/4"),
        (NoteStep::A, 4, 0, "a/4"),
        (NoteStep::B, 4, 0, "b/4"),
    ];

    for (step, octave, alter, expected) in test_cases {
        let pitch = Pitch { step, octave, alter };
        assert_eq!(
            pitch.to_vexflow(),
            expected,
            "Pitch {:?} should convert to {}",
            pitch,
            expected
        );
    }
}

#[test]
fn test_pitch_to_vexflow_sharps() {
    // Test single and double sharps
    let test_cases = vec![
        (NoteStep::C, 4, 1, "c#/4"),
        (NoteStep::F, 5, 1, "f#/5"),
        (NoteStep::G, 3, 1, "g#/3"),
        (NoteStep::D, 4, 2, "d##/4"), // Double sharp
        (NoteStep::A, 5, 2, "a##/5"), // Double sharp
    ];

    for (step, octave, alter, expected) in test_cases {
        let pitch = Pitch { step, octave, alter };
        assert_eq!(
            pitch.to_vexflow(),
            expected,
            "Sharp pitch {:?} should convert to {}",
            pitch,
            expected
        );
    }
}

#[test]
fn test_pitch_to_vexflow_flats() {
    // Test single and double flats
    let test_cases = vec![
        (NoteStep::B, 3, -1, "bb/3"),
        (NoteStep::E, 4, -1, "eb/4"),
        (NoteStep::A, 5, -1, "ab/5"),
        (NoteStep::B, 4, -2, "bbb/4"), // Double flat
        (NoteStep::E, 3, -2, "ebb/3"), // Double flat
    ];

    for (step, octave, alter, expected) in test_cases {
        let pitch = Pitch { step, octave, alter };
        assert_eq!(
            pitch.to_vexflow(),
            expected,
            "Flat pitch {:?} should convert to {}",
            pitch,
            expected
        );
    }
}

#[test]
fn test_pitch_to_vexflow_all_octaves() {
    // Test octave range 0-9 (VexFlow standard range)
    for octave in 0..=9 {
        let pitch = Pitch {
            step: NoteStep::C,
            octave,
            alter: 0,
        };
        let expected = format!("c/{}", octave);
        assert_eq!(
            pitch.to_vexflow(),
            expected,
            "C at octave {} should convert to {}",
            octave,
            expected
        );
    }
}

#[test]
fn test_pitch_to_vexflow_format_validation() {
    // Comprehensive format validation: [step][accidental]/[octave]
    let pitch1 = Pitch {
        step: NoteStep::C,
        octave: 4,
        alter: 0,
    };
    let vexflow1 = pitch1.to_vexflow();
    assert!(
        vexflow1.contains('/'),
        "VexFlow format must contain '/' separator"
    );
    assert_eq!(vexflow1.chars().filter(|&c| c == '/').count(), 1, "Must have exactly one '/'");

    let pitch2 = Pitch {
        step: NoteStep::F,
        octave: 5,
        alter: 1,
    };
    let vexflow2 = pitch2.to_vexflow();
    assert!(vexflow2.starts_with('f'), "Must start with lowercase note step");
    assert!(vexflow2.ends_with('5'), "Must end with octave number");
}

// ═══════════════════════════════════════════════════════════════════
// DURATION CONVERSION TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_duration_to_vexflow_all_bases() {
    // Test all base duration values
    let test_cases = vec![
        (DurationBase::Whole, "w"),
        (DurationBase::Half, "h"),
        (DurationBase::Quarter, "q"),
        (DurationBase::Eighth, "8"),
        (DurationBase::Sixteenth, "16"),
        (DurationBase::ThirtySecond, "32"),
    ];

    for (base, expected) in test_cases {
        let duration = Duration::new(base);
        assert_eq!(
            duration.to_vexflow(),
            expected,
            "Duration {:?} should convert to {}",
            base,
            expected
        );
    }
}

#[test]
fn test_duration_to_vexflow_single_dot() {
    // Test dotted notes (single dot)
    let test_cases = vec![
        (DurationBase::Whole, "wd"),
        (DurationBase::Half, "hd"),
        (DurationBase::Quarter, "qd"),
        (DurationBase::Eighth, "8d"),
        (DurationBase::Sixteenth, "16d"),
        (DurationBase::ThirtySecond, "32d"),
    ];

    for (base, expected) in test_cases {
        let duration = Duration::new(base).dotted();
        assert_eq!(
            duration.to_vexflow(),
            expected,
            "Dotted {:?} should convert to {}",
            base,
            expected
        );
    }
}

#[test]
fn test_duration_to_vexflow_double_dot() {
    // Test double-dotted notes
    let test_cases = vec![
        (DurationBase::Quarter, "qdd"),
        (DurationBase::Half, "hdd"),
        (DurationBase::Eighth, "8dd"),
    ];

    for (base, expected) in test_cases {
        let duration = Duration {
            base,
            dots: 2,
            tuplet: None,
        };
        assert_eq!(
            duration.to_vexflow(),
            expected,
            "Double-dotted {:?} should convert to {}",
            base,
            expected
        );
    }
}

#[test]
fn test_duration_to_vexflow_format_validation() {
    // Validate format: [base][dots]
    let dur1 = Duration::new(DurationBase::Quarter);
    assert_eq!(dur1.to_vexflow(), "q", "Quarter note should be 'q'");

    let dur2 = Duration::new(DurationBase::Quarter).dotted();
    assert_eq!(dur2.to_vexflow(), "qd", "Dotted quarter should be 'qd'");

    let dur3 = Duration {
        base: DurationBase::Eighth,
        dots: 2,
        tuplet: None,
    };
    assert_eq!(dur3.to_vexflow(), "8dd", "Double-dotted eighth should be '8dd'");
}

#[test]
fn test_duration_beats_calculation() {
    // Validate beat calculations for VexFlow timing
    let quarter = Duration::new(DurationBase::Quarter);
    assert_eq!(quarter.to_beats(), 1.0, "Quarter note = 1 beat");

    let half = Duration::new(DurationBase::Half);
    assert_eq!(half.to_beats(), 2.0, "Half note = 2 beats");

    let whole = Duration::new(DurationBase::Whole);
    assert_eq!(whole.to_beats(), 4.0, "Whole note = 4 beats");

    let eighth = Duration::new(DurationBase::Eighth);
    assert_eq!(eighth.to_beats(), 0.5, "Eighth note = 0.5 beats");

    let sixteenth = Duration::new(DurationBase::Sixteenth);
    assert_eq!(sixteenth.to_beats(), 0.25, "Sixteenth note = 0.25 beats");

    // Test dotted durations
    let dotted_quarter = Duration::new(DurationBase::Quarter).dotted();
    assert_eq!(
        dotted_quarter.to_beats(),
        1.5,
        "Dotted quarter = 1.5 beats"
    );

    let dotted_half = Duration::new(DurationBase::Half).dotted();
    assert_eq!(dotted_half.to_beats(), 3.0, "Dotted half = 3 beats");

    // Test double-dotted
    let double_dotted_quarter = Duration {
        base: DurationBase::Quarter,
        dots: 2,
        tuplet: None,
    };
    assert_eq!(
        double_dotted_quarter.to_beats(),
        1.75,
        "Double-dotted quarter = 1.75 beats"
    );
}

#[test]
fn test_duration_tuplet_beats() {
    // Test tuplet duration calculations
    let triplet_eighth = Duration {
        base: DurationBase::Eighth,
        dots: 0,
        tuplet: Some((3, 2)), // 3 in the time of 2
    };
    // Eighth = 0.5, triplet = 0.5 * 2/3 = 0.333...
    assert!((triplet_eighth.to_beats() - 0.333333).abs() < 0.0001);

    let quintuplet_sixteenth = Duration {
        base: DurationBase::Sixteenth,
        dots: 0,
        tuplet: Some((5, 4)), // 5 in the time of 4
    };
    // Sixteenth = 0.25, quintuplet = 0.25 * 4/5 = 0.2
    assert_eq!(quintuplet_sixteenth.to_beats(), 0.2);
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_pitch_extreme_values() {
    // Test edge cases for pitches

    // Lowest reasonable pitch (C0)
    let lowest = Pitch {
        step: NoteStep::C,
        octave: 0,
        alter: 0,
    };
    assert_eq!(lowest.to_vexflow(), "c/0");

    // Highest reasonable pitch (B9)
    let highest = Pitch {
        step: NoteStep::B,
        octave: 9,
        alter: 0,
    };
    assert_eq!(highest.to_vexflow(), "b/9");

    // Middle C (standard reference)
    let middle_c = Pitch {
        step: NoteStep::C,
        octave: 4,
        alter: 0,
    };
    assert_eq!(middle_c.to_vexflow(), "c/4");
}

#[test]
fn test_enharmonic_pitches() {
    // Test that enharmonic equivalents produce different VexFlow strings
    // (VexFlow respects the spelling choice)

    let c_sharp = Pitch {
        step: NoteStep::C,
        octave: 4,
        alter: 1,
    };
    let d_flat = Pitch {
        step: NoteStep::D,
        octave: 4,
        alter: -1,
    };

    assert_ne!(
        c_sharp.to_vexflow(),
        d_flat.to_vexflow(),
        "Enharmonic pitches should have different VexFlow representations"
    );
    assert_eq!(c_sharp.to_vexflow(), "c#/4");
    assert_eq!(d_flat.to_vexflow(), "db/4");
}

#[test]
fn test_all_chromatic_pitches_in_octave() {
    // Test all 12 chromatic pitches in one octave using sharps
    let sharps = vec![
        (NoteStep::C, 0, "c/4"),
        (NoteStep::C, 1, "c#/4"),
        (NoteStep::D, 0, "d/4"),
        (NoteStep::D, 1, "d#/4"),
        (NoteStep::E, 0, "e/4"),
        (NoteStep::F, 0, "f/4"),
        (NoteStep::F, 1, "f#/4"),
        (NoteStep::G, 0, "g/4"),
        (NoteStep::G, 1, "g#/4"),
        (NoteStep::A, 0, "a/4"),
        (NoteStep::A, 1, "a#/4"),
        (NoteStep::B, 0, "b/4"),
    ];

    for (step, alter, expected) in sharps {
        let pitch = Pitch {
            step,
            octave: 4,
            alter,
        };
        assert_eq!(pitch.to_vexflow(), expected);
    }
}

#[test]
fn test_duration_no_dots() {
    // Verify zero dots produces no 'd' suffix
    for base in [
        DurationBase::Whole,
        DurationBase::Half,
        DurationBase::Quarter,
        DurationBase::Eighth,
        DurationBase::Sixteenth,
        DurationBase::ThirtySecond,
    ] {
        let duration = Duration::new(base);
        let vexflow = duration.to_vexflow();
        assert!(
            !vexflow.contains('d'),
            "Duration without dots should not contain 'd': {}",
            vexflow
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// MIDI CONVERSION TESTS (For Validation)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_pitch_to_midi_roundtrip_validation() {
    // Validate that pitches convert to reasonable MIDI numbers
    let middle_c = Pitch {
        step: NoteStep::C,
        octave: 4,
        alter: 0,
    };
    assert_eq!(middle_c.to_midi(), 60, "Middle C should be MIDI 60");

    let a440 = Pitch {
        step: NoteStep::A,
        octave: 4,
        alter: 0,
    };
    assert_eq!(a440.to_midi(), 69, "A440 should be MIDI 69");

    let c_sharp_4 = Pitch {
        step: NoteStep::C,
        octave: 4,
        alter: 1,
    };
    assert_eq!(c_sharp_4.to_midi(), 61, "C#4 should be MIDI 61");

    let b_flat_3 = Pitch {
        step: NoteStep::B,
        octave: 3,
        alter: -1,
    };
    assert_eq!(b_flat_3.to_midi(), 58, "Bb3 should be MIDI 58");
}

#[test]
fn test_midi_range_validation() {
    // Ensure MIDI values stay in valid range (0-127)
    for octave in 0..=9 {
        for step in [
            NoteStep::C,
            NoteStep::D,
            NoteStep::E,
            NoteStep::F,
            NoteStep::G,
            NoteStep::A,
            NoteStep::B,
        ] {
            for alter in [-2, -1, 0, 1, 2] {
                let pitch = Pitch { step, octave, alter };
                let midi = pitch.to_midi();
                assert!(
                    midi <= 127,
                    "MIDI value {} for pitch {:?} exceeds 127",
                    midi,
                    pitch
                );
                assert!(
                    midi >= 0,
                    "MIDI value {} for pitch {:?} is negative",
                    midi,
                    pitch
                );
            }
        }
    }
}
