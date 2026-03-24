<script lang="ts">
  /**
   * SheetMusic.svelte
   * Placeholder for VexFlow-based notation rendering.
   * This component will consume buffered StepTrigger data and render it as sheet music.
   */
  import { onMount } from 'svelte';

  let {
    barNumber = 1,
    timeSignature = { numerator: 4, denominator: 4 },
    pattern = [] as boolean[]
  } = $props();

  let container: HTMLDivElement;

  onMount(() => {
    // Future: Initialize VexFlow renderer here
    console.log('SheetMusic mounted for bar', barNumber);
  });
</script>

<div class="sheet-music-container">
  <div class="header">
    <h3>Bar {barNumber} ({timeSignature.numerator}/{timeSignature.denominator})</h3>
  </div>
  
  <div bind:this={container} class="vexflow-canvas">
    <!-- VexFlow will render here -->
    <div class="placeholder-staff">
      {#each pattern as hit, i}
        <div class="step {hit ? 'hit' : ''}" title="Step {i}"></div>
      {/each}
    </div>
  </div>
</div>

<style>
  .sheet-music-container {
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 1rem;
    margin-top: 1rem;
    color: #eee;
  }

  .header h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: #888;
  }

  .vexflow-canvas {
    min-height: 150px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #222;
    border-radius: 4px;
  }

  .placeholder-staff {
    display: flex;
    gap: 4px;
    width: 100%;
    padding: 0 1rem;
  }

  .step {
    flex: 1;
    height: 40px;
    background: #333;
    border-radius: 2px;
  }

  .step.hit {
    background: #4cc9f0;
    box-shadow: 0 0 10px rgba(76, 201, 240, 0.5);
  }
</style>
