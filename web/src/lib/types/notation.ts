/**
 * TypeScript type definitions for HarmoniumScore notation format.
 *
 * These types mirror the Rust types in `harmonium_core/src/notation.rs`.
 * They enable type-safe access to the JSON produced by `get_lookahead_score()`.
 *
 * Sync architecture:
 * - Each `ScoreNoteEvent.id` matches the `AudioEvent.NoteOn.id` generated at the same time.
 * - The frontend can derive currently-playing note IDs from the current engine step
 *   and use them to highlight matching notes in VexFlow.
 */

// ─────────────────────────────────────────────────────────
// Root score
// ─────────────────────────────────────────────────────────

export interface HarmoniumScore {
	version: '1.0';
	title?: string;
	/** Tempo in BPM */
	tempo: number;
	/** [numerator, denominator] e.g. [4, 4] */
	time_signature: [number, number];
	key_signature: KeySignature;
	/** One entry per instrument (lead, bass, drums) */
	parts: Part[];
}

// ─────────────────────────────────────────────────────────
// Key signature
// ─────────────────────────────────────────────────────────

export interface KeySignature {
	/** Root note name, e.g. "C", "F#", "Bb" */
	root: string;
	mode: KeyMode;
	/** Circle-of-fifths position: positive = sharps, negative = flats */
	fifths: number;
}

export type KeyMode = 'major' | 'minor';

// ─────────────────────────────────────────────────────────
// Instrument part
// ─────────────────────────────────────────────────────────

export interface Part {
	/** "lead" | "bass" | "drums" */
	id: string;
	name: string;
	clef: Clef;
	transposition?: Transposition;
	measures: Measure[];
}

export type Clef = 'treble' | 'bass' | 'percussion';

// ─────────────────────────────────────────────────────────
// Transposition
// ─────────────────────────────────────────────────────────

export interface Transposition {
	interval: TransposeInterval;
	/** -1, 0, or +1 */
	octave_shift?: number;
}

export type TransposeInterval = 'P1' | 'M2' | 'm3' | 'P4' | 'P5';

// ─────────────────────────────────────────────────────────
// Measure
// ─────────────────────────────────────────────────────────

export interface Measure {
	/** 1-indexed */
	number: number;
	time_signature?: [number, number];
	key_signature?: KeySignature;
	events: ScoreNoteEvent[];
	chords?: ChordSymbol[];
}

// ─────────────────────────────────────────────────────────
// Note event (with sync ID)
// ─────────────────────────────────────────────────────────

/**
 * A note event in the score.
 *
 * The `id` field is shared with the corresponding `AudioEvent.NoteOn.id`,
 * enabling real-time highlighting: when the audio plays a note with id=42,
 * the frontend highlights the ScoreNoteEvent with id=42.
 */
export interface ScoreNoteEvent {
	/** Unique note ID — matches the AudioEvent.NoteOn id */
	id: number;
	/** Position in measure (1-indexed beat), e.g. 1.0, 1.5, 2.0 */
	beat: number;
	type: NoteEventType;
	pitches?: Pitch[];
	duration: Duration;
	dynamic?: Dynamic;
	articulation?: Articulation;
}

export type NoteEventType = 'note' | 'rest' | 'chord' | 'drum';

// ─────────────────────────────────────────────────────────
// Pitch
// ─────────────────────────────────────────────────────────

export interface Pitch {
	step: NoteStep;
	/** 0–9, middle C = 4 */
	octave: number;
	/** -2 = double-flat, -1 = flat, 0 = natural, 1 = sharp, 2 = double-sharp */
	alter?: number;
}

export type NoteStep = 'C' | 'D' | 'E' | 'F' | 'G' | 'A' | 'B';

// ─────────────────────────────────────────────────────────
// Duration
// ─────────────────────────────────────────────────────────

export interface Duration {
	base: DurationBase;
	/** Number of augmentation dots (0, 1, or 2) */
	dots?: number;
	/** Tuplet ratio [num, denom], e.g. [3, 2] = triplet */
	tuplet?: [number, number];
}

export type DurationBase = 'whole' | 'half' | 'quarter' | 'eighth' | '16th' | '32nd';

// ─────────────────────────────────────────────────────────
// Dynamics & articulations
// ─────────────────────────────────────────────────────────

export type Dynamic = 'ppp' | 'pp' | 'p' | 'mp' | 'mf' | 'f' | 'ff' | 'fff';

export type Articulation = 'staccato' | 'accent' | 'tenuto' | 'marcato';

// ─────────────────────────────────────────────────────────
// Drum symbols
// ─────────────────────────────────────────────────────────

export type DrumSymbol = 'K' | 'S' | 'H' | 'Ho' | 'R' | 'C' | 'T1' | 'T2' | 'T3';

// ─────────────────────────────────────────────────────────
// Chord symbols
// ─────────────────────────────────────────────────────────

export interface ChordSymbol {
	beat: number;
	/** Duration in beats */
	duration: number;
	root: string;
	quality: string;
	/** Slash-chord bass note, e.g. "E" in "C/E" */
	bass?: string;
	scale?: ScaleSuggestion;
}

export interface ScaleSuggestion {
	name: string;
	degrees: string[];
	chordTones: string[];
}
