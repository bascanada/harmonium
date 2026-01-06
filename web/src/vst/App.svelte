<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  // Import VstBridge directly to avoid loading WasmBridge and WASM module
  import { VstBridge } from '$lib/bridge/vst-bridge';
  import { type HarmoniumBridge, type EngineState, createEmptyState } from '$lib/bridge/types';
  import ControlPanel from '$lib/components/controls/ControlPanel.svelte';
  import RhythmVisualizer from '$lib/components/visualizations/RhythmVisualizer.svelte';
  import ChordProgression from '$lib/components/visualizations/ChordProgression.svelte';

  let bridge: HarmoniumBridge | null = null;
  let state: EngineState = createEmptyState();
  let unsubscribe: (() => void) | null = null;
  let error = '';
  let totalSteps = 0;
  let lastEngineStep = -1;
  let lastPrimarySteps = 16;
  let lastRhythmMode = 0;
  let lastIsEmotionMode = true;

  // Progression chords tracking
  let progressionChords: string[] = [];

  onMount(async () => {
    try {
      // Force VST bridge - we're in VST build, no auto-detection needed
      bridge = new VstBridge();
      await bridge.connect();

      // Subscribe to state updates
      unsubscribe = bridge.subscribe((newState) => {
        // Reset step tracking when mode or steps change significantly
        const rhythmModeChanged = newState.rhythmMode !== lastRhythmMode;
        const stepsChanged = newState.primarySteps !== lastPrimarySteps;
        const emotionModeChanged = newState.isEmotionMode !== lastIsEmotionMode;

        if (rhythmModeChanged || stepsChanged || emotionModeChanged) {
          lastEngineStep = -1;
          lastPrimarySteps = newState.primarySteps;
          lastRhythmMode = newState.rhythmMode;
          lastIsEmotionMode = newState.isEmotionMode;
        }

        // Track continuous step counter
        const rawStep = newState.currentStep;
        if (rawStep !== lastEngineStep) {
          let delta = rawStep - lastEngineStep;
          if (delta < 0) {
            delta += newState.primarySteps;
          }
          if (lastEngineStep === -1) {
            totalSteps = rawStep;
          } else {
            totalSteps += delta;
          }
          lastEngineStep = rawStep;
        }

        // Update progression chords
        if (newState.progressionLength !== progressionChords.length) {
          progressionChords = Array(newState.progressionLength).fill('?');
        }
        const chordIndex =
          newState.currentMeasure > 0
            ? (newState.currentMeasure - 1) % progressionChords.length
            : 0;
        if (chordIndex < progressionChords.length) {
          progressionChords[chordIndex] = newState.currentChord;
          progressionChords = [...progressionChords];
        }

        state = newState;
      });
    } catch (e) {
      console.error('Failed to connect bridge:', e);
      error = String(e);
    }
  });

  onDestroy(() => {
    unsubscribe?.();
    bridge?.disconnect();
  });
</script>

<div class="vst-container">
  <header class="header">
    <h1>Harmonium</h1>
    <span class="subtitle">Morphing Music Engine</span>
    {#if state.key && state.scale}
      <span class="key-info">{state.key} {state.scale}</span>
    {/if}
  </header>

  {#if error}
    <div class="error">{error}</div>
  {:else}
    <div class="main-content">
      <!-- Left: Visualizations -->
      <div class="visualizations">
        <RhythmVisualizer
          rhythmMode={state.rhythmMode}
          primarySteps={state.primarySteps}
          primaryPulses={state.primaryPulses}
          primaryRotation={state.primaryRotation}
          primaryPattern={state.primaryPattern}
          secondarySteps={state.secondarySteps}
          secondaryPulses={state.secondaryPulses}
          secondaryRotation={state.secondaryRotation}
          secondaryPattern={state.secondaryPattern}
          currentStep={totalSteps}
          rhythmDensity={state.rhythmDensity}
          rhythmTension={state.rhythmTension}
        />

        <ChordProgression
          currentChord={state.currentChord}
          currentMeasure={state.currentMeasure}
          isMinorChord={state.isMinorChord}
          progressionName={state.progressionName}
          {progressionChords}
          harmonyMode={state.harmonyMode}
        />
      </div>

      <!-- Right: Controls -->
      <div class="controls">
        {#if bridge}
          {#key bridge}
            <ControlPanel {state} {bridge} />
          {/key}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .vst-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #171717;
    color: #f5f5f5;
    padding: 1.5rem 2rem;
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding-bottom: 1.25rem;
    border-bottom: 1px solid #333;
    margin-bottom: 1.5rem;
  }

  .header h1 {
    font-size: 1.75rem;
    font-weight: bold;
    color: #a855f7;
  }

  .subtitle {
    color: #737373;
    font-size: 1rem;
  }

  .key-info {
    margin-left: auto;
    color: #c084fc;
    font-family: monospace;
    font-size: 1.125rem;
  }

  .error {
    color: #ef4444;
    text-align: center;
    padding: 2rem;
  }

  .main-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    flex: 1;
    overflow: hidden;
  }

  .visualizations {
    display: flex;
    flex-direction: column;
    gap: 2rem;
    overflow-y: auto;
    padding: 0.5rem 1rem 1rem 0.5rem;
  }

  .controls {
    overflow-y: auto;
    padding-right: 0.5rem;
  }

  /* Scrollbar styling */
  ::-webkit-scrollbar {
    width: 6px;
  }

  ::-webkit-scrollbar-track {
    background: #262626;
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb {
    background: #525252;
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: #737373;
  }
</style>
