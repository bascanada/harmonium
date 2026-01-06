// Reactive Svelte stores for engine state
import { writable, derived, type Writable, type Readable } from 'svelte/store';
import type { EngineState, HarmoniumBridge } from '$lib/bridge';
import { createEmptyState } from '$lib/bridge';

// Main engine state store
export const engineState: Writable<EngineState> = writable(createEmptyState());

// Bridge instance store
export const bridge: Writable<HarmoniumBridge | null> = writable(null);

// Derived stores for convenience
export const isPlaying: Readable<boolean> = derived(bridge, $bridge => $bridge?.isConnected() ?? false);

export const currentChord: Readable<string> = derived(engineState, $state => $state.currentChord);
export const currentMeasure: Readable<number> = derived(engineState, $state => $state.currentMeasure);
export const currentStep: Readable<number> = derived(engineState, $state => $state.currentStep);

export const isEmotionMode: Readable<boolean> = derived(engineState, $state => $state.isEmotionMode);

// Calculated BPM from arousal (70 + arousal * 110)
export const calculatedBpm: Readable<number> = derived(
  engineState,
  $state => 70 + $state.arousal * 110
);

// Current rhythm mode (depends on control mode)
export const currentRhythmMode: Readable<number> = derived(
  engineState,
  $state => $state.rhythmMode
);

// Progression chords (reactive array)
export const progressionChords: Writable<string[]> = writable([]);

// Helper to sync bridge state to stores
export function syncBridgeToStores(bridgeInstance: HarmoniumBridge): () => void {
  bridge.set(bridgeInstance);

  return bridgeInstance.subscribe(state => {
    engineState.set(state);
  });
}

// Helper to reset all stores
export function resetStores(): void {
  engineState.set(createEmptyState());
  bridge.set(null);
  progressionChords.set([]);
}
