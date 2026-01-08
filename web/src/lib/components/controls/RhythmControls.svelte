<script lang="ts">
  import type { HarmoniumBridge } from '$lib/bridge';

  // Props
  export let bridge: HarmoniumBridge;
  export let rhythmMode: number;
  export let rhythmSteps: number;
  export let rhythmPulses: number;
  export let rhythmRotation: number;
  export let rhythmDensity: number;
  export let rhythmTension: number;
  export let secondarySteps: number;
  export let secondaryPulses: number;
  export let secondaryRotation: number;

  // Local state for controls - decoupled during active editing
  let local = {
    rhythmMode,
    rhythmSteps,
    rhythmPulses,
    rhythmRotation,
    rhythmDensity,
    rhythmTension,
    secondarySteps,
    secondaryPulses,
    secondaryRotation,
  };

  // Track if user is actively editing (prevent prop overwrite)
  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  
  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
      rhythmMode,
      rhythmSteps,
      rhythmPulses,
      rhythmRotation,
      rhythmDensity,
      rhythmTension,
      secondarySteps,
      secondaryPulses,
      secondaryRotation,
    };
  }

  function onSliderStart() {
    isEditing = true;
  }

  function onSliderEnd() {
    isEditing = false;
  }

  function update() {
    // Direct update without timeout locking
    bridge.setAllRhythmParams(
      local.rhythmMode,
      local.rhythmSteps,
      local.rhythmPulses,
      local.rhythmRotation,
      local.rhythmDensity,
      local.rhythmTension,
      local.secondarySteps,
      local.secondaryPulses,
      local.secondaryRotation
    );
  }

  function setRhythmMode(mode: number) {
    local.rhythmMode = mode;
    // Default steps per mode: Euclidean=16, PerfectBalance=48, ClassicGroove=16
    local.rhythmSteps = mode === 1 ? 48 : 16;
    update();
  }

  function setPolySteps(steps: number) {
    local.rhythmSteps = steps;
    update();
  }
</script>

<div class="p-5 bg-neutral-900/50 rounded-lg border-l-4 border-orange-500">
  <h3 class="text-lg font-semibold text-orange-400 mb-4">Rhythm</h3>

  <!-- Mode Toggle -->
  <div class="flex rounded-lg bg-neutral-800 p-1.5 mb-5">
    <button
      onclick={() => setRhythmMode(0)}
      class="flex-1 py-2.5 px-4 rounded-md text-sm font-semibold transition-all duration-200
        {local.rhythmMode === 0
          ? 'bg-orange-600 text-white shadow-lg'
          : 'text-neutral-400 hover:text-neutral-200'}"
    >
      Euclidean
    </button>
    <button
      onclick={() => setRhythmMode(1)}
      class="flex-1 py-2.5 px-4 rounded-md text-sm font-semibold transition-all duration-200
        {local.rhythmMode === 1
          ? 'bg-purple-600 text-white shadow-lg'
          : 'text-neutral-400 hover:text-neutral-200'}"
    >
      PerfectBalance
    </button>
    <button
      onclick={() => setRhythmMode(2)}
      class="flex-1 py-2.5 px-4 rounded-md text-sm font-semibold transition-all duration-200
        {local.rhythmMode === 2
          ? 'bg-teal-600 text-white shadow-lg'
          : 'text-neutral-400 hover:text-neutral-200'}"
    >
      ClassicGroove
    </button>
  </div>

  {#if local.rhythmMode === 0}
    <!-- EUCLIDEAN MODE -->
    <p class="text-xs text-neutral-500 mb-4">Bjorklund algorithm - Classic polyrhythms</p>

    <!-- Primary Sequencer -->
    <div class="mb-4 p-3 bg-neutral-800/50 rounded-lg">
      <div class="text-xs text-orange-300 font-semibold mb-2">Primary (Kick)</div>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <span class="text-xs text-neutral-400">Steps: {local.rhythmSteps}</span>
          <input
            type="range"
            min="4"
            max="32"
            step="1"
            bind:value={local.rhythmSteps}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-orange-500"
          />
        </div>
        <div>
          <span class="text-xs text-neutral-400">Pulses: {local.rhythmPulses}</span>
          <input
            type="range"
            min="1"
            max={local.rhythmSteps}
            step="1"
            bind:value={local.rhythmPulses}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-orange-500"
          />
        </div>
        <div>
          <span class="text-xs text-neutral-400">Rotation: {local.rhythmRotation}</span>
          <input
            type="range"
            min="0"
            max={local.rhythmSteps - 1}
            step="1"
            bind:value={local.rhythmRotation}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-orange-500"
          />
        </div>
      </div>
    </div>

    <!-- Secondary Sequencer -->
    <div class="p-3 bg-neutral-800/50 rounded-lg border border-green-500/30">
      <div class="text-xs text-green-300 font-semibold mb-2">Secondary (Snare) - Polyrhythm</div>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <span class="text-xs text-neutral-400">Steps: {local.secondarySteps}</span>
          <input
            type="range"
            min="4"
            max="32"
            step="1"
            bind:value={local.secondarySteps}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-500"
          />
        </div>
        <div>
          <span class="text-xs text-neutral-400">Pulses: {local.secondaryPulses}</span>
          <input
            type="range"
            min="1"
            max={local.secondarySteps}
            step="1"
            bind:value={local.secondaryPulses}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-500"
          />
        </div>
        <div>
          <span class="text-xs text-neutral-400">Rotation: {local.secondaryRotation}</span>
          <input
            type="range"
            min="0"
            max={local.secondarySteps - 1}
            step="1"
            bind:value={local.secondaryRotation}
            oninput={update}
            onpointerdown={onSliderStart}
            onpointerup={onSliderEnd}
            onpointercancel={onSliderEnd}
            class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-500"
          />
        </div>
      </div>
      <p class="text-xs text-neutral-600 mt-2">
        {local.rhythmSteps}:{local.secondarySteps} polyrhythm
      </p>
    </div>
  {:else if local.rhythmMode === 1}
    <!-- PERFECTBALANCE MODE -->
    <p class="text-xs text-neutral-500 mb-4">XronoMorph style - Regular polygons</p>

    <!-- Poly Steps Selection -->
    <div class="mb-4">
      <span class="text-xs text-neutral-400 mb-2 block">Resolution (steps per measure)</span>
      <div class="flex gap-2">
        {#each [48, 96, 192] as s}
          <button
            onclick={() => setPolySteps(s)}
            class="flex-1 py-2 px-3 rounded font-mono text-sm transition-colors
              {local.rhythmSteps === s
                ? 'bg-purple-600 text-white'
                : 'bg-neutral-800 text-neutral-400 hover:bg-neutral-700'}"
          >
            {s}
          </button>
        {/each}
      </div>
    </div>

    <!-- Density & Tension -->
    <div class="grid grid-cols-2 gap-4">
      <div>
        <span class="text-xs text-neutral-400">Density: {local.rhythmDensity.toFixed(2)}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.rhythmDensity}
          oninput={update}
          onpointerdown={onSliderStart}
          onpointerup={onSliderEnd}
          onpointercancel={onSliderEnd}
          class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-purple-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Sparse</span>
          <span>Dense</span>
        </div>
      </div>
      <div>
        <span class="text-xs text-neutral-400">Tension: {local.rhythmTension.toFixed(2)}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.rhythmTension}
          oninput={update}
          onpointerdown={onSliderStart}
          onpointerup={onSliderEnd}
          onpointercancel={onSliderEnd}
          class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-purple-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Simple</span>
          <span>Complex</span>
        </div>
      </div>
    </div>
  {:else}
    <!-- CLASSICGROOVE MODE -->
    <p class="text-xs text-neutral-500 mb-4">Realistic drum patterns - Ghost notes & grooves</p>

    <!-- Density & Tension -->
    <div class="grid grid-cols-2 gap-4">
      <div>
        <span class="text-xs text-neutral-400">Density: {local.rhythmDensity.toFixed(2)}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.rhythmDensity}
          oninput={update}
          onpointerdown={onSliderStart}
          onpointerup={onSliderEnd}
          onpointercancel={onSliderEnd}
          class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-teal-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Half-time</span>
          <span>Breakbeat</span>
        </div>
      </div>
      <div>
        <span class="text-xs text-neutral-400">Tension: {local.rhythmTension.toFixed(2)}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.rhythmTension}
          oninput={update}
          onpointerdown={onSliderStart}
          onpointerup={onSliderEnd}
          onpointercancel={onSliderEnd}
          class="w-full h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-teal-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Clean</span>
          <span>Ghost notes</span>
        </div>
      </div>
    </div>
  {/if}
</div>
