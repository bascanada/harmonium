//! Headless export mode for batch recording
//!
//! Runs the engine for a fixed duration without the REPL, capturing recordings
//! and engine state snapshots as fast as possible.

use anyhow::Result;
use harmonium_core::{events::RecordFormat, EngineCommand, HarmoniumController};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Run the engine in export mode for the given duration.
pub fn run(
    mut controller: HarmoniumController,
    finished_recordings: Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>,
    duration_secs: u64,
    record_wav: Option<String>,
    record_midi: Option<String>,
    record_musicxml: Option<String>,
    record_truth: Option<String>,
) -> Result<()> {
    let has_recordings = record_wav.is_some()
        || record_midi.is_some()
        || record_musicxml.is_some();

    // Start recordings
    if record_wav.is_some() {
        let _ = controller.send(EngineCommand::StartRecording(RecordFormat::Wav));
    }
    if record_midi.is_some() {
        let _ = controller.send(EngineCommand::StartRecording(RecordFormat::Midi));
    }
    if record_musicxml.is_some() {
        let _ = controller.send(EngineCommand::StartRecording(RecordFormat::MusicXml));
    }

    if has_recordings {
        println!("Recording started...");
    }

    // Collect engine state snapshots for ground truth
    let mut truth_snapshots: Vec<serde_json::Value> = Vec::new();
    let snapshot_interval = Duration::from_millis(500);
    let mut last_snapshot = Instant::now();

    let start = Instant::now();
    let duration = Duration::from_secs(duration_secs);

    println!("Export running for {}s...", duration_secs);

    // Main loop: poll reports and collect truth snapshots
    while start.elapsed() < duration {
        let _ = controller.poll_reports();

        // Capture truth snapshots at regular intervals
        if record_truth.is_some() && last_snapshot.elapsed() >= snapshot_interval {
            if let Some(report) = controller.get_state() {
                let snapshot = serde_json::json!({
                    "timestamp_ms": start.elapsed().as_millis(),
                    "bar": report.current_bar,
                    "beat": report.current_beat,
                    "step": report.current_step,
                    "chord": report.current_chord.as_str(),
                    "chord_root_offset": report.chord_root_offset,
                    "chord_is_minor": report.chord_is_minor,
                    "harmony_mode": format!("{:?}", report.harmony_mode),
                    "rhythm_mode": format!("{:?}", report.rhythm_mode),
                    "bpm": report.musical_params.bpm,
                    "time_signature": format!("{}/{}", report.time_signature.numerator, report.time_signature.denominator),
                    "primary_steps": report.primary_steps,
                    "primary_pulses": report.primary_pulses,
                    "session_key": report.session_key.as_str(),
                    "session_scale": report.session_scale.as_str(),
                });
                truth_snapshots.push(snapshot);
            }
            last_snapshot = Instant::now();
        }

        // Print progress every 5 seconds
        let elapsed = start.elapsed().as_secs();
        if elapsed > 0 && elapsed % 5 == 0 {
            let remaining = duration_secs.saturating_sub(elapsed);
            if remaining > 0 {
                // Only print once per 5s boundary
                let ms = start.elapsed().as_millis() % 5000;
                if ms < 50 {
                    if let Some(report) = controller.get_state() {
                        println!(
                            "  [{:>3}s / {}s] bar {} | {} | {:.0} BPM",
                            elapsed,
                            duration_secs,
                            report.current_bar,
                            report.current_chord,
                            report.musical_params.bpm,
                        );
                    }
                }
            }
        }

        // Sleep briefly to not busy-loop but still poll frequently
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("Export duration reached. Stopping recordings...");

    // Stop recordings
    if record_wav.is_some() {
        let _ = controller.send(EngineCommand::StopRecording(RecordFormat::Wav));
    }
    if record_midi.is_some() {
        let _ = controller.send(EngineCommand::StopRecording(RecordFormat::Midi));
    }
    if record_musicxml.is_some() {
        let _ = controller.send(EngineCommand::StopRecording(RecordFormat::MusicXml));
    }

    // Wait for recordings to finalize
    println!("Waiting for recordings to finalize...");
    std::thread::sleep(Duration::from_millis(2000));

    // Collect finished recordings with timeout
    let mut saved = 0;
    let expected = i32::from(record_wav.is_some())
        + i32::from(record_midi.is_some())
        + i32::from(record_musicxml.is_some());

    let collect_start = Instant::now();
    let collect_timeout = Duration::from_secs(10);

    while saved < expected && collect_start.elapsed() < collect_timeout {
        if let Ok(mut queue) = finished_recordings.lock() {
            while let Some((fmt, data)) = queue.pop() {
                let filename = match fmt {
                    RecordFormat::Wav => record_wav.as_deref().unwrap_or("output.wav"),
                    RecordFormat::Midi => record_midi.as_deref().unwrap_or("output.mid"),
                    RecordFormat::MusicXml => record_musicxml.as_deref().unwrap_or("output.musicxml"),
                };

                match std::fs::write(filename, &data) {
                    Ok(()) => {
                        println!("✓ Saved {} ({} bytes)", filename, data.len());
                        saved += 1;
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to write {}: {}", filename, e);
                    }
                }
            }
        }

        if saved < expected {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    if saved < expected {
        eprintln!(
            "Warning: Only saved {}/{} recordings (timeout after {}s)",
            saved,
            expected,
            collect_timeout.as_secs()
        );
    }

    // Save truth snapshots
    if let Some(truth_path) = record_truth {
        let truth_json = serde_json::to_string_pretty(&truth_snapshots)?;
        std::fs::write(&truth_path, &truth_json)?;
        println!(
            "✓ Saved ground truth to {} ({} snapshots, {} bytes)",
            truth_path,
            truth_snapshots.len(),
            truth_json.len()
        );
    }

    println!("Export complete.");
    Ok(())
}
