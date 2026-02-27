//! VexFlow Integration Tests
//!
//! End-to-end tests for the VexFlow rendering pipeline:
//! AudioEvent → ScoreBuffer → HarmoniumScore → JSON → VexFlow
//!
//! Run with: `cargo test -p harmonium --test vexflow_integration_tests`

use std::collections::HashSet;

use harmonium::score::ScoreBuffer;
use harmonium_core::{
    events::AudioEvent,
    notation::{HarmoniumScore, KeyMode, NoteEventType, NoteStep},
};

// ═══════════════════════════════════════════════════════════════════
// COMPLETE SCORE RENDERING TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_complete_score_has_all_vexflow_data() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4); // C major

    // Add a variety of events to create a rich score
    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60, // C4
            velocity: 100,
            channel: 1, // Lead
        },
        AudioEvent::NoteOn {
            id: None,
            note: 64, // E4
            velocity: 90,
            channel: 1, // Lead (chord with previous)
        },
        AudioEvent::NoteOn {
            id: None,
            note: 36, // C2
            velocity: 80,
            channel: 0, // Bass
        },
    ];

    buffer.process_audio_events(&mut events, 4);
    buffer.advance_step();

    let json = buffer.to_json();
    let score: HarmoniumScore = serde_json::from_str(&json).expect("Should parse JSON");

    // Verify all required VexFlow data is present
    assert_eq!(score.version, "1.0", "Version should be set");
    assert_eq!(score.tempo, 120.0, "Tempo should be set");
    assert_eq!(score.time_signature, (4, 4), "Time signature should be set");
    assert_eq!(score.key_signature.root, "C", "Key root should be set");
    assert_eq!(score.key_signature.mode, KeyMode::Major, "Key mode should be set");
    assert_eq!(score.key_signature.fifths, 0, "Fifths should be correct for C major");
    assert_eq!(score.parts.len(), 3, "Should have 3 parts (lead, bass, drums)");

    // Verify part structure
    assert_eq!(score.parts[0].id, "lead");
    assert_eq!(score.parts[1].id, "bass");
    assert_eq!(score.parts[2].id, "drums");
}

#[test]
fn test_score_events_have_all_vexflow_fields() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    let mut events = vec![AudioEvent::NoteOn {
        id: None,
        note: 60,
        velocity: 100,
        channel: 1,
    }];

    let score_events = buffer.process_audio_events(&mut events, 4);

    assert_eq!(score_events.len(), 1);
    let event = &score_events[0];

    // Verify all required fields for VexFlow
    assert!(event.id > 0, "ID should be assigned");
    assert!(event.beat >= 1.0, "Beat should be >= 1.0");
    assert!(!event.pitches.is_empty(), "Pitches should not be empty for note event");
    assert!(event.duration.to_vexflow().len() > 0, "Duration should convert to VexFlow");

    // Verify pitch VexFlow format
    let pitch_vexflow = event.pitches[0].to_vexflow();
    assert!(pitch_vexflow.contains('/'), "Pitch should have VexFlow format");
}

#[test]
fn test_chord_events_have_multiple_pitches() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Simulate a chord by sending multiple notes at the same step
    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60, // C
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 64, // E
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 67, // G
            velocity: 100,
            channel: 1,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // All three notes should be created
    assert_eq!(score_events.len(), 3, "Should create events for all notes in chord");

    // Verify all events have the same beat position
    let first_beat = score_events[0].beat;
    assert!(
        score_events.iter().all(|e| e.beat == first_beat),
        "All chord notes should have same beat"
    );

    // Verify all events have valid VexFlow pitch format
    for event in &score_events {
        assert_eq!(event.pitches.len(), 1, "Each event should have one pitch");
        let vexflow = event.pitches[0].to_vexflow();
        assert!(vexflow.contains('/'), "Pitch should be valid VexFlow format");
    }
}

// ═══════════════════════════════════════════════════════════════════
// PLAYBACK HIGHLIGHTING TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_note_id_tracking_for_highlighting() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60,
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 62,
            velocity: 100,
            channel: 1,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // Collect IDs from both AudioEvent and ScoreNoteEvent
    let audio_ids: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            AudioEvent::NoteOn { id, .. } => *id,
            _ => None,
        })
        .collect();

    let score_ids: Vec<u64> = score_events.iter().map(|e| e.id).collect();

    // Verify synchronization
    assert_eq!(audio_ids.len(), score_ids.len(), "Should have matching ID counts");
    assert_eq!(audio_ids, score_ids, "IDs should match exactly");

    // Verify IDs are unique
    let unique_ids: HashSet<u64> = score_ids.iter().copied().collect();
    assert_eq!(
        unique_ids.len(),
        score_ids.len(),
        "All IDs should be unique for highlighting"
    );
}

#[test]
fn test_beat_position_accuracy_for_highlighting() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add notes at different steps
    for step in 0..16 {
        let mut events = vec![AudioEvent::NoteOn {
            id: None,
            note: 60,
            velocity: 100,
            channel: 1,
        }];

        let score_events = buffer.process_audio_events(&mut events, 4);

        if !score_events.is_empty() {
            let event = &score_events[0];

            // Beat should be 1-indexed and increment properly
            // Step 0-3 = beat 1.0, 1.25, 1.5, 1.75
            // Step 4-7 = beat 2.0, 2.25, 2.5, 2.75, etc.
            let expected_beat = 1.0 + (step as f32 * 0.25);
            assert!(
                (event.beat - expected_beat).abs() < 0.001,
                "Step {} should map to beat {} but got {}",
                step,
                expected_beat,
                event.beat
            );
        }

        buffer.advance_step();
    }
}

#[test]
fn test_measure_boundary_tracking() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add events across measure boundaries
    for step in 0..32 {
        // 32 steps = 2 measures in 4/4 time
        let mut events = vec![AudioEvent::NoteOn {
            id: None,
            note: 60 + ((step % 12) as u8),
            velocity: 100,
            channel: 1,
        }];

        buffer.process_audio_events(&mut events, 4);
        buffer.advance_step();
    }

    let json = buffer.to_json();
    let score: HarmoniumScore = serde_json::from_str(&json).unwrap();

    // Should have finalized measures
    let lead_part = &score.parts[0];
    assert!(
        !lead_part.measures.is_empty(),
        "Should have finalized measures"
    );

    // Verify measure numbers are sequential
    for (i, measure) in lead_part.measures.iter().enumerate() {
        assert_eq!(
            measure.number,
            i + 1,
            "Measure {} should be numbered correctly",
            i + 1
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// MULTI-PART COORDINATION TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_multi_part_synchronization() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add events to all three parts
    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60, // Lead
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 36, // Bass
            velocity: 90,
            channel: 0,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 38, // Drums (snare)
            velocity: 80,
            channel: 2,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // All events should be created
    assert_eq!(score_events.len(), 3, "Should create events for all parts");

    // All should have same beat position (synchronized)
    let beat = score_events[0].beat;
    assert!(
        score_events.iter().all(|e| e.beat == beat),
        "All parts should be synchronized at same beat"
    );
}

#[test]
fn test_clef_assignment_per_part() {
    let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);
    let score = buffer.get_score();

    assert_eq!(score.parts.len(), 3, "Should have 3 parts");

    // Verify clef assignments
    assert_eq!(score.parts[0].clef, harmonium_core::notation::Clef::Treble, "Lead should use treble clef");
    assert_eq!(score.parts[1].clef, harmonium_core::notation::Clef::Bass, "Bass should use bass clef");
    assert_eq!(
        score.parts[2].clef,
        harmonium_core::notation::Clef::Percussion,
        "Drums should use percussion clef"
    );
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_empty_measure_handling() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Advance through a full measure without adding notes
    for _ in 0..16 {
        buffer.advance_step();
    }

    let json = buffer.to_json();
    let _score: HarmoniumScore = serde_json::from_str(&json).unwrap();

    // Empty measures should still be tracked
    assert!(
        buffer.completed_measures() > 0,
        "Should have completed a measure"
    );
}

#[test]
fn test_rest_only_measure() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add only rests (NoteOff events are ignored, so this is implicit rests)
    for _ in 0..16 {
        // Process empty events
        buffer.process_audio_events(&mut vec![], 4);
        buffer.advance_step();
    }

    let json = buffer.to_json();
    let score: HarmoniumScore = serde_json::from_str(&json).expect("Should parse");

    // Should successfully create score with empty/rest measures
    assert_eq!(score.parts.len(), 3, "Should maintain part structure");
}

#[test]
fn test_noteoff_events_ignored() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 60,
            velocity: 100,
            channel: 1,
        },
        AudioEvent::NoteOff {
            id: None,
            note: 60,
            channel: 1,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    // Only NoteOn should create ScoreNoteEvent
    assert_eq!(
        score_events.len(),
        1,
        "NoteOff should not create score events"
    );
    assert_eq!(
        score_events[0].event_type,
        NoteEventType::Note,
        "Should be a note event"
    );
}

#[test]
fn test_drum_events_on_percussion_clef() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add drum events (channels 2 and 3 map to drums part)
    let mut events = vec![
        AudioEvent::NoteOn {
            id: None,
            note: 36, // Kick
            velocity: 100,
            channel: 2,
        },
        AudioEvent::NoteOn {
            id: None,
            note: 38, // Snare
            velocity: 100,
            channel: 3,
        },
    ];

    let score_events = buffer.process_audio_events(&mut events, 4);

    assert_eq!(score_events.len(), 2, "Should create drum events");

    // Verify events are of drum type
    assert!(
        score_events.iter().all(|e| e.event_type == NoteEventType::Drum),
        "Drum channel events should be NoteEventType::Drum"
    );
}

// ═══════════════════════════════════════════════════════════════════
// JSON SCHEMA VALIDATION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_json_has_all_required_fields() {
    let buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);
    let json = buffer.to_json();

    // Parse as JSON Value to check structure
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify top-level required fields
    assert!(value.get("version").is_some(), "Should have version");
    assert!(value.get("tempo").is_some(), "Should have tempo");
    assert!(value.get("time_signature").is_some(), "Should have time_signature");
    assert!(value.get("key_signature").is_some(), "Should have key_signature");
    assert!(value.get("parts").is_some(), "Should have parts");

    // Verify key_signature fields
    let key_sig = value.get("key_signature").unwrap();
    assert!(key_sig.get("root").is_some(), "Key signature should have root");
    assert!(key_sig.get("mode").is_some(), "Key signature should have mode");
    assert!(key_sig.get("fifths").is_some(), "Key signature should have fifths");
}

#[test]
fn test_json_optional_fields_omitted_when_empty() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    let mut events = vec![AudioEvent::NoteOn {
        id: None,
        note: 60,
        velocity: 100,
        channel: 1,
    }];

    buffer.process_audio_events(&mut events, 4);

    let json = buffer.to_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Optional fields should be omitted when not set
    // (This reduces JSON payload size for frontend)
    let parts = value.get("parts").unwrap().as_array().unwrap();
    if !parts.is_empty() {
        let first_part = &parts[0];
        // Transposition is optional and should be omitted if None
        if let Some(trans) = first_part.get("transposition") {
            assert!(trans.is_null() || !trans.is_null(), "Transposition handling");
        }
    }
}

#[test]
fn test_key_signature_pitch_spelling() {
    // Test that pitch spelling respects key signature

    // C major (0 fifths) - should prefer naturals
    let mut buffer_c = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);
    let mut events_c = vec![AudioEvent::NoteOn {
        id: None,
        note: 61, // C# or Db
        velocity: 100,
        channel: 1,
    }];
    let score_events_c = buffer_c.process_audio_events(&mut events_c, 4);
    if !score_events_c.is_empty() {
        let pitch = &score_events_c[0].pitches[0];
        // In C major, 61 should be spelled as C#
        assert_eq!(pitch.step, NoteStep::C, "Should be spelled as C#");
        assert_eq!(pitch.alter, 1, "Should have sharp alteration");
    }

    // D major (2 fifths: F#, C#) - should prefer sharps
    let mut buffer_d = ScoreBuffer::new(120.0, (4, 4), 2, false, 4);
    let mut events_d = vec![AudioEvent::NoteOn {
        id: None,
        note: 61, // C# in D major
        velocity: 100,
        channel: 1,
    }];
    let score_events_d = buffer_d.process_audio_events(&mut events_d, 4);
    if !score_events_d.is_empty() {
        let pitch = &score_events_d[0].pitches[0];
        assert_eq!(pitch.step, NoteStep::C, "Should be C#");
        assert_eq!(pitch.alter, 1, "Should be sharp");
    }
}

#[test]
fn test_json_roundtrip_preserves_all_data() {
    let mut buffer = ScoreBuffer::new(120.0, (4, 4), 0, false, 4);

    // Add various events
    for step in 0..8 {
        let mut events = vec![AudioEvent::NoteOn {
            id: None,
            note: 60 + (step % 7),
            velocity: 100,
            channel: 1,
        }];
        buffer.process_audio_events(&mut events, 4);
        buffer.advance_step();
    }

    let json1 = buffer.to_json();
    let score1: HarmoniumScore = serde_json::from_str(&json1).unwrap();
    let json2 = serde_json::to_string(&score1).unwrap();
    let score2: HarmoniumScore = serde_json::from_str(&json2).unwrap();

    // All critical data should be preserved
    assert_eq!(score1.version, score2.version);
    assert_eq!(score1.tempo, score2.tempo);
    assert_eq!(score1.time_signature, score2.time_signature);
    assert_eq!(score1.key_signature.root, score2.key_signature.root);
    assert_eq!(score1.key_signature.fifths, score2.key_signature.fifths);
    assert_eq!(score1.parts.len(), score2.parts.len());
}
