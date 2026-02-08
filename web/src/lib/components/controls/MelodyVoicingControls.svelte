<script lang="ts">
	import type { HarmoniumBridge } from '$lib/bridge';
	import Card from '$lib/components/ui/card.svelte';
	import Slider from '$lib/components/ui/slider.svelte';

	// Props
	let {
		bridge,
		melodySmoothness = $bindable(),
		voicingDensity = $bindable()
	}: {
		bridge: HarmoniumBridge;
		melodySmoothness: number;
		voicingDensity: number;
	} = $props();

	// Local state for controls - logic to decouple changes while dragging
	let isEditing = $state(false);
	let localSmoothness = $state(melodySmoothness);
	let localDensity = $state(voicingDensity);

	// Sync props to local ONLY when not editing
	$effect(() => {
		if (!isEditing) {
			localSmoothness = melodySmoothness;
			localDensity = voicingDensity;
		}
	});

	function onSliderStart() {
		isEditing = true;
	}

	function onSliderEnd() {
		isEditing = false;
		// Commit local changes back to bindable props (optional, depends on if we want 2-way sync strictly)
		melodySmoothness = localSmoothness;
		voicingDensity = localDensity;
	}

	function update() {
		bridge.setDirectMelodySmoothness(localSmoothness);
		bridge.setDirectVoicingDensity(localDensity);
	}
</script>

<Card
	title="Melody & Voicing"
	description="MIDI note generation (backend-agnostic)"
	class="border-l-4 border-l-blue-500"
>
	<div class="grid grid-cols-2 gap-5">
		<div>
			<Slider
				label="Smoothness"
				min={0}
				max={1}
				step={0.01}
				bind:value={localSmoothness}
				onValueChange={update}
				onpointerdown={onSliderStart}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-blue-500"
			/>
			<div class="mt-2 flex justify-between text-xs text-neutral-500">
				<span>Erratic</span>
				<span>Smooth</span>
			</div>
		</div>
		<div>
			<Slider
				label="Density"
				min={0}
				max={1}
				step={0.01}
				bind:value={localDensity}
				onValueChange={update}
				onpointerdown={onSliderStart}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-blue-500"
			/>
			<div class="mt-2 flex justify-between text-xs text-neutral-500">
				<span>Sparse</span>
				<span>Dense</span>
			</div>
		</div>
	</div>
</Card>
