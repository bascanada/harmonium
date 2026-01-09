<script lang="ts">
  import type { HarmoniumBridge } from '$lib/bridge';
  import Card from '$lib/components/ui/card.svelte';
  import Slider from '$lib/components/ui/slider.svelte';
  import ToggleGroup from '$lib/components/ui/toggle-group.svelte';
  import ToggleGroupItem from '$lib/components/ui/toggle-group-item.svelte';

  // Props
  let { 
    bridge, 
    rhythmMode = $bindable(),
    rhythmSteps = $bindable(),
    rhythmPulses = $bindable(),
    rhythmRotation = $bindable(),
    rhythmDensity = $bindable(),
    rhythmTension = $bindable(),
    secondarySteps = $bindable(),
    secondaryPulses = $bindable(),
    secondaryRotation = $bindable()
  }: {
    bridge: HarmoniumBridge;
    rhythmMode: number;
    rhythmSteps: number;
    rhythmPulses: number;
    rhythmRotation: number;
    rhythmDensity: number;
    rhythmTension: number;
    secondarySteps: number;
    secondaryPulses: number;
    secondaryRotation: number;
  } = $props();

  // Local state for controls
  let isEditing = $state(false);
  
  // Local state variables
  let localMode = $state(rhythmMode);
  // We need to type the toggle group value as string (it returns string by default) then convert
  let localModeString = $derived(localMode.toString());

  let localSteps = $state(rhythmSteps);
  let localPulses = $state(rhythmPulses);
  let localRotation = $state(rhythmRotation);
  let localDensity = $state(rhythmDensity);
  let localTension = $state(rhythmTension);
  let localSecSteps = $state(secondarySteps);
  let localSecPulses = $state(secondaryPulses);
  let localSecRotation = $state(secondaryRotation);

  // Sync props to local ONLY when not editing
  $effect(() => {
    if (!isEditing) {
      localMode = rhythmMode;
      localSteps = rhythmSteps;
      localPulses = rhythmPulses;
      localRotation = rhythmRotation;
      localDensity = rhythmDensity;
      localTension = rhythmTension;
      localSecSteps = secondarySteps;
      localSecPulses = secondaryPulses;
      localSecRotation = secondaryRotation;
    }
  });

  function onSliderStart() {
    isEditing = true;
  }

  function onSliderEnd() {
    isEditing = false;
    // Commit back
    rhythmSteps = localSteps;
    rhythmPulses = localPulses;
    rhythmRotation = localRotation;
    rhythmDensity = localDensity;
    rhythmTension = localTension;
    secondarySteps = localSecSteps;
    secondaryPulses = localSecPulses;
    secondaryRotation = localSecRotation;
  }

  function update() {
    bridge.setAllRhythmParams(
      localMode,
      localSteps,
      localPulses,
      localRotation,
      localDensity,
      localTension,
      localSecSteps,
      localSecPulses,
      localSecRotation
    );
  }

  function handleModeChange(value: string | undefined) {
    if (value === undefined) return;
    const newMode = parseInt(value);
    
    localMode = newMode;
    rhythmMode = newMode;

    // Default steps per mode
    if (newMode === 1) { // PerfectBalance
        localSteps = 48;
    } else if (newMode === 2) { // ClassicGroove
        localSteps = 16;
    } else { // Euclidean
         // Keep current or reset to something standard? Leaving as is for now to match old logic mostly
         // Old logic: local.rhythmSteps = mode === 1 ? 48 : 16;
         // It seems it forced 16 for Euclidean/ClassicGroove
        localSteps = 16;
    }
    
    rhythmSteps = localSteps;
    update();
  }

  // Helper for Poly Steps buttons
  function setPolySteps(steps: number) {
    localSteps = steps;
    rhythmSteps = steps;
    update();
  }
</script>

<Card
  title="Rhythm"
  class="border-l-4 border-l-orange-500"
>
  <!-- Mode Toggle -->
  <div class="mb-5">
     <ToggleGroup type="single" value={localModeString} onValueChange={handleModeChange} class="bg-neutral-800 w-full justify-stretch">
        <ToggleGroupItem value="0" class="flex-1 data-[state=on]:bg-orange-600">Euclidean</ToggleGroupItem>
        <ToggleGroupItem value="1" class="flex-1 data-[state=on]:bg-purple-600">PerfectBalance</ToggleGroupItem>
        <ToggleGroupItem value="2" class="flex-1 data-[state=on]:bg-teal-600">ClassicGroove</ToggleGroupItem>
     </ToggleGroup>
  </div>

  {#if localMode === 0}
    <!-- EUCLIDEAN MODE -->
    <p class="text-xs text-neutral-500 mb-4">Bjorklund algorithm - Classic polyrhythms</p>

    <!-- Primary Sequencer -->
    <div class="mb-4 p-3 bg-neutral-800/50 rounded-lg">
      <div class="text-xs text-orange-300 font-semibold mb-2">Primary (Kick)</div>
      <div class="grid grid-cols-3 gap-3">
        <Slider
            label={`Steps: ${localSteps}`}
            min={4} max={32} step={1}
            bind:value={localSteps}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-orange-500"
        />
        <Slider
            label={`Pulses: ${localPulses}`}
            min={1} max={localSteps} step={1}
            bind:value={localPulses}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-orange-500"
        />
        <Slider
            label={`Rotation: ${localRotation}`}
            min={0} max={localSteps - 1} step={1}
            bind:value={localRotation}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-orange-500"
        />
      </div>
    </div>

    <!-- Secondary Sequencer -->
    <div class="p-3 bg-neutral-800/50 rounded-lg border border-green-500/30">
      <div class="text-xs text-green-300 font-semibold mb-2">Secondary (Snare) - Polyrhythm</div>
      <div class="grid grid-cols-3 gap-3">
        <Slider
            label={`Steps: ${localSecSteps}`}
            min={4} max={32} step={1}
            bind:value={localSecSteps}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-green-500"
        />
        <Slider
            label={`Pulses: ${localSecPulses}`}
            min={1} max={localSecSteps} step={1}
            bind:value={localSecPulses}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-green-500"
        />
        <Slider
            label={`Rotation: ${localSecRotation}`}
            min={0} max={localSecSteps - 1} step={1}
            bind:value={localSecRotation}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-green-500"
        />
      </div>
      <p class="text-xs text-neutral-600 mt-2">
        {localSteps}:{localSecSteps} polyrhythm
      </p>
    </div>
  {:else if localMode === 1}
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
              {localSteps === s
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
        <Slider
            label={`Density: ${localDensity.toFixed(2)}`}
            min={0} max={1} step={0.01}
            bind:value={localDensity}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-purple-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Sparse</span>
          <span>Dense</span>
        </div>
      </div>
      <div>
        <Slider
            label={`Tension: ${localTension.toFixed(2)}`}
            min={0} max={1} step={0.01}
            bind:value={localTension}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-purple-500"
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
        <Slider
            label={`Density: ${localDensity.toFixed(2)}`}
            min={0} max={1} step={0.01}
            bind:value={localDensity}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-teal-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Half-time</span>
          <span>Breakbeat</span>
        </div>
      </div>
      <div>
        <Slider
            label={`Tension: ${localTension.toFixed(2)}`}
            min={0} max={1} step={0.01}
            bind:value={localTension}
            onValueChange={update}
            onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
            class="accent-teal-500"
        />
        <div class="flex justify-between text-xs text-neutral-600 mt-1">
          <span>Clean</span>
          <span>Ghost notes</span>
        </div>
      </div>
    </div>
  {/if}
</Card>
