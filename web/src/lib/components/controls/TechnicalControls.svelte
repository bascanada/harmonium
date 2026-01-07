<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';
  import RhythmControls from './RhythmControls.svelte';
  import HarmonyControls from './HarmonyControls.svelte';
  import MelodyVoicingControls from './MelodyVoicingControls.svelte';
  import AudioBackendControls from './AudioBackendControls.svelte';

  // Props - bridge passed from parent
  export let bridge: HarmoniumBridge;

  // Full state from parent for reading harmonyMode
  export let state: EngineState;

  // Audio mode detection (true = web audio rendering, false = VST MIDI-only)
  export let isAudioMode = true;

  // Audio backend selection
  export let audioBackend: 'fundsp' | 'odin2' = 'odin2';

  // Props (from parent state) - MIDI Layer
  export let enableRhythm = true;
  export let enableHarmony = true;
  export let enableMelody = true;
  export let enableVoicing = false;
  export let bpm = 120;
  export let rhythmMode = 0;
  export let rhythmSteps = 16;
  export let rhythmPulses = 4;
  export let rhythmRotation = 0;
  export let rhythmDensity = 0.5;
  export let rhythmTension = 0.3;
  export let secondarySteps = 12;
  export let secondaryPulses = 3;
  export let secondaryRotation = 0;
  export let harmonyValence = 0.3;
  export let harmonyTension = 0.3;
  export let melodySmoothness = 0.7;
  export let voicingDensity = 0.5;

  // Audio Backend Layer
  export let filterCutoff = 0.7;
  export let filterResonance = 0.3;
  export let chorusMix = 0.0;
  export let delayMix = 0.0;
  export let reverbMix = 0.3;
  export let expression = 0.5;

  // Local state for controls - decoupled during active editing
  let local = {
    enableRhythm,
    enableHarmony,
    enableMelody,
    enableVoicing,
    bpm,
    rhythmMode,
    rhythmSteps,
    rhythmPulses,
    rhythmRotation,
    rhythmDensity,
    rhythmTension,
    secondarySteps,
    secondaryPulses,
    secondaryRotation,
    harmonyValence,
    harmonyTension,
    melodySmoothness,
    voicingDensity,
  };

  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
      enableRhythm,
      enableHarmony,
      enableMelody,
      enableVoicing,
      bpm,
      rhythmMode,
      rhythmSteps,
      rhythmPulses,
      rhythmRotation,
      rhythmDensity,
      rhythmTension,
      secondarySteps,
      secondaryPulses,
      secondaryRotation,
      harmonyValence,
      harmonyTension,
      melodySmoothness,
      voicingDensity,
    };
  }

  function startEditing() {
    isEditing = true;
    if (editTimeout) clearTimeout(editTimeout);
    editTimeout = setTimeout(() => {
      isEditing = false;
    }, 500);
  }

  function update() {
    startEditing();
    bridge.setDirectBpm(local.bpm);
    bridge.setDirectEnableRhythm(local.enableRhythm);
    bridge.setDirectEnableHarmony(local.enableHarmony);
    bridge.setDirectEnableMelody(local.enableMelody);
    bridge.setDirectEnableVoicing(local.enableVoicing);
    bridge.setDirectRhythmMode(local.rhythmMode);
    bridge.setDirectRhythmSteps(local.rhythmSteps);
    bridge.setDirectRhythmPulses(local.rhythmPulses);
    bridge.setDirectRhythmRotation(local.rhythmRotation);
    bridge.setDirectRhythmDensity(local.rhythmDensity);
    bridge.setDirectRhythmTension(local.rhythmTension);
    bridge.setDirectSecondarySteps(local.secondarySteps);
    bridge.setDirectSecondaryPulses(local.secondaryPulses);
    bridge.setDirectSecondaryRotation(local.secondaryRotation);
    bridge.setDirectHarmonyTension(local.harmonyTension);
    bridge.setDirectHarmonyValence(local.harmonyValence);
    bridge.setDirectMelodySmoothness(local.melodySmoothness);
    bridge.setDirectVoicingDensity(local.voicingDensity);
  }

  function toggleModule(module: 'rhythm' | 'harmony' | 'melody' | 'voicing') {
    startEditing();
    if (module === 'rhythm') local.enableRhythm = !local.enableRhythm;
    else if (module === 'harmony') local.enableHarmony = !local.enableHarmony;
    else if (module === 'melody') local.enableMelody = !local.enableMelody;
    else if (module === 'voicing') local.enableVoicing = !local.enableVoicing;
    update();
  }
</script>

<div class="technical-controls space-y-6">
  <!-- Module Toggles -->
  <div class="p-5 bg-neutral-900 rounded-lg">
    <h3 class="text-base font-semibold text-neutral-400 mb-4">Modules</h3>
    <div class="flex gap-3">
      <button
        onclick={() => toggleModule('rhythm')}
        class="flex-1 py-3 px-4 rounded-lg text-base font-medium transition-colors
          {local.enableRhythm ? 'bg-orange-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
      >
        Rhythm
      </button>
      <button
        onclick={() => toggleModule('harmony')}
        class="flex-1 py-3 px-4 rounded-lg text-base font-medium transition-colors
          {local.enableHarmony ? 'bg-green-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
      >
        Harmony
      </button>
      <button
        onclick={() => toggleModule('melody')}
        class="flex-1 py-3 px-4 rounded-lg text-base font-medium transition-colors
          {local.enableMelody ? 'bg-blue-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
      >
        Melody
      </button>
      <button
        onclick={() => toggleModule('voicing')}
        class="flex-1 py-3 px-4 rounded-lg text-base font-medium transition-colors
          {local.enableVoicing ? 'bg-purple-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
      >
        Voicing
      </button>
    </div>
  </div>

  <!-- BPM Direct -->
  <div class="py-2">
    <div class="flex justify-between mb-3">
      <span class="text-xl font-semibold">BPM</span>
      <span class="text-lg text-cyan-400 font-mono">{local.bpm}</span>
    </div>
    <input
      type="range"
      min="30"
      max="200"
      step="1"
      bind:value={local.bpm}
      oninput={update}
      class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-cyan-600"
    />
  </div>

  <!-- Conditional child components -->
  {#if local.enableRhythm}
    <RhythmControls
      {bridge}
      rhythmMode={local.rhythmMode}
      rhythmSteps={local.rhythmSteps}
      rhythmPulses={local.rhythmPulses}
      rhythmRotation={local.rhythmRotation}
      rhythmDensity={local.rhythmDensity}
      rhythmTension={local.rhythmTension}
      secondarySteps={local.secondarySteps}
      secondaryPulses={local.secondaryPulses}
      secondaryRotation={local.secondaryRotation}
    />
  {/if}

  {#if local.enableHarmony}
    <HarmonyControls {bridge} {state} harmonyValence={local.harmonyValence} harmonyTension={local.harmonyTension} />
  {/if}

  {#if local.enableMelody || local.enableVoicing}
    <MelodyVoicingControls
      {bridge}
      melodySmoothness={local.melodySmoothness}
      voicingDensity={local.voicingDensity}
    />
  {/if}

  <!-- Audio Backend Controls (only in audio mode, not VST MIDI-only) -->
  {#if isAudioMode}
    <AudioBackendControls
      {bridge}
      {audioBackend}
      {filterCutoff}
      {filterResonance}
      {chorusMix}
      {delayMix}
      {reverbMix}
      {expression}
    />
  {/if}
</div>
