<script lang="ts">
	import EuclideanCircle from './EuclideanCircle.svelte';

	let {
		rhythmMode = 0,
		primarySteps = 16,
		primaryPulses = 4,
		primaryRotation = 0,
		primaryPattern = [] as boolean[],
		secondarySteps = 12,
		secondaryPulses = 3,
		secondaryRotation = 0,
		secondaryPattern = [] as boolean[],
		currentStep = 0,
		rhythmDensity = 0.5,
		rhythmTension = 0.3
	} = $props();
</script>

<div class="rounded-lg border border-neutral-700 bg-neutral-800 p-4 shadow-xl">
	<!-- Header with current mode -->
	<div class="mb-3 flex items-center justify-center gap-3">
		<span
			class="rounded-full px-2 py-0.5 text-xs font-semibold
      {rhythmMode === 0
				? 'border border-orange-500/50 bg-orange-500/20 text-orange-400'
				: rhythmMode === 1
					? 'border border-purple-500/50 bg-purple-500/20 text-purple-400'
					: 'border border-teal-500/50 bg-teal-500/20 text-teal-400'}"
		>
			{rhythmMode === 0 ? 'Euclidean' : rhythmMode === 1 ? 'PerfectBalance' : 'ClassicGroove'}
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
			color={rhythmMode === 0 ? '#ff3e00' : rhythmMode === 1 ? '#a855f7' : '#14b8a6'}
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
