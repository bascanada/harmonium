/**
 * Svelte stores for real-time score playback highlighting.
 *
 * Architecture:
 * 1. Call `loadScore(bridge, bars)` once after the engine connects to fetch a
 *    pre-generated HarmoniumScore for the requested number of bars.
 * 2. The `playingNoteIds` store is automatically derived from `engineState`
 *    (current measure + step) and the loaded score — no additional polling required.
 * 3. In VexFlow components, use `$playingNoteIds.has(event.id)` to highlight notes.
 *
 * Synchronisation note:
 * The lookahead score is a *simulation* of the engine's near future.  The note IDs
 * it contains were assigned by the same atomic counter used for the real audio events,
 * so IDs are globally unique but the lookahead score must be re-fetched whenever
 * musical parameters change significantly (key, tempo, rhythm pattern).
 */

import { writable, derived } from 'svelte/store';
import type { Readable } from 'svelte/store';
import type { HarmoniumScore } from '$lib/types/notation';
import type { HarmoniumBridge } from '$lib/bridge';
import { parseScore, getActiveNoteIds, stepToBeat, DEFAULT_STEPS_PER_QUARTER } from '$lib/utils/notation';
import { engineState } from '$lib/stores/engine-state';

// ─────────────────────────────────────────────────────────
// Core stores
// ─────────────────────────────────────────────────────────

/**
 * The currently loaded HarmoniumScore.
 * Set via `loadScore()` or `setCurrentScore()`.
 */
export const currentScore = writable<HarmoniumScore | null>(null);

/**
 * Whether a score fetch is in progress.
 */
export const scoreLoading = writable(false);

/**
 * Error message from the last failed score fetch, or null.
 */
export const scoreError = writable<string | null>(null);

// ─────────────────────────────────────────────────────────
// Derived: currently playing note IDs
// ─────────────────────────────────────────────────────────

/**
 * Set of note IDs that are currently sounding, derived from the engine's
 * current position and the loaded score.
 *
 * Update frequency: re-evaluated whenever `engineState` or `currentScore` changes.
 * The engine state is polled at ~60 fps by the WASM bridge's RAF loop.
 */
export const playingNoteIds: Readable<Set<number>> = derived(
	[engineState, currentScore],
	([$state, $score]) => {
		if (!$score) return new Set<number>();

		// currentStep is the 0-based step within the current measure.
		// primarySteps is the total steps per measure (determines the sequencer pattern length).
		const stepInMeasure = $state.currentStep;
		const beat = stepToBeat(stepInMeasure, DEFAULT_STEPS_PER_QUARTER);

		return getActiveNoteIds($score, $state.currentMeasure, beat);
	}
);

// ─────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────

/**
 * Returns a derived store that is `true` when a specific note is playing.
 *
 * @example
 * const active = isNoteActive(event.id);
 * // in template: class:highlighted={$active}
 */
export function isNoteActive(noteId: number): Readable<boolean> {
	return derived(playingNoteIds, ($ids) => $ids.has(noteId));
}

/**
 * Fetch a HarmoniumScore from the engine for the next `bars` measures,
 * parse it, and store it in `currentScore`.
 *
 * Call this once after the bridge connects, and again whenever the musical
 * context changes substantially (e.g. key change, rhythm update).
 *
 * @param bridge  - Connected HarmoniumBridge instance.
 * @param bars    - Number of bars to generate (default: 8).
 */
export async function loadScore(bridge: HarmoniumBridge, bars = 8): Promise<void> {
	scoreLoading.set(true);
	scoreError.set(null);
	try {
		// get_lookahead_score is synchronous in WASM but we keep the async signature
		// for forward-compatibility with future IPC-based implementations.
		const json = bridge.getLookaheadScore(bars);
		const score = parseScore(json);
		if (!score) {
			throw new Error('Engine returned an empty or invalid score');
		}
		currentScore.set(score);
	} catch (e) {
		const message = e instanceof Error ? e.message : String(e);
		console.error('[playback] loadScore failed:', message);
		scoreError.set(message);
	} finally {
		scoreLoading.set(false);
	}
}

/**
 * Directly set the current score (e.g. from a pre-parsed JSON for testing).
 */
export function setCurrentScore(score: HarmoniumScore | null): void {
	currentScore.set(score);
}

/**
 * Clear the current score and reset all derived state.
 */
export function clearScore(): void {
	currentScore.set(null);
	scoreError.set(null);
}
