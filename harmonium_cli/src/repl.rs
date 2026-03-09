//! REPL (Read-Eval-Print Loop) for Harmonium CLI

use anyhow::Result;
use colored::Colorize;
use harmonium_ai::mapper::EmotionMapper;
use harmonium_core::{params::EngineParams, HarmoniumController, EngineCommand};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::sync::{Arc, Mutex};

use crate::{completer::HarmoniumCompleter, help, parser};

/// Emotion state for relative adjustments
struct EmotionState {
    arousal: f32,
    valence: f32,
    density: f32,
    tension: f32,
}

impl Default for EmotionState {
    fn default() -> Self {
        Self {
            arousal: 0.5,
            valence: 0.0,
            density: 0.5,
            tension: 0.5,
        }
    }
}

/// Run the interactive REPL
pub fn run(
    mut controller: HarmoniumController,
    finished_recordings: Arc<Mutex<Vec<(harmonium_core::events::RecordFormat, Vec<u8>)>>>,
) -> Result<()> {
    // Track emotion state for relative adjustments
    let mut emotion_state = EmotionState::default();

    // Create EmotionMapper for translating emotions to musical params
    let emotion_mapper = EmotionMapper::new();

    // Create readline editor with autocomplete
    let helper = HarmoniumCompleter::new();
    let mut rl = Editor::new()?;
    rl.set_helper(Some(helper));

    // Load history if it exists
    let history_path = dirs::home_dir()
        .map(|mut p| {
            p.push(".harmonium_history");
            p
        });

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    // Print welcome message
    print_welcome();

    // Main REPL loop
    loop {
        // Poll for reports from engine
        let _ = controller.poll_reports();

        // Poll for finished recordings and save them
        if let Ok(mut queue) = finished_recordings.lock() {
            while let Some((format, data)) = queue.pop() {
                let filename = match format {
                    harmonium_core::events::RecordFormat::Wav => "output.wav",
                    harmonium_core::events::RecordFormat::Midi => "output.mid",
                    harmonium_core::events::RecordFormat::MusicXml => "output.musicxml",
                };

                match std::fs::write(filename, &data) {
                    Ok(_) => {
                        println!("\n{} Saved {} ({} bytes)",
                            "[RECORDING]".cyan().bold(),
                            filename.green(),
                            data.len());
                    }
                    Err(e) => {
                        println!("\n{} Failed to write {}: {}",
                            "[ERROR]".red().bold(),
                            filename,
                            e);
                    }
                }
            }
        }

        // Create prompt with current state
        let prompt = create_prompt(&controller);

        // Read line
        match rl.readline(&prompt) {
            Ok(line) => {
                // Add to history
                let _ = rl.add_history_entry(line.trim());

                // Handle command
                match handle_command(&line, &mut controller, &mut emotion_state, &emotion_mapper) {
                    Ok(should_continue) => {
                        if !should_continue {
                            break; // User requested quit
                        }
                    }
                    Err(e) => {
                        println!("{} {}", "[ERROR]".red().bold(), e);
                    }
                }
            }

            Err(ReadlineError::Interrupted) => {
                // Quit immediately on Ctrl+C
                break;
            }

            Err(ReadlineError::Eof) => {
                println!("{}", "EOF".dimmed());
                break;
            }

            Err(err) => {
                println!("{} {:?}", "[ERROR]".red().bold(), err);
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    println!("{}", "Goodbye!".cyan().bold());
    Ok(())
}

/// Handle a single command
fn handle_command(
    line: &str,
    controller: &mut HarmoniumController,
    emotion_state: &mut EmotionState,
    emotion_mapper: &EmotionMapper,
) -> Result<bool> {
    let line = line.trim();

    // Empty line
    if line.is_empty() {
        return Ok(true);
    }

    // Special cases (help and quit)
    let tokens: Vec<&str> = line.split_whitespace().collect();

    match tokens[0] {
        "help" | "?" => {
            if tokens.len() > 1 {
                help::print_command_help(tokens[1]);
            } else {
                help::print_help();
            }
            return Ok(true);
        }

        "quit" | "exit" => {
            return Ok(false); // Signal to quit
        }

        "state" | "show" | "status" => {
            print_state(controller);
            return Ok(true);
        }

        "stop" => {
            // Stop specific format or all recordings
            if tokens.len() > 1 {
                let format = match tokens[1].to_lowercase().as_str() {
                    "wav" => harmonium_core::events::RecordFormat::Wav,
                    "midi" | "mid" => harmonium_core::events::RecordFormat::Midi,
                    "musicxml" | "xml" => harmonium_core::events::RecordFormat::MusicXml,
                    _ => {
                        println!("{} Unknown format: {}. Use: wav, midi, musicxml", "[ERROR]".red().bold(), tokens[1]);
                        return Ok(true);
                    }
                };
                let _ = controller.send(EngineCommand::StopRecording(format));
                println!("{} Recording {:?} stopped", "[OK]".green().bold(), format);
            } else {
                // Stop all recording formats
                let _ = controller.send(EngineCommand::StopRecording(harmonium_core::events::RecordFormat::Wav));
                let _ = controller.send(EngineCommand::StopRecording(harmonium_core::events::RecordFormat::Midi));
                let _ = controller.send(EngineCommand::StopRecording(harmonium_core::events::RecordFormat::MusicXml));
                println!("{} All recordings stopped", "[OK]".green().bold());
            }
            return Ok(true);
        }

        // Handle relative emotion adjustments
        "emotion" if tokens.len() > 1 && has_relative_values(&tokens[1..]) => {
            return handle_relative_emotion(controller, emotion_state, emotion_mapper, &tokens[1..]);
        }

        _ => {}
    }

    // Parse command
    match parser::parse_command(line) {
        Ok(cmd) => {
            // Handle emotion commands specially - apply mapper
            if let EngineCommand::SetEmotionParams { arousal, valence, density, tension } = &cmd {
                return apply_emotions(controller, emotion_state, emotion_mapper, *arousal, *valence, *density, *tension);
            }

            // Send other commands directly to engine
            match controller.send(cmd.clone()) {
                Ok(_) => {
                    // Success feedback
                    print_success(&cmd);
                }
                Err(e) => {
                    println!("{} {:?}", "[ERROR]".red().bold(), e);
                }
            }
        }
        Err(e) => {
            // Check if it's a special error (help/quit/stop already handled)
            let msg = e.to_string();
            if msg != "help" && msg != "quit" && msg != "stop" {
                println!("{} {}", "[ERROR]".red().bold(), e);
            }
        }
    }

    Ok(true)
}

/// Check if tokens contain relative value syntax (+/-)
fn has_relative_values(tokens: &[&str]) -> bool {
    tokens.iter().any(|t| t.contains('+') || t.contains('-'))
}

/// Apply emotion parameters using the EmotionMapper
fn apply_emotions(
    controller: &mut HarmoniumController,
    emotion_state: &mut EmotionState,
    emotion_mapper: &EmotionMapper,
    arousal: f32,
    valence: f32,
    density: f32,
    tension: f32,
) -> Result<bool> {
    // Update emotion state
    emotion_state.arousal = arousal;
    emotion_state.valence = valence;
    emotion_state.density = density;
    emotion_state.tension = tension;

    // Create EngineParams from emotions
    let engine_params = EngineParams {
        arousal,
        valence,
        density,
        tension,
        ..EngineParams::default()
    };

    // Apply EmotionMapper to get MusicalParams
    let musical_params = emotion_mapper.map(&engine_params);

    // Send individual commands for each parameter
    let _ = controller.send(EngineCommand::SetBpm(musical_params.bpm));
    let _ = controller.send(EngineCommand::SetRhythmMode(musical_params.rhythm_mode));
    let _ = controller.send(EngineCommand::SetRhythmDensity(musical_params.rhythm_density));
    let _ = controller.send(EngineCommand::SetRhythmTension(musical_params.rhythm_tension));
    let _ = controller.send(EngineCommand::SetRhythmSteps(musical_params.rhythm_steps));
    let _ = controller.send(EngineCommand::SetRhythmPulses(musical_params.rhythm_pulses));
    let _ = controller.send(EngineCommand::SetRhythmRotation(musical_params.rhythm_rotation));

    let _ = controller.send(EngineCommand::SetHarmonyMode(musical_params.harmony_mode));
    let _ = controller.send(EngineCommand::SetHarmonyStrategy(musical_params.harmony_strategy));
    let _ = controller.send(EngineCommand::SetHarmonyTension(musical_params.harmony_tension));
    let _ = controller.send(EngineCommand::SetHarmonyValence(musical_params.harmony_valence));

    let _ = controller.send(EngineCommand::SetMelodySmoothness(musical_params.melody_smoothness));
    let _ = controller.send(EngineCommand::SetMelodyOctave(musical_params.melody_octave));
    let _ = controller.send(EngineCommand::SetVoicingDensity(musical_params.voicing_density));

    println!("{} A={:.2} V={:.2} D={:.2} T={:.2} → BPM={:.0} Density={:.2}",
        "[OK]".green().bold(),
        arousal, valence, density, tension,
        musical_params.bpm, musical_params.rhythm_density);

    Ok(true)
}

/// Handle relative emotion adjustments (e.g., "emotion a+0.1 v-0.2")
fn handle_relative_emotion(
    controller: &mut HarmoniumController,
    emotion_state: &mut EmotionState,
    emotion_mapper: &EmotionMapper,
    tokens: &[&str],
) -> Result<bool> {
    for token in tokens {
        // Parse format: a+0.1, v-0.2, d+5, t-10
        if token.len() < 2 {
            continue;
        }

        let param = token.chars().next().unwrap().to_lowercase().to_string();
        let value_part = &token[1..];

        // Parse the value (could be +0.1, -0.2, +5, -10)
        let delta = if value_part.starts_with('+') || value_part.starts_with('-') {
            value_part.parse::<f32>().unwrap_or(0.0)
        } else {
            // Absolute value
            continue;
        };

        // Apply delta to appropriate parameter
        match param.as_str() {
            "a" => emotion_state.arousal = (emotion_state.arousal + delta).clamp(0.0, 1.0),
            "v" => emotion_state.valence = (emotion_state.valence + delta).clamp(-1.0, 1.0),
            "d" => emotion_state.density = (emotion_state.density + delta).clamp(0.0, 1.0),
            "t" => emotion_state.tension = (emotion_state.tension + delta).clamp(0.0, 1.0),
            _ => {
                println!("{} Unknown emotion parameter: {}", "[WARN]".yellow().bold(), param);
            }
        }
    }

    // Apply the updated emotions using the mapper
    apply_emotions(
        controller,
        emotion_state,
        emotion_mapper,
        emotion_state.arousal,
        emotion_state.valence,
        emotion_state.density,
        emotion_state.tension,
    )
}

/// Print welcome message
fn print_welcome() {
    println!();
    println!("{}", "╔════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                                                        ║".cyan());
    println!("{}", "║     🎵  Harmonium CLI - Interactive Music Engine  🎵   ║".cyan().bold());
    println!("{}", "║                                                        ║".cyan());
    println!("{}", "╚════════════════════════════════════════════════════════╝".cyan());
    println!();
    println!("  Type {} for available commands", "help".green().bold());
    println!("  Type {} or {} to exit", "quit".green(), "exit".green());
    println!();
}

/// Create prompt string with current state
fn create_prompt(controller: &HarmoniumController) -> String {
    // Get current state
    let state = controller.get_state();

    if let Some(report) = state {
        // Show BPM and current chord
        let bpm = report.musical_params.bpm;
        let chord = &report.current_chord;
        let bar = report.current_bar + 1; // 1-indexed for display

        format!(
            "{} {} {} {} ",
            "harmonium".cyan().bold(),
            format!("{}bpm", bpm as u32).yellow(),
            chord.to_string().magenta(),
            format!("[bar:{}]", bar).dimmed(),
        )
    } else {
        // No state yet
        format!("{} ", "harmonium".cyan().bold())
    }
}

/// Print success message for a command
fn print_success(cmd: &EngineCommand) {
    let msg = match cmd {
        EngineCommand::SetBpm(bpm) => format!("BPM set to {}", bpm),
        EngineCommand::SetRhythmMode(mode) => format!("Rhythm mode set to {:?}", mode),
        EngineCommand::SetRhythmSteps(steps) => format!("Steps set to {}", steps),
        EngineCommand::SetRhythmPulses(pulses) => format!("Pulses set to {}", pulses),
        EngineCommand::SetRhythmRotation(rotation) => format!("Rotation set to {}", rotation),
        EngineCommand::SetRhythmDensity(density) => format!("Density set to {:.2}", density),
        EngineCommand::SetHarmonyMode(mode) => format!("Harmony mode set to {:?}", mode),
        EngineCommand::SetHarmonyValence(valence) => format!("Valence set to {:.2}", valence),
        EngineCommand::SetMelodySmoothness(smoothness) => format!("Smoothness set to {:.2}", smoothness),
        EngineCommand::EnableRhythm(enabled) => format!("Rhythm {}", if *enabled { "enabled" } else { "disabled" }),
        EngineCommand::EnableHarmony(enabled) => format!("Harmony {}", if *enabled { "enabled" } else { "disabled" }),
        EngineCommand::EnableMelody(enabled) => format!("Melody {}", if *enabled { "enabled" } else { "disabled" }),
        EngineCommand::EnableVoicing(enabled) => format!("Voicing {}", if *enabled { "enabled" } else { "disabled" }),
        EngineCommand::SetEmotionParams { arousal, valence, density, tension } => {
            format!("Emotions set: A={:.2} V={:.2} D={:.2} T={:.2}", arousal, valence, density, tension)
        }
        EngineCommand::UseEmotionMode => "Switched to Emotion mode".to_string(),
        EngineCommand::UseDirectMode => "Switched to Direct mode".to_string(),
        EngineCommand::StartRecording(format) => {
            format!("Recording {:?} started", format)
        }
        EngineCommand::StopRecording(format) => {
            format!("Recording {:?} stopped", format)
        }
        _ => format!("{:?}", cmd),
    };

    println!("{} {}", "[OK]".green().bold(), msg);
}

/// Print current engine state
fn print_state(controller: &mut HarmoniumController) {
    // Poll for latest state
    let _ = controller.poll_reports();

    let state = controller.get_state();

    if let Some(report) = state {
        println!();
        println!("{}", "╔══════════════════════════════════════════════════╗".cyan());
        println!("{}", "║              ENGINE STATE                        ║".cyan().bold());
        println!("{}", "╚══════════════════════════════════════════════════╝".cyan());
        println!();

        // Timing
        println!("{}", "TIMING:".yellow().bold());
        println!("  BPM: {}", format!("{:.1}", report.musical_params.bpm).green());
        println!("  Time Signature: {}", format!("{}/{}", report.time_signature.numerator, report.time_signature.denominator).green());
        println!("  Bar: {}", format!("{}", report.current_bar + 1).green());
        println!("  Beat: {}", format!("{}", report.current_beat + 1).green());
        println!("  Step: {}", format!("{}", report.current_step + 1).green());
        println!();

        // Harmony
        println!("{}", "HARMONY:".yellow().bold());
        println!("  Mode: {}", format!("{:?}", report.harmony_mode).green());
        println!("  Current Chord: {}", format!("{}", report.current_chord).magenta().bold());
        println!("  Progression: {}", format!("{} ({} chords)", report.progression_name, report.progression_length).green());
        println!("  Key: {}", format!("{} {}", report.session_key, report.session_scale).green());
        println!();

        // Rhythm
        println!("{}", "RHYTHM:".yellow().bold());
        println!("  Mode: {}", format!("{:?}", report.rhythm_mode).green());
        println!("  Primary: {} steps, {} pulses, rotation {}",
            format!("{}", report.primary_steps).green(),
            format!("{}", report.primary_pulses).green(),
            format!("{}", report.primary_rotation).green()
        );
        println!("  Secondary: {} steps, {} pulses, rotation {}",
            format!("{}", report.secondary_steps).green(),
            format!("{}", report.secondary_pulses).green(),
            format!("{}", report.secondary_rotation).green()
        );

        // Print pattern (first 32 steps)
        print!("  Pattern: ");
        for (i, &trigger) in report.primary_pattern.iter().enumerate().take(32) {
            if i > 0 && i % 16 == 0 {
                print!("| ");
            }
            if trigger {
                print!("{}", "█".green());
            } else {
                print!("{}", "·".dimmed());
            }
        }
        println!();
        println!();

        // Modules
        println!("{}", "MODULES:".yellow().bold());
        println!("  Rhythm:  {}", if report.musical_params.enable_rhythm { "ON".green() } else { "OFF".red() });
        println!("  Harmony: {}", if report.musical_params.enable_harmony { "ON".green() } else { "OFF".red() });
        println!("  Melody:  {}", if report.musical_params.enable_melody { "ON".green() } else { "OFF".red() });
        println!("  Voicing: {}", if report.musical_params.enable_voicing { "ON".green() } else { "OFF".red() });
        println!();

    } else {
        println!("{}", "No state available yet. Engine may still be initializing.".yellow());
    }
}
