<script lang="ts">
	import type { HarmoniumBridge } from '$lib/bridge';
	import FundSPControls from './FundSPControls.svelte';
	import Odin2Controls from './Odin2Controls.svelte';

	// Props
	export let bridge: HarmoniumBridge;
	export let audioBackend: 'fundsp' | 'odin2';
	export let filterCutoff: number;
	export let filterResonance: number;
	export let chorusMix: number;
	export let delayMix: number;
	export let reverbMix: number;
	export let expression: number;
</script>

<div class="mb-6">
	<h2 class="mb-4 text-xl font-semibold text-neutral-300">Audio Backend</h2>
	<p class="mb-4 text-xs text-neutral-500">Sound synthesis controls (post-MIDI generation)</p>

	{#if audioBackend === 'fundsp'}
		<FundSPControls {bridge} {filterCutoff} {reverbMix} {expression} />
	{:else if audioBackend === 'odin2'}
		<Odin2Controls {bridge} {filterCutoff} {filterResonance} {chorusMix} {delayMix} {reverbMix} />
	{/if}
</div>
