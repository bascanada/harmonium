<script lang="ts">
  import type { HarmoniumBridge } from '$lib/bridge';

  // Props - bridge passed from parent
  export let bridge: HarmoniumBridge;

  // Props for initial/backend values
  export let arousal = 0.5;
  export let valence = 0.3;
  export let density = 0.5;
  export let tension = 0.3;

  // Local state for sliders - decoupled from props during active editing
  let localArousal = arousal;
  let localValence = valence;
  let localDensity = density;
  let localTension = tension;

  // Track which slider is being actively edited (prevent backend overwrite)
  let activeSlider: string | null = null;
  let editTimeout: ReturnType<typeof setTimeout> | null = null;

  // Sync props to local state ONLY when not actively editing
  $: if (activeSlider !== 'arousal') localArousal = arousal;
  $: if (activeSlider !== 'valence') localValence = valence;
  $: if (activeSlider !== 'density') localDensity = density;
  $: if (activeSlider !== 'tension') localTension = tension;

  // Calculated BPM from local arousal (for display)
  $: bpm = 70 + localArousal * 110;

  // Mark slider as active and reset timeout
  function startEditing(slider: string) {
    activeSlider = slider;
    if (editTimeout) clearTimeout(editTimeout);
    // Allow prop sync after 500ms of no input
    editTimeout = setTimeout(() => {
      activeSlider = null;
    }, 500);
  }

  function updateArousal() {
    startEditing('arousal');
    bridge.setArousal(localArousal);
  }

  function updateValence() {
    startEditing('valence');
    bridge.setValence(localValence);
  }

  function updateDensity() {
    startEditing('density');
    bridge.setDensity(localDensity);
  }

  function updateTension() {
    startEditing('tension');
    bridge.setTension(localTension);
  }
</script>

<div class="emotional-controls space-y-6">
  <!-- BPM Display (calculated from Arousal) -->
  <div class="p-5 bg-neutral-900 rounded-lg border-l-4 border-purple-600">
    <div class="flex justify-between items-center">
      <span class="text-xl font-semibold">BPM</span>
      <span class="text-4xl font-mono text-purple-400">
        {bpm.toFixed(0)}
      </span>
    </div>
    <p class="text-sm text-neutral-500 mt-2">Calculated from Arousal</p>
  </div>

  <!-- Arousal -->
  <div class="py-2">
    <div class="flex justify-between mb-3">
      <label for="arousal" class="text-xl font-semibold">Arousal</label>
      <span class="text-lg text-purple-400 font-mono">{localArousal.toFixed(2)}</span>
    </div>
    <input
      id="arousal"
      type="range"
      min="0"
      max="1"
      step="0.01"
      bind:value={localArousal}
      oninput={updateArousal}
      class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-red-600"
    />
    <div class="text-sm text-neutral-500 mt-2 text-right">Energy / Tempo</div>
  </div>

  <!-- Valence -->
  <div class="py-2">
    <div class="flex justify-between mb-3">
      <label for="valence" class="text-xl font-semibold">Valence</label>
      <span class="text-lg text-purple-400 font-mono">{localValence.toFixed(2)}</span>
    </div>
    <input
      id="valence"
      type="range"
      min="-1"
      max="1"
      step="0.01"
      bind:value={localValence}
      oninput={updateValence}
      class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-600"
    />
    <div class="text-sm text-neutral-500 mt-2 text-right">Emotion / Harmony</div>
  </div>

  <!-- Density -->
  <div class="py-2">
    <div class="flex justify-between mb-3">
      <label for="density" class="text-xl font-semibold">Density</label>
      <span class="text-lg text-purple-400 font-mono">{localDensity.toFixed(2)}</span>
    </div>
    <input
      id="density"
      type="range"
      min="0"
      max="1"
      step="0.01"
      bind:value={localDensity}
      oninput={updateDensity}
      class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600"
    />
    <div class="text-sm text-neutral-500 mt-2 text-right">Rhythm Complexity</div>
  </div>

  <!-- Tension -->
  <div class="py-2">
    <div class="flex justify-between mb-3">
      <label for="tension" class="text-xl font-semibold">Tension</label>
      <span class="text-lg text-purple-400 font-mono">{localTension.toFixed(2)}</span>
    </div>
    <input
      id="tension"
      type="range"
      min="0"
      max="1"
      step="0.01"
      bind:value={localTension}
      oninput={updateTension}
      class="w-full h-3 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-yellow-600"
    />
    <div class="text-sm text-neutral-500 mt-2 text-right">Dissonance / Rotation</div>
  </div>
</div>
