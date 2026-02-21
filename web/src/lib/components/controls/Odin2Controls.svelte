<script lang="ts">
	import type { HarmoniumBridge } from '$lib/bridge';

	// Svelte 5 Props Destructuring
	let { bridge, filterCutoff, filterResonance, chorusMix, delayMix, reverbMix } = $props<{
		bridge: HarmoniumBridge;
		filterCutoff: number;
		filterResonance: number;
		chorusMix: number;
		delayMix: number;
		reverbMix: number;
	}>();

	// Svelte 5 State - Local state for controls (decoupled during active editing)
	// Initialize with literal defaults; $effect below syncs from props when not editing
	let local = $state({
		filterCutoff: 0.7,
		filterResonance: 0.3,
		chorusMix: 0.0,
		delayMix: 0.0,
		reverbMix: 0.3
	});

	let isEditing = $state(false);
	let editTimeout: ReturnType<typeof setTimeout> | null = $state(null);

	// Svelte 5 Effect - Sync props to local ONLY when not editing
	$effect(() => {
		if (!isEditing) {
			if (local.filterCutoff !== filterCutoff) local.filterCutoff = filterCutoff;
			if (local.filterResonance !== filterResonance) local.filterResonance = filterResonance;
			if (local.chorusMix !== chorusMix) local.chorusMix = chorusMix;
			if (local.delayMix !== delayMix) local.delayMix = delayMix;
			if (local.reverbMix !== reverbMix) local.reverbMix = reverbMix;
		}
	});

	function startEditing() {
		isEditing = true;
		if (editTimeout) clearTimeout(editTimeout);
		editTimeout = setTimeout(() => {
			isEditing = false;
		}, 500);
	}

	function update() {
		startEditing();
		bridge.setDirectVoicingTension(local.filterCutoff);
		// TODO: Add bridge methods for other Odin2 parameters when backend supports them
	}
</script>

<div class="rounded-lg border-l-4 border-pink-500 bg-neutral-900/50 p-5">
	<h3 class="mb-4 text-lg font-semibold text-pink-400">Odin2 Audio Controls</h3>
	<p class="mb-4 text-xs text-neutral-500">Analog modeling synthesis</p>

	<!-- Filter Section -->
	<div class="mb-5">
		<h4 class="mb-3 text-sm font-semibold text-pink-300">Filter</h4>
		<div class="grid grid-cols-2 gap-5">
			<div>
				<span class="mb-2 block text-sm text-neutral-400">Cutoff</span>
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					bind:value={local.filterCutoff}
					oninput={update}
					class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-pink-500"
				/>
				<div class="mt-2 flex justify-between text-xs text-neutral-500">
					<span>Dark</span>
					<span>Bright</span>
				</div>
			</div>
			<div>
				<span class="mb-2 block text-sm text-neutral-400">Resonance</span>
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					bind:value={local.filterResonance}
					oninput={update}
					class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-pink-500"
					disabled
				/>
				<div class="mt-2 flex justify-between text-xs text-neutral-500">
					<span>Flat</span>
					<span>Peaked</span>
				</div>
				<p class="mt-1 text-xs text-neutral-600">Coming soon</p>
			</div>
		</div>
	</div>

	<!-- Effects Section -->
	<div>
		<h4 class="mb-3 text-sm font-semibold text-pink-300">Effects Mix</h4>
		<div class="grid grid-cols-3 gap-5">
			<div>
				<span class="mb-2 block text-sm text-neutral-400">Chorus</span>
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					bind:value={local.chorusMix}
					oninput={update}
					class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-pink-500"
					disabled
				/>
				<div class="mt-2 flex justify-between text-xs text-neutral-500">
					<span>Dry</span>
					<span>Wet</span>
				</div>
				<p class="mt-1 text-xs text-neutral-600">MIDI CC 93</p>
			</div>
			<div>
				<span class="mb-2 block text-sm text-neutral-400">Delay</span>
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					bind:value={local.delayMix}
					oninput={update}
					class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-pink-500"
					disabled
				/>
				<div class="mt-2 flex justify-between text-xs text-neutral-500">
					<span>Dry</span>
					<span>Wet</span>
				</div>
				<p class="mt-1 text-xs text-neutral-600">MIDI CC 94</p>
			</div>
			<div>
				<span class="mb-2 block text-sm text-neutral-400">Reverb</span>
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					bind:value={local.reverbMix}
					oninput={update}
					class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-pink-500"
					disabled
				/>
				<div class="mt-2 flex justify-between text-xs text-neutral-500">
					<span>Dry</span>
					<span>Wet</span>
				</div>
				<p class="mt-1 text-xs text-neutral-600">MIDI CC 91</p>
			</div>
		</div>
	</div>
</div>
