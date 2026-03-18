//! Command parser for Harmonium CLI
//!
//! Parses user input into EngineCommand variants

use anyhow::{anyhow, Result};
use harmonium_core::{harmony::HarmonyMode, sequencer::RhythmMode, EngineCommand};

/// Parse a command line into an EngineCommand
pub fn parse_command(line: &str) -> Result<EngineCommand> {
    let line = line.trim();

    // Empty line
    if line.is_empty() {
        return Err(anyhow!("Empty command"));
    }

    // Split into tokens
    let tokens: Vec<&str> = line.split_whitespace().collect();

    match tokens[0] {
        // === GLOBAL ===
        "set" => parse_set_command(&tokens[1..]),

        // === CONTROL MODE ===
        "emotion" => parse_emotion_command(&tokens[1..]),
        "direct" => parse_direct_command(&tokens[1..]),

        // === MODULE TOGGLES ===
        "enable" => parse_enable_command(&tokens[1..]),
        "disable" => parse_disable_command(&tokens[1..]),

        // === RECORDING ===
        "record" => parse_record_command(&tokens[1..]),

        // === UTILITY ===
        "state" | "show" | "status" => Ok(EngineCommand::GetState),
        "reset" => Ok(EngineCommand::Reset),

        // === HELP ===
        "help" | "?" => Err(anyhow!("help")), // Special case handled by REPL
        "quit" | "exit" => Err(anyhow!("quit")), // Special case handled by REPL
        "stop" => Err(anyhow!("stop")),       // Special case handled by REPL

        _ => Err(anyhow!("Unknown command: {}. Type 'help' for available commands.", tokens[0])),
    }
}

/// Parse "set <param> <value>" commands
fn parse_set_command(tokens: &[&str]) -> Result<EngineCommand> {
    if tokens.is_empty() {
        return Err(anyhow!("Usage: set <param> <value>"));
    }

    let param = tokens[0];
    let value = tokens.get(1).ok_or_else(|| anyhow!("Missing value for '{}'", param))?;

    match param {
        // === GLOBAL ===
        "bpm" => {
            let bpm = value.parse::<f32>().map_err(|_| anyhow!("Invalid BPM value: {}", value))?;
            Ok(EngineCommand::SetBpm(bpm))
        }

        "volume" | "master_volume" => {
            let volume =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid volume value: {}", value))?;
            Ok(EngineCommand::SetMasterVolume(volume))
        }

        "time" | "time_signature" => {
            let parts: Vec<&str> = value.split('/').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid time signature format. Use: 4/4"));
            }
            let numerator = parts[0]
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid numerator: {}", parts[0]))?;
            let denominator = parts[1]
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid denominator: {}", parts[1]))?;
            Ok(EngineCommand::SetTimeSignature { numerator, denominator })
        }

        // === RHYTHM ===
        "rhythm_mode" | "rhythm-mode" => {
            let mode = match value.to_lowercase().as_str() {
                "euclidean" | "e" => RhythmMode::Euclidean,
                "perfect" | "perfectbalance" | "perfect_balance" | "pb" => {
                    RhythmMode::PerfectBalance
                }
                "classic" | "classicgroove" | "classic_groove" | "cg" => RhythmMode::ClassicGroove,
                _ => {
                    return Err(anyhow!(
                        "Unknown rhythm mode: {}. Use: euclidean, perfect, classic",
                        value
                    ))
                }
            };
            Ok(EngineCommand::SetRhythmMode(mode))
        }

        "rhythm_steps" | "rhythm-steps" | "steps" => {
            let steps =
                value.parse::<usize>().map_err(|_| anyhow!("Invalid steps value: {}", value))?;
            Ok(EngineCommand::SetRhythmSteps(steps))
        }

        "rhythm_pulses" | "rhythm-pulses" | "pulses" => {
            let pulses =
                value.parse::<usize>().map_err(|_| anyhow!("Invalid pulses value: {}", value))?;
            Ok(EngineCommand::SetRhythmPulses(pulses))
        }

        "rhythm_rotation" | "rhythm-rotation" | "rotation" => {
            let rotation =
                value.parse::<usize>().map_err(|_| anyhow!("Invalid rotation value: {}", value))?;
            Ok(EngineCommand::SetRhythmRotation(rotation))
        }

        "rhythm_density" | "rhythm-density" | "density" => {
            let density =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid density value: {}", value))?;
            Ok(EngineCommand::SetRhythmDensity(density))
        }

        "rhythm_tension" | "rhythm-tension" => {
            let tension =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid tension value: {}", value))?;
            Ok(EngineCommand::SetRhythmTension(tension))
        }

        // === HARMONY ===
        "harmony_mode" | "harmony-mode" => {
            let mode = match value.to_lowercase().as_str() {
                "basic" | "b" => HarmonyMode::Basic,
                "driver" | "d" => HarmonyMode::Driver,
                _ => return Err(anyhow!("Unknown harmony mode: {}. Use: basic, driver", value)),
            };
            Ok(EngineCommand::SetHarmonyMode(mode))
        }

        "harmony_tension" | "harmony-tension" => {
            let tension =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid tension value: {}", value))?;
            Ok(EngineCommand::SetHarmonyTension(tension))
        }

        "harmony_valence" | "harmony-valence" | "valence" => {
            let valence =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid valence value: {}", value))?;
            Ok(EngineCommand::SetHarmonyValence(valence))
        }

        // === MELODY ===
        "melody_smoothness" | "melody-smoothness" | "smoothness" => {
            let smoothness =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid smoothness value: {}", value))?;
            Ok(EngineCommand::SetMelodySmoothness(smoothness))
        }

        "melody_octave" | "melody-octave" | "octave" => {
            let octave =
                value.parse::<i32>().map_err(|_| anyhow!("Invalid octave value: {}", value))?;
            Ok(EngineCommand::SetMelodyOctave(octave))
        }

        // === VOICING ===
        "voicing_density" | "voicing-density" => {
            let density =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid voicing density: {}", value))?;
            Ok(EngineCommand::SetVoicingDensity(density))
        }

        "voicing_tension" | "voicing-tension" => {
            let tension =
                value.parse::<f32>().map_err(|_| anyhow!("Invalid voicing tension: {}", value))?;
            Ok(EngineCommand::SetVoicingTension(tension))
        }

        // === MIXER ===
        "gain" => {
            let channel = tokens.get(1).ok_or_else(|| anyhow!("Missing channel number"))?;
            let gain_value = tokens.get(2).ok_or_else(|| anyhow!("Missing gain value"))?;
            let ch = channel.parse::<u8>().map_err(|_| anyhow!("Invalid channel: {}", channel))?;
            let g =
                gain_value.parse::<f32>().map_err(|_| anyhow!("Invalid gain: {}", gain_value))?;
            Ok(EngineCommand::SetChannelGain { channel: ch, gain: g })
        }

        "mute" => {
            let channel = value.parse::<u8>().map_err(|_| anyhow!("Invalid channel: {}", value))?;
            Ok(EngineCommand::SetChannelMute { channel, muted: true })
        }

        "unmute" => {
            let channel = value.parse::<u8>().map_err(|_| anyhow!("Invalid channel: {}", value))?;
            Ok(EngineCommand::SetChannelMute { channel, muted: false })
        }

        _ => {
            Err(anyhow!("Unknown parameter: {}. Type 'help set' for available parameters.", param))
        }
    }
}

/// Parse "emotion <arousal> <valence> <density> <tension>" command
fn parse_emotion_command(tokens: &[&str]) -> Result<EngineCommand> {
    if tokens.is_empty() {
        // Switch to emotion mode
        return Ok(EngineCommand::UseEmotionMode);
    }

    // Set emotion parameters
    if tokens.len() < 4 {
        return Err(anyhow!("Usage: emotion <arousal> <valence> <density> <tension>"));
    }

    let arousal =
        tokens[0].parse::<f32>().map_err(|_| anyhow!("Invalid arousal: {}", tokens[0]))?;
    let valence =
        tokens[1].parse::<f32>().map_err(|_| anyhow!("Invalid valence: {}", tokens[1]))?;
    let density =
        tokens[2].parse::<f32>().map_err(|_| anyhow!("Invalid density: {}", tokens[2]))?;
    let tension =
        tokens[3].parse::<f32>().map_err(|_| anyhow!("Invalid tension: {}", tokens[3]))?;

    Ok(EngineCommand::SetEmotionParams { arousal, valence, density, tension })
}

/// Parse "direct" command (switch to direct mode)
fn parse_direct_command(_tokens: &[&str]) -> Result<EngineCommand> {
    Ok(EngineCommand::UseDirectMode)
}

/// Parse "enable <module>" command
fn parse_enable_command(tokens: &[&str]) -> Result<EngineCommand> {
    if tokens.is_empty() {
        return Err(anyhow!("Usage: enable <rhythm|harmony|melody|voicing>"));
    }

    match tokens[0].to_lowercase().as_str() {
        "rhythm" | "r" => Ok(EngineCommand::EnableRhythm(true)),
        "harmony" | "h" => Ok(EngineCommand::EnableHarmony(true)),
        "melody" | "m" => Ok(EngineCommand::EnableMelody(true)),
        "voicing" | "v" => Ok(EngineCommand::EnableVoicing(true)),
        _ => Err(anyhow!("Unknown module: {}. Use: rhythm, harmony, melody, voicing", tokens[0])),
    }
}

/// Parse "disable <module>" command
fn parse_disable_command(tokens: &[&str]) -> Result<EngineCommand> {
    if tokens.is_empty() {
        return Err(anyhow!("Usage: disable <rhythm|harmony|melody|voicing>"));
    }

    match tokens[0].to_lowercase().as_str() {
        "rhythm" | "r" => Ok(EngineCommand::EnableRhythm(false)),
        "harmony" | "h" => Ok(EngineCommand::EnableHarmony(false)),
        "melody" | "m" => Ok(EngineCommand::EnableMelody(false)),
        "voicing" | "v" => Ok(EngineCommand::EnableVoicing(false)),
        _ => Err(anyhow!("Unknown module: {}. Use: rhythm, harmony, melody, voicing", tokens[0])),
    }
}

/// Parse "record <wav|midi|musicxml>" command
/// Note: filename is not supported yet in the command interface
fn parse_record_command(tokens: &[&str]) -> Result<EngineCommand> {
    if tokens.is_empty() {
        return Err(anyhow!("Usage: record <wav|midi|musicxml>"));
    }

    let format = match tokens[0].to_lowercase().as_str() {
        "wav" => harmonium_core::events::RecordFormat::Wav,
        "midi" | "mid" => harmonium_core::events::RecordFormat::Midi,
        "musicxml" | "xml" => harmonium_core::events::RecordFormat::MusicXml,
        _ => return Err(anyhow!("Unknown format: {}. Use: wav, midi, musicxml", tokens[0])),
    };

    Ok(EngineCommand::StartRecording(format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set_bpm() {
        let cmd = parse_command("set bpm 140").unwrap();
        assert!(matches!(cmd, EngineCommand::SetBpm(140.0)));
    }

    #[test]
    fn test_parse_rhythm_mode() {
        let cmd = parse_command("set rhythm_mode euclidean").unwrap();
        assert!(matches!(cmd, EngineCommand::SetRhythmMode(RhythmMode::Euclidean)));
    }

    #[test]
    fn test_parse_emotion() {
        let cmd = parse_command("emotion 0.9 0.5 0.7 0.6").unwrap();
        assert!(matches!(cmd, EngineCommand::SetEmotionParams { .. }));
    }

    #[test]
    fn test_parse_enable() {
        let cmd = parse_command("enable rhythm").unwrap();
        assert!(matches!(cmd, EngineCommand::EnableRhythm(true)));
    }

    #[test]
    fn test_invalid_command() {
        let result = parse_command("invalid");
        assert!(result.is_err());
    }
}
