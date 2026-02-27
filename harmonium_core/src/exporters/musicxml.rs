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


use crate::notation::{
        ChordSymbol as NotationChordSymbol, Clef, Duration as NotationDuration, DurationBase,
        HarmoniumScore, NoteEventType, NoteStep, Part, Pitch, ScoreNoteEvent,
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

// ═══════════════════════════════════════════════════════════════════
// HARMONIUM SCORE TO MUSICXML
// ═══════════════════════════════════════════════════════════════════

/// Convert a HarmoniumScore to MusicXML string
///
/// This is the primary export function for VexFlow-compatible score data.
/// It generates a complete MusicXML 4.0 document that can be imported into
/// notation software like MuseScore, Finale, or Sibelius.
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
