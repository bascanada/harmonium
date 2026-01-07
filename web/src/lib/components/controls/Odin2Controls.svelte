<script lang="ts">
  import type { HarmoniumBridge } from '$lib/bridge';

  // Props
  export let bridge: HarmoniumBridge;
  export let filterCutoff: number;
  export let filterResonance: number;
  export let chorusMix: number;
  export let delayMix: number;
  export let reverbMix: number;

  // Local state for controls - decoupled during active editing
  let local = {
    filterCutoff,
    filterResonance,
    chorusMix,
    delayMix,
    reverbMix,
  };

  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
      filterCutoff,
      filterResonance,
      chorusMix,
      delayMix,
      reverbMix,
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
    bridge.setDirectVoicingTension(local.filterCutoff);
    // TODO: Add bridge methods for other Odin2 parameters when backend supports them
  }
</script>

<div class="p-5 bg-neutral-900/50 rounded-lg border-l-4 border-pink-500">
  <h3 class="text-lg font-semibold text-pink-400 mb-4">Odin2 Audio Controls</h3>
  <p class="text-xs text-neutral-500 mb-4">Analog modeling synthesis</p>

  <!-- Filter Section -->
  <div class="mb-5">
    <h4 class="text-sm font-semibold text-pink-300 mb-3">Filter</h4>
    <div class="grid grid-cols-2 gap-5">
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Cutoff</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.filterCutoff}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-pink-500"
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Dark</span>
          <span>Bright</span>
        </div>
      </div>
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Resonance</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.filterResonance}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-pink-500"
          disabled
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Flat</span>
          <span>Peaked</span>
        </div>
        <p class="text-xs text-neutral-600 mt-1">Coming soon</p>
      </div>
    </div>
  </div>

  <!-- Effects Section -->
  <div>
    <h4 class="text-sm font-semibold text-pink-300 mb-3">Effects Mix</h4>
    <div class="grid grid-cols-3 gap-5">
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Chorus</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.chorusMix}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-pink-500"
          disabled
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Dry</span>
          <span>Wet</span>
        </div>
        <p class="text-xs text-neutral-600 mt-1">MIDI CC 93</p>
      </div>
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Delay</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.delayMix}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-pink-500"
          disabled
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Dry</span>
          <span>Wet</span>
        </div>
        <p class="text-xs text-neutral-600 mt-1">MIDI CC 94</p>
      </div>
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Reverb</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.reverbMix}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-pink-500"
          disabled
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Dry</span>
          <span>Wet</span>
        </div>
        <p class="text-xs text-neutral-600 mt-1">MIDI CC 91</p>
      </div>
    </div>
  </div>
</div>
