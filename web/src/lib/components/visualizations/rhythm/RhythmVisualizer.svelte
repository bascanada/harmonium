<script lang="ts">
	import EuclideanCircle from './EuclideanCircle.svelte';

	// Rhythm mode (0 = Euclidean, 1 = PerfectBalance)
	export let rhythmMode = 0;

	// Primary rhythm
	export let primarySteps = 16;
	export let primaryPulses = 4;
	export let primaryRotation = 0;
	export let primaryPattern: boolean[] = [];

	// Secondary rhythm (for Euclidean polyrhythm)
	export let secondarySteps = 12;
	export let secondaryPulses = 3;
	export let secondaryRotation = 0;
	export let secondaryPattern: boolean[] = [];

	// Current step for animation
	export let currentStep = 0;

	// Density/Tension for PerfectBalance mode display
	export let rhythmDensity = 0.5;
	export let rhythmTension = 0.3;
</script>

<div class="rounded-lg border border-neutral-700 bg-neutral-800 p-4 shadow-xl">
	<!-- Header with current mode -->
	<div class="mb-3 flex items-center justify-center gap-3">
		<span
			class="rounded-full px-2 py-0.5 text-xs font-semibold
      {rhythmMode === 0
				? 'border border-orange-500/50 bg-orange-500/20 text-orange-400'
				: 'border border-purple-500/50 bg-purple-500/20 text-purple-400'}"
		>
			{rhythmMode === 0 ? 'Euclidean' : 'PerfectBalance'}
		</span>
		<span class="font-mono text-xs text-neutral-500">
			{primarySteps} steps
		</span>
	</div>

	<div class="flex flex-wrap items-center justify-center gap-4 py-2">
		<EuclideanCircle
			steps={primarySteps}
			pulses={primaryPulses}
			rotation={primaryRotation}
			externalPattern={primaryPattern.length > 0 ? primaryPattern : null}
			color={rhythmMode === 0 ? '#ff3e00' : '#a855f7'}
			label={rhythmMode === 0 ? 'PRIMARY' : 'GROOVE'}
			{currentStep}
			radius={rhythmMode === 0 ? 80 : 100}
		/>
		{#if rhythmMode === 0}
			<!-- Euclidean mode: 2 independent circles (polyrhythm) -->
			<EuclideanCircle
				steps={secondarySteps}
				pulses={secondaryPulses}
				rotation={secondaryRotation}
				externalPattern={secondaryPattern.length > 0 ? secondaryPattern : null}
				color="#4ade80"
				label="SECONDARY"
				{currentStep}
				radius={80}
			/>
		{/if}
	</div>

	<!-- Contextual info -->
	<div class="mt-2 text-center">
		{#if rhythmMode === 0}
			<p class="text-[10px] text-neutral-500">
				{primarySteps}:{secondarySteps} polyrhythm ({primaryPulses}/{primarySteps} vs {secondaryPulses}/{secondarySteps})
			</p>
		{:else}
			<p class="text-[10px] text-neutral-500">
				Density: {(rhythmDensity * 100).toFixed(0)}% | Tension: {(rhythmTension * 100).toFixed(0)}%
			</p>
		{/if}
	</div>
</div>
