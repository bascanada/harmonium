<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';
  import EmotionalControls from './EmotionalControls.svelte';
  import TechnicalControls from './TechnicalControls.svelte';
  import ChannelMixer from './ChannelMixer.svelte';

  // Props - bridge passed from parent
  export let bridge: HarmoniumBridge;
  export let state: EngineState;

  // Audio mode detection (true = web audio rendering, false = VST MIDI-only)
  export let isAudioMode = true;

  // Local control mode (decoupled from state during transitions)
  let localIsEmotionMode = true;
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync with state ONLY when not actively toggling
  $: if (!isEditing) localIsEmotionMode = state.isEmotionMode;

  function toggleControlMode() {
    isEditing = true;
    if (editTimeout) clearTimeout(editTimeout);
    editTimeout = setTimeout(() => { isEditing = false; }, 500);

    localIsEmotionMode = !localIsEmotionMode;
    if (localIsEmotionMode) {
      bridge.useEmotionMode();
    } else {
      bridge.useDirectMode();
    }
  }
</script>

<div class="bg-neutral-800 rounded-lg p-4 shadow-xl h-fit sticky top-8">
  <!-- MODE TOGGLE -->
  <div class="mb-4">
    <div class="flex rounded-lg bg-neutral-900 p-1.5">
      <button
        onclick={() => {
          if (!localIsEmotionMode) toggleControlMode();
        }}
        class="flex-1 py-2 px-4 rounded-md text-sm font-semibold transition-all duration-200
          {localIsEmotionMode
            ? 'bg-purple-600 text-white shadow-lg'
            : 'text-neutral-400 hover:text-neutral-200'}"
      >
        Emotional
      </button>
      <button
        onclick={() => {
          if (localIsEmotionMode) toggleControlMode();
        }}
        class="flex-1 py-2 px-4 rounded-md text-sm font-semibold transition-all duration-200
          {!localIsEmotionMode
            ? 'bg-cyan-600 text-white shadow-lg'
            : 'text-neutral-400 hover:text-neutral-200'}"
      >
        Technical
      </button>
    </div>
    <p class="text-xs text-neutral-500 text-center mt-2">
      {localIsEmotionMode ? "Russell's Circumplex Model" : 'Direct Musical Parameters'}
    </p>
  </div>

  <!-- CHANNEL MIXER (always visible) -->
  <div class="mb-4">
    <ChannelMixer {bridge} {state} />
  </div>

  {#if localIsEmotionMode}
    <EmotionalControls
      {bridge}
      arousal={state.arousal}
      valence={state.valence}
      density={state.density}
      tension={state.tension}
    />
  {:else}
    <TechnicalControls
      {bridge}
      {state}
      {isAudioMode}
      audioBackend={state.audioBackend}
      enableRhythm={state.enableRhythm}
      enableHarmony={state.enableHarmony}
      enableMelody={state.enableMelody}
      enableVoicing={state.enableVoicing}
      bpm={state.bpm}
      rhythmMode={state.rhythmMode}
      rhythmSteps={state.primarySteps}
      rhythmPulses={state.primaryPulses}
      rhythmRotation={state.primaryRotation}
      rhythmDensity={state.rhythmDensity}
      rhythmTension={state.rhythmTension}
      secondarySteps={state.secondarySteps}
      secondaryPulses={state.secondaryPulses}
      secondaryRotation={state.secondaryRotation}
      harmonyValence={state.harmonyValence}
      harmonyTension={state.harmonyTension}
      melodySmoothness={state.melodySmoothness}
      voicingDensity={state.voicingDensity}
      filterCutoff={state.voicingTension}
      filterResonance={0.3}
      chorusMix={0.0}
      delayMix={0.0}
      reverbMix={0.3}
      expression={0.5}
    />
  {/if}
</div>
