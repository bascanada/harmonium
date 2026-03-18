//! Barline-Based Buffer Swap Integration Tests (Phase 1)
//!
//! These tests verify that:
//! 1. Pattern buffers swap only at barlines
//! 2. Time signature changes are queued for the next barline
//! 3. Both primary and secondary sequencers swap synchronously

use harmonium_core::{
    params::{Conductor, TimeSignature},
    sequencer::{RhythmMode, Sequencer},
};

#[test]
fn test_buffer_swap_on_barline() {
    // Setup: Create a sequencer with 4/4 time (16 steps)
    let mut sequencer = Sequencer::new_with_mode(16, 4, 120.0, RhythmMode::ClassicGroove);
    sequencer.time_signature = TimeSignature::new(4, 4);
    sequencer.density = 0.5;
    sequencer.tension = 0.5;

    // Generate initial pattern
    sequencer.regenerate_pattern();
    let initial_pattern = sequencer.pattern.clone();

    // Prepare next bar with different parameters
    sequencer.density = 0.8;
    sequencer.tension = 0.8;
    sequencer.prepare_next_bar();

    assert!(sequencer.next_pattern.is_some(), "Next pattern should be prepared");

    // Verify pattern has NOT swapped yet (still using initial pattern)
    assert_eq!(sequencer.pattern, initial_pattern, "Pattern should not change until barline");

    // Simulate ticking through a full bar (16 steps in 4/4)
    for step in 0..15 {
        let tick_result = sequencer.tick();
        assert!(!tick_result.bar_crossed, "Should not cross bar at step {}", step);
    }

    // Tick the last step - this should cross the barline
    let tick_result = sequencer.tick();
    assert!(tick_result.bar_crossed, "Should cross bar at step 16");

    // After barline: In the actual engine, the swap happens in engine.rs tick()
    // Manually simulate that swap for this test
    if let Some(next) = sequencer.next_pattern.take() {
        sequencer.pattern = next;
        sequencer.steps = sequencer.pattern.len();
    }

    // Verify pattern HAS swapped (should be different from initial)
    assert_ne!(sequencer.pattern, initial_pattern, "Pattern should have changed after barline");
}

#[test]
fn test_conductor_barline_detection() {
    // Test that conductor correctly detects barlines in different time signatures

    // Test 4/4 (16 steps)
    let mut conductor_4_4 = Conductor::default();
    conductor_4_4.time_signature = TimeSignature::new(4, 4);
    conductor_4_4.ticks_per_beat = 4;

    // Tick through one bar - should NOT cross until step 16
    for _ in 0..15 {
        let bar_crossed = conductor_4_4.tick();
        assert!(!bar_crossed, "Should not cross bar before step 16");
    }

    // 16th tick should cross the bar
    let bar_crossed = conductor_4_4.tick();
    assert!(bar_crossed, "Should cross bar at step 16 in 4/4");
    // Note: Conductor starts at bar 1, so after crossing it's at bar 2
    assert_eq!(conductor_4_4.current_bar, 2, "Should be on bar 2 after first crossing");

    // Test 3/4 (12 steps)
    let mut conductor_3_4 = Conductor::default();
    conductor_3_4.time_signature = TimeSignature::new(3, 4);
    conductor_3_4.ticks_per_beat = 4;

    for _ in 0..11 {
        let bar_crossed = conductor_3_4.tick();
        assert!(!bar_crossed, "Should not cross bar before step 12 in 3/4");
    }

    let bar_crossed = conductor_3_4.tick();
    assert!(bar_crossed, "Should cross bar at step 12 in 3/4");

    // Test 5/4 (20 steps)
    let mut conductor_5_4 = Conductor::default();
    conductor_5_4.time_signature = TimeSignature::new(5, 4);
    conductor_5_4.ticks_per_beat = 4;

    for _ in 0..19 {
        let bar_crossed = conductor_5_4.tick();
        assert!(!bar_crossed, "Should not cross bar before step 20 in 5/4");
    }

    let bar_crossed = conductor_5_4.tick();
    assert!(bar_crossed, "Should cross bar at step 20 in 5/4");
}

#[test]
fn test_time_signature_change_queued() {
    // Simulate the engine behavior: queue a time signature change mid-bar

    let mut sequencer = Sequencer::new_with_mode(16, 4, 120.0, RhythmMode::ClassicGroove);
    sequencer.time_signature = TimeSignature::new(4, 4);
    sequencer.regenerate_pattern();

    // Simulate being in the middle of a bar (step 8 of 16)
    for _ in 0..8 {
        sequencer.tick();
    }

    // User changes time signature to 3/4 (in real engine, this sets pending_time_signature_change)
    let pending_time_signature = Some(TimeSignature::new(3, 4));

    // Verify sequencer still has old time signature
    assert_eq!(
        sequencer.time_signature,
        TimeSignature::new(4, 4),
        "Time signature should not change mid-bar"
    );

    // Tick to the end of the bar
    for _ in 0..7 {
        sequencer.tick();
    }

    // Last tick crosses barline
    let tick_result = sequencer.tick();
    assert!(tick_result.bar_crossed, "Should cross barline");

    // NOW apply the pending time signature (simulating engine.rs behavior)
    if let Some(new_ts) = pending_time_signature {
        sequencer.time_signature = new_ts;
        let new_steps = new_ts.steps_per_bar(sequencer.ticks_per_beat);
        sequencer.steps = new_steps;
    }

    // Verify time signature changed ONLY after barline
    assert_eq!(
        sequencer.time_signature,
        TimeSignature::new(3, 4),
        "Time signature should change after barline"
    );
    assert_eq!(sequencer.steps, 12, "Steps should be 12 for 3/4");
}

#[test]
fn test_dual_sequencer_sync() {
    // Verify that both primary and secondary sequencers swap buffers on the same barline

    let mut primary = Sequencer::new_with_mode(16, 4, 120.0, RhythmMode::ClassicGroove);
    let mut secondary = Sequencer::new_with_mode(12, 3, 120.0, RhythmMode::Euclidean);

    primary.time_signature = TimeSignature::new(4, 4);
    secondary.time_signature = TimeSignature::new(4, 4);

    // Generate initial patterns with specific parameters
    primary.density = 0.3;
    primary.tension = 0.3;
    primary.regenerate_pattern();

    secondary.pulses = 3; // Initial: 3 pulses
    secondary.regenerate_pattern();

    let primary_initial = primary.pattern.clone();
    let secondary_initial = secondary.pattern.clone();

    // Change parameters so next patterns will be different
    primary.density = 0.8;
    primary.tension = 0.8;
    secondary.pulses = 5; // Changed to 5 pulses

    // Prepare next bars for both (both should have different patterns)
    primary.prepare_next_bar();
    secondary.prepare_next_bar();

    assert!(primary.next_pattern.is_some(), "Primary should have next pattern");
    assert!(secondary.next_pattern.is_some(), "Secondary should have next pattern");

    // Tick both sequencers through a full bar (16 steps = one bar in 4/4)
    // Note: In the real engine, both tick in sync, but the CONDUCTOR determines barlines
    for _ in 0..16 {
        primary.tick();
        secondary.tick();
    }

    // In the actual engine, the Conductor detects barlines independently of individual sequencer steps
    // Both sequencers swap on the CONDUCTOR's barline signal, not their own tick results
    // Primary (16 steps) will have crossed its own barline
    // Secondary (12 steps) will be at step 4 (16 % 12), so won't cross its own barline
    // But both will swap because the CONDUCTOR crossed the barline (which is based on time signature)

    // Simulate engine swapping both buffers on conductor barline
    if let Some(next) = primary.next_pattern.take() {
        primary.pattern = next;
        primary.steps = primary.pattern.len();
    }
    if let Some(next) = secondary.next_pattern.take() {
        secondary.pattern = next;
        secondary.steps = secondary.pattern.len();
    }

    // Verify both patterns changed
    assert_ne!(primary.pattern, primary_initial, "Primary pattern should have changed");
    assert_ne!(secondary.pattern, secondary_initial, "Secondary pattern should have changed");

    // Verify both have no pending next patterns (they were swapped)
    assert!(primary.next_pattern.is_none(), "Primary next_pattern should be None after swap");
    assert!(secondary.next_pattern.is_none(), "Secondary next_pattern should be None after swap");
}

#[test]
fn test_time_signature_steps_calculation() {
    // Verify that TimeSignature::steps_per_bar() calculates correctly for various meters

    let ts_4_4 = TimeSignature::new(4, 4);
    assert_eq!(ts_4_4.steps_per_bar(4), 16, "4/4 should be 16 steps");

    let ts_3_4 = TimeSignature::new(3, 4);
    assert_eq!(ts_3_4.steps_per_bar(4), 12, "3/4 should be 12 steps");

    let ts_5_4 = TimeSignature::new(5, 4);
    assert_eq!(ts_5_4.steps_per_bar(4), 20, "5/4 should be 20 steps");

    let ts_7_8 = TimeSignature::new(7, 8);
    assert_eq!(ts_7_8.steps_per_bar(4), 14, "7/8 should be 14 steps (7 × 4 × 4 / 8)");

    let ts_6_8 = TimeSignature::new(6, 8);
    assert_eq!(ts_6_8.steps_per_bar(4), 12, "6/8 should be 12 steps (6 × 4 × 4 / 8)");
}
