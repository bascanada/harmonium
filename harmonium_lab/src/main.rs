//! Harmonium Lab CLI
//!
//! Command-line interface for Musical DNA extraction, benchmarking, and tuning.
//!
//! ## Commands
//!
//! - `ingest` - Ingest MusicXML files and extract DNA
//! - `profile` - Build style profiles from DNA collections
//! - `compare` - Compare generated DNA against reference profiles
//! - `tune` - Interactive LLM-assisted tuning session

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use harmonium_core::{exporters::dna::GlobalMetrics, tuning::TuningParams};
use harmonium_lab::{DNAComparator, MusicXMLIngester, StyleProfile, agent::ClaudeAgent};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "harmonium-lab")]
#[command(author, version, about = "Musical DNA extraction and LLM-assisted tuning")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest MusicXML files and extract Musical DNA
    Ingest {
        /// Source directory containing MusicXML files
        #[arg(short, long)]
        source: PathBuf,

        /// Output directory for DNA JSON files
        #[arg(short, long)]
        output: PathBuf,

        /// Recurse into subdirectories
        #[arg(short, long, default_value = "true")]
        recursive: bool,
    },

    /// Build a style profile from DNA files
    Profile {
        /// Name for the style profile
        #[arg(short, long)]
        name: String,

        /// Source directory containing DNA JSON files
        #[arg(short, long)]
        source: PathBuf,

        /// Output file for the profile
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Compare generated DNA against a reference profile
    Compare {
        /// Generated DNA JSON file
        #[arg(short, long)]
        generated: PathBuf,

        /// Reference profile JSON file
        #[arg(short, long)]
        reference: PathBuf,

        /// Output detailed comparison report
        #[arg(short, long)]
        verbose: bool,
    },

    /// Interactive LLM-assisted tuning session
    Tune {
        /// Target style profile
        #[arg(short, long)]
        target_style: PathBuf,

        /// Current tuning parameters file (TOML)
        #[arg(short = 'p', long)]
        tuning: PathBuf,

        /// Maximum iterations
        #[arg(short, long, default_value = "10")]
        iterations: usize,

        /// Anthropic API key (or set ANTHROPIC_API_KEY env var)
        #[arg(long, env = "ANTHROPIC_API_KEY")]
        api_key: Option<String>,
    },

    /// Extract DNA from a single MusicXML file (for testing)
    ExtractOne {
        /// Input MusicXML file
        #[arg(short, long)]
        input: PathBuf,

        /// Output DNA JSON file (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { source, output, recursive } => {
            cmd_ingest(&source, &output, recursive)?;
        }
        Commands::Profile { name, source, output } => {
            cmd_profile(&name, &source, &output)?;
        }
        Commands::Compare { generated, reference, verbose } => {
            cmd_compare(&generated, &reference, verbose)?;
        }
        Commands::Tune { target_style, tuning, iterations, api_key } => {
            cmd_tune(&target_style, &tuning, iterations, api_key)?;
        }
        Commands::ExtractOne { input, output } => {
            cmd_extract_one(&input, output.as_deref())?;
        }
    }

    Ok(())
}

fn cmd_ingest(source: &PathBuf, output: &PathBuf, recursive: bool) -> Result<()> {
    println!("Ingesting MusicXML files from: {}", source.display());

    let ingester = MusicXMLIngester::new();
    let files = ingester.find_musicxml_files(source, recursive)?;

    if files.is_empty() {
        println!("No MusicXML files found.");
        return Ok(());
    }

    println!("Found {} MusicXML files", files.len());

    // Create output directory
    std::fs::create_dir_all(output)?;

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
            )?
            .progress_chars("#>-"),
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for file in &files {
        pb.set_message(file.file_name().unwrap_or_default().to_string_lossy().to_string());

        match ingester.ingest_file(file) {
            Ok(dna) => {
                // Create output filename
                let stem = file.file_stem().unwrap_or_default();
                let out_file = output.join(format!("{}.dna.json", stem.to_string_lossy()));

                if let Ok(json) = dna.to_json() {
                    if std::fs::write(&out_file, json).is_ok() {
                        success_count += 1;
                    } else {
                        error_count += 1;
                    }
                } else {
                    error_count += 1;
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done");
    println!("Ingested: {} success, {} errors", success_count, error_count);

    Ok(())
}

fn cmd_profile(name: &str, source: &PathBuf, output: &PathBuf) -> Result<()> {
    println!("Building style profile '{}' from: {}", name, source.display());

    let profile = StyleProfile::from_directory(name, source)?;

    println!("Profile built from {} DNA files", profile.sample_count);
    println!("Average voice leading effort: {:.2}", profile.metrics.average_voice_leading_effort);
    println!("Tension variance: {:.4}", profile.metrics.tension_variance);
    println!("Harmonic rhythm: {:.2} chords/measure", profile.metrics.harmonic_rhythm);

    // Save profile
    let json = serde_json::to_string_pretty(&profile)?;
    std::fs::write(output, json)?;

    println!("Profile saved to: {}", output.display());

    Ok(())
}

fn cmd_compare(generated: &PathBuf, reference: &PathBuf, verbose: bool) -> Result<()> {
    println!("Comparing DNA files...");

    let gen_json = std::fs::read_to_string(generated)?;
    let ref_json = std::fs::read_to_string(reference)?;

    let gen_dna = harmonium_core::MusicalDNA::from_json(&gen_json)?;
    let ref_profile: StyleProfile = serde_json::from_str(&ref_json)?;

    let comparator = DNAComparator::new();
    let report = comparator.compare(&gen_dna, &ref_profile);

    println!("\n=== Comparison Report ===\n");
    println!("Overall similarity: {:.1}%", report.overall_similarity * 100.0);
    println!();

    if verbose {
        println!("Detailed Metrics:");
        println!("  Voice leading divergence: {:.2}", report.voice_leading_divergence);
        println!("  Tension divergence: {:.4}", report.tension_divergence);
        println!("  Harmonic rhythm divergence: {:.2}", report.harmonic_rhythm_divergence);
        println!();
        println!("Suggestions:");
        for suggestion in &report.suggestions {
            println!("  - {}", suggestion);
        }
    }

    Ok(())
}

fn cmd_tune(
    target_style: &PathBuf,
    tuning_path: &PathBuf,
    iterations: usize,
    api_key: Option<String>,
) -> Result<()> {
    println!("\n");
    println!("═══════════════════════════════════════════════════════════════════════");
    println!("                   HARMONIUM INTERACTIVE TUNING SESSION                 ");
    println!("═══════════════════════════════════════════════════════════════════════");
    println!();

    // Validate API key
    let api_key = api_key
        .context("Anthropic API key required. Set ANTHROPIC_API_KEY env var or use --api-key")?;

    // 1. Load target style profile
    println!("Loading target style profile: {}", target_style.display());
    let profile_json =
        std::fs::read_to_string(target_style).context("Failed to read target style profile")?;
    let target_profile: StyleProfile =
        serde_json::from_str(&profile_json).context("Failed to parse target style profile")?;

    println!("  Style: {}", target_profile.name);
    println!("  Based on {} samples", target_profile.sample_count);
    println!();

    // 2. Load or create tuning params
    let mut tuning = if tuning_path.exists() {
        println!("Loading tuning parameters: {}", tuning_path.display());
        TuningParams::from_toml_file(tuning_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tuning: {}", e))?
    } else {
        println!("Creating default tuning parameters: {}", tuning_path.display());
        let default_tuning = TuningParams::default();
        // Save initial tuning file
        default_tuning
            .to_toml_file(tuning_path)
            .map_err(|e| anyhow::anyhow!("Failed to save tuning: {}", e))?;
        default_tuning
    };

    // Validate tuning parameters
    tuning.validate().map_err(|e| anyhow::anyhow!("Invalid tuning parameters: {}", e))?;

    // Create Claude agent
    let agent = ClaudeAgent::new().with_api_key(&api_key);
    println!("Claude API configured (model: claude-sonnet-4-20250514)");
    println!();

    // Display target metrics
    print_metrics_header("TARGET STYLE", &target_profile.metrics);
    println!();

    // 3. Interactive tuning loop
    for iteration in 1..=iterations {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "                         ITERATION {}/{}                              ",
            iteration, iterations
        );
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        // Generate music with current tuning and extract DNA
        println!("[1/4] Generating music with current parameters...");
        let generated_metrics = simulate_music_generation(&tuning);
        print_metrics_comparison("GENERATED", &generated_metrics, &target_profile.metrics);
        println!();

        // Calculate divergence score
        let divergence = calculate_divergence(&generated_metrics, &target_profile.metrics);
        println!("Overall divergence: {:.2}% (lower is better)", divergence * 100.0);
        println!();

        // Check for convergence
        if divergence < 0.05 {
            println!("Convergence achieved! Divergence < 5%");
            break;
        }

        // Call Claude API for suggestions
        println!("[2/4] Consulting Claude for parameter suggestions...");
        let suggestion = match agent.suggest_tuning_blocking(
            &target_profile.metrics,
            &generated_metrics,
            &tuning,
        ) {
            Ok(s) => s,
            Err(e) => {
                println!("  Error calling Claude API: {}", e);
                println!("  Skipping this iteration...");
                continue;
            }
        };

        println!();
        println!("[3/4] Claude's Analysis:");
        println!("─────────────────────────────────────────────────────────────────────────");
        println!("{}", suggestion.reasoning);
        println!();
        println!("Confidence: {:.0}%", suggestion.confidence * 100.0);
        println!();

        if !suggestion.has_changes() {
            println!("No parameter changes suggested.");
            continue;
        }

        println!("Suggested Changes:");
        for change in &suggestion.parameter_changes {
            println!("  {} : {} → {}", change.name, change.current, change.suggested);
        }
        println!();

        // Present options to user
        println!("[4/4] What would you like to do?");
        println!();
        println!("  [A]pply  - Apply suggested changes");
        println!("  [S]kip   - Skip this iteration");
        println!("  [E]dit   - Manually edit a parameter");
        println!("  [V]iew   - View current tuning parameters");
        println!("  [Q]uit   - Save and exit");
        println!();

        loop {
            print!("Choice: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim().to_lowercase();

            match choice.as_str() {
                "a" | "apply" => {
                    println!("Applying changes...");
                    tuning = suggestion.apply_to(&tuning);
                    save_tuning(&tuning, tuning_path)?;
                    println!("Tuning saved to: {}", tuning_path.display());
                    break;
                }
                "s" | "skip" => {
                    println!("Skipping this iteration.");
                    break;
                }
                "e" | "edit" => {
                    tuning = manual_edit_parameter(&tuning)?;
                    save_tuning(&tuning, tuning_path)?;
                    println!("Tuning saved.");
                    break;
                }
                "v" | "view" => {
                    print_current_tuning(&tuning);
                }
                "q" | "quit" => {
                    println!("Saving and exiting...");
                    save_tuning(&tuning, tuning_path)?;
                    println!("Final tuning saved to: {}", tuning_path.display());
                    return Ok(());
                }
                _ => {
                    println!("Invalid choice. Please enter A, S, E, V, or Q.");
                }
            }
        }

        println!();
    }

    // Final summary
    println!();
    println!("═══════════════════════════════════════════════════════════════════════");
    println!("                          SESSION COMPLETE                              ");
    println!("═══════════════════════════════════════════════════════════════════════");
    println!();
    println!("Final tuning saved to: {}", tuning_path.display());
    print_current_tuning(&tuning);

    Ok(())
}

/// Simulate music generation based on tuning parameters.
/// This produces estimated GlobalMetrics based on the parameter values.
/// In a full implementation, this would run the actual harmonium engine.
fn simulate_music_generation(tuning: &TuningParams) -> GlobalMetrics {
    // Estimate voice leading effort from max_semitone_movement
    // Lower max movement = lower effort (smoother voice leading)
    let voice_leading_effort = f32::from(tuning.max_semitone_movement) * 0.8;

    // Estimate tension variance from hysteresis settings
    // Wider hysteresis = less variance (more stable)
    let hysteresis_range = tuning.neo_riemannian_upper_threshold - tuning.steedman_lower_threshold;
    let tension_variance = 0.1 / hysteresis_range.max(0.1);

    // Estimate tension/release balance from TRQ threshold
    // Higher threshold = more tension
    let tension_release_balance = tuning.trq_threshold;

    // Diatonic percentage depends on strategy thresholds
    // More Steedman (lower thresholds) = more diatonic
    let steedman_bias = 1.0 - tuning.steedman_upper_threshold;
    let diatonic_percentage = 60.0 + steedman_bias * 30.0;

    // Harmonic rhythm from polygon vertices (more vertices = faster changes)
    let avg_vertices =
        (tuning.kick_high_density_vertices + tuning.snare_high_density_vertices) as f32 / 2.0;
    let harmonic_rhythm = avg_vertices / 4.0;

    GlobalMetrics {
        average_voice_leading_effort: voice_leading_effort,
        tension_variance,
        tension_release_balance,
        diatonic_percentage,
        harmonic_rhythm,
        total_duration_beats: 64.0, // Simulate 16 measures at 4 beats each
        chord_change_count: 16,     // Simulate one chord change per measure
    }
}

/// Calculate overall divergence between generated and target metrics
fn calculate_divergence(generated: &GlobalMetrics, target: &GlobalMetrics) -> f32 {
    let vl_diff = (generated.average_voice_leading_effort - target.average_voice_leading_effort)
        .abs()
        / target.average_voice_leading_effort.max(0.1);
    let tv_diff = (generated.tension_variance - target.tension_variance).abs()
        / target.tension_variance.max(0.01);
    let trb_diff = (generated.tension_release_balance - target.tension_release_balance).abs();
    let dp_diff = (generated.diatonic_percentage - target.diatonic_percentage).abs() / 100.0;
    let hr_diff = (generated.harmonic_rhythm - target.harmonic_rhythm).abs()
        / target.harmonic_rhythm.max(0.1);

    // Weighted average
    (vl_diff * 0.25 + tv_diff * 0.2 + trb_diff * 0.2 + dp_diff * 0.15 + hr_diff * 0.2).min(1.0)
}

/// Print metrics header
fn print_metrics_header(label: &str, metrics: &GlobalMetrics) {
    println!("{} PROFILE:", label);
    println!("  Voice Leading Effort: {:.2}", metrics.average_voice_leading_effort);
    println!("  Tension Variance:     {:.4}", metrics.tension_variance);
    println!("  Tension/Release:      {:.2}", metrics.tension_release_balance);
    println!("  Diatonic %:           {:.1}%", metrics.diatonic_percentage);
    println!("  Harmonic Rhythm:      {:.2} chords/measure", metrics.harmonic_rhythm);
}

/// Print metrics comparison with direction indicators
fn print_metrics_comparison(label: &str, generated: &GlobalMetrics, target: &GlobalMetrics) {
    fn indicator(generated_val: f32, target_val: f32, threshold: f32) -> &'static str {
        let diff = generated_val - target_val;
        if diff.abs() < threshold {
            "="
        } else if diff > 0.0 {
            "↑"
        } else {
            "↓"
        }
    }

    println!("{} METRICS:", label);
    println!(
        "  Voice Leading Effort: {:.2} {} (target: {:.2})",
        generated.average_voice_leading_effort,
        indicator(generated.average_voice_leading_effort, target.average_voice_leading_effort, 0.1),
        target.average_voice_leading_effort
    );
    println!(
        "  Tension Variance:     {:.4} {} (target: {:.4})",
        generated.tension_variance,
        indicator(generated.tension_variance, target.tension_variance, 0.01),
        target.tension_variance
    );
    println!(
        "  Tension/Release:      {:.2} {} (target: {:.2})",
        generated.tension_release_balance,
        indicator(generated.tension_release_balance, target.tension_release_balance, 0.05),
        target.tension_release_balance
    );
    println!(
        "  Diatonic %:           {:.1}% {} (target: {:.1}%)",
        generated.diatonic_percentage,
        indicator(generated.diatonic_percentage, target.diatonic_percentage, 5.0),
        target.diatonic_percentage
    );
    println!(
        "  Harmonic Rhythm:      {:.2} {} (target: {:.2})",
        generated.harmonic_rhythm,
        indicator(generated.harmonic_rhythm, target.harmonic_rhythm, 0.2),
        target.harmonic_rhythm
    );
}

/// Print current tuning parameters
fn print_current_tuning(tuning: &TuningParams) {
    println!();
    println!("CURRENT TUNING PARAMETERS:");
    println!("─────────────────────────────────────────────────────────────────────────");
    println!("  HARMONY:");
    println!("    max_semitone_movement:       {}", tuning.max_semitone_movement);
    println!("    cardinality_morph_enabled:   {}", tuning.cardinality_morph_enabled);
    println!("    trq_threshold:               {:.2}", tuning.trq_threshold);
    println!();
    println!("  STRATEGY THRESHOLDS:");
    println!("    steedman_lower:              {:.2}", tuning.steedman_lower_threshold);
    println!("    steedman_upper:              {:.2}", tuning.steedman_upper_threshold);
    println!("    neo_riemannian_lower:        {:.2}", tuning.neo_riemannian_lower_threshold);
    println!("    neo_riemannian_upper:        {:.2}", tuning.neo_riemannian_upper_threshold);
    println!("    hysteresis_boost:            {:.2}", tuning.hysteresis_boost);
    println!();
    println!("  RHYTHM (Perfect Balance):");
    println!("    kick_low_density_vertices:   {}", tuning.kick_low_density_vertices);
    println!("    kick_high_density_vertices:  {}", tuning.kick_high_density_vertices);
    println!("    snare_low_density_vertices:  {}", tuning.snare_low_density_vertices);
    println!("    snare_high_density_vertices: {}", tuning.snare_high_density_vertices);
    println!();
}

/// Save tuning to file
fn save_tuning(tuning: &TuningParams, path: &Path) -> Result<()> {
    tuning.to_toml_file(path).map_err(|e| anyhow::anyhow!("Failed to save tuning: {}", e))
}

/// Manual parameter editing
fn manual_edit_parameter(tuning: &TuningParams) -> Result<TuningParams> {
    let mut new_tuning = tuning.clone();

    println!();
    println!("Available parameters:");
    println!("  1. max_semitone_movement (current: {})", tuning.max_semitone_movement);
    println!("  2. trq_threshold (current: {:.2})", tuning.trq_threshold);
    println!("  3. steedman_lower_threshold (current: {:.2})", tuning.steedman_lower_threshold);
    println!("  4. steedman_upper_threshold (current: {:.2})", tuning.steedman_upper_threshold);
    println!(
        "  5. neo_riemannian_lower_threshold (current: {:.2})",
        tuning.neo_riemannian_lower_threshold
    );
    println!(
        "  6. neo_riemannian_upper_threshold (current: {:.2})",
        tuning.neo_riemannian_upper_threshold
    );
    println!("  7. hysteresis_boost (current: {:.2})", tuning.hysteresis_boost);
    println!("  8. kick_high_density_vertices (current: {})", tuning.kick_high_density_vertices);
    println!("  9. snare_high_density_vertices (current: {})", tuning.snare_high_density_vertices);
    println!();

    print!("Enter parameter number (1-9): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let param_num: usize = input.trim().parse().unwrap_or(0);

    print!("Enter new value: ");
    io::stdout().flush()?;

    let mut value_input = String::new();
    io::stdin().read_line(&mut value_input)?;
    let value_str = value_input.trim();

    match param_num {
        1 => {
            if let Ok(v) = value_str.parse::<u8>() {
                new_tuning.max_semitone_movement = v;
            }
        }
        2 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.trq_threshold = v;
            }
        }
        3 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.steedman_lower_threshold = v;
            }
        }
        4 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.steedman_upper_threshold = v;
            }
        }
        5 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.neo_riemannian_lower_threshold = v;
            }
        }
        6 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.neo_riemannian_upper_threshold = v;
            }
        }
        7 => {
            if let Ok(v) = value_str.parse::<f32>() {
                new_tuning.hysteresis_boost = v;
            }
        }
        8 => {
            if let Ok(v) = value_str.parse::<usize>() {
                new_tuning.kick_high_density_vertices = v;
            }
        }
        9 => {
            if let Ok(v) = value_str.parse::<usize>() {
                new_tuning.snare_high_density_vertices = v;
            }
        }
        _ => {
            println!("Invalid parameter number.");
        }
    }

    // Validate the new tuning
    if let Err(e) = new_tuning.validate() {
        println!("Warning: Invalid value - {}", e);
        println!("Reverting to previous value.");
        return Ok(tuning.clone());
    }

    println!("Parameter updated.");
    Ok(new_tuning)
}

fn cmd_extract_one(input: &Path, output: Option<&Path>) -> Result<()> {
    println!("Extracting DNA from: {}", input.display());

    let ingester = MusicXMLIngester::new();
    let dna = ingester.ingest_file(input)?;

    let json = dna.to_json()?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &json)?;
        println!("DNA saved to: {}", out_path.display());
    } else {
        println!("{}", json);
    }

    Ok(())
}
