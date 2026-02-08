<script lang="ts">
	import { engineState, lookAheadBuffer } from '$lib/stores/engine-state';
	import { fade } from 'svelte/transition';

	let expanded = false;
</script>

<div class="mt-4 rounded-lg border border-red-900/50 bg-black/80 p-4 font-mono text-[10px] shadow-2xl">
	<div class="mb-2 flex items-center justify-between border-b border-red-900/30 pb-2">
		<h3 class="font-bold uppercase tracking-widest text-red-500">Raw Data Monitor (Debug)</h3>
		<button
			onclick={() => (expanded = !expanded)}
			class="rounded bg-red-900/30 px-2 py-1 text-red-400 hover:bg-red-900/50"
		>
			{expanded ? 'Collapse' : 'Expand JSON'}
		</button>
	</div>

	<div class="grid grid-cols-2 gap-4">
		<div class="space-y-1">
			<p><span class="text-neutral-500">Step:</span> <span class="text-white">{$engineState.currentStep}</span></p>
			<p><span class="text-neutral-500">Measure:</span> <span class="text-white">{$engineState.currentMeasure}</span></p>
			<p><span class="text-neutral-500">Buffer Size:</span> <span class="text-emerald-400">{$lookAheadBuffer.length}</span></p>
		</div>
		<div class="space-y-1">
			<p><span class="text-neutral-500">Key:</span> <span class="text-purple-400">{$engineState.key} {$engineState.scale}</span></p>
			<p><span class="text-neutral-500">BPM:</span> <span class="text-yellow-400">{$engineState.bpm.toFixed(1)}</span></p>
			{#if $lookAheadBuffer.length > 0}
				<p>
					<span class="text-neutral-500">1st Trigger:</span> 
					<span class="text-[8px] text-white">
						{Object.entries($lookAheadBuffer[0].trigger || {})
							.filter(([_, v]) => v === true)
							.map(([k]) => k)
							.join(', ') || 'none'}
					</span>
				</p>
			{/if}
		</div>
	</div>

	{#if expanded}
		<div class="mt-4 max-h-60 overflow-y-auto rounded bg-neutral-900 p-2 text-neutral-400" transition:fade>
			<pre>{JSON.stringify($lookAheadBuffer.slice(0, 4), null, 2)}</pre>
			{#if $lookAheadBuffer.length > 4}
				<p class="mt-1 text-center italic">... and {$lookAheadBuffer.length - 4} more steps</p>
			{/if}
		</div>
	{/if}
</div>
