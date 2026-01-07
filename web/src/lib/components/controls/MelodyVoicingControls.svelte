<script lang="ts">
  import type { HarmoniumBridge } from '$lib/bridge';

  // Props
  export let bridge: HarmoniumBridge;
  export let melodySmoothness: number;
  export let voicingDensity: number;

  // Local state for controls - decoupled during active editing
  let local = {
    melodySmoothness,
    voicingDensity,
  };

  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
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
    bridge.setDirectMelodySmoothness(local.melodySmoothness);
    bridge.setDirectVoicingDensity(local.voicingDensity);
  }
</script>

<div class="p-5 bg-neutral-900/50 rounded-lg border-l-4 border-blue-500">
  <h3 class="text-lg font-semibold text-blue-400 mb-4">Melody & Voicing</h3>
  <p class="text-xs text-neutral-500 mb-4">MIDI note generation (backend-agnostic)</p>
  <div class="grid grid-cols-2 gap-5">
    <div>
      <span class="text-sm text-neutral-400 mb-2 block">Smoothness</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        bind:value={local.melodySmoothness}
        oninput={update}
        class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Erratic</span>
        <span>Smooth</span>
      </div>
    </div>
    <div>
      <span class="text-sm text-neutral-400 mb-2 block">Density</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        bind:value={local.voicingDensity}
        oninput={update}
        class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Sparse</span>
        <span>Dense</span>
      </div>
    </div>
  </div>
</div>
