// Harmonium Web — Component Library Exports

// === Visualizations ===
export { default as EuclideanCircle } from './components/visualizations/EuclideanCircle.svelte';
export { default as ChordProgression } from './components/visualizations/ChordProgression.svelte';
export { default as RhythmVisualizer } from './components/visualizations/RhythmVisualizer.svelte';
export { default as MorphPlane } from './components/visualizations/MorphPlane.svelte';
export { default as MorphVisualization } from './components/visualizations/MorphVisualization.svelte';
export { default as SheetMusic } from './components/visualizations/SheetMusic.svelte';

// === Controls ===
export { default as ControlPanel } from './components/controls/ControlPanel.svelte';
export { default as ChannelMixer } from './components/controls/ChannelMixer.svelte';
export { default as EmotionalControls } from './components/controls/EmotionalControls.svelte';
export { default as TechnicalControls } from './components/controls/TechnicalControls.svelte';
export { default as HarmonyControls } from './components/controls/HarmonyControls.svelte';
export { default as RhythmControls } from './components/controls/RhythmControls.svelte';
export { default as MelodyVoicingControls } from './components/controls/MelodyVoicingControls.svelte';

// === Composite ===
export { default as HarmoniumDemo } from './components/HarmoniumDemo.svelte';

// === Bridge (types & implementations) ===
export type { HarmoniumBridge, EngineState, AudioBackendType, BridgeFactory } from './bridge/types';
export { createEmptyState } from './bridge/types';
export { WasmBridge } from './bridge/wasm-bridge';
export { BaseBridge } from './bridge/base-bridge';
export { createBridge, createAndConnectBridge, isVstMode } from './bridge/index';
