<script lang="ts">
	import type { HarmoniumBridge } from '$lib/bridge';

	// Props
	export let bridge: HarmoniumBridge;
	export let filterCutoff: number;
	export let reverbMix: number;
	export let expression: number;

	// Local state for controls - decoupled during active editing
	let local = {
		filterCutoff,
		reverbMix,
		expression
	};

	// Track if user is actively editing (prevent prop overwrite)
	let isEditing = false;
	let editTimeout: ReturnType<typeof setTimeout> | null = null;

	// Sync props to local ONLY when not editing
	$: if (!isEditing) {
		local = {
			filterCutoff,
			reverbMix,
			expression
		};
	}

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
		// TODO: Add bridge methods for reverb and expression when backend supports them
	}
</script>

<div class="rounded-lg border-l-4 border-emerald-500 bg-neutral-900/50 p-5">
	<h3 class="mb-4 text-lg font-semibold text-emerald-400">FundSP Audio Controls</h3>
	<p class="mb-4 text-xs text-neutral-500">FM synthesis + SoundFont rendering</p>
	<div class="grid grid-cols-3 gap-5">
		<div>
			<span class="mb-2 block text-sm text-neutral-400">Filter Cutoff</span>
			<input
				type="range"
				min="0"
				max="1"
				step="0.01"
				bind:value={local.filterCutoff}
				oninput={update}
				class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-emerald-500"
			/>
			<div class="mt-2 flex justify-between text-xs text-neutral-500">
				<span>Muffled</span>
				<span>Bright</span>
			</div>
			<p class="mt-1 text-xs text-neutral-600">MIDI CC 1 (Modulation)</p>
		</div>
		<div>
			<span class="mb-2 block text-sm text-neutral-400">Reverb Mix</span>
			<input
				type="range"
				min="0"
				max="1"
				step="0.01"
				bind:value={local.reverbMix}
				oninput={update}
				class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-emerald-500"
				disabled
			/>
			<div class="mt-2 flex justify-between text-xs text-neutral-500">
				<span>Dry</span>
				<span>Wet</span>
			</div>
			<p class="mt-1 text-xs text-neutral-600">MIDI CC 91 (coming soon)</p>
		</div>
		<div>
			<span class="mb-2 block text-sm text-neutral-400">Expression</span>
			<input
				type="range"
				min="0"
				max="1"
				step="0.01"
				bind:value={local.expression}
				oninput={update}
				class="h-2.5 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-emerald-500"
				disabled
			/>
			<div class="mt-2 flex justify-between text-xs text-neutral-500">
				<span>Soft</span>
				<span>Hard</span>
			</div>
			<p class="mt-1 text-xs text-neutral-600">MIDI CC 11 (coming soon)</p>
		</div>
	</div>
</div>
