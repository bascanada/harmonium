//! `MusicXML` Export Module
//!
//! Converts `HarmoniumScore` notation data to MusicXML format.
//!
//! # Example
//! ```ignore
//! use harmonium_core::export::score_to_musicxml;
//! use harmonium_core::notation::HarmoniumScore;
//!
//! let score = HarmoniumScore::default();
//! let xml = score_to_musicxml(&score);
//! std::fs::write("output.musicxml", xml).unwrap();
//! ```

use std::collections::HashMap;

use crate::{
    events::AudioEvent,
    notation::{
        ChordSymbol as NotationChordSymbol, Clef, Duration as NotationDuration, DurationBase,
        HarmoniumScore, NoteEventType, NoteStep, Part, Pitch, ScoreNoteEvent,
    },
    params::MusicalParams,
};

/// Escape special XML characters to produce valid XML output.
/// Handles: & < > " '
fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
}
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fmt::Write as FmtWrite, io::Write, path::Path};

/// Get current date in YYYY-MM-DD format (no external dependencies)
fn chrono_date() -> String {
    #[cfg(target_arch = "wasm32")]
    return String::from("2024-01-01");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();

        // Simple date calculation (no leap second handling, good enough for metadata)
        let days_since_epoch = secs / 86400;
        let mut year = 1970;
        let mut remaining_days = days_since_epoch as i64; // Explicitly use i64 for calculation

        loop {
            let days_in_year = if is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        let days_in_months: [i64; 12] = if is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        for &days in &days_in_months {
            if remaining_days < days {
                break;
            }
            remaining_days -= days;
            month += 1;
        }

        let day = remaining_days + 1;
        format!("{year:04}-{month:02}-{day:02}")
    }
}

#[cfg(not(target_arch = "wasm32"))]
const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Git version info (tag and short SHA)
///
/// Version information is captured at compile time via build.rs, making the binary
/// portable without requiring git to be installed at runtime.
#[derive(Clone, Debug)]
pub struct GitVersion {
    /// Version tag (e.g., "v0.1.0" or crate version if no tag)
    pub tag: String,
    /// Short commit SHA (e.g., "abc1234")
    pub sha: String,
}

impl std::fmt::Display for GitVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.tag, self.sha)
    }
}

impl GitVersion {
    /// Get git version captured at compile time
    ///
    /// This uses environment variables set by build.rs, avoiding the need for
    /// git to be installed at runtime. Falls back to crate version if git info
    /// was not available at compile time.
    #[must_use]
    pub fn detect() -> Self {
        // These are set at compile time by build.rs
        let tag = env!("GIT_VERSION_TAG").to_string();
        let sha = env!("GIT_VERSION_SHA").to_string();
        Self { tag, sha }
    }
}

impl Default for GitVersion {
    fn default() -> Self {
        Self::detect()
    }
}

#[derive(Clone, Debug)]
pub struct ChordSymbol {
    pub step: usize,
    /// Root pitch class (0-11, where 0=C)
    pub root: u8,
    /// MusicXML chord kind (e.g., "major", "minor", "dominant", "major-seventh")
    pub kind: String,
    /// Display text (e.g., "Cmaj7", "Am")
    pub text: String,
}

impl ChordSymbol {
    /// Create a chord symbol from a root pitch class and chord type suffix
    #[must_use]
    pub fn new(step: usize, root: u8, chord_type: &str) -> Self {
        let (kind, text) = Self::type_to_musicxml_kind(root, chord_type);
        Self { step, root: root % 12, kind, text }
    }

    /// Convert chord type suffix to `MusicXML` kind and display text
    fn type_to_musicxml_kind(root: u8, chord_type: &str) -> (String, String) {
        let root_name = Self::root_name(root);

        let (kind, suffix) = match chord_type {
            "Minor" | "m" | "min" => ("minor", "m"),
            "Dominant" | "7" | "dom" | "Dominant7" => ("dominant", "7"),
            "dim" | "Diminished" => ("diminished", "dim"),
            "maj7" | "Major7" => ("major-seventh", "maj7"),
            "m7" | "Minor7" => ("minor-seventh", "m7"),
            "m7b5" | "HalfDiminished" => ("half-diminished", "m7b5"),
            "dim7" | "Diminished7" => ("diminished-seventh", "dim7"),
            "sus2" | "Sus2" => ("suspended-second", "sus2"),
            "sus4" | "Sus4" => ("suspended-fourth", "sus4"),
            "mMaj7" | "MinorMajor7" => ("major-minor", "mMaj7"),
            "7#5" | "Augmented7" => ("augmented-seventh", "7#5"),
            "6" | "Major6" => ("major-sixth", "6"),
            "m6" | "Minor6" => ("minor-sixth", "m6"),
            "7sus4" | "Dominant7Sus4" => ("dominant-11th", "7sus4"),
            "add9" | "Add9" => ("major-ninth", "add9"),
            _ => ("major", ""),
        };

        (kind.to_string(), format!("{root_name}{suffix}"))
    }

    /// Get root note name with proper enharmonic spelling
    const fn root_name(root: u8) -> &'static str {
        match root % 12 {
            10 => "A#",
            11 => "B",
            _ => "C",
        }
    }

    /// Get `MusicXML` root-step and root-alter
    const fn root_step_alter(&self) -> (&'static str, i8) {
        match self.root % 12 {
            10 => ("B", -1), // Bb
            11 => ("B", 0),
            _ => ("C", 0),
        }
    }
}

/// A note computed from NoteOn/NoteOff pair with duration
#[derive(Clone, Debug)]
pub struct ScoreNote {
    /// MIDI pitch (0-127)
    pub pitch: u8,
    /// Start time in steps
    pub start_step: usize,
    /// Duration in steps (from `NoteOn` to `NoteOff`)
    pub duration_steps: usize,
    /// Channel (0=Bass, 1=Lead, 2=Snare, 3=Hat)
    pub channel: u8,
    /// Velocity (for dynamics notation)
    pub velocity: u8,
}

/// Key signature mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    Major,
    Minor,
}

/// Clef type for a part
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClefType {
    Treble,
    Bass,
    Percussion,
}

/// Calculate circle of fifths position from `key_root` and mode
///
/// Returns the `<fifths>` value for `MusicXML` (-7 to +7)
/// - Positive values = sharps (G=1, D=2, A=3, E=4, B=5, F#=6, C#=7)
/// - Negative values = flats (F=-1, Bb=-2, Eb=-3, Ab=-4, Db=-5, Gb=-6, Cb=-7)
/// - Zero = C major / A minor
#[must_use]
pub const fn fifths_from_key(key_root: u8, is_minor: bool) -> i8 {
    let root = key_root % 12;

    // Major key fifths mapping
    // This maps pitch class to circle of fifths position
    let major_fifths: [i8; 12] = [
        0,  // C  = 0 fifths
        -5, // Db = -5 fifths (prefer flat over C#=7)
        2,  // D  = 2 fifths
        -3, // Eb = -3 fifths (prefer flat over D#=9)
        4,  // E  = 4 fifths
        -1, // F  = -1 fifths
        6,  // F# = 6 fifths (or Gb=-6, prefer sharp for symmetry)
        1,  // G  = 1 fifth
        -4, // Ab = -4 fifths (prefer flat over G#=8)
        3,  // A  = 3 fifths
        -2, // Bb = -2 fifths (prefer flat over A#=10)
        5,  // B  = 5 fifths
    ];

    if is_minor {
        // Minor keys use the same key signature as their relative major
        // Relative major is 3 semitones up from minor root
        let relative_major = (root + 3) % 12;
        major_fifths[relative_major as usize]
    } else {
        major_fifths[root as usize]
    }
}

/// Get the key name string for display
#[must_use]
pub const fn key_name(key_root: u8, is_minor: bool) -> &'static str {
    let root = key_root % 12;
    if is_minor {
        match root {
            0 => "C minor",
            1 => "C# minor",
            2 => "D minor",
            3 => "Eb minor",
            4 => "E minor",
            5 => "F minor",
            6 => "F# minor",
            7 => "G minor",
            8 => "G# minor",
            9 => "A minor",
            10 => "Bb minor",
            11 => "B minor",
            _ => "Unknown",
        }
    } else {
        match root {
            0 => "C major",
            1 => "Db major",
            2 => "D major",
            3 => "Eb major",
            4 => "E major",
            5 => "F major",
            6 => "F# major",
            7 => "G major",
            8 => "Ab major",
            9 => "A major",
            10 => "Bb major",
            11 => "B major",
            _ => "Unknown",
        }
    }
}

// REMOVED: time_signature_from_steps() - Use explicit time signature from MusicalParams
// REMOVED: steps_per_quarter() - Use explicit steps_per_quarter from MusicalParams

/// Main builder for `MusicXML` output
pub struct MusicXmlBuilder {
    /// Event history (timestamp in musical steps, event)
    events: Vec<(f64, AudioEvent)>, // Changed from u64 to f64
    /// Musical parameters
    params: MusicalParams,
    /// Computed score notes
    notes: Vec<ScoreNote>,
    /// Chord symbols for harmony annotations
    chord_symbols: Vec<ChordSymbol>,
    /// Key signature fifths value
    fifths: i8,
    /// Key mode (major/minor)
    mode: KeyMode,
    /// Time signature (beats, `beat_type`)
    time_sig: (u8, u8),
    /// Steps per quarter note (`MusicXML` divisions)
    divisions: usize,
}

impl MusicXmlBuilder {
    /// Create a new `MusicXML` builder from event history
    #[must_use]
    pub fn new(
        events: Vec<(f64, AudioEvent)>, // Changed to f64
        params: &MusicalParams,
        samples_per_step: usize,
    ) -> Self {
        Self::with_chords(events, Vec::new(), params, samples_per_step)
    }

    /// Create a new `MusicXML` builder with chord symbols
    #[must_use]
    pub fn with_chords(
        events: Vec<(f64, AudioEvent)>, // Changed to f64
        chord_symbols: Vec<ChordSymbol>,
        params: &MusicalParams,
        _samples_per_step: usize,
    ) -> Self {
        let is_minor = params.harmony_valence < 0.0;
        let fifths = fifths_from_key(params.key_root, is_minor);
        let mode = if is_minor { KeyMode::Minor } else { KeyMode::Major };

        // Use explicit time signature from params (no inference)
        let time_sig = (params.time_signature.numerator, params.time_signature.denominator);
        let divisions = params.steps_per_quarter;

        let mut builder = Self {
            events,
            params: params.clone(),
            notes: Vec::new(),
            chord_symbols,
            fifths,
            mode,
            time_sig,
            divisions,
        };
        builder.compute_notes();
        builder
    }

    /// Build the `MusicXML` string (auto-detects git version)
    #[must_use]
    pub fn build(&self) -> String {
        let git = GitVersion::detect();
        self.build_with_version(&git.tag, &git.sha)
    }

    /// Build the `MusicXML` string with version info
    ///
    /// Note: Writing to a String in Rust cannot fail (except for OOM which would panic anyway),
    /// so we use `let _ =` to acknowledge write results without panicking behavior.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn build_with_version(&self, version: &str, git_sha: &str) -> String {
        let mut xml = String::new();

        // Escape version info to prevent XML injection
        let version_escaped = xml_escape(version);
        let git_sha_escaped = xml_escape(git_sha);

        // XML header
        let _ = writeln!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = writeln!(
            xml,
            r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#
        );
        let _ = writeln!(xml, r#"<score-partwise version="4.0">"#);

        // Work title with version
        // Handle version with or without 'v' prefix
        let version_display = if version_escaped.starts_with('v') {
            version_escaped
        } else {
            format!("v{version_escaped}")
        };
        let _ = writeln!(xml, "  <work>");
        let _ = writeln!(
            xml,
            "    <work-title>Harmonium {version_display}-{git_sha_escaped}</work-title>"
        );
        let _ = writeln!(xml, "  </work>");

        // Identification
        let key_name = key_name(self.params.key_root, self.mode == KeyMode::Minor);
        let _ = writeln!(xml, "  <identification>");
        let _ = writeln!(xml, r#"    <creator type="composer">Harmonium</creator>"#);
        let _ = writeln!(xml, "    <encoding>");
        let _ = writeln!(
            xml,
            "      <software>harmonium_core {version_display}-{git_sha_escaped}</software>"
        );
        let _ = writeln!(xml, "      <encoding-date>{}</encoding-date>", chrono_date());
        let _ = writeln!(xml, "    </encoding>");
        let _ = writeln!(xml, "    <miscellaneous>");
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"key\">{key_name}</miscellaneous-field>"
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"bpm\">{}</miscellaneous-field>",
            self.params.bpm
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"time_signature\">{}/{}</miscellaneous-field>",
            self.time_sig.0, self.time_sig.1
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"rhythm_mode\">{:?}</miscellaneous-field>",
            self.params.rhythm_mode
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"rhythm_steps\">{}</miscellaneous-field>",
            self.params.rhythm_steps
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"rhythm_pulses\">{}</miscellaneous-field>",
            self.params.rhythm_pulses
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"rhythm_density\">{:.2}</miscellaneous-field>",
            self.params.rhythm_density
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"rhythm_tension\">{:.2}</miscellaneous-field>",
            self.params.rhythm_tension
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"harmony_tension\">{:.2}</miscellaneous-field>",
            self.params.harmony_tension
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"harmony_valence\">{:.2}</miscellaneous-field>",
            self.params.harmony_valence
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"harmony_measures_per_chord\">{}</miscellaneous-field>",
            self.params.harmony_measures_per_chord
        );
        let _ = writeln!(
            xml,
            "      <miscellaneous-field name=\"melody_octave\">{}</miscellaneous-field>",
            self.params.melody_octave
        );
        let _ = writeln!(xml, "    </miscellaneous>");
        let _ = writeln!(xml, "  </identification>");

        // Credits for display on score (MuseScore shows these)
        // Compact title with version and date - positioned at top
        let _ = writeln!(xml, "  <credit page=\"1\">");
        let _ = writeln!(xml, "    <credit-type>title</credit-type>");
        let _ = writeln!(
            xml,
            r#"    <credit-words default-x="600" default-y="1550" font-size="14" justify="center" valign="top">Harmonium {}-{} | {}</credit-words>"#,
            version_display,
            git_sha_escaped,
            chrono_date()
        );
        let _ = writeln!(xml, "  </credit>");

        // Compact parameters (single line) - positioned below title
        // Format: Key | BPM | Time | Mode(steps,pulses,d,t) | Harm(t,v,m)
        let mode_full = format!("{:?}", self.params.rhythm_mode);
        let mode_short = match mode_full.as_str() {
            "Euclidean" => "Euc",
            "PerfectBalance" => "PB",
            "ClassicGroove" => "CG",
            _ => &mode_full,
        };
        let _ = writeln!(xml, "  <credit page=\"1\">");
        let _ = writeln!(xml, "    <credit-type>subtitle</credit-type>");
        let _ = writeln!(
            xml,
            r#"    <credit-words default-x="600" default-y="1520" font-size="9" justify="center" valign="top">{} | {} BPM | {}/{} | {}({},{},{:.1},{:.1}) | H({:.1},{:.1},{})</credit-words>"#,
            key_name,
            self.params.bpm,
            self.time_sig.0,
            self.time_sig.1,
            mode_short,
            self.params.rhythm_steps,
            self.params.rhythm_pulses,
            self.params.rhythm_density,
            self.params.rhythm_tension,
            self.params.harmony_tension,
            self.params.harmony_valence,
            self.params.harmony_measures_per_chord
        );
        let _ = writeln!(xml, "  </credit>");

        // Part list
        let _ = writeln!(xml, "  <part-list>");
        let _ =
            writeln!(xml, r#"    <score-part id="P1"><part-name>Lead</part-name></score-part>"#);
        let _ =
            writeln!(xml, r#"    <score-part id="P2"><part-name>Bass</part-name></score-part>"#);
        let _ =
            writeln!(xml, r#"    <score-part id="P3"><part-name>Drums</part-name></score-part>"#);
        let _ = writeln!(xml, "  </part-list>");

        // Parts
        self.write_part(&mut xml, "P1", 1, ClefType::Treble);
        self.write_part(&mut xml, "P2", 0, ClefType::Bass);
        self.write_drum_part(&mut xml, "P3");

        let _ = writeln!(xml, "</score-partwise>");
        xml
    }

    /// Compute `ScoreNotes` from NoteOn/NoteOff events
    fn compute_notes(&mut self) {
        eprintln!("MusicXML export: Processing {} total events", self.events.len());

        // Map of (channel, pitch) -> (start_step, velocity)
        let mut pending: HashMap<(u8, u8), (usize, u8)> = HashMap::new();
        let mut notes = Vec::new();

        for (step_timestamp, event) in &self.events {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let step = step_timestamp.round() as usize;

            match event {
                AudioEvent::NoteOn { note, velocity, channel, .. } => {
                    if *velocity > 0 {
                        // If this note is already pending (common for percussion), finalize it first
                        if let Some((old_start, old_vel)) = pending.get(&(*channel, *note)) {
                            // Give it a short default duration (percussion hits)
                            let default_duration = if *channel == 2 || *channel == 3 {
                                2 // Short percussion hit
                            } else {
                                4 // Quarter note for other instruments
                            };

                            notes.push(ScoreNote {
                                pitch: *note,
                                start_step: *old_start,
                                duration_steps: default_duration,
                                channel: *channel,
                                velocity: *old_vel,
                            });
                        }
                        pending.insert((*channel, *note), (step, *velocity));
                    } else {
                        // Velocity 0 = NoteOff
                        if let Some((start, vel)) = pending.remove(&(*channel, *note)) {
                            notes.push(ScoreNote {
                                pitch: *note,
                                start_step: start,
                                duration_steps: step.saturating_sub(start).max(1),
                                channel: *channel,
                                velocity: vel,
                            });
                        }
                    }
                }
                AudioEvent::NoteOff { note, channel, .. } => {
                    if let Some((start, vel)) = pending.remove(&(*channel, *note)) {
                        notes.push(ScoreNote {
                            pitch: *note,
                            start_step: start,
                            duration_steps: step.saturating_sub(start).max(1),
                            channel: *channel,
                            velocity: vel,
                        });
                    }
                }
                _ => {}
            }
        }

        // Handle pending notes (no NoteOff received)
        let pending_count = pending.len();
        for ((channel, pitch), (start, vel)) in pending {
            // Percussion channels (2=snare, 3=hat) get short default duration
            // Other channels get full measure duration
            let default_duration = if channel == 2 || channel == 3 {
                2 // Short percussion hit (equivalent to 8th note at 16 steps/measure)
            } else {
                self.params.rhythm_steps // Full measure for sustained notes
            };

            notes.push(ScoreNote {
                pitch,
                start_step: start,
                duration_steps: default_duration,
                channel,
                velocity: vel,
            });
        }

        if pending_count > 0 {
            eprintln!("MusicXML export: {pending_count} notes still pending (no NoteOff received)");
        }

        notes.sort_by_key(|n| (n.start_step, n.channel, n.pitch));

        // Debug: Count notes per channel
        let mut channel_counts = [0; 16];
        for note in &notes {
            channel_counts[note.channel as usize] += 1;
        }
        eprintln!("MusicXML: Created {} total notes from events", notes.len());
        eprintln!("MusicXML: Note counts by channel:");
        for (ch, count) in channel_counts.iter().enumerate() {
            if *count > 0 {
                eprintln!("  Channel {ch}: {count} notes");
            }
        }

        self.notes = notes;
    }

    /// Write a pitched part (Lead or Bass)
    /// Lead part (channel 1) includes chord symbols
    fn write_part(&self, xml: &mut String, part_id: &str, channel: u8, clef: ClefType) {
        let _ = writeln!(xml, r#"  <part id="{part_id}">"#);

        let part_notes: Vec<&ScoreNote> =
            self.notes.iter().filter(|n| n.channel == channel).collect();

        let steps_per_measure = self.steps_per_measure();
        let total_measures = self.calculate_total_measures(&part_notes);

        // Only show chord symbols on Lead part (channel 1)
        let show_chords = channel == 1;

        for measure in 0..total_measures.max(1) {
            let _ = writeln!(xml, r#"    <measure number="{}">"#, measure + 1);

            // Attributes on first measure
            if measure == 0 {
                self.write_attributes(xml, clef);
            }

            // Get notes for this measure
            let measure_start = measure * steps_per_measure;
            let measure_end = (measure + 1) * steps_per_measure;

            // Get chord symbols for this measure (only for Lead)
            let chords_in_measure: Vec<&ChordSymbol> = if show_chords {
                self.chord_symbols
                    .iter()
                    .filter(|c| c.step >= measure_start && c.step < measure_end)
                    .collect()
            } else {
                Vec::new()
            };

            let notes_in_measure: Vec<&&ScoreNote> = part_notes
                .iter()
                .filter(|n| n.start_step >= measure_start && n.start_step < measure_end)
                .collect();

            let mut current_pos = measure_start;
            let mut chord_idx = 0;

            // Group notes by start_step for chord detection
            // Use a tolerance of 1 step for chord grouping (real-time recording can have slight timing differences)
            const CHORD_TOLERANCE: usize = 1;
            let mut i = 0;
            while i < notes_in_measure.len() {
                let note = notes_in_measure[i];

                // Skip notes that start at or after the measure end
                if note.start_step >= measure_end {
                    i += 1;
                    continue;
                }

                // Write any chord symbols that come before or at this note
                while chord_idx < chords_in_measure.len()
                    && chords_in_measure[chord_idx].step <= note.start_step
                {
                    self.write_harmony(xml, chords_in_measure[chord_idx]);
                    chord_idx += 1;
                }

                // Calculate remaining space in measure
                let remaining = measure_end.saturating_sub(current_pos);
                if remaining == 0 {
                    // Measure is full, skip remaining notes
                    break;
                }

                // Fill rest before note (only if there's a significant gap)
                let note_pos = note.start_step.max(current_pos);
                if note_pos > current_pos {
                    let rest_duration = (note_pos - current_pos).min(remaining);
                    if rest_duration > 0 {
                        self.write_rest(xml, rest_duration);
                        current_pos += rest_duration;
                    }
                }

                // Recalculate remaining after rest
                let remaining = measure_end.saturating_sub(current_pos);
                if remaining == 0 {
                    break;
                }

                // Check for simultaneous notes (chord) - use tolerance for real-time recordings
                let mut chord_notes = vec![note];
                let mut j = i + 1;
                while j < notes_in_measure.len() {
                    let time_diff = notes_in_measure[j].start_step.saturating_sub(note.start_step);
                    if time_diff <= CHORD_TOLERANCE {
                        chord_notes.push(notes_in_measure[j]);
                        j += 1;
                    } else {
                        break;
                    }
                }

                // Clamp note duration to fit in remaining measure space
                let clamped_duration = chord_notes[0].duration_steps.min(remaining);
                if clamped_duration == 0 {
                    i = j;
                    continue;
                }

                // Write chord notes (all with same duration)
                for (idx, chord_note) in chord_notes.iter().enumerate() {
                    self.write_pitched_note_with_duration(
                        xml,
                        chord_note,
                        idx > 0,
                        clamped_duration,
                    );
                }

                current_pos += clamped_duration;
                i = j;
            }

            // Write any remaining chord symbols at end of measure
            while chord_idx < chords_in_measure.len() {
                self.write_harmony(xml, chords_in_measure[chord_idx]);
                chord_idx += 1;
            }

            // Fill rest at end of measure
            if current_pos < measure_end {
                self.write_rest(xml, measure_end - current_pos);
            }

            let _ = writeln!(xml, "    </measure>");
        }

        let _ = writeln!(xml, "  </part>");
    }

    /// Write a harmony (chord symbol) element
    #[allow(clippy::unused_self)]
    fn write_harmony(&self, xml: &mut String, chord: &ChordSymbol) {
        let (root_step, root_alter) = chord.root_step_alter();
        // Escape chord text to prevent XML injection
        let text_escaped = xml_escape(&chord.text);
        let kind_escaped = xml_escape(&chord.kind);

        let _ = writeln!(xml, "      <harmony>");
        let _ = writeln!(xml, "        <root>");
        let _ = writeln!(xml, "          <root-step>{root_step}</root-step>");
        if root_alter != 0 {
            let _ = writeln!(xml, "          <root-alter>{root_alter}</root-alter>");
        }
        let _ = writeln!(xml, "        </root>");
        let _ = writeln!(xml, "        <kind text=\"{text_escaped}\">{kind_escaped}</kind>");
        let _ = writeln!(xml, "      </harmony>");
    }

    /// Write the drum part (channels 2 and 3 combined)
    fn write_drum_part(&self, xml: &mut String, part_id: &str) {
        let _ = writeln!(xml, r#"  <part id="{part_id}">"#);

        let drum_notes: Vec<&ScoreNote> =
            self.notes.iter().filter(|n| n.channel == 2 || n.channel == 3).collect();

        let steps_per_measure = self.steps_per_measure();
        let total_measures = self.calculate_total_measures(&drum_notes);

        for measure in 0..total_measures.max(1) {
            let _ = writeln!(xml, r#"    <measure number="{}">"#, measure + 1);

            // Attributes on first measure (percussion clef)
            if measure == 0 {
                let _ = writeln!(xml, "      <attributes>");
                let _ = writeln!(xml, "        <divisions>{}</divisions>", self.divisions);
                let _ = writeln!(xml, "        <key><fifths>0</fifths></key>");
                let _ = writeln!(
                    xml,
                    "        <time><beats>{}</beats><beat-type>{}</beat-type></time>",
                    self.time_sig.0, self.time_sig.1
                );
                let _ = writeln!(xml, "        <clef><sign>percussion</sign><line>2</line></clef>");
                let _ = writeln!(xml, "      </attributes>");
            }

            let measure_start = measure * steps_per_measure;
            let measure_end = (measure + 1) * steps_per_measure;

            let notes_in_measure: Vec<&&ScoreNote> = drum_notes
                .iter()
                .filter(|n| n.start_step >= measure_start && n.start_step < measure_end)
                .collect();

            let mut current_pos = measure_start;

            // Use same chord tolerance as pitched parts
            const CHORD_TOLERANCE: usize = 1;
            let mut i = 0;
            while i < notes_in_measure.len() {
                let note = notes_in_measure[i];

                // Skip notes that start at or after the measure end
                if note.start_step >= measure_end {
                    i += 1;
                    continue;
                }

                // Calculate remaining space in measure
                let remaining = measure_end.saturating_sub(current_pos);
                if remaining == 0 {
                    break;
                }

                // Fill rest before note
                let note_pos = note.start_step.max(current_pos);
                if note_pos > current_pos {
                    let rest_duration = (note_pos - current_pos).min(remaining);
                    if rest_duration > 0 {
                        self.write_rest(xml, rest_duration);
                        current_pos += rest_duration;
                    }
                }

                // Recalculate remaining after rest
                let remaining = measure_end.saturating_sub(current_pos);
                if remaining == 0 {
                    break;
                }

                // Check for simultaneous drum hits - use tolerance
                let mut chord_notes = vec![note];
                let mut j = i + 1;
                while j < notes_in_measure.len() {
                    let time_diff = notes_in_measure[j].start_step.saturating_sub(note.start_step);
                    if time_diff <= CHORD_TOLERANCE {
                        chord_notes.push(notes_in_measure[j]);
                        j += 1;
                    } else {
                        break;
                    }
                }

                // Clamp duration to fit in remaining measure space
                let clamped_duration = chord_notes[0].duration_steps.min(remaining);
                if clamped_duration == 0 {
                    i = j;
                    continue;
                }

                // Write drum notes (all with same duration)
                for (idx, chord_note) in chord_notes.iter().enumerate() {
                    self.write_drum_note_with_duration(xml, chord_note, idx > 0, clamped_duration);
                }

                current_pos += clamped_duration;
                i = j;
            }

            if current_pos < measure_end {
                self.write_rest(xml, measure_end - current_pos);
            }

            let _ = writeln!(xml, "    </measure>");
        }

        let _ = writeln!(xml, "  </part>");
    }

    /// Write measure attributes (key, time, clef)
    fn write_attributes(&self, xml: &mut String, clef: ClefType) {
        let _ = writeln!(xml, "      <attributes>");
        let _ = writeln!(xml, "        <divisions>{}</divisions>", self.divisions);

        // Key signature
        let mode_str = match self.mode {
            KeyMode::Major => "major",
            KeyMode::Minor => "minor",
        };
        let _ = writeln!(
            xml,
            "        <key><fifths>{}</fifths><mode>{}</mode></key>",
            self.fifths, mode_str
        );

        // Time signature
        let _ = writeln!(
            xml,
            "        <time><beats>{}</beats><beat-type>{}</beat-type></time>",
            self.time_sig.0, self.time_sig.1
        );

        // Clef
        let (sign, line) = match clef {
            ClefType::Treble => ("G", 2),
            ClefType::Bass => ("F", 4),
            ClefType::Percussion => ("percussion", 2),
        };
        let _ = writeln!(xml, "        <clef><sign>{sign}</sign><line>{line}</line></clef>");

        let _ = writeln!(xml, "      </attributes>");
    }

    /// Write a pitched note with explicit duration (for measure clamping)
    fn write_pitched_note_with_duration(
        &self,
        xml: &mut String,
        note: &ScoreNote,
        is_chord: bool,
        duration: usize,
    ) {
        let (step, alter, octave) = self.midi_to_pitch(note.pitch);
        let (note_type, dots) = self.duration_to_type(duration);

        let _ = writeln!(xml, "      <note>");
        if is_chord {
            let _ = writeln!(xml, "        <chord/>");
        }
        let _ = writeln!(xml, "        <pitch>");
        let _ = writeln!(xml, "          <step>{step}</step>");
        if alter != 0 {
            let _ = writeln!(xml, "          <alter>{alter}</alter>");
        }
        let _ = writeln!(xml, "          <octave>{octave}</octave>");
        let _ = writeln!(xml, "        </pitch>");
        let _ = writeln!(xml, "        <duration>{duration}</duration>");
        let _ = writeln!(xml, "        <type>{note_type}</type>");
        for _ in 0..dots {
            let _ = writeln!(xml, "        <dot/>");
        }
        // Add accidental for visual display if altered
        if alter != 0 {
            let acc = if alter > 0 { "sharp" } else { "flat" };
            let _ = writeln!(xml, "        <accidental>{acc}</accidental>");
        }
        let _ = writeln!(xml, "      </note>");
    }

    /// Write a drum note with explicit duration (for measure clamping)
    fn write_drum_note_with_duration(
        &self,
        xml: &mut String,
        note: &ScoreNote,
        is_chord: bool,
        duration: usize,
    ) {
        // Map channel to display position
        // Channel 2 = Snare (middle of staff, E4)
        // Channel 3 = Hi-hat (above staff, G5)
        let (display_step, display_octave) = match note.channel {
            2 => ("E", 4), // Snare
            3 => ("G", 5), // Hi-hat
            _ => ("F", 4), // Default (kick would be here)
        };

        let (note_type, dots) = self.duration_to_type(duration);

        let _ = writeln!(xml, "      <note>");
        if is_chord {
            let _ = writeln!(xml, "        <chord/>");
        }
        let _ = writeln!(xml, "        <unpitched>");
        let _ = writeln!(xml, "          <display-step>{display_step}</display-step>");
        let _ = writeln!(xml, "          <display-octave>{display_octave}</display-octave>");
        let _ = writeln!(xml, "        </unpitched>");
        let _ = writeln!(xml, "        <duration>{duration}</duration>");
        let _ = writeln!(xml, "        <type>{note_type}</type>");
        for _ in 0..dots {
            let _ = writeln!(xml, "        <dot/>");
        }
        let _ = writeln!(xml, "      </note>");
    }

    /// Write a rest
    fn write_rest(&self, xml: &mut String, duration: usize) {
        if duration == 0 {
            return;
        }
        let (note_type, dots) = self.duration_to_type(duration);

        let _ = writeln!(xml, "      <note>");
        let _ = writeln!(xml, "        <rest/>");
        let _ = writeln!(xml, "        <duration>{duration}</duration>");
        let _ = writeln!(xml, "        <type>{note_type}</type>");
        for _ in 0..dots {
            let _ = writeln!(xml, "        <dot/>");
        }
        let _ = writeln!(xml, "      </note>");
    }

    /// Convert MIDI pitch to `MusicXML` pitch components
    /// Returns (step, alter, octave)
    const fn midi_to_pitch(&self, midi: u8) -> (&'static str, i8, u8) {
        let octave = (midi / 12).saturating_sub(1);
        let pitch_class = midi % 12;

        // Use key signature to determine enharmonic spelling
        // Sharp keys (fifths >= 0): prefer sharps
        // Flat keys (fifths < 0): prefer flats
        let use_sharps = self.fifths >= 0;

        let (step, alter) = match pitch_class {
            0 => ("C", 0),
            1 => {
                if use_sharps {
                    ("C", 1)
                } else {
                    ("D", -1)
                }
            }
            2 => ("D", 0),
            3 => {
                if use_sharps {
                    ("D", 1)
                } else {
                    ("E", -1)
                }
            }
            4 => ("E", 0),
            5 => ("F", 0),
            6 => {
                if use_sharps {
                    ("F", 1)
                } else {
                    ("G", -1)
                }
            }
            7 => ("G", 0),
            8 => {
                if use_sharps {
                    ("G", 1)
                } else {
                    ("A", -1)
                }
            }
            9 => ("A", 0),
            10 => {
                if use_sharps {
                    ("A", 1)
                } else {
                    ("B", -1)
                }
            }
            11 => ("B", 0),
            _ => ("C", 0),
        };

        (step, alter, octave)
    }

    /// Convert duration in steps to `MusicXML` note type
    /// Returns (`type_name`, `dot_count`)
    fn duration_to_type(&self, duration: usize) -> (&'static str, u8) {
        #[allow(clippy::cast_precision_loss)]
        // Calculate ratio to quarter note
        let quarters = duration as f32 / self.divisions as f32;

        // Standard note types: whole=4, half=2, quarter=1, eighth=0.5, 16th=0.25
        if quarters >= 4.0 {
            ("whole", 0)
        } else if quarters >= 3.0 {
            ("half", 1) // dotted half
        } else if quarters >= 2.0 {
            ("half", 0)
        } else if quarters >= 1.5 {
            ("quarter", 1) // dotted quarter
        } else if quarters >= 1.0 {
            ("quarter", 0)
        } else if quarters >= 0.75 {
            ("eighth", 1) // dotted eighth
        } else if quarters >= 0.5 {
            ("eighth", 0)
        } else if quarters >= 0.375 {
            ("16th", 1) // dotted 16th
        } else if quarters >= 0.25 {
            ("16th", 0)
        } else {
            ("32nd", 0)
        }
    }

    /// Calculate steps per measure
    fn steps_per_measure(&self) -> usize {
        self.params.steps_per_measure()
    }

    /// Calculate total measures needed based on ALL notes (not just one part)
    /// All parts must have the same number of measures in `MusicXML`
    fn calculate_total_measures(&self, _notes: &[&ScoreNote]) -> usize {
        let steps_per_measure = self.steps_per_measure();
        // Calculate based on ALL notes across ALL parts
        let max_step =
            self.notes.iter().map(|n| n.start_step + n.duration_steps).max().unwrap_or(0);
        max_step.div_ceil(steps_per_measure)
    }
}

/// Convert event history + musical params to `MusicXML` string
///
/// # Arguments
/// * `events` - Vector of (`timestamp_samples`, `AudioEvent`) tuples
/// * `params` - Musical parameters containing key, time signature info
/// * `samples_per_step` - Number of audio samples per sequencer step
///
/// # Returns
/// A complete `MusicXML` 4.0 string ready to be saved or opened in notation software
#[must_use]
#[deprecated(since = "0.2.0", note = "Use score_to_musicxml with ScoreBuffer instead")]
pub fn to_musicxml(
    events: &[(f64, AudioEvent)], // Changed from u64 to f64
    params: &MusicalParams,
    samples_per_step: usize, // Kept for backward compatibility
) -> String {
    let builder = MusicXmlBuilder::new(events.to_vec(), params, samples_per_step);
    builder.build()
}

/// Write `MusicXML` to a file
///
/// # Arguments
/// * `events` - Vector of (`timestamp_samples`, `AudioEvent`) tuples
/// * `params` - Musical parameters containing key, time signature info
/// * `samples_per_step` - Number of audio samples per sequencer step
/// * `path` - Output file path
#[deprecated(since = "0.2.0", note = "Use score_to_musicxml with ScoreBuffer instead")]
pub fn write_musicxml(
    events: &[(f64, AudioEvent)],
    params: &MusicalParams,
    samples_per_step: usize,
    path: &Path,
) -> std::io::Result<()> {
    #[allow(deprecated)]
    let xml = to_musicxml(events, params, samples_per_step);
    let mut file = std::fs::File::create(path)?;
    file.write_all(xml.as_bytes())?;
    Ok(())
}

/// Convert event history + chord symbols + musical params to `MusicXML` string
///
/// # Arguments
/// * `events` - Vector of (`timestamp_samples`, `AudioEvent`) tuples
/// * `chords` - Vector of chord symbols with step positions
/// * `params` - Musical parameters containing key, time signature info
/// * `samples_per_step` - Number of audio samples per sequencer step
///
/// # Returns
/// A complete `MusicXML` 4.0 string with chord symbols above the lead part
#[must_use]
#[deprecated(since = "0.2.0", note = "Use score_to_musicxml with ScoreBuffer instead")]
pub fn to_musicxml_with_chords(
    events: &[(f64, AudioEvent)], // Changed from u64 to f64
    chords: &[ChordSymbol],
    params: &MusicalParams,
    samples_per_step: usize, // Kept for backward compatibility
) -> String {
    let builder =
        MusicXmlBuilder::with_chords(events.to_vec(), chords.to_vec(), params, samples_per_step);
    builder.build()
}

/// Write `MusicXML` with chord symbols to a file
///
/// # Arguments
/// * `events` - Vector of (`timestamp_samples`, `AudioEvent`) tuples
/// * `chords` - Vector of chord symbols with step positions
/// * `params` - Musical parameters containing key, time signature info
/// * `samples_per_step` - Number of audio samples per sequencer step
/// * `path` - Output file path
#[deprecated(since = "0.2.0", note = "Use score_to_musicxml with ScoreBuffer instead")]
pub fn write_musicxml_with_chords(
    events: &[(f64, AudioEvent)],
    chords: &[ChordSymbol],
    params: &MusicalParams,
    samples_per_step: usize,
    path: &Path,
) -> std::io::Result<()> {
    #[allow(deprecated)]
    let xml = to_musicxml_with_chords(events, chords, params, samples_per_step);
    let mut file = std::fs::File::create(path)?;
    file.write_all(xml.as_bytes())?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// HARMONIUM SCORE TO MUSICXML
// ═══════════════════════════════════════════════════════════════════

/// Convert a HarmoniumScore to MusicXML string
///
/// This provides a direct path from the musical notation format to MusicXML
/// without requiring conversion through AudioEvents.
///
/// # Arguments
/// * `score` - The HarmoniumScore containing musical notation data
///
/// # Returns
/// A complete MusicXML 4.0 string
#[must_use]
pub fn score_to_musicxml(score: &HarmoniumScore) -> String {
    let git = GitVersion::detect();
    score_to_musicxml_with_version(score, &git.tag, &git.sha)
}

/// Convert a HarmoniumScore to MusicXML with explicit version info
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn score_to_musicxml_with_version(
    score: &HarmoniumScore,
    version: &str,
    git_sha: &str,
) -> String {
    let mut xml = String::new();

    // Escape version info
    let version_escaped = xml_escape(version);
    let git_sha_escaped = xml_escape(git_sha);

    // XML header
    let _ = writeln!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        xml,
        r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#
    );
    let _ = writeln!(xml, r#"<score-partwise version="4.0">"#);

    // Work title
    let version_display = if version_escaped.starts_with('v') {
        version_escaped.clone()
    } else {
        format!("v{version_escaped}")
    };
    let _ = writeln!(xml, "  <work>");
    let _ =
        writeln!(xml, "    <work-title>Harmonium {version_display}-{git_sha_escaped}</work-title>");
    let _ = writeln!(xml, "  </work>");

    // Identification
    let _ = writeln!(xml, "  <identification>");
    let _ = writeln!(xml, r#"    <creator type="composer">Harmonium</creator>"#);
    let _ = writeln!(xml, "    <encoding>");
    let _ = writeln!(
        xml,
        "      <software>harmonium_core {version_display}-{git_sha_escaped}</software>"
    );
    let _ = writeln!(xml, "      <encoding-date>{}</encoding-date>", chrono_date());
    let _ = writeln!(xml, "    </encoding>");
    let _ = writeln!(xml, "    <miscellaneous>");
    let _ = writeln!(
        xml,
        "      <miscellaneous-field name=\"key\">{} {}</miscellaneous-field>",
        score.key_signature.root,
        format!("{:?}", score.key_signature.mode).to_lowercase()
    );
    let _ = writeln!(
        xml,
        "      <miscellaneous-field name=\"bpm\">{}</miscellaneous-field>",
        score.tempo
    );
    let _ = writeln!(
        xml,
        "      <miscellaneous-field name=\"time_signature\">{}/{}</miscellaneous-field>",
        score.time_signature.0, score.time_signature.1
    );
    let _ = writeln!(xml, "    </miscellaneous>");
    let _ = writeln!(xml, "  </identification>");

    // Part list
    let _ = writeln!(xml, "  <part-list>");
    for part in &score.parts {
        let part_id_escaped = xml_escape(&part.id);
        let part_name_escaped = xml_escape(&part.name);
        let _ = writeln!(xml, r#"    <score-part id="P-{part_id_escaped}">"#);
        let _ = writeln!(xml, "      <part-name>{part_name_escaped}</part-name>");
        let _ = writeln!(xml, "    </score-part>");
    }
    let _ = writeln!(xml, "  </part-list>");

    // Write each part
    for part in &score.parts {
        write_score_part(&mut xml, score, part);
    }

    let _ = writeln!(xml, "</score-partwise>");
    xml
}

/// Write a single part from HarmoniumScore to MusicXML
fn write_score_part(xml: &mut String, score: &HarmoniumScore, part: &Part) {
    let part_id_escaped = xml_escape(&part.id);
    let _ = writeln!(xml, r#"  <part id="P-{part_id_escaped}">"#);

    // Calculate divisions (steps per quarter note)
    // Standard: 4 steps per quarter = 16th note resolution
    let divisions = 4;

    // Get maximum measure count across all parts
    let max_measures = score.parts.iter().map(|p| p.measures.len()).max().unwrap_or(1);

    for measure_idx in 0..max_measures {
        let measure_num = measure_idx + 1;
        let _ = writeln!(xml, r#"    <measure number="{measure_num}">"#);

        // Attributes on first measure
        if measure_idx == 0 {
            let _ = writeln!(xml, "      <attributes>");
            let _ = writeln!(xml, "        <divisions>{divisions}</divisions>");
            let _ =
                writeln!(xml, "        <key><fifths>{}</fifths></key>", score.key_signature.fifths);
            let _ = writeln!(
                xml,
                "        <time><beats>{}</beats><beat-type>{}</beat-type></time>",
                score.time_signature.0, score.time_signature.1
            );

            // Clef
            match part.clef {
                Clef::Treble => {
                    let _ = writeln!(xml, "        <clef><sign>G</sign><line>2</line></clef>");
                }
                Clef::Bass => {
                    let _ = writeln!(xml, "        <clef><sign>F</sign><line>4</line></clef>");
                }
                Clef::Percussion => {
                    let _ =
                        writeln!(xml, "        <clef><sign>percussion</sign><line>2</line></clef>");
                }
            }
            let _ = writeln!(xml, "      </attributes>");
        }

        // Get the measure if it exists
        if let Some(measure) = part.measures.get(measure_idx) {
            // Write chord symbols first (for lead part)
            for chord in &measure.chords {
                write_notation_harmony(xml, chord);
            }

            // Write events
            let beats_per_measure = score.time_signature.0 as f32;
            let mut current_beat = 1.0f32;

            // Sort events by beat position
            let mut events: Vec<&ScoreNoteEvent> = measure.events.iter().collect();
            events.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap_or(std::cmp::Ordering::Equal));

            for event in events {
                // Fill rest before this event if needed
                if event.beat > current_beat {
                    let rest_beats = event.beat - current_beat;
                    let rest_divisions = (rest_beats * divisions as f32) as usize;
                    if rest_divisions > 0 {
                        write_score_rest(xml, rest_divisions, divisions);
                    }
                    current_beat = event.beat;
                }

                // Write the event
                let event_duration_beats = event.duration.to_beats();
                let event_divisions = (event_duration_beats * divisions as f32) as usize;

                match event.event_type {
                    NoteEventType::Rest => {
                        write_score_rest(xml, event_divisions, divisions);
                    }
                    NoteEventType::Note | NoteEventType::Chord => {
                        for (i, pitch) in event.pitches.iter().enumerate() {
                            write_score_note(
                                xml,
                                pitch,
                                &event.duration,
                                i > 0,
                                event.dynamic.as_ref(),
                            );
                        }
                    }
                    NoteEventType::Drum => {
                        // Map to drum display position
                        write_score_drum_note(xml, &event.duration, event.pitches.first());
                    }
                }

                current_beat += event_duration_beats;
            }

            // Fill rest at end of measure
            if current_beat < beats_per_measure + 1.0 {
                let rest_beats = (beats_per_measure + 1.0) - current_beat;
                let rest_divisions = (rest_beats * divisions as f32) as usize;
                if rest_divisions > 0 {
                    write_score_rest(xml, rest_divisions, divisions);
                }
            }
        } else {
            // Empty measure - write full rest
            let beats_per_measure = score.time_signature.0;
            let rest_divisions = beats_per_measure as usize * divisions;
            write_score_rest(xml, rest_divisions, divisions);
        }

        let _ = writeln!(xml, "    </measure>");
    }

    let _ = writeln!(xml, "  </part>");
}

/// Write a harmony element from notation ChordSymbol
fn write_notation_harmony(xml: &mut String, chord: &NotationChordSymbol) {
    let root_escaped = xml_escape(&chord.root);
    let quality_escaped = xml_escape(&chord.quality);

    // Parse root into step and alter
    let (root_step, root_alter) = parse_note_name(&chord.root);

    // Map quality to MusicXML kind
    let kind = quality_to_musicxml_kind(&chord.quality);

    let _ = writeln!(xml, "      <harmony>");
    let _ = writeln!(xml, "        <root>");
    let _ = writeln!(xml, "          <root-step>{root_step}</root-step>");
    if root_alter != 0 {
        let _ = writeln!(xml, "          <root-alter>{root_alter}</root-alter>");
    }
    let _ = writeln!(xml, "        </root>");
    let _ = writeln!(xml, "        <kind text=\"{root_escaped}{quality_escaped}\">{kind}</kind>");
    let _ = writeln!(xml, "      </harmony>");
}

/// Parse a note name like "C", "F#", "Bb" into (step, alter)
fn parse_note_name(name: &str) -> (&'static str, i8) {
    let chars: Vec<char> = name.chars().collect();
    if chars.is_empty() {
        return ("C", 0);
    }

    let step = match chars[0].to_ascii_uppercase() {
        'C' => "C",
        'D' => "D",
        'E' => "E",
        'F' => "F",
        'G' => "G",
        'A' => "A",
        'B' => "B",
        _ => "C",
    };

    let alter = if chars.len() > 1 {
        match chars[1] {
            '#' => 1,
            'b' => -1,
            _ => 0,
        }
    } else {
        0
    };

    (step, alter)
}

/// Convert chord quality string to MusicXML kind
fn quality_to_musicxml_kind(quality: &str) -> &'static str {
    match quality {
        "" | "maj" | "major" | "M" => "major",
        "m" | "min" | "minor" => "minor",
        "7" | "dom7" => "dominant",
        "maj7" | "M7" => "major-seventh",
        "m7" | "min7" => "minor-seventh",
        "dim" => "diminished",
        "dim7" => "diminished-seventh",
        "m7b5" | "half-dim" => "half-diminished",
        "aug" | "+" => "augmented",
        "sus2" => "suspended-second",
        "sus4" => "suspended-fourth",
        "6" => "major-sixth",
        "m6" => "minor-sixth",
        "9" => "dominant-ninth",
        "add9" => "major-ninth",
        _ => "major",
    }
}

/// Write a pitched note from ScoreNoteEvent
fn write_score_note(
    xml: &mut String,
    pitch: &Pitch,
    duration: &NotationDuration,
    is_chord: bool,
    dynamic: Option<&crate::notation::Dynamic>,
) {
    let step = match pitch.step {
        NoteStep::C => "C",
        NoteStep::D => "D",
        NoteStep::E => "E",
        NoteStep::F => "F",
        NoteStep::G => "G",
        NoteStep::A => "A",
        NoteStep::B => "B",
    };

    let (note_type, dots) = duration_to_musicxml_type(duration);
    let divisions = (duration.to_beats() * 4.0) as usize; // 4 divisions per quarter

    let _ = writeln!(xml, "      <note>");
    if is_chord {
        let _ = writeln!(xml, "        <chord/>");
    }
    let _ = writeln!(xml, "        <pitch>");
    let _ = writeln!(xml, "          <step>{step}</step>");
    if pitch.alter != 0 {
        let _ = writeln!(xml, "          <alter>{}</alter>", pitch.alter);
    }
    let _ = writeln!(xml, "          <octave>{}</octave>", pitch.octave);
    let _ = writeln!(xml, "        </pitch>");
    let _ = writeln!(xml, "        <duration>{divisions}</duration>");
    let _ = writeln!(xml, "        <type>{note_type}</type>");
    for _ in 0..dots {
        let _ = writeln!(xml, "        <dot/>");
    }
    if pitch.alter != 0 {
        let acc = if pitch.alter > 0 { "sharp" } else { "flat" };
        let _ = writeln!(xml, "        <accidental>{acc}</accidental>");
    }
    if let Some(dyn_mark) = dynamic {
        let dyn_str = format!("{dyn_mark:?}").to_lowercase();
        let _ = writeln!(xml, "        <dynamics><{dyn_str}/></dynamics>");
    }
    let _ = writeln!(xml, "      </note>");
}

/// Write a drum note
fn write_score_drum_note(xml: &mut String, duration: &NotationDuration, pitch: Option<&Pitch>) {
    // Use pitch octave to determine drum type, or default to snare
    let (display_step, display_octave) = if let Some(p) = pitch {
        // Map common drum sounds
        match p.octave {
            2 => ("F", 4), // Kick
            3 => ("E", 4), // Snare
            4 => ("G", 5), // Hi-hat
            _ => ("E", 4), // Default snare
        }
    } else {
        ("E", 4) // Default snare
    };

    let (note_type, dots) = duration_to_musicxml_type(duration);
    let divisions = (duration.to_beats() * 4.0) as usize;

    let _ = writeln!(xml, "      <note>");
    let _ = writeln!(xml, "        <unpitched>");
    let _ = writeln!(xml, "          <display-step>{display_step}</display-step>");
    let _ = writeln!(xml, "          <display-octave>{display_octave}</display-octave>");
    let _ = writeln!(xml, "        </unpitched>");
    let _ = writeln!(xml, "        <duration>{divisions}</duration>");
    let _ = writeln!(xml, "        <type>{note_type}</type>");
    for _ in 0..dots {
        let _ = writeln!(xml, "        <dot/>");
    }
    let _ = writeln!(xml, "      </note>");
}

/// Write a rest
fn write_score_rest(xml: &mut String, divisions: usize, divisions_per_quarter: usize) {
    if divisions == 0 {
        return;
    }

    let quarters = divisions as f32 / divisions_per_quarter as f32;
    let (note_type, dots) = quarters_to_musicxml_type(quarters);

    let _ = writeln!(xml, "      <note>");
    let _ = writeln!(xml, "        <rest/>");
    let _ = writeln!(xml, "        <duration>{divisions}</duration>");
    let _ = writeln!(xml, "        <type>{note_type}</type>");
    for _ in 0..dots {
        let _ = writeln!(xml, "        <dot/>");
    }
    let _ = writeln!(xml, "      </note>");
}

/// Convert notation Duration to MusicXML type
fn duration_to_musicxml_type(duration: &NotationDuration) -> (&'static str, usize) {
    let note_type = match duration.base {
        DurationBase::Whole => "whole",
        DurationBase::Half => "half",
        DurationBase::Quarter => "quarter",
        DurationBase::Eighth => "eighth",
        DurationBase::Sixteenth => "16th",
        DurationBase::ThirtySecond => "32nd",
    };
    (note_type, duration.dots)
}

/// Convert duration in quarter notes to MusicXML type
fn quarters_to_musicxml_type(quarters: f32) -> (&'static str, usize) {
    if quarters >= 4.0 {
        ("whole", 0)
    } else if quarters >= 3.0 {
        ("half", 1)
    } else if quarters >= 2.0 {
        ("half", 0)
    } else if quarters >= 1.5 {
        ("quarter", 1)
    } else if quarters >= 1.0 {
        ("quarter", 0)
    } else if quarters >= 0.75 {
        ("eighth", 1)
    } else if quarters >= 0.5 {
        ("eighth", 0)
    } else if quarters >= 0.25 {
        ("16th", 0)
    } else {
        ("32nd", 0)
    }
}

/// Write HarmoniumScore to MusicXML file
pub fn write_score_musicxml(score: &HarmoniumScore, path: &Path) -> std::io::Result<()> {
    let xml = score_to_musicxml(score);
    let mut file = std::fs::File::create(path)?;
    file.write_all(xml.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_to_musicxml_empty_score() {
        let score = HarmoniumScore::default();
        let xml = score_to_musicxml(&score);

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<score-partwise"));
        assert!(xml.contains("</score-partwise>"));
    }

    #[test]
    fn test_score_to_musicxml_with_parts() {
        use crate::notation::{KeyMode as NotationKeyMode, KeySignature, Measure};

        let mut score = HarmoniumScore::default();
        score.tempo = 120.0;
        score.time_signature = (4, 4);
        score.key_signature =
            KeySignature { root: "C".to_string(), mode: NotationKeyMode::Major, fifths: 0 };
        score.parts = vec![
            Part {
                id: "lead".to_string(),
                name: "Lead".to_string(),
                clef: Clef::Treble,
                transposition: None,
                measures: vec![Measure::new(1)],
            },
            Part {
                id: "bass".to_string(),
                name: "Bass".to_string(),
                clef: Clef::Bass,
                transposition: None,
                measures: vec![Measure::new(1)],
            },
        ];

        let xml = score_to_musicxml(&score);

        // Check parts are created
        assert!(xml.contains(r#"<score-part id="P-lead">"#));
        assert!(xml.contains(r#"<score-part id="P-bass">"#));
        assert!(xml.contains("<part-name>Lead</part-name>"));
        assert!(xml.contains("<part-name>Bass</part-name>"));

        // Check clefs
        assert!(xml.contains("<sign>G</sign>")); // Treble
        assert!(xml.contains("<sign>F</sign>")); // Bass
    }

    #[test]
    fn test_score_to_musicxml_with_notes() {
        use crate::notation::{Dynamic, KeyMode as NotationKeyMode, KeySignature, Measure};

        let mut score = HarmoniumScore::default();
        score.key_signature =
            KeySignature { root: "C".to_string(), mode: NotationKeyMode::Major, fifths: 0 };

        // Create a measure with a C4 quarter note
        let mut measure = Measure::new(1);
        measure.events.push(ScoreNoteEvent {
            id: 1,
            beat: 1.0,
            event_type: NoteEventType::Note,
            pitches: vec![Pitch::new(NoteStep::C, 4, 0)],
            duration: NotationDuration::new(DurationBase::Quarter),
            dynamic: Some(Dynamic::MezzoForte),
            articulation: None,
        });

        score.parts = vec![Part {
            id: "lead".to_string(),
            name: "Lead".to_string(),
            clef: Clef::Treble,
            transposition: None,
            measures: vec![measure],
        }];

        let xml = score_to_musicxml(&score);

        assert!(xml.contains("<pitch>"));
        assert!(xml.contains("<step>C</step>"));
        assert!(xml.contains("<octave>4</octave>"));
        assert!(xml.contains("<type>quarter</type>"));
    }

    #[test]
    fn test_score_to_musicxml_with_accidentals() {
        use crate::notation::{KeyMode as NotationKeyMode, KeySignature, Measure};

        let mut score = HarmoniumScore::default();
        score.key_signature =
            KeySignature { root: "C".to_string(), mode: NotationKeyMode::Major, fifths: 0 };

        // Create a measure with a C#4 quarter note
        let mut measure = Measure::new(1);
        measure.events.push(ScoreNoteEvent {
            id: 1,
            beat: 1.0,
            event_type: NoteEventType::Note,
            pitches: vec![Pitch::new(NoteStep::C, 4, 1)], // C#
            duration: NotationDuration::new(DurationBase::Quarter),
            dynamic: None,
            articulation: None,
        });

        score.parts = vec![Part {
            id: "lead".to_string(),
            name: "Lead".to_string(),
            clef: Clef::Treble,
            transposition: None,
            measures: vec![measure],
        }];

        let xml = score_to_musicxml(&score);

        assert!(xml.contains("<step>C</step>"));
        assert!(xml.contains("<alter>1</alter>"));
        assert!(xml.contains("<accidental>sharp</accidental>"));
    }

    #[test]
    fn test_score_to_musicxml_with_chord_symbols() {
        use crate::notation::{KeyMode as NotationKeyMode, KeySignature, Measure};

        let mut score = HarmoniumScore::default();
        score.key_signature =
            KeySignature { root: "C".to_string(), mode: NotationKeyMode::Major, fifths: 0 };

        let mut measure = Measure::new(1);
        measure.chords.push(NotationChordSymbol {
            beat: 1.0,
            duration: 4.0,
            root: "C".to_string(),
            quality: "maj7".to_string(),
            bass: None,
            scale: None,
        });

        score.parts = vec![Part {
            id: "lead".to_string(),
            name: "Lead".to_string(),
            clef: Clef::Treble,
            transposition: None,
            measures: vec![measure],
        }];

        let xml = score_to_musicxml(&score);

        assert!(xml.contains("<harmony>"));
        assert!(xml.contains("<root-step>C</root-step>"));
        assert!(xml.contains("major-seventh")); // maj7 -> major-seventh
    }

    #[test]
    fn test_parse_note_name() {
        assert_eq!(parse_note_name("C"), ("C", 0));
        assert_eq!(parse_note_name("D"), ("D", 0));
        assert_eq!(parse_note_name("F#"), ("F", 1));
        assert_eq!(parse_note_name("Bb"), ("B", -1));
        assert_eq!(parse_note_name("G#"), ("G", 1));
    }

    #[test]
    fn test_quality_to_musicxml_kind() {
        assert_eq!(quality_to_musicxml_kind(""), "major");
        assert_eq!(quality_to_musicxml_kind("m"), "minor");
        assert_eq!(quality_to_musicxml_kind("7"), "dominant");
        assert_eq!(quality_to_musicxml_kind("maj7"), "major-seventh");
        assert_eq!(quality_to_musicxml_kind("m7"), "minor-seventh");
        assert_eq!(quality_to_musicxml_kind("dim"), "diminished");
        assert_eq!(quality_to_musicxml_kind("m7b5"), "half-diminished");
    }
}
