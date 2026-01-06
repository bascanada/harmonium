<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';

  // Props - bridge passed from parent
  export let bridge: HarmoniumBridge;

  // Full state from parent for reading harmonyMode
  export let state: EngineState;

  // Props (from parent state)
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
  export let voicingTension = 0.3;

  // Local state for controls - decoupled during active editing
  let local = {
    enableRhythm, enableHarmony, enableMelody, enableVoicing,
    bpm, rhythmMode, rhythmSteps, rhythmPulses, rhythmRotation,
    rhythmDensity, rhythmTension,
    secondarySteps, secondaryPulses, secondaryRotation,
    harmonyValence, harmonyTension,
    melodySmoothness, voicingDensity, voicingTension
  };

  // Track if user is actively editing (prevent prop overwrite)
  let isEditing = false;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local ONLY when not editing
  $: if (!isEditing) {
    local = {
      enableRhythm, enableHarmony, enableMelody, enableVoicing,
      bpm, rhythmMode, rhythmSteps, rhythmPulses, rhythmRotation,
      rhythmDensity, rhythmTension,
      secondarySteps, secondaryPulses, secondaryRotation,
      harmonyValence, harmonyTension,
      melodySmoothness, voicingDensity, voicingTension
    };
  }

  function startEditing() {
    isEditing = true;
    if (editTimeout) clearTimeout(editTimeout);
    editTimeout = setTimeout(() => { isEditing = false; }, 500);
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
    bridge.setDirectVoicingTension(local.voicingTension);
  }

  function toggleModule(module: 'rhythm' | 'harmony' | 'melody' | 'voicing') {
    startEditing();
    if (module === 'rhythm') local.enableRhythm = !local.enableRhythm;
    else if (module === 'harmony') local.enableHarmony = !local.enableHarmony;
    else if (module === 'melody') local.enableMelody = !local.enableMelody;
    else if (module === 'voicing') local.enableVoicing = !local.enableVoicing;
    update();
  }

  function setRhythmMode(mode: number) {
    startEditing();
    local.rhythmMode = mode;
    // Default steps per mode: Euclidean=16, PerfectBalance=48, ClassicGroove=16
    local.rhythmSteps = mode === 1 ? 48 : 16;
    update();
  }

  function setPolySteps(steps: number) {
    startEditing();
    local.rhythmSteps = steps;
    update();
  }

  function setHarmonyMode(mode: number) {
    bridge.setHarmonyMode(mode);
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

  <!-- Rhythm Section -->
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

  <!-- Harmony Section -->
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

  <!-- Melody/Voicing Section -->
  <div class="p-5 bg-neutral-900/50 rounded-lg border-l-4 border-blue-500">
    <h3 class="text-lg font-semibold text-blue-400 mb-4">Melody & Voicing</h3>
    <div class="grid grid-cols-3 gap-5">
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
      <div>
        <span class="text-sm text-neutral-400 mb-2 block">Filter</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          bind:value={local.voicingTension}
          oninput={update}
          class="w-full h-2.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
        />
        <div class="flex justify-between text-xs text-neutral-500 mt-2">
          <span>Muffled</span>
          <span>Bright</span>
        </div>
      </div>
    </div>
  </div>
</div>
