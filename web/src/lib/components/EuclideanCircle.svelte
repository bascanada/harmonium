<script lang="ts">
  export let primarySteps = 16;
  export let primaryPulses = 4;
  export let primaryRotation = 0;
  
  export let secondarySteps = 12;
  export let secondaryPulses = 3;
  export let secondaryRotation = 0;

  export let currentStep = 0;
  export let radius = 150; // Plus grand pour accommoder les deux

  const PRIMARY_COLOR = "#ff3e00"; // Bass (Red)
  const SECONDARY_COLOR = "#4ade80"; // Lead (Green)

  // Fonction utilitaire pour générer le pattern de Bjorklund (Euclidien)
  function getPattern(steps: number, pulses: number): boolean[] {
    let pattern = new Array(steps).fill(false);
    if (steps === 0) return pattern;
    let bucket = 0;
    for (let i = 0; i < steps; i++) {
      bucket += pulses;
      if (bucket >= steps) {
        bucket -= steps;
        pattern[i] = true;
      }
    }
    return pattern;
  }

  // --- PRIMARY RING (Bass) ---
  $: primaryPattern = getPattern(primarySteps, primaryPulses);
  $: primaryRotated = [...primaryPattern.slice(primarySteps - primaryRotation), ...primaryPattern.slice(0, primarySteps - primaryRotation)];
  
  $: primaryPoints = Array.from({ length: primarySteps }).map((_, i) => {
    const angle = (i * 2 * Math.PI) / primarySteps - Math.PI / 2;
    const r = 45; // Shared radius
    return {
      x: 50 + r * Math.cos(angle),
      y: 50 + r * Math.sin(angle),
      active: primaryRotated[i],
      isCurrent: (currentStep % primarySteps) === i
    };
  });

  $: primaryPath = primaryPoints.filter(p => p.active).map(p => `${p.x},${p.y}`).join(" ");

  // --- SECONDARY RING (Lead) ---
  $: secondaryPattern = getPattern(secondarySteps, secondaryPulses);
  $: secondaryRotated = [...secondaryPattern.slice(secondarySteps - secondaryRotation), ...secondaryPattern.slice(0, secondarySteps - secondaryRotation)];

  $: secondaryPoints = Array.from({ length: secondarySteps }).map((_, i) => {
    const angle = (i * 2 * Math.PI) / secondarySteps - Math.PI / 2;
    const r = 45; // Shared radius
    return {
      x: 50 + r * Math.cos(angle),
      y: 50 + r * Math.sin(angle),
      active: secondaryRotated[i],
      isCurrent: (currentStep % secondarySteps) === i
    };
  });

  $: secondaryPath = secondaryPoints.filter(p => p.active).map(p => `${p.x},${p.y}`).join(" ");

</script>

<div class="circle-container" style="width: {radius * 2}px; height: {radius * 2}px;">
  <svg viewBox="0 0 100 100">
    <!-- Background Circle -->
    <circle cx="50" cy="50" r="45" stroke="#333" stroke-width="0.5" fill="none" />
    
    <!-- Polygons -->
    <polygon points={primaryPath} fill={PRIMARY_COLOR} fill-opacity="0.1" stroke={PRIMARY_COLOR} stroke-width="1.5" />
    <polygon points={secondaryPath} fill={SECONDARY_COLOR} fill-opacity="0.1" stroke={SECONDARY_COLOR} stroke-width="1.5" />

    <!-- Primary Dots (Bass) -->
    {#each primaryPoints as point}
      {#if point.active}
        <circle 
          cx={point.x} 
          cy={point.y} 
          r="2" 
          fill={PRIMARY_COLOR}
        />
      {/if}
    {/each}

    <!-- Secondary Dots (Lead) -->
    {#each secondaryPoints as point}
      {#if point.active}
        <circle 
          cx={point.x} 
          cy={point.y} 
          r="2" 
          fill={SECONDARY_COLOR}
        />
      {/if}
    {/each}

    <!-- Cursor Line (Based on Primary Step) -->
    <!-- On dessine une ligne qui tourne selon le step principal -->
    {#if primaryPoints.length > 0}
        <line 
        x1="50" y1="50" 
        x2={primaryPoints[currentStep % primarySteps].x} 
        y2={primaryPoints[currentStep % primarySteps].y} 
        stroke="white" 
        stroke-width="1"
        stroke-dasharray="2,2"
        opacity="0.8"
        />
    {/if}
    
    <text x="50" y="48" text-anchor="middle" font-size="4" fill={SECONDARY_COLOR} font-weight="bold">LEAD</text>
    <text x="50" y="54" text-anchor="middle" font-size="4" fill={PRIMARY_COLOR} font-weight="bold">BASS</text>
  </svg>
</div>

<style>
  .circle-container {
    display: inline-block;
    position: relative;
  }
  
  .current {
    stroke: white;
    stroke-width: 1.5px;
    r: 3.5px; 
    transition: all 0.1s ease;
  }
</style>
