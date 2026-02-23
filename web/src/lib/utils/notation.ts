/**
 * Utility functions for the HarmoniumScore notation format.
 *
 * Provides conversions to VexFlow-compatible strings, beat/step arithmetic,
 * and helpers for determining which notes are active at a given playback position.
 */

import type {
	Duration,
	DurationBase,
	HarmoniumScore,
	Measure,
	Pitch,
	ScoreNoteEvent
} from '$lib/types/notation';

// ─────────────────────────────────────────────────────────
// Pitch utilities
// ─────────────────────────────────────────────────────────

/**
 * Convert a Pitch to a VexFlow key string.
 *
 * @example
 * pitchToVexFlow({ step: 'C', octave: 4 })         // "c/4"
 * pitchToVexFlow({ step: 'F', octave: 5, alter: 1 }) // "f#/5"
 * pitchToVexFlow({ step: 'B', octave: 3, alter: -1 }) // "bb/3"
 */
export function pitchToVexFlow(pitch: Pitch): string {
	const step = pitch.step.toLowerCase();
	const alter = pitch.alter ?? 0;
	const alterStr =
		alter === 2 ? '##' : alter === 1 ? '#' : alter === -1 ? 'b' : alter === -2 ? 'bb' : '';
	return `${step}${alterStr}/${pitch.octave}`;
}

/**
 * Convert a Pitch to a human-readable notation string.
 *
 * @example
 * pitchToString({ step: 'C', octave: 4 })          // "C4"
 * pitchToString({ step: 'F', octave: 5, alter: 1 }) // "F#5"
 * pitchToString({ step: 'B', octave: 3, alter: -1 }) // "Bb3"
 */
export function pitchToString(pitch: Pitch): string {
	const alter = pitch.alter ?? 0;
	const alterStr =
		alter === 2 ? '##' : alter === 1 ? '#' : alter === -1 ? 'b' : alter === -2 ? 'bb' : '';
	return `${pitch.step}${alterStr}${pitch.octave}`;
}

/**
 * Convert a Pitch array to a VexFlow keys array.
 */
export function pitchesToVexFlow(pitches: Pitch[]): string[] {
	return pitches.map(pitchToVexFlow);
}

// ─────────────────────────────────────────────────────────
// Duration utilities
// ─────────────────────────────────────────────────────────

const BASE_BEATS: Record<DurationBase, number> = {
	whole: 4,
	half: 2,
	quarter: 1,
	eighth: 0.5,
	'16th': 0.25,
	'32nd': 0.125
};

const BASE_VEXFLOW: Record<DurationBase, string> = {
	whole: 'w',
	half: 'h',
	quarter: 'q',
	eighth: '8',
	'16th': '16',
	'32nd': '32'
};

/**
 * Convert a Duration to a VexFlow duration string.
 *
 * @example
 * durationToVexFlow({ base: 'quarter' })           // "q"
 * durationToVexFlow({ base: 'half', dots: 1 })     // "hd"
 * durationToVexFlow({ base: 'eighth', dots: 2 })   // "8dd"
 */
export function durationToVexFlow(duration: Duration): string {
	const base = BASE_VEXFLOW[duration.base] ?? 'q';
	const dots = 'd'.repeat(duration.dots ?? 0);
	return `${base}${dots}`;
}

/**
 * Convert a Duration to the number of beats it occupies (quarter = 1.0).
 *
 * @example
 * durationToBeats({ base: 'quarter' })         // 1.0
 * durationToBeats({ base: 'half', dots: 1 })   // 3.0
 * durationToBeats({ base: 'eighth' })          // 0.5
 */
export function durationToBeats(duration: Duration): number {
	const base = BASE_BEATS[duration.base] ?? 1;
	let total = base;
	let add = base / 2;
	for (let i = 0; i < (duration.dots ?? 0); i++) {
		total += add;
		add /= 2;
	}
	if (duration.tuplet) {
		const [num, denom] = duration.tuplet;
		total *= denom / num;
	}
	return total;
}

// ─────────────────────────────────────────────────────────
// Step ↔ beat arithmetic
// ─────────────────────────────────────────────────────────

/** Default steps-per-quarter-note resolution (16th-note grid). */
export const DEFAULT_STEPS_PER_QUARTER = 4;

/**
 * Convert a step index within a measure to a beat position (1-indexed).
 *
 * @param stepInMeasure - Zero-based step index within the measure.
 * @param stepsPerQuarter - Resolution (default: 4 = 16th-note grid).
 */
export function stepToBeat(
	stepInMeasure: number,
	stepsPerQuarter = DEFAULT_STEPS_PER_QUARTER
): number {
	return 1 + stepInMeasure / stepsPerQuarter;
}

/**
 * Convert a beat position (1-indexed) to a step index within a measure.
 *
 * @param beat - 1-indexed beat position.
 * @param stepsPerQuarter - Resolution (default: 4 = 16th-note grid).
 */
export function beatToStep(beat: number, stepsPerQuarter = DEFAULT_STEPS_PER_QUARTER): number {
	return Math.round((beat - 1) * stepsPerQuarter);
}

// ─────────────────────────────────────────────────────────
// Playback highlighting helpers
// ─────────────────────────────────────────────────────────

/**
 * Determine whether a note is sounding at the given beat position.
 *
 * A note is considered active if `noteStart <= beat < noteEnd`.
 */
export function isNoteActiveAtBeat(event: ScoreNoteEvent, beat: number): boolean {
	const noteEnd = event.beat + durationToBeats(event.duration);
	return event.beat <= beat && beat < noteEnd;
}

/**
 * Collect all note IDs that are sounding at a given (measureNumber, beat) position
 * across all parts of a score.
 *
 * This is the primary function for real-time highlighting: call it every RAF tick
 * using the current engine measure/step values, and use the returned Set to update
 * VexFlow note classes.
 *
 * @param score - The HarmoniumScore obtained from `getLookaheadScore()`.
 * @param measureNumber - 1-indexed current measure (from `engineState.currentMeasure`).
 * @param beat - Current beat position (use `stepToBeat(stepInMeasure)`).
 */
export function getActiveNoteIds(
	score: HarmoniumScore,
	measureNumber: number,
	beat: number
): Set<number> {
	const ids = new Set<number>();
	for (const part of score.parts) {
		const measure = findMeasure(part.measures, measureNumber);
		if (!measure) continue;
		for (const event of measure.events) {
			if (isNoteActiveAtBeat(event, beat)) {
				ids.add(event.id);
			}
		}
	}
	return ids;
}

/**
 * Collect all note events sounding at a given (measureNumber, beat) position.
 *
 * Like `getActiveNoteIds` but returns the full event objects.
 */
export function getActiveNoteEvents(
	score: HarmoniumScore,
	measureNumber: number,
	beat: number
): ScoreNoteEvent[] {
	const events: ScoreNoteEvent[] = [];
	for (const part of score.parts) {
		const measure = findMeasure(part.measures, measureNumber);
		if (!measure) continue;
		for (const event of measure.events) {
			if (isNoteActiveAtBeat(event, beat)) {
				events.push(event);
			}
		}
	}
	return events;
}

/**
 * Find a measure by its 1-indexed number in a measure array.
 */
export function findMeasure(measures: Measure[], number: number): Measure | undefined {
	return measures.find((m) => m.number === number);
}

// ─────────────────────────────────────────────────────────
// Score parsing
// ─────────────────────────────────────────────────────────

/**
 * Parse a HarmoniumScore JSON string from the engine.
 * Returns null if the string is empty or invalid.
 */
export function parseScore(json: string): HarmoniumScore | null {
	if (!json || json === '{}') return null;
	try {
		return JSON.parse(json) as HarmoniumScore;
	} catch (e) {
		console.error('[notation] Failed to parse score JSON:', e);
		return null;
	}
}

/**
 * Get the total number of measures in a score (across first part).
 */
export function scoreMeasureCount(score: HarmoniumScore): number {
	return score.parts[0]?.measures.length ?? 0;
}
