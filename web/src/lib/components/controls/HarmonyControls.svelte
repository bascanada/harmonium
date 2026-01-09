<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';
  import Card from '$lib/components/ui/card.svelte';
  import Slider from '$lib/components/ui/slider.svelte';
  import ToggleGroup from '$lib/components/ui/toggle-group.svelte';
  import ToggleGroupItem from '$lib/components/ui/toggle-group-item.svelte';

  // Props
  let { 
    bridge, 
    state: engineState,
    harmonyValence = $bindable(), 
    harmonyTension = $bindable() 
  }: {
    bridge: HarmoniumBridge;
    state: EngineState; // For reading harmonyMode
    harmonyValence: number;
    harmonyTension: number;
  } = $props();

  // Local state for controls
  let isEditing = $state(false);
  let localValence = $state(harmonyValence);
  let localTension = $state(harmonyTension);

  // Sync props to local ONLY when not editing
  $effect(() => {
    if (!isEditing) {
      localValence = harmonyValence;
      localTension = harmonyTension;
    }
  });

  function onSliderStart() {
    isEditing = true;
  }

  function onSliderEnd() {
    isEditing = false;
    // Commit back
    harmonyValence = localValence;
    harmonyTension = localTension;
  }

  function update() {
    bridge.setDirectHarmonyValence(localValence);
    bridge.setDirectHarmonyTension(localTension);
  }

  function setHarmonyMode(value: string | undefined) {
    if (value === undefined) return;
    const mode = parseInt(value);
    bridge.setHarmonyMode(mode);
  }

  let harmonyModeString = $derived(engineState.harmonyMode.toString());
</script>

<Card 
  title="Harmony" 
  class="border-l-4 border-l-green-500"
>
  <!-- Harmony Engine Mode -->
  <div class="mb-4">
    <ToggleGroup 
      type="single" 
      value={harmonyModeString} 
      onValueChange={setHarmonyMode}
      class="bg-neutral-800 w-full justify-stretch"
    >
      <ToggleGroupItem value="0" class="flex-1 data-[state=on]:bg-green-600">Basic</ToggleGroupItem>
      <ToggleGroupItem value="1" class="flex-1 data-[state=on]:bg-cyan-600">Driver</ToggleGroupItem>
    </ToggleGroup>
  </div>
  
  <p class="text-xs text-neutral-500 mb-4 text-center">
    {engineState.harmonyMode === 0 ? 'Russell Circumplex (I-IV-vi-V)' : 'Steedman + Neo-Riemannian + LCC'}
  </p>

  <div class="grid grid-cols-2 gap-6">
    <div>
      <Slider
        label={`Valence: ${localValence.toFixed(2)}`}
        min={-1} max={1} step={0.01}
        bind:value={localValence}
        onValueChange={update}
        onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
        class="accent-green-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Minor</span>
        <span>Major</span>
      </div>
    </div>
    <div>
      <Slider
        label={`Tension: ${localTension.toFixed(2)}`}
        min={0} max={1} step={0.01}
        bind:value={localTension}
        onValueChange={update}
        onpointerdown={onSliderStart} onpointerup={onSliderEnd} onpointercancel={onSliderEnd}
        class="accent-green-500"
      />
      <div class="flex justify-between text-xs text-neutral-500 mt-2">
        <span>Consonant</span>
        <span>Dissonant</span>
      </div>
    </div>
  </div>
</Card>
