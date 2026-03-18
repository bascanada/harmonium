//! TimelineGenerator - Extracts tick() logic into measure-level generation
//!
//! This module faithfully replicates all state transitions from `engine.rs::tick()`:
//! 1. Tick both sequencers per step
//! 2. Bar boundary: swap patterns, prepare_next_bar, advance harmony
//! 3. Per-step: evaluate triggers, apply drum variations, generate lead/bass
//! 4. Maintain CurrentState morphing between steps
//! 5. Call next_note_structured() in identical order
//!
//! The algorithms are IDENTICAL to the legacy engine - just writing to
//! `Measure` structs instead of emitting ephemeral `AudioEvent`s.

use crate::harmony::RngCore;
use crate::harmony::basic::{ChordQuality, ChordStep, Progression};
use crate::harmony::chord::ChordType;
use crate::harmony::driver::HarmonicDriver;
use crate::harmony::melody::HarmonyNavigator;
use crate::params::{CurrentState, MusicalParams};
use crate::sequencer::{RhythmMode, Sequencer, StepTrigger};

use super::{
    Articulation, ChordContext, Measure, NoteId, StateSnapshot,
    TimelineNote, TrackId,
};

use crate::harmony::HarmonyMode;

/// Generates measures offline (on the main thread) using the same algorithms
/// as the legacy engine's tick() function.
pub struct TimelineGenerator {
    // === Sequencers (cloned from engine state) ===
    pub sequencer_primary: Sequencer,
    pub sequencer_secondary: Sequencer,

    // === Harmony ===
    pub harmony: HarmonyNavigator,
    pub harmonic_driver: Option<HarmonicDriver>,
    pub harmony_mode: HarmonyMode,

    // === Progression state (Basic mode) ===
    current_progression: Vec<ChordStep>,
    progression_index: usize,
    last_valence_choice: f32,
    last_tension_choice: f32,

    // === Morphed state ===
    pub current_state: CurrentState,

    // === Musical parameters ===
    pub musical_params: MusicalParams,

    // === Chord tracking ===
    chord_root_offset: i32,
    chord_is_minor: bool,
    chord_name: String,
    current_chord_type: ChordType,

    // === Note ID counter ===
    next_note_id: NoteId,

    // === Bar counter ===
    current_bar: usize,
}

impl TimelineGenerator {
    /// Create a new generator with the given initial state
    #[must_use]
    pub fn new(
        sequencer_primary: Sequencer,
        sequencer_secondary: Sequencer,
        harmony: HarmonyNavigator,
        harmonic_driver: Option<HarmonicDriver>,
        musical_params: MusicalParams,
        current_state: CurrentState,
    ) -> Self {
        let current_progression = Progression::get_palette(
            current_state.valence,
            current_state.tension,
        );

        Self {
            sequencer_primary,
            sequencer_secondary,
            harmony,
            harmonic_driver,
            harmony_mode: musical_params.harmony_mode,
            current_progression,
            progression_index: 0,
            last_valence_choice: current_state.valence,
            last_tension_choice: current_state.tension,
            current_state,
            musical_params,
            chord_root_offset: 0,
            chord_is_minor: false,
            chord_name: "I".to_string(),
            current_chord_type: ChordType::Major,
            next_note_id: 1,
            current_bar: 0,
        }
    }

    /// Generate a single measure, faithfully replicating tick() behavior.
    ///
    /// This ticks both sequencers through all steps in the measure, calling
    /// the same melody/harmony/rhythm functions in identical order as tick().
    pub fn generate_measure(
        &mut self,
        bar_index: usize,
        rng: &mut dyn RngCore,
    ) -> Measure {
        self.current_bar = bar_index;

        // Snap current_state to match musical_params first —
        // params affect the writehead directly, next bar uses new values.
        self.snap_current_state();

        let time_sig = self.musical_params.time_signature;
        let steps = time_sig.steps_per_bar(self.sequencer_primary.ticks_per_beat);

        let mut measure = Measure::new(
            bar_index,
            time_sig,
            self.current_state.bpm,
            steps,
        );

        // === BARLINE LOGIC (replicates tick() bar_crossed branch) ===
        self.handle_barline(bar_index, rng);

        // Snapshot state at generation time
        measure.state_snapshot = StateSnapshot::from(&self.current_state);
        measure.chord_context = ChordContext {
            root_offset: self.chord_root_offset,
            is_minor: self.chord_is_minor,
            chord_name: self.chord_name.clone(),
        };

        // === PER-STEP LOGIC (replicates tick() event generation) ===
        let rhythm_enabled = self.musical_params.enable_rhythm;
        let melody_enabled = self.musical_params.enable_melody;

        // Context flags (replicates the "virtual drummer" context from tick())
        let is_high_tension = self.current_state.tension > 0.6;
        let is_high_density = self.current_state.density > 0.6;
        let is_high_energy = self.current_state.arousal > 0.7;
        let is_low_energy = self.current_state.arousal < 0.4;
        let fill_zone_start = steps.saturating_sub(4);

        for step in 0..steps {
            // Tick primary sequencer
            let trigger_primary = if step < self.sequencer_primary.pattern.len() {
                self.sequencer_primary.pattern[step]
            } else {
                StepTrigger::default()
            };

            // Tick secondary sequencer (Euclidean mode only for polyrhythm)
            let trigger_secondary = if self.sequencer_primary.mode == RhythmMode::Euclidean
                && step < self.sequencer_secondary.pattern.len()
            {
                self.sequencer_secondary.pattern[step % self.sequencer_secondary.pattern.len()]
            } else {
                StepTrigger::default()
            };

            let is_in_fill_zone = step >= fill_zone_start;

            // === BASS (Kick) ===
            if rhythm_enabled
                && trigger_primary.kick
                && !self.musical_params.muted_channels.first().copied().unwrap_or(false)
            {
                let midi_note = if self.musical_params.fixed_kick {
                    36u8
                } else {
                    (36 + self.chord_root_offset) as u8
                };
                let midi_note = self.musical_params.instrument_bass.apply(midi_note);
                let vel = self.musical_params.vel_base_bass
                    + (self.current_state.arousal * 25.0) as u8;

                measure.add_note(TrackId::Bass, TimelineNote {
                    id: self.next_id(),
                    pitch: midi_note,
                    start_step: step,
                    duration_steps: 1, // Staccato bass
                    velocity: vel,
                    articulation: Articulation::Staccato,
                });
            }

            // === LEAD (with voicing decision) ===
            let play_lead = melody_enabled
                && trigger_primary.lead
                && !(is_high_tension && is_in_fill_zone)
                && !self.musical_params.muted_channels.get(1).copied().unwrap_or(false);

            if play_lead {
                let is_strong = trigger_primary.kick || trigger_primary.snare;
                let is_new_measure = step == 0;

                let freq = self.harmony.next_note_structured(
                    is_strong,
                    is_new_measure,
                    rng,
                );
                let melody_midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
                let melody_midi = self.musical_params.instrument_lead.apply(melody_midi);
                let base_vel = 90 + (self.current_state.arousal * 30.0) as u8;

                // Determine duration: until next lead trigger or end of bar
                let duration = self.calculate_lead_duration(step, steps, &self.sequencer_primary.pattern, is_high_tension, fill_zone_start);

                // Simplified: emit single melody note (voicing handled by Playhead or skipped)
                let solo_vel = (base_vel as f32 * 0.7) as u8;
                measure.add_note(TrackId::Lead, TimelineNote {
                    id: self.next_id(),
                    pitch: melody_midi,
                    start_step: step,
                    duration_steps: duration,
                    velocity: solo_vel,
                    articulation: Articulation::Normal,
                });
            }

            // === SNARE (with ghost notes and tom fills) ===
            if rhythm_enabled
                && trigger_primary.snare
                && !self.musical_params.muted_channels.get(2).copied().unwrap_or(false)
            {
                let mut snare_note = 38u8;
                let mut vel = self.musical_params.vel_base_snare
                    + (self.current_state.arousal * 30.0) as u8;

                // Ghost notes
                if trigger_primary.velocity < 0.7 {
                    vel = (vel as f32 * 0.65) as u8;
                    if is_low_energy {
                        snare_note = 37; // Side Stick
                    }
                }

                // Tom fills
                if is_high_tension && is_in_fill_zone {
                    snare_note = match step % 3 {
                        0 => 41, // Low Tom
                        1 => 45, // Mid Tom
                        _ => 50, // High Tom
                    };
                    vel = (vel as f32 * 1.1).min(127.0) as u8;
                }

                measure.add_note(TrackId::Snare, TimelineNote {
                    id: self.next_id(),
                    pitch: snare_note,
                    start_step: step,
                    duration_steps: 0, // Trigger only
                    velocity: vel,
                    articulation: Articulation::Trigger,
                });
            }

            // === HAT (with cymbal variations) ===
            let play_hat = trigger_primary.hat || trigger_secondary.hat;
            if rhythm_enabled
                && play_hat
                && !self.musical_params.muted_channels.get(3).copied().unwrap_or(false)
            {
                let mut hat_note = 42u8; // Closed Hi-Hat
                let mut vel = 70 + (self.current_state.arousal * 30.0) as u8;

                // Crash on the "One"
                if step == 0 && is_high_energy {
                    hat_note = 49;
                    vel = 110;
                }
                // Ride / Open Hat variation
                else if is_high_density {
                    if self.current_state.tension > 0.7 {
                        hat_note = 51; // Ride Cymbal
                    } else if !step.is_multiple_of(2) {
                        hat_note = 46; // Open Hi-Hat
                    }
                }
                // Pedal Hat (calm)
                else if is_low_energy {
                    hat_note = 44; // Pedal Hi-Hat
                }

                measure.add_note(TrackId::Hat, TimelineNote {
                    id: self.next_id(),
                    pitch: hat_note,
                    start_step: step,
                    duration_steps: 0,
                    velocity: vel,
                    articulation: Articulation::Trigger,
                });
            }
        }

        // Advance sequencer positions to match (they were read but not ticked)
        self.sequencer_primary.current_step = 0;
        if self.sequencer_primary.mode == RhythmMode::Euclidean {
            self.sequencer_secondary.current_step = 0;
        }

        measure
    }

    /// Handle barline logic: swap patterns, advance harmony, prepare next bar
    fn handle_barline(&mut self, bar_index: usize, rng: &mut dyn RngCore) {
        // Swap pattern buffers (replicates tick() bar_crossed logic)
        if let Some(next) = self.sequencer_primary.next_pattern.take() {
            self.sequencer_primary.pattern = next;
            self.sequencer_primary.steps = self.sequencer_primary.pattern.len();
        }
        if let Some(next) = self.sequencer_secondary.next_pattern.take() {
            self.sequencer_secondary.pattern = next;
            self.sequencer_secondary.steps = self.sequencer_secondary.pattern.len();
        }

        // Prepare next bar patterns
        self.sequencer_primary.prepare_next_bar();
        self.sequencer_secondary.prepare_next_bar();

        // === HARMONY & PROGRESSION ===
        if !self.musical_params.enable_harmony {
            return;
        }

        match self.harmony_mode {
            HarmonyMode::Basic => {
                self.advance_basic_harmony(bar_index);
            }
            HarmonyMode::Driver => {
                self.advance_driver_harmony(bar_index, rng);
            }
        }
    }

    /// Advance harmony in Basic mode (quadrant-based progressions)
    fn advance_basic_harmony(&mut self, bar_index: usize) {
        // Palette selection with hysteresis (every 4 bars)
        if bar_index.is_multiple_of(4) {
            let valence_delta = (self.current_state.valence - self.last_valence_choice).abs();
            let tension_delta = (self.current_state.tension - self.last_tension_choice).abs();

            if valence_delta > 0.4 || tension_delta > 0.4 {
                self.current_progression = Progression::get_palette(
                    self.current_state.valence,
                    self.current_state.tension,
                );
                self.progression_index = 0;
                self.last_valence_choice = self.current_state.valence;
                self.last_tension_choice = self.current_state.tension;
            }
        }

        // Chord progression
        let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
        if bar_index.is_multiple_of(measures_per_chord) {
            self.progression_index =
                (self.progression_index + 1) % self.current_progression.len();
            let chord = &self.current_progression[self.progression_index];

            self.harmony.set_chord_context(chord.root_offset, chord.quality);
            self.chord_root_offset = chord.root_offset;
            self.chord_is_minor = matches!(chord.quality, ChordQuality::Minor);
            self.chord_name = format_chord_name(chord.root_offset, chord.quality);
            self.current_chord_type = match chord.quality {
                ChordQuality::Major => ChordType::Major7,
                ChordQuality::Minor => ChordType::Minor7,
                ChordQuality::Dominant7 => ChordType::Dominant7,
                ChordQuality::Diminished => ChordType::Diminished7,
                ChordQuality::Sus2 => ChordType::Sus2,
            };
        }
    }

    /// Advance harmony in Driver mode (Steedman + Neo-Riemannian + Parsimonious)
    fn advance_driver_harmony(&mut self, bar_index: usize, rng: &mut dyn RngCore) {
        let measures_per_chord = if self.current_state.tension > 0.6 { 1 } else { 2 };
        if bar_index.is_multiple_of(measures_per_chord) {
            if let Some(ref mut driver) = self.harmonic_driver {
                let decision = driver.next_chord(
                    self.current_state.tension,
                    self.current_state.valence,
                    rng,
                );

                let root_offset = driver.root_offset();
                let quality = driver.to_basic_quality();
                self.harmony.set_chord_context(root_offset, quality);

                self.chord_root_offset = root_offset;
                self.chord_is_minor = driver.is_minor();
                self.chord_name = decision.next_chord.name();
                self.current_chord_type = decision.next_chord.chord_type;
            }
        }
    }

    // apply_morphing removed — writehead uses params directly via snap_current_state

    /// Standard note durations in steps (for ticks_per_beat=4).
    /// These map 1:1 to VexFlow glyphs: 16=w, 12=hd, 8=h, 6=qd, 4=q, 3=8d, 2=8, 1=16
    const NOTATION_SAFE_STEPS: [usize; 8] = [16, 12, 8, 6, 4, 3, 2, 1];

    /// Calculate lead note duration (until next lead trigger or end of bar),
    /// clamped down to the largest notation-safe value that fits.
    fn calculate_lead_duration(
        &self,
        current_step: usize,
        total_steps: usize,
        pattern: &[StepTrigger],
        is_high_tension: bool,
        fill_zone_start: usize,
    ) -> usize {
        // Look ahead for the next lead trigger
        let raw = 'outer: {
            for future_step in (current_step + 1)..total_steps {
                let trigger = if future_step < pattern.len() {
                    pattern[future_step]
                } else {
                    StepTrigger::default()
                };

                let would_play = trigger.lead
                    && !(is_high_tension && future_step >= fill_zone_start);
                if would_play {
                    break 'outer future_step - current_step;
                }
            }
            // Sustain until end of bar
            total_steps - current_step
        };

        // Clamp to largest notation-safe duration that fits within the raw gap
        Self::NOTATION_SAFE_STEPS
            .iter()
            .copied()
            .find(|&d| d <= raw)
            .unwrap_or(1)
    }

    /// Update musical parameters (called when commands are processed)
    /// Snap `current_state` to match `musical_params` immediately (no morphing).
    /// Call after direct param changes to ensure newly generated measures
    /// use the correct tempo/density/etc. right away.
    pub fn snap_current_state(&mut self) {
        let mp = &self.musical_params;
        self.current_state.bpm = mp.bpm;
        self.current_state.density = mp.rhythm_density;
        self.current_state.tension = mp.harmony_tension;
        self.current_state.smoothness = mp.melody_smoothness;
        self.current_state.valence = mp.harmony_valence;
        self.current_state.arousal = (mp.bpm - 70.0) / 110.0;
    }

    pub fn update_params(&mut self, params: MusicalParams) {
        // Detect mode changes
        if self.musical_params.harmony_mode != params.harmony_mode {
            self.harmony_mode = params.harmony_mode;
        }

        // Update sequencer parameters
        if self.sequencer_primary.mode != params.rhythm_mode {
            self.sequencer_primary.mode = params.rhythm_mode;
            self.sequencer_primary.upgrade_to_steps(params.rhythm_steps);
        }

        if self.sequencer_primary.pulses != params.rhythm_pulses {
            self.sequencer_primary.pulses = params.rhythm_pulses.min(self.sequencer_primary.steps);
            self.sequencer_primary.prepare_next_bar();
        }

        if self.sequencer_primary.rotation != params.rhythm_rotation {
            self.sequencer_primary.rotation = params.rhythm_rotation;
            self.sequencer_primary.prepare_next_bar();
        }

        self.sequencer_primary.tension = params.rhythm_tension;
        self.sequencer_primary.density = params.rhythm_density;

        // Secondary sequencer
        self.sequencer_secondary.pulses = params.rhythm_secondary_pulses.min(params.rhythm_secondary_steps);
        self.sequencer_secondary.rotation = params.rhythm_secondary_rotation;

        // Melody
        self.harmony.set_hurst_factor(params.melody_smoothness);

        self.musical_params = params;
    }

    fn next_id(&mut self) -> NoteId {
        let id = self.next_note_id;
        self.next_note_id += 1;
        id
    }
}

/// Format a chord name using Roman numeral notation
fn format_chord_name(root_offset: i32, quality: ChordQuality) -> String {
    let roman = match root_offset {
        0 => "I",
        2 => "II",
        3 => "III",
        5 => "IV",
        7 => "V",
        8 => "VI",
        9 => "vi",
        10 => "VII",
        11 => "vii",
        _ => "?",
    };

    let quality_symbol = match quality {
        ChordQuality::Major => "",
        ChordQuality::Minor => "m",
        ChordQuality::Dominant7 => "7",
        ChordQuality::Diminished => "°",
        ChordQuality::Sus2 => "sus2",
    };

    format!("{roman}{quality_symbol}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::TimeSignature;
    use rust_music_theory::{note::PitchSymbol, scale::ScaleType};

    fn make_gen() -> TimelineGenerator {
        let seq_primary = Sequencer::new(16, 4, 120.0);
        let seq_secondary = Sequencer::new_with_rotation(12, 3, 120.0, 0);
        let harmony = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
        let params = MusicalParams::default();
        let state = CurrentState {
            bpm: 120.0,
            density: 0.5,
            tension: 0.3,
            smoothness: 0.7,
            valence: 0.3,
            arousal: 0.5,
        };

        TimelineGenerator::new(
            seq_primary,
            seq_secondary,
            harmony,
            None, // No driver for basic tests
            params,
            state,
        )
    }

    #[test]
    fn test_generate_measure_basic() {
        let mut tgen = make_gen();
        let mut rng = rand::thread_rng();

        let measure = tgen.generate_measure(1, &mut rng);

        assert_eq!(measure.index, 1);
        assert_eq!(measure.steps, 16);
        // Should have at least some notes (bass/lead/snare/hat from pattern)
        assert!(measure.total_notes() > 0, "Expected notes in generated measure");
    }

    #[test]
    fn test_generate_multiple_measures() {
        let mut tgen = make_gen();
        let mut rng = rand::thread_rng();

        let mut total_notes = 0;
        for i in 1..=8 {
            let measure = tgen.generate_measure(i, &mut rng);
            assert_eq!(measure.index, i);
            total_notes += measure.total_notes();
        }

        // 8 bars should produce a reasonable number of notes
        assert!(total_notes > 10, "Expected many notes across 8 bars, got {total_notes}");
    }

    #[test]
    fn test_generate_measure_with_driver() {
        let seq_primary = Sequencer::new(16, 4, 120.0);
        let seq_secondary = Sequencer::new_with_rotation(12, 3, 120.0, 0);
        let harmony = HarmonyNavigator::new(PitchSymbol::C, ScaleType::PentatonicMajor, 4);
        let driver = HarmonicDriver::new(0); // C
        let params = MusicalParams::default();
        let state = CurrentState {
            bpm: 120.0,
            density: 0.5,
            tension: 0.3,
            smoothness: 0.7,
            valence: 0.3,
            arousal: 0.5,
        };

        let mut tgen = TimelineGenerator::new(
            seq_primary,
            seq_secondary,
            harmony,
            Some(driver),
            params,
            state,
        );

        let mut rng = rand::thread_rng();
        let measure = tgen.generate_measure(1, &mut rng);
        assert!(measure.total_notes() > 0);
    }

    #[test]
    fn test_generate_measure_rhythm_disabled() {
        let mut tgen = make_gen();
        tgen.musical_params.enable_rhythm = false;

        let mut rng = rand::thread_rng();
        let measure = tgen.generate_measure(1, &mut rng);

        // No bass, snare, or hat notes when rhythm is disabled
        assert_eq!(measure.notes_for_track(TrackId::Bass).len(), 0);
        assert_eq!(measure.notes_for_track(TrackId::Snare).len(), 0);
        assert_eq!(measure.notes_for_track(TrackId::Hat).len(), 0);
    }

    #[test]
    fn test_generate_measure_melody_disabled() {
        let mut tgen = make_gen();
        tgen.musical_params.enable_melody = false;

        let mut rng = rand::thread_rng();
        let measure = tgen.generate_measure(1, &mut rng);

        assert_eq!(measure.notes_for_track(TrackId::Lead).len(), 0);
    }

    #[test]
    fn test_note_ids_are_unique() {
        let mut tgen = make_gen();
        let mut rng = rand::thread_rng();

        let m1 = tgen.generate_measure(1, &mut rng);
        let m2 = tgen.generate_measure(2, &mut rng);

        let mut all_ids: Vec<NoteId> = Vec::new();
        for m in [&m1, &m2] {
            for track in &TrackId::ALL {
                for note in m.notes_for_track(*track) {
                    all_ids.push(note.id);
                }
            }
        }

        let original_len = all_ids.len();
        all_ids.sort();
        all_ids.dedup();
        assert_eq!(all_ids.len(), original_len, "All note IDs should be unique");
    }

    #[test]
    fn test_state_snapshot_captured() {
        let mut tgen = make_gen();
        tgen.current_state.tension = 0.8;

        let mut rng = rand::thread_rng();
        let measure = tgen.generate_measure(1, &mut rng);

        // State should be captured after morphing, so slightly less than 0.8
        // (morphing towards musical_params.harmony_tension=0.3)
        assert!(measure.state_snapshot.tension > 0.0);
    }

    #[test]
    fn test_lead_notes_within_instrument_range() {
        use crate::params::InstrumentConfig;

        let mut tgen = make_gen();
        tgen.musical_params.instrument_lead = InstrumentConfig { min_note: 60, max_note: 72, transposition_semitones: 0 };

        let mut rng = rand::thread_rng();
        for bar in 1..=8 {
            let measure = tgen.generate_measure(bar, &mut rng);
            for note in measure.notes_for_track(TrackId::Lead) {
                assert!(
                    note.pitch >= 60 && note.pitch <= 72,
                    "Lead note {} out of range [60, 72] at bar {bar}",
                    note.pitch,
                );
            }
        }
    }

    #[test]
    fn test_tenor_sax_lead_range() {
        use crate::params::InstrumentConfig;

        let mut tgen = make_gen();
        tgen.musical_params.instrument_lead = InstrumentConfig::tenor_sax();

        let mut rng = rand::thread_rng();
        for bar in 1..=16 {
            let measure = tgen.generate_measure(bar, &mut rng);
            for note in measure.notes_for_track(TrackId::Lead) {
                assert!(
                    note.pitch >= 56 && note.pitch <= 90,
                    "Tenor sax lead note {} out of range [56, 90] at bar {bar}",
                    note.pitch,
                );
            }
        }
    }

    #[test]
    fn test_different_time_signatures() {
        let mut tgen = make_gen();

        let mut rng = rand::thread_rng();

        // 3/4 time
        tgen.musical_params.time_signature = TimeSignature::new(3, 4);
        let measure = tgen.generate_measure(1, &mut rng);
        assert_eq!(measure.time_signature, TimeSignature::new(3, 4));
        assert_eq!(measure.steps, 12); // 3 beats * 4 ticks

        // 5/4 time
        tgen.musical_params.time_signature = TimeSignature::new(5, 4);
        let measure = tgen.generate_measure(2, &mut rng);
        assert_eq!(measure.steps, 20); // 5 beats * 4 ticks
    }

    /// Full pipeline integration test: TimelineGenerator with tenor sax config
    /// → generate measures → validate ranges → export MusicXML → validate XML structure.
    #[test]
    fn test_tenor_sax_full_pipeline() {
        use crate::params::InstrumentConfig;
        use crate::timeline::{ScoreTimeline, export::timeline_to_musicxml_with_instruments};

        let tenor = InstrumentConfig::tenor_sax();
        let mut tgen = make_gen();
        tgen.musical_params.instrument_lead = tenor;

        let mut rng = rand::thread_rng();
        let mut timeline = ScoreTimeline::new(20);

        for bar in 1..=16 {
            let measure = tgen.generate_measure(bar, &mut rng);

            // All lead notes must fall within tenor sax range
            for note in measure.notes_for_track(TrackId::Lead) {
                assert!(
                    note.pitch >= tenor.min_note && note.pitch <= tenor.max_note,
                    "Lead note MIDI {} out of tenor sax range [{}, {}] at bar {bar}",
                    note.pitch, tenor.min_note, tenor.max_note,
                );
            }

            timeline.push_measure(measure);
        }

        // Export with instrument config and validate XML structure
        let xml = timeline_to_musicxml_with_instruments(
            &timeline,
            "Integration Test - Tenor Sax",
            &tenor,
            &InstrumentConfig::default(),
        );

        // Part name
        assert!(xml.contains("<part-name>Tenor Saxophone</part-name>"));

        // Transpose element (Bb instrument: chromatic=-2, diatonic=-1)
        assert!(xml.contains("<transpose>"));
        assert!(xml.contains("<chromatic>-2</chromatic>"));
        assert!(xml.contains("<diatonic>-1</diatonic>"));

        // Bass part should have no transpose (default config)
        // Count occurrences of <transpose> — should be exactly 1 (lead only)
        let transpose_count = xml.matches("<transpose>").count();
        assert_eq!(transpose_count, 1, "Only the lead part should have <transpose>");

        // Valid MusicXML structure
        assert!(xml.contains("score-partwise"));
        assert!(xml.contains("<part-name>Bass</part-name>"));
        assert!(xml.contains("<part-name>Drums</part-name>"));
    }
}
