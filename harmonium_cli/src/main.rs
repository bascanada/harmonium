//! Harmonium CLI - Interactive command-line interface for the Harmonium music engine
//!
//! This CLI provides a REPL for testing and controlling the Harmonium engine,
//! validating the command/report queue architecture before the frontend rebuild.
//!
//! Use `--export` mode for headless batch export (no REPL, runs as fast as possible).

mod completer;
mod export;
mod help;
mod parser;
mod repl;

use anyhow::Result;
use clap::Parser;
use harmonium::audio;

#[derive(Parser)]
#[command(name = "harmonium-cli")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Audio backend to use (fundsp or odin2)
    #[arg(short, long, default_value = "fundsp")]
    backend: String,

    /// Engine type: timeline (seekable, default) or legacy (event-streaming)
    #[arg(short, long, default_value = "timeline")]
    engine: String,

    /// Path to SoundFont file (.sf2) for audio synthesis
    #[arg(short, long)]
    soundfont: Option<String>,

    /// Export mode: skip REPL, run engine for --duration seconds and save recordings
    #[arg(long)]
    export: bool,

    /// Duration in seconds for export mode (required with --export)
    #[arg(short, long)]
    duration: Option<u64>,

    /// Record WAV output to file
    #[arg(long, value_name = "PATH")]
    record_wav: Option<String>,

    /// Record MIDI output to file
    #[arg(long, value_name = "PATH")]
    record_midi: Option<String>,

    /// Record MusicXML output to file
    #[arg(long, value_name = "PATH")]
    record_musicxml: Option<String>,

    /// Record engine state snapshots to JSON file (ground truth for testing)
    #[arg(long, value_name = "PATH")]
    record_truth: Option<String>,
}

fn main() -> Result<()> {
    // Disable engine logs to avoid breaking REPL input
    std::env::set_var("HARMONIUM_CLI", "1");

    let args = Args::parse();

    // Validate export mode arguments
    if args.export && args.duration.is_none() {
        eprintln!("Error: --export requires --duration <seconds>");
        std::process::exit(1);
    }

    // Parse backend type
    let backend_type = match args.backend.as_str() {
        "fundsp" => audio::AudioBackendType::FundSP,
        #[cfg(feature = "odin2")]
        "odin2" => audio::AudioBackendType::Odin2,
        _ => {
            eprintln!("Unknown backend: {}. Using fundsp.", args.backend);
            audio::AudioBackendType::FundSP
        }
    };

    // Load SoundFont if provided
    let sf2_bytes = if let Some(sf2_path) = args.soundfont {
        println!("Loading SoundFont: {}", sf2_path);
        match std::fs::read(&sf2_path) {
            Ok(bytes) => {
                println!("✓ Loaded SoundFont ({} bytes)", bytes.len());
                Some(bytes)
            }
            Err(e) => {
                eprintln!("Failed to load SoundFont: {}", e);
                return Err(e.into());
            }
        }
    } else {
        None
    };

    // Create audio stream and controller
    let use_timeline = args.engine.to_lowercase() == "timeline";
    println!(
        "Initializing Harmonium engine ({})...",
        if use_timeline { "timeline" } else { "legacy" }
    );

    let (_stream, config, controller, _font_queue, finished_recordings) = if use_timeline {
        audio::create_timeline_stream(sf2_bytes.as_deref(), backend_type)
    } else {
        audio::create_stream(sf2_bytes.as_deref(), backend_type)
    }
    .map_err(|e| anyhow::anyhow!(e))?;

    println!("✓ Engine initialized");
    println!("  Sample rate: 44100 Hz");
    println!("  Key: {} {}", config.key, config.scale);
    println!("  BPM: {:.1}", config.bpm);
    println!();

    if args.export {
        // Export mode: run engine for duration, save recordings, exit
        export::run(
            controller,
            finished_recordings,
            args.duration.unwrap(),
            args.record_wav,
            args.record_midi,
            args.record_musicxml,
            args.record_truth,
        )?;
    } else {
        // Interactive REPL mode
        repl::run(controller, finished_recordings)?;
    }

    Ok(())
}
