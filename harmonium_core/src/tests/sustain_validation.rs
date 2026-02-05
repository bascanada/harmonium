//! Integration tests for True Sustain validation using MusicXML export
//!
//! These tests generate musical sequences and verify that:
//! 1. Notes have variable durations (not all 16th notes)
//! 2. The engine properly sustains notes without re-triggering
//! 3. MusicXML export correctly captures sustained notes

use crate::events::AudioEvent;
use crate::export::to_musicxml;
use crate::harmony::{HarmonyNavigator, MelodicEvent, ChordQuality};
use crate::params::MusicalParams;
use rust_music_theory::note::PitchSymbol;
use rust_music_theory::scale::ScaleType;

/// Helper: Convert MelodicEvent sequence to AudioEvent timeline
/// Simulates how the engine processes melodic events
fn melodic_events_to_audio_events(
    events: Vec<MelodicEvent>,
    samples_per_step: usize,
) -> Vec<(f64, AudioEvent)> {
    let mut audio_events = Vec::new();
    let mut active_notes: Vec<u8> = Vec::new();

    for (step, event) in events.iter().enumerate() {
        let timestamp = (step * samples_per_step) as f64;

        match event {
            MelodicEvent::NoteOn { frequency } => {
                // Stop previous notes
                for note in active_notes.drain(..) {
                    audio_events.push((timestamp, AudioEvent::NoteOff { note, channel: 1 }));
                }

                // Start new note
                let midi = (69.0 + 12.0 * (frequency / 440.0).log2()).round() as u8;
                audio_events.push((timestamp, AudioEvent::NoteOn {
                    note: midi,
                    velocity: 100,
                    channel: 1,
                }));
                active_notes.push(midi);
            }
            MelodicEvent::Legato { frequency } => {
                let midi = (69.0 + 12.0 * (frequency / 440.0).log2()).round() as u8;

                // TRUE SUSTAIN CHECK: If already playing, do nothing
                if active_notes.contains(&midi) {
                    // Let note ring - no events generated
                    continue;
                }

                // Different note: perform legato transition
                for note in active_notes.drain(..) {
                    audio_events.push((timestamp, AudioEvent::NoteOff { note, channel: 1 }));
                }
                audio_events.push((timestamp, AudioEvent::NoteOn {
                    note: midi,
                    velocity: 85, // Softer for legato
                    channel: 1,
                }));
                active_notes.push(midi);
            }
            MelodicEvent::Rest => {
                // Stop all notes
                for note in active_notes.drain(..) {
                    audio_events.push((timestamp, AudioEvent::NoteOff { note, channel: 1 }));
                }
            }
        }
    }

    // Stop any remaining notes at end
    let final_timestamp = (events.len() * samples_per_step) as f64;
    for note in active_notes.drain(..) {
        audio_events.push((final_timestamp, AudioEvent::NoteOff { note, channel: 1 }));
    }

    audio_events
}

/// Helper: Extract note durations from MusicXML export
/// Parses the generated XML to verify note durations match expectations
fn extract_note_durations_from_xml(xml: &str) -> Vec<usize> {
    // Simple XML parsing for <duration> tags
    let mut durations = Vec::new();

    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<duration>") && trimmed.ends_with("</duration>") {
            let content = trimmed
                .strip_prefix("<duration>").unwrap()
                .strip_suffix("</duration>").unwrap();
            if let Ok(duration) = content.parse::<usize>() {
                durations.push(duration);
            }
        }
    }

    durations
}

#[test]
fn test_musicxml_export_shows_sustained_notes() {
    // Generate a melody sequence
    let mut nav = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
    nav.set_chord_context(0, ChordQuality::Major);

    let mut melodic_events = Vec::new();
    for i in 0..64 {
        melodic_events.push(nav.next_melodic_event(i % 4 == 0, i % 16 == 0));
    }

    // Convert to audio events (simulating engine behavior)
    let samples_per_step = 11025; // 120 BPM, 16th notes
    let audio_events = melodic_events_to_audio_events(melodic_events, samples_per_step);

    // Export to MusicXML
    let params = MusicalParams {
        rhythm_steps: 16,
        bpm: 120.0,
        ..Default::default()
    };
    let xml = to_musicxml(&audio_events, &params, samples_per_step);

    // Extract note durations from XML
    let durations = extract_note_durations_from_xml(&xml);

    // Verify we have notes
    assert!(!durations.is_empty(), "Should generate notes in MusicXML");

    // Count distribution
    let steps_per_quarter = 4; // 16 steps / 4 beats
    let single_step = durations.iter().filter(|&&d| d == steps_per_quarter).count();
    let multi_step = durations.iter().filter(|&&d| d > steps_per_quarter).count();
    let long_notes = durations.iter().filter(|&&d| d >= steps_per_quarter * 2).count();

    // Verify variable note lengths (not all 16th notes)
    let multi_step_ratio = multi_step as f32 / durations.len() as f32;
    assert!(multi_step_ratio >= 0.15,
            "At least 15% of notes should be longer than 16th notes in MusicXML export, got {:.1}% ({} / {})",
            multi_step_ratio * 100.0, multi_step, durations.len());

    // Verify some quarter notes or longer exist
    assert!(long_notes >= 2,
            "Should have at least 2 quarter notes or longer in MusicXML export, got {}",
            long_notes);

    // Debug output
    eprintln!("MusicXML Export Analysis:");
    eprintln!("  Total notes: {}", durations.len());
    eprintln!("  16th notes: {}", single_step);
    eprintln!("  Longer notes: {}", multi_step);
    eprintln!("  Quarter+ notes: {}", long_notes);
    eprintln!("  Multi-step ratio: {:.1}%", multi_step_ratio * 100.0);
}

#[test]
fn test_legato_engine_simulation_prevents_retrigger() {
    // Test that the engine simulation correctly implements true sustain
    // When Legato event has same frequency, it should NOT emit NoteOff+NoteOn

    let samples_per_step = 11025;

    // Create a sequence with repeated notes
    let events = vec![
        MelodicEvent::NoteOn { frequency: 261.63 },  // C4 - step 0
        MelodicEvent::Legato { frequency: 261.63 },  // Same C4 - step 1 (SHOULD NOT RETRIGGER)
        MelodicEvent::Legato { frequency: 261.63 },  // Same C4 - step 2 (SHOULD NOT RETRIGGER)
        MelodicEvent::Legato { frequency: 293.66 },  // D4 - step 3 (different note)
        MelodicEvent::Rest,                           // Rest - step 4
    ];

    let audio_events = melodic_events_to_audio_events(events, samples_per_step);

    // Count events
    let note_ons: Vec<_> = audio_events.iter()
        .filter(|(_, e)| matches!(e, AudioEvent::NoteOn { .. }))
        .collect();
    let note_offs: Vec<_> = audio_events.iter()
        .filter(|(_, e)| matches!(e, AudioEvent::NoteOff { .. }))
        .collect();

    eprintln!("Engine Simulation Results:");
    eprintln!("  NoteOn events: {}", note_ons.len());
    eprintln!("  NoteOff events: {}", note_offs.len());

    // Should have exactly 2 NoteOns: C4 (step 0), D4 (step 3)
    // The repeated C4 at steps 1-2 should NOT generate NoteOn events
    assert_eq!(note_ons.len(), 2,
               "Should have exactly 2 NoteOn events (C4 initial, D4 transition), got {}",
               note_ons.len());

    // First NoteOn should be C4 (MIDI 60)
    if let AudioEvent::NoteOn { note, .. } = note_ons[0].1 {
        assert_eq!(note, 60, "First note should be C4 (MIDI 60), got {}", note);
    }

    // Second NoteOn should be D4 (MIDI 62)
    if let AudioEvent::NoteOn { note, .. } = note_ons[1].1 {
        assert_eq!(note, 62, "Second note should be D4 (MIDI 62), got {}", note);
    }

    // Verify note durations in export
    let params = MusicalParams {
        rhythm_steps: 16,
        bpm: 120.0,
        ..Default::default()
    };
    let xml = to_musicxml(&audio_events, &params, samples_per_step);
    let durations = extract_note_durations_from_xml(&xml);

    // First note (C4) should last 3 steps (sustained through Legato events)
    // Second note (D4) should last 1 step (until Rest)
    assert!(!durations.is_empty(), "Should have notes in export");

    // Find the longest duration (should be the sustained C4)
    let max_duration = durations.iter().max().unwrap();
    assert!(*max_duration >= 12, // 3 steps * 4 divisions = 12
            "Sustained C4 should last at least 3 steps (12 divisions), got {}",
            max_duration);
}

#[test]
fn test_no_machine_gun_effect_in_export() {
    // Verify that the export doesn't show "machine gun" pattern (all 16th notes)
    // This would indicate the sustain fix is NOT working

    let mut nav = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
    nav.set_chord_context(0, ChordQuality::Major);

    // Generate longer sequence
    let mut melodic_events = Vec::new();
    for i in 0..128 {
        melodic_events.push(nav.next_melodic_event(i % 4 == 0, i % 16 == 0));
    }

    let samples_per_step = 11025;
    let audio_events = melodic_events_to_audio_events(melodic_events, samples_per_step);

    let params = MusicalParams {
        rhythm_steps: 16,
        bpm: 120.0,
        ..Default::default()
    };
    let xml = to_musicxml(&audio_events, &params, samples_per_step);
    let durations = extract_note_durations_from_xml(&xml);

    assert!(!durations.is_empty(), "Should generate notes");

    let steps_per_quarter = 4;
    let sixteenth_notes = durations.iter().filter(|&&d| d == steps_per_quarter).count();
    let total = durations.len();

    // "Machine gun" would be 100% 16th notes
    // After fix, should have significant variety
    let sixteenth_ratio = sixteenth_notes as f32 / total as f32;

    eprintln!("Machine Gun Detection:");
    eprintln!("  16th notes: {} / {} = {:.1}%", sixteenth_notes, total, sixteenth_ratio * 100.0);

    // CRITICAL: Should NOT be all 16th notes (machine gun effect)
    assert!(sixteenth_ratio < 0.85,
            "Too many 16th notes ({:.1}%) - indicates machine gun effect (sustain not working)",
            sixteenth_ratio * 100.0);

    // Should have good variety of durations
    let unique_durations: std::collections::HashSet<_> = durations.iter().collect();
    assert!(unique_durations.len() >= 3,
            "Should have at least 3 different note durations, got {}",
            unique_durations.len());
}
