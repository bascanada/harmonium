//! Score Integration Tests
//!
//! Tests for ScoreNoteEvent generation, ID synchronization with AudioEvent,
//! and HarmoniumScore format validation.
//!
//! Run with: `cargo test -p harmonium --test score_integration_tests`

use std::collections::HashSet;

use harmonium::score::ScoreBuffer;
use harmonium_core::{
    events::AudioEvent,
    notation::{DurationBase, HarmoniumScore, KeyMode, NoteStep},
};

// ═══════════════════════════════════════════════════════════════════
// ID SYNCHRONIZATION TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_audio_score_id_synchronization_single_note() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![AudioEvent::NoteOn {
        id: None,
        note: 60, // Middle C
        velocity: 100,
        channel: 1, // Lead
    }];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // Verify one score event created
    assert_eq!(score_events.len(), 1, "Should create one score event for one note");

    // Verify ID was assigned to AudioEvent
    if let AudioEvent::NoteOn { id, .. } = &events[0] {
        assert!(id.is_some(), "AudioEvent should have ID assigned");
        assert_eq!(
            *id,
            Some(score_events[0].id),
            "AudioEvent and ScoreNoteEvent should have same ID"
        );
    } else {
        panic!("Expected NoteOn event");
    }
}

#[test]
fn test_audio_score_id_synchronization_multiple_notes() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60, // C4
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 36, // C2 bass
            velocity: 90,
            channel: 0,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 38, // Snare
            velocity: 80,
            channel: 2,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 42, // Hi-hat
            velocity: 70,
            channel: 3,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // Verify all events generated
    assert_eq!(score_events.len(), 4, "Should create score event for each NoteOn");

    // Verify each AudioEvent.id matches corresponding ScoreNoteEvent.id
    for (i, event) in events.iter().enumerate() {
        if let AudioEvent::NoteOn { id, .. } = event {
            assert!(id.is_some(), "Event {} should have ID assigned", i);
            assert_eq!(*id, Some(score_events[i].id), "Event {} IDs should match", i);
        }
    }

    // Verify all IDs are unique
    let ids: HashSet<u64> = score_events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 4, "All IDs should be unique");
}

#[test]
fn test_audio_score_id_uniqueness_across_measures() {
    // Note: We don't reset the counter because tests run in parallel.
    // We just verify that all IDs returned in THIS test are unique.
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
    let mut all_ids = HashSet::new();
    let mut event_count = 0;

    // Generate events across multiple measures
    for _ in 0..4 {
        // 4 measures
        for _ in 0..16 {
            // 16 steps per measure
            let mut events =
                vec![AudioEvent::NoteOn { id: None, note: 60, velocity: 100, channel: 1 }];

            let score_events = buffer.process_audio_events(&mut events, 4);

            for event in &score_events {
                assert!(
                    all_ids.insert(event.id),
                    "ID {} should be unique across all measures (event #{})",
                    event.id,
                    event_count
                );
                event_count += 1;
            }

            buffer.advance_step();
        }
    }

    // Should have 64 unique IDs (4 measures * 16 steps)
    assert_eq!(all_ids.len(), 64, "Should have 64 unique IDs");
    assert_eq!(event_count, 64, "Should have processed 64 events");
}

#[test]
fn test_noteoff_events_ignored() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![
        AudioEvent::NoteOn { id: None, note: 60, velocity: 100, channel: 1 },
        AudioEvent::NoteOff { id: Some(1), note: 60, channel: 1 },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // Only NoteOn should create ScoreNoteEvent
    assert_eq!(score_events.len(), 1, "Only NoteOn should create score events");
}

// ═══════════════════════════════════════════════════════════════════
// FORMAT VALIDATION TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_score_format_basic_structure() {
    let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
    let json = buffer.to_json();

    // Parse JSON back to validate structure
    let score: HarmoniumScore =
        serde_json::from_str(&json).expect("Should be valid HarmoniumScore JSON");

    assert_eq!(score.version, "1.0", "Version should be 1.0");
    assert_eq!(score.tempo, 120.0, "Tempo should match");
    assert_eq!(score.time_signature, (4, 4), "Time signature should match");
    assert_eq!(score.parts.len(), 3, "Should have 3 parts");

    // Validate part structure
    let part_ids: Vec<&str> = score.parts.iter().map(|p| p.id.as_str()).collect();
    assert!(part_ids.contains(&"lead"), "Should have lead part");
    assert!(part_ids.contains(&"bass"), "Should have bass part");
    assert!(part_ids.contains(&"drums"), "Should have drums part");
}

#[test]
fn test_score_key_signatures() {
    // Test C major (0 fifths)
    let buffer_c = ScoreBuffer::new(120.0, (4, 4), 0, false);
    let json_c = buffer_c.to_json();
    let score_c: HarmoniumScore = serde_json::from_str(&json_c).unwrap();
    assert_eq!(score_c.key_signature.root, "C");
    assert_eq!(score_c.key_signature.mode, KeyMode::Major);
    assert_eq!(score_c.key_signature.fifths, 0);

    // Test G major (1 sharp)
    let buffer_g = ScoreBuffer::new(120.0, (4, 4), 7, false); // G = pitch class 7
    let json_g = buffer_g.to_json();
    let score_g: HarmoniumScore = serde_json::from_str(&json_g).unwrap();
    assert_eq!(score_g.key_signature.root, "G");
    assert_eq!(score_g.key_signature.fifths, 1);

    // Test F major (1 flat)
    let buffer_f = ScoreBuffer::new(120.0, (4, 4), 5, false); // F = pitch class 5
    let json_f = buffer_f.to_json();
    let score_f: HarmoniumScore = serde_json::from_str(&json_f).unwrap();
    assert_eq!(score_f.key_signature.root, "F");
    assert_eq!(score_f.key_signature.fifths, -1);

    // Test A minor (relative to C, same fifths)
    let buffer_am = ScoreBuffer::new(120.0, (4, 4), 9, true); // A = pitch class 9
    let json_am = buffer_am.to_json();
    let score_am: HarmoniumScore = serde_json::from_str(&json_am).unwrap();
    assert_eq!(score_am.key_signature.root, "A");
    assert_eq!(score_am.key_signature.mode, KeyMode::Minor);
    assert_eq!(score_am.key_signature.fifths, 0);
}

#[test]
fn test_score_time_signatures() {
    // Test 4/4
    let buffer_44 = ScoreBuffer::new(120.0, (4, 4), 0, false);
    let json_44 = buffer_44.to_json();
    let score_44: HarmoniumScore = serde_json::from_str(&json_44).unwrap();
    assert_eq!(score_44.time_signature, (4, 4));

    // Test 3/4
    let buffer_34 = ScoreBuffer::new(120.0, (3, 4), 0, false);
    let json_34 = buffer_34.to_json();
    let score_34: HarmoniumScore = serde_json::from_str(&json_34).unwrap();
    assert_eq!(score_34.time_signature, (3, 4));

    // Test 6/8
    let buffer_68 = ScoreBuffer::new(120.0, (6, 8), 0, false);
    let json_68 = buffer_68.to_json();
    let score_68: HarmoniumScore = serde_json::from_str(&json_68).unwrap();
    assert_eq!(score_68.time_signature, (6, 8));
}

// ═══════════════════════════════════════════════════════════════════
// JSON ROUNDTRIP TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_score_json_roundtrip() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    // Add some events
    for step in 0..8 {
        let mut events = vec![AudioEvent::NoteOn {
            id: None,
            note: 60 + (step as u8 % 7), // C to B
            velocity: 100,
            channel: 1,
        }];
        let _ = buffer.process_audio_events(&mut events, 4);
        buffer.advance_step();
    }

    // Serialize to JSON
    let json = buffer.to_json();

    // Deserialize
    let score: HarmoniumScore =
        serde_json::from_str(&json).expect("Should deserialize successfully");

    // Re-serialize
    let json2 = serde_json::to_string(&score).expect("Should serialize again");

    // Deserialize again
    let score2: HarmoniumScore = serde_json::from_str(&json2).expect("Should deserialize again");

    // Compare
    assert_eq!(score.version, score2.version);
    assert_eq!(score.tempo, score2.tempo);
    assert_eq!(score.time_signature, score2.time_signature);
    assert_eq!(score.parts.len(), score2.parts.len());
}

#[test]
fn test_score_json_pretty_format() {
    let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);
    let json = buffer.to_json_pretty();

    // Pretty format should have newlines and indentation
    assert!(json.contains('\n'), "Pretty JSON should have newlines");
    assert!(json.contains("  "), "Pretty JSON should have indentation");

    // Should still be valid JSON
    let _score: HarmoniumScore = serde_json::from_str(&json).expect("Pretty JSON should be valid");
}

// ═══════════════════════════════════════════════════════════════════
// PITCH AND DURATION VALIDATION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_pitch_conversion_in_score() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false); // C major

    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60, // C4
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 61, // C#4 in sharp key
            velocity: 100,
            channel: 1,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // C4 should be C4
    assert_eq!(score_events[0].pitches.len(), 1);
    let c4 = &score_events[0].pitches[0];
    assert_eq!(c4.step, NoteStep::C);
    assert_eq!(c4.octave, 4);
    assert_eq!(c4.alter, 0);

    // C#4 in C major (0 fifths, use sharps)
    assert_eq!(score_events[1].pitches.len(), 1);
    let cs4 = &score_events[1].pitches[0];
    assert_eq!(cs4.step, NoteStep::C);
    assert_eq!(cs4.octave, 4);
    assert_eq!(cs4.alter, 1); // Sharp
}

#[test]
fn test_duration_assignment_in_score() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![AudioEvent::NoteOn { id: None, note: 60, velocity: 100, channel: 1 }];

    // Default duration of 2 steps = eighth note (at 4 steps per quarter)
    let score_events = buffer.process_audio_events(&mut events, 2);

    assert_eq!(score_events.len(), 1);
    let duration = &score_events[0].duration;
    assert_eq!(duration.base, DurationBase::Eighth);
    assert_eq!(duration.dots, 0);
}

// ═══════════════════════════════════════════════════════════════════
// MEASURE ADVANCEMENT TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_measure_counting() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    assert_eq!(buffer.current_measure(), 1, "Should start at measure 1");
    assert_eq!(buffer.completed_measures(), 0, "No completed measures initially");

    // Advance through first measure (16 steps for 4/4 at 4 steps/beat)
    for _ in 0..16 {
        buffer.advance_step();
    }

    assert_eq!(buffer.current_measure(), 2, "Should be at measure 2");
    assert_eq!(buffer.completed_measures(), 1, "One measure completed");

    // Advance through second measure
    for _ in 0..16 {
        buffer.advance_step();
    }

    assert_eq!(buffer.current_measure(), 3, "Should be at measure 3");
    assert_eq!(buffer.completed_measures(), 2, "Two measures completed");
}

// ═══════════════════════════════════════════════════════════════════
// CHANNEL MAPPING TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_channel_to_part_mapping() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 36, // Low note
            velocity: 100,
            channel: 0, // Bass
        },
        AudioEvent::NoteOn {
            id: None,
            note: 72, // High note
            velocity: 100,
            channel: 1, // Lead
        },
        AudioEvent::NoteOn {
            id: None,
            note: 38, // Snare
            velocity: 100,
            channel: 2, // Drums
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // All should create events - channel mapping is internal
    assert_eq!(score_events.len(), 3);

    // Verify the score has events in different parts after advancing
    buffer.advance_step();

    // Advance to finalize measure
    for _ in 0..15 {
        buffer.advance_step();
    }

    // Check that events were distributed to parts
    // (The full verification would require inspecting the score structure)
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_empty_events_handled() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events: Vec<AudioEvent> = vec![];
    let score_events = buffer.process_audio_events(&mut events, 4);

    assert!(score_events.is_empty(), "Empty input should produce empty output");
}

#[test]
fn test_control_change_events_ignored() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let mut events = vec![AudioEvent::ControlChange { ctrl: 1, value: 64, channel: 1 }];

    let score_events = buffer.process_audio_events(&mut events, 4);

    assert!(score_events.is_empty(), "ControlChange should not create score events");
}

#[test]
fn test_various_velocities_to_dynamics() {
    // Note: Don't reset the global counter in parallel tests
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false);

    let velocities = [20, 40, 64, 80, 100, 127];
    let mut events: Vec<AudioEvent> = velocities
        .iter()
        .map(|&v| AudioEvent::NoteOn { id: None, note: 60, velocity: v, channel: 1 })
        .collect();

    let score_events = buffer.process_audio_events(&mut events, 4);

    assert_eq!(score_events.len(), 6);

    // All should have dynamics assigned
    for event in &score_events {
        assert!(event.dynamic.is_some(), "Each event should have a dynamic");
    }
}
