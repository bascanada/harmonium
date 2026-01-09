<script lang="ts">
  import { onMount } from 'svelte';

  // Props
  export let title = '';
  export let xAxisLabel = 'X';
  export let yAxisLabel = 'Y';
  
  // Axis ranges (default 0-1)
  export let xMin = 0;
  export let xMax = 1;
  export let yMin = 0;
  export let yMax = 1;

  // Points to visualize and control
  // Each point has: id, x, y, label, color, editable
  export let points: Array<{
    id: string;
    x: number;
    y: number;
    label?: string;
    color?: string;
    editable?: boolean;
    size?: number; // visual size
  }> = [];

  // Callbacks
  export let onPointChange: (id: string, x: number, y: number) => void = () => {};

  let container: HTMLDivElement;
  let draggingPointId: string | null = null;

  // Convert value to percentage (0-100)
  function toPercentX(val: number) {
    return ((val - xMin) / (xMax - xMin)) * 100;
  }

  function toPercentY(val: number) {
    // Y is usually inverted in CSS (0 is top), but for graphs, 0 is bottom.
    // Let's assume standard graph: bottom-left is min,min
    return 100 - ((val - yMin) / (yMax - yMin)) * 100;
  }

  function fromPercentX(pct: number) {
    return xMin + (pct / 100) * (xMax - xMin);
  }

  function fromPercentY(pct: number) {
    return yMin + ((100 - pct) / 100) * (yMax - yMin);
  }

  function handlePointerDown(e: PointerEvent, id: string) {
    const point = points.find(p => p.id === id);
    if (point && point.editable !== false) {
      draggingPointId = id;
      (e.target as Element).setPointerCapture(e.pointerId);
      e.stopPropagation(); // Prevent container click
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (!draggingPointId || !container) return;
    
    const rect = container.getBoundingClientRect();
    const x = Math.max(0, Math.min(100, ((e.clientX - rect.left) / rect.width) * 100));
    const y = Math.max(0, Math.min(100, ((e.clientY - rect.top) / rect.height) * 100));

    const newValX = fromPercentX(x);
    const newValY = fromPercentY(y);

    onPointChange(draggingPointId, newValX, newValY);
  }

  function handlePointerUp(e: PointerEvent) {
    if (draggingPointId) {
      (e.target as Element).releasePointerCapture(e.pointerId);
      draggingPointId = null;
    }
  }

  // Handle click on background to move the primary point (if generic interaction desired)
  // For now, let's strictly drag points.
</script>

<div class="flex flex-col items-center">
  {#if title}
    <h4 class="text-xs font-semibold text-neutral-400 mb-2 uppercase tracking-wider">{title}</h4>
  {/if}
  
  <div 
    bind:this={container}
    class="relative w-full aspect-square bg-neutral-900 rounded-lg border border-neutral-700 touch-none overflow-hidden"
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
  >
    <!-- Grid / Axis Lines -->
    <div class="absolute inset-0 pointer-events-none opacity-20">
      <div class="absolute top-1/2 left-0 right-0 h-px bg-neutral-500"></div>
      <div class="absolute left-1/2 top-0 bottom-0 w-px bg-neutral-500"></div>
    </div>

    <!-- Labels -->
    <div class="absolute bottom-1 left-2 text-[10px] text-neutral-500 pointer-events-none">{xAxisLabel} ({xMin})</div>
    <div class="absolute bottom-1 right-2 text-[10px] text-neutral-500 pointer-events-none">{xAxisLabel} ({xMax})</div>
    <div class="absolute top-2 left-1 text-[10px] text-neutral-500 pointer-events-none origin-bottom-left -rotate-90 translate-y-full">{yAxisLabel} ({yMax})</div>
    <div class="absolute bottom-8 left-1 text-[10px] text-neutral-500 pointer-events-none origin-bottom-left -rotate-90 translate-y-full">{yAxisLabel} ({yMin})</div>

    <!-- Points -->
    {#each points as point (point.id)}
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div
        class="absolute transform -translate-x-1/2 -translate-y-1/2 flex flex-col items-center group cursor-grab active:cursor-grabbing"
        style="
          left: {toPercentX(point.x)}%; 
          top: {toPercentY(point.y)}%; 
          z-index: {draggingPointId === point.id ? 50 : 10};
        "
        onpointerdown={(e) => handlePointerDown(e, point.id)}
      >
        <!-- Dot -->
        <div 
          class="rounded-full shadow-lg border-2 border-white transition-transform group-hover:scale-125"
          style="
            width: {point.size || 12}px; 
            height: {point.size || 12}px; 
            background-color: {point.color || '#888'};
          "
        ></div>
        
        <!-- Tooltip / Label -->
        {#if point.label}
          <div class="absolute top-full mt-1 px-1.5 py-0.5 bg-black/80 text-white text-[10px] rounded whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
            {point.label}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>
