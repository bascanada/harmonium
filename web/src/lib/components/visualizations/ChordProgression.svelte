<script lang="ts">
  export let currentChord = 'I';
  export let currentMeasure = 1;
  export let isMinorChord = false;
  export let progressionName = '';
  export let progressionChords: string[] = [];
  export let harmonyMode = 1; // 0 = Basic, 1 = Driver
</script>

<div class="bg-neutral-800 rounded-xl p-6 shadow-xl border border-neutral-700">
  <h2 class="text-xl font-bold mb-2 text-center text-purple-300">Harmonic Context</h2>

  <div class="flex justify-center gap-2 mb-2">
    <span
      class="text-xs px-2 py-1 rounded {harmonyMode === 0
        ? 'bg-green-500/30 text-green-300'
        : 'bg-cyan-500/30 text-cyan-300'}"
    >
      {harmonyMode === 0 ? 'Basic' : 'Driver'}
    </span>
  </div>

  <p class="text-xs text-neutral-400 text-center mb-4">{progressionName}</p>

  <div class="flex justify-center items-center gap-3 flex-wrap mb-4">
    {#each progressionChords as chord, index}
      <div class="flex items-center">
        <div
          class="w-12 h-12 rounded-full flex items-center justify-center text-lg font-bold transition-all duration-300
            {currentChord === chord
              ? 'bg-purple-600 text-white scale-110 shadow-lg shadow-purple-500/50'
              : 'bg-neutral-700 text-neutral-400'}"
        >
          {chord || '?'}
        </div>
        {#if index < progressionChords.length - 1}
          <div class="text-neutral-600 text-sm mx-1">-></div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="flex justify-between items-center bg-neutral-900/50 rounded p-2">
    <span class="text-sm text-neutral-400">Measure {currentMeasure}</span>
    <span class="text-2xl font-bold {isMinorChord ? 'text-blue-400' : 'text-yellow-400'}">
      {currentChord}
    </span>
  </div>
</div>
