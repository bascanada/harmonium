//! Headless offline export mode for batch recording
//!
//! Drives the engine in a tight loop without any audio device, rendering
//! the requested duration as fast as the CPU allows.

use anyhow::Result;
use harmonium::audio;
use harmonium_core::{events::RecordFormat, EngineCommand};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const SAMPLE_RATE: f64 = 44100.0;
const CHANNELS: usize = 2;
const BUFFER_SIZE: usize = 1024;

/// Run the engine offline for the given duration, saving recordings and snapshots.
pub fn run(
    sf2_bytes: Option<&[u8]>,
    backend_type: audio::AudioBackendType,
    duration_secs: u64,
    record_wav: Option<String>,
    record_midi: Option<String>,
    record_musicxml: Option<String>,
    record_truth: Option<String>,
) -> Result<()> {
    let (mut engine, mut controller, finished_recordings) =
        audio::create_offline_engine(sf2_bytes, backend_type, SAMPLE_RATE)
            .map_err(|e| anyhow::anyhow!(e))?;

    let has_recordings = record_wav.is_some()
        || record_midi.is_some()
        || record_musicxml.is_some();

    // Start recordings via the command queue
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

    let total_samples = (SAMPLE_RATE * duration_secs as f64) as usize;
    let mut rendered_samples = 0usize;
    let mut buffer = vec![0.0f32; BUFFER_SIZE * CHANNELS];

    // Truth snapshot tracking
    let mut truth_snapshots: Vec<serde_json::Value> = Vec::new();
    let snapshot_interval_samples = (SAMPLE_RATE * 0.5) as usize; // every 500ms of audio
    let mut samples_since_snapshot = 0usize;

    let wall_start = Instant::now();
    let mut last_progress_sec = 0u64;

    println!("Offline rendering {}s of audio...", duration_secs);

    while rendered_samples < total_samples {
        let remaining = total_samples - rendered_samples;
        let chunk_samples = remaining.min(BUFFER_SIZE);
        let chunk_len = chunk_samples * CHANNELS;

        // Zero the buffer slice we'll use
        for s in &mut buffer[..chunk_len] {
            *s = 0.0;
        }

        // Drive the engine
        engine.process_buffer(&mut buffer[..chunk_len], CHANNELS);
        rendered_samples += chunk_samples;

        // Poll reports so controller state stays fresh
        let _ = controller.poll_reports();

        // Truth snapshots at regular audio-time intervals
        samples_since_snapshot += chunk_samples;
        if record_truth.is_some() && samples_since_snapshot >= snapshot_interval_samples {
            samples_since_snapshot = 0;
            if let Some(report) = controller.get_state() {
                let audio_time_ms = (rendered_samples as f64 / SAMPLE_RATE * 1000.0) as u64;
                let snapshot = serde_json::json!({
                    "timestamp_ms": audio_time_ms,
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
        }

        // Progress every 5 simulated seconds
        let audio_sec = (rendered_samples as f64 / SAMPLE_RATE) as u64;
        if audio_sec >= last_progress_sec + 5 {
            last_progress_sec = audio_sec;
            let wall_elapsed = wall_start.elapsed().as_secs_f64();
            let speedup = audio_sec as f64 / wall_elapsed;
            println!(
                "  [{:>3}s / {}s] {:.1}x realtime",
                audio_sec, duration_secs, speedup
            );
        }
    }

    let wall_elapsed = wall_start.elapsed();
    println!(
        "Rendering complete: {}s of audio in {:.2}s ({:.1}x realtime)",
        duration_secs,
        wall_elapsed.as_secs_f64(),
        duration_secs as f64 / wall_elapsed.as_secs_f64()
    );

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

    // The engine needs one more process_buffer call to handle the stop commands
    buffer.fill(0.0);
    engine.process_buffer(&mut buffer, CHANNELS);
    let _ = controller.poll_reports();

    // Collect finished recordings
    save_recordings(&finished_recordings, &record_wav, &record_midi, &record_musicxml);

    // Save truth snapshots
    if let Some(truth_path) = record_truth {
        let truth_json = serde_json::to_string_pretty(&truth_snapshots)?;
        std::fs::write(&truth_path, &truth_json)?;
        println!(
            "Saved ground truth to {} ({} snapshots, {} bytes)",
            truth_path,
            truth_snapshots.len(),
            truth_json.len()
        );
    }

    println!("Export complete.");
    Ok(())
}

fn save_recordings(
    finished_recordings: &Arc<Mutex<Vec<(RecordFormat, Vec<u8>)>>>,
    record_wav: &Option<String>,
    record_midi: &Option<String>,
    record_musicxml: &Option<String>,
) {
    if let Ok(mut queue) = finished_recordings.lock() {
        while let Some((fmt, data)) = queue.pop() {
            let filename = match fmt {
                RecordFormat::Wav => record_wav.as_deref().unwrap_or("output.wav"),
                RecordFormat::Midi => record_midi.as_deref().unwrap_or("output.mid"),
                RecordFormat::MusicXml => record_musicxml.as_deref().unwrap_or("output.musicxml"),
            };

            match std::fs::write(filename, &data) {
                Ok(()) => {
                    println!("Saved {} ({} bytes)", filename, data.len());
                }
                Err(e) => {
                    eprintln!("Failed to write {}: {}", filename, e);
                }
            }
        }
    }
}
