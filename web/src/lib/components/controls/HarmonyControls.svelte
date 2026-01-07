<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';

  // Props
  export let bridge: HarmoniumBridge;
  export let state: EngineState; // For reading harmonyMode
  export let harmonyValence: number;
  export let harmonyTension: number;

  // Local state for controls - decoupled during active editing
  let local = {
    harmonyValence,
    harmonyTension,
  };

  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
      harmonyValence,
      harmonyTension,
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
    bridge.setDirectHarmonyValence(local.harmonyValence);
    bridge.setDirectHarmonyTension(local.harmonyTension);
  }

  function setHarmonyMode(mode: number) {
    bridge.setHarmonyMode(mode);
  }
</script>

<div class="p-5 bg-neutral-900/50 rounded-lg border-l-4 border-green-500">
  <h3 class="text-lg font-semibold text-green-400 mb-4">Harmony</h3>

  <!-- Harmony Engine Mode -->
  <div class="flex rounded-lg bg-neutral-800 p-1.5 mb-4">
    <button
      onclick={() => setHarmonyMode(0)}
      class="flex-1 py-2.5 px-4 rounded-md text-sm font-semibold transition-all duration-200
        {state.harmonyMode === 0
          ? 'bg-green-600 text-white shadow-lg'
          : 'text-neutral-400 hover:text-neutral-200'}"
    >
      Basic
    </button>
    <button
      onclick={() => setHarmonyMode(1)}
      class="flex-1 py-2.5 px-4 rounded-md text-sm font-semibold transition-all duration-200
        {state.harmonyMode === 1
          ? 'bg-cyan-600 text-white shadow-lg'
          : 'text-neutral-400 hover:text-neutral-200'}"
    >
      Driver
    </button>
  </div>
  <p class="text-xs text-neutral-500 mb-4 text-center">
    {state.harmonyMode === 0 ? 'Russell Circumplex (I-IV-vi-V)' : 'Steedman + Neo-Riemannian + LCC'}
  </p>

  <div class="grid grid-cols-2 gap-6">
    <div>
      <span class="text-sm text-neutral-400 mb-2 block">Valence: {local.harmonyValence.toFixed(2)}</span>
      <input
        type="range"
        min="-1"
        max="1"
        step="0.01"
        bind:value={local.harmonyValence}
        oninput={update}
        class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Minor</span>
        <span>Major</span>
      </div>
    </div>
    <div>
      <span class="text-sm text-neutral-400 mb-2 block">Tension: {local.harmonyTension.toFixed(2)}</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        bind:value={local.harmonyTension}
        oninput={update}
        class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Consonant</span>
        <span>Dissonant</span>
      </div>
    </div>
  </div>
</div>
