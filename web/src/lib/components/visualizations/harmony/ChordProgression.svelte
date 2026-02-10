<script lang="ts">
	export let currentChord = 'I';
	export let currentMeasure = 1;
	export let isMinorChord = false;
	export let progressionName = '';
	export let progressionChords: string[] = [];
	export let harmonyMode = 1; // 0 = Basic, 1 = Driver
</script>

<div class="rounded-xl border border-neutral-700 bg-neutral-800 p-6 shadow-xl">
	<h2 class="mb-2 text-center text-xl font-bold text-purple-300">Harmonic Context</h2>

	<div class="mb-2 flex justify-center gap-2">
		<span
			class="rounded px-2 py-1 text-xs {harmonyMode === 0
				? 'bg-green-500/30 text-green-300'
				: 'bg-cyan-500/30 text-cyan-300'}"
		>
			{harmonyMode === 0 ? 'Basic' : 'Driver'}
		</span>
	</div>

	<p class="mb-4 text-center text-xs text-neutral-400">{progressionName}</p>

	<div class="mb-4 flex flex-wrap items-center justify-center gap-3">
		{#each progressionChords as chord, index}
			<div class="flex items-center">
				<div
					data-testid="progression-chord"
					data-active={currentChord === chord}
					class="flex h-12 w-12 items-center justify-center rounded-full text-lg font-bold transition-all duration-300
            {currentChord === chord
						? 'scale-110 bg-purple-600 text-white shadow-lg shadow-purple-500/50'
						: 'bg-neutral-700 text-neutral-400'}"
				>
					{chord || '?'}
				</div>
				{#if index < progressionChords.length - 1}
					<div class="mx-1 text-sm text-neutral-600">-></div>
				{/if}
			</div>
		{/each}
	</div>

	<div class="flex items-center justify-between rounded bg-neutral-900/50 p-2">
		<span class="text-sm text-neutral-400" data-testid="measure-display">Measure {currentMeasure}</span>
		<span data-testid="current-chord-display" class="text-2xl font-bold {isMinorChord ? 'text-blue-400' : 'text-yellow-400'}">
			{currentChord}
		</span>
	</div>
</div>
