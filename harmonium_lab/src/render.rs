//! Headless audio rendering for style profile validation.
//!
//! Renders N bars of music from a `TuningParams` configuration to WAV audio
//! without requiring an audio device. Uses the same offline engine as the CLI
//! export command.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use harmonium::audio::{self, AudioBackendType};
use harmonium::playback::PlaybackCommand;
use harmonium_core::events::RecordFormat;
use harmonium_core::tuning::TuningParams;

const SAMPLE_RATE: f64 = 44100.0;
const CHANNELS: usize = 2;
const BUFFER_SIZE: usize = 1024;

/// Render `bars` bars of music using the given `TuningParams` and return WAV bytes.
///
/// Uses a deterministic seed so the same tuning + seed always produces identical
/// audio for fair A/B comparison.
pub fn render_to_wav(
    tuning: &TuningParams,
    bars: usize,
    bpm: f32,
    seed: u64,
    sf2_bytes: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let (mut composer, mut playback, mut cmd_tx, mut report_rx, finished_recordings) =
        audio::create_offline_engine(sf2_bytes, AudioBackendType::FundSP, SAMPLE_RATE)
            .map_err(|e| anyhow::anyhow!("Failed to create offline engine: {}", e))?;

    // Apply tuning and BPM
    composer.set_tuning(tuning.clone());
    composer.set_bpm(bpm);
    composer.set_rng_seed(seed);

    // Pre-generate bars
    composer.set_writehead_lookahead(bars.max(4) + 4);
    composer.generate_bars(bars);

    // Start WAV recording
    let _ = cmd_tx.push(PlaybackCommand::StartRecording(RecordFormat::Wav));

    // Calculate total samples needed
    // At the given BPM with 4/4 time, each bar = 4 beats = 4 * (60/bpm) seconds
    let seconds_per_bar = 4.0 * 60.0 / f64::from(bpm);
    let total_samples = (SAMPLE_RATE * seconds_per_bar * bars as f64) as usize;
    let mut rendered_samples = 0usize;
    let mut buffer = vec![0.0f32; BUFFER_SIZE * CHANNELS];

    while rendered_samples < total_samples {
        composer.generate_ahead();

        let remaining = total_samples - rendered_samples;
        let chunk_samples = remaining.min(BUFFER_SIZE);
        let chunk_len = chunk_samples * CHANNELS;

        for s in &mut buffer[..chunk_len] {
            *s = 0.0;
        }

        playback.process_buffer(&mut buffer[..chunk_len], CHANNELS);
        rendered_samples += chunk_samples;

        // Drain reports to avoid ring buffer backup
        while report_rx.pop().is_ok() {}
    }

    // Stop recording
    let _ = cmd_tx.push(PlaybackCommand::StopRecording(RecordFormat::Wav));
    // Process stop command
    let mut stop_buf = vec![0.0f32; BUFFER_SIZE * CHANNELS];
    playback.process_buffer(&mut stop_buf, CHANNELS);

    // Collect WAV bytes
    let recordings = finished_recordings
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    for (format, data) in recordings.iter() {
        if *format == RecordFormat::Wav {
            return Ok(data.clone());
        }
    }

    Err(anyhow::anyhow!("No WAV data produced"))
}

/// Render to WAV and save to a file.
pub fn render_to_wav_file(
    tuning: &TuningParams,
    bars: usize,
    bpm: f32,
    seed: u64,
    path: &Path,
    sf2_bytes: Option<&[u8]>,
) -> Result<()> {
    let wav_bytes = render_to_wav(tuning, bars, bpm, seed, sf2_bytes)?;
    std::fs::write(path, &wav_bytes)
        .with_context(|| format!("Failed to write WAV to {}", path.display()))?;
    Ok(())
}

/// Play a WAV file using the system's default audio player (non-blocking).
pub fn play_wav(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .context("Failed to open WAV with system player")?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("Failed to open WAV with system player")?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
            .context("Failed to open WAV with system player")?;
    }
    Ok(())
}
