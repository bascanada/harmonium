<script lang="ts">
	import type { HarmoniumBridge } from '$lib/bridge';
	import { ai, aiStatus, aiError } from '$lib/ai';
	import Card from '$lib/components/ui/card.svelte';
	import Slider from '$lib/components/ui/slider.svelte';
	import Input from '$lib/components/ui/input.svelte';

	// Props - bridge passed from parent
	// Props for initial/backend values
	let {
		bridge,
		arousal = $bindable(0.5),
		valence = $bindable(0.3),
		density = $bindable(0.5),
		tension = $bindable(0.3)
	} = $props();

	// Local state for sliders - decoupled from props during active editing
	let localArousal = $state(arousal);
	let localValence = $state(valence);
	let localDensity = $state(density);
	let localTension = $state(tension);

	// Track which slider is being actively edited (prevent backend overwrite)
	let activeSlider = $state<string | null>(null);

	// Sync props to local state ONLY when not actively editing
	$effect(() => {
		if (activeSlider !== 'arousal') localArousal = arousal;
		if (activeSlider !== 'valence') localValence = valence;
		if (activeSlider !== 'density') localDensity = density;
		if (activeSlider !== 'tension') localTension = tension;
	});

	// Calculated BPM from local arousal (for display)
	let bpm = $derived(70 + localArousal * 110);

	// AI Input
	let aiInputText = $state('');
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let isLoading = $derived($aiStatus === 'loading');

	// Slider interaction handlers
	function onSliderStart(name: string) {
		activeSlider = name;
	}

	function onSliderEnd() {
		// Commit adjustments if needed
		if (activeSlider === 'arousal') arousal = localArousal;
		if (activeSlider === 'valence') valence = localValence;
		if (activeSlider === 'density') density = localDensity;
		if (activeSlider === 'tension') tension = localTension;
		activeSlider = null;
	}

	function updateArousal() {
		bridge.setArousal(localArousal);
	}

	function updateValence() {
		bridge.setValence(localValence);
	}

	function updateDensity() {
		bridge.setDensity(localDensity);
	}

	function updateTension() {
		bridge.setTension(localTension);
	}

	async function analyzeText() {
		if (!aiInputText) return;

		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(async () => {
			// Logic from ancient runes version
			if ($aiStatus === 'idle' || $aiStatus === 'error') {
				await ai.init();
			}

			if ($aiStatus === 'ready') {
				const params = await ai.predictParameters(aiInputText);
				if (params) {
					console.log('Applying AI Params:', params);
					localArousal = params.arousal;
					localValence = params.valence;
					localTension = params.tension;
					localDensity = params.density;

					bridge.setArousal(localArousal);
					bridge.setValence(localValence);
					bridge.setTension(localTension);
					bridge.setDensity(localDensity);

					arousal = localArousal;
					valence = localValence;
					tension = localTension;
					density = localDensity;
				} else {
					console.warn('AI could not determine parameters for this input.');
				}
			}
		}, 600);
	}
</script>

<div class="space-y-6">
	<!-- AI Director -->
	<Card title="AI Director" class="border-neutral-700 bg-neutral-800">
		<div class="flex gap-2">
			<Input
				bind:value={aiInputText}
				placeholder="Enter words to describe emotions (e.g. 'battle fire danger')"
				onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && analyzeText()}
			/>
			<button
				onclick={analyzeText}
				disabled={isLoading}
				class="rounded bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-700 disabled:opacity-50"
			>
				{isLoading ? '...' : 'Set'}
			</button>
		</div>
		{#if $aiError}
			<div class="mt-2 text-xs text-red-400">{$aiError}</div>
		{/if}
		{#if $aiStatus === 'ready' && !aiInputText}
			<div class="mt-2 text-xs text-green-400">AI Engine Ready</div>
		{/if}
	</Card>

	<!-- BPM Display (calculated from Arousal) -->
	<div class="rounded-lg border-l-4 border-purple-600 bg-neutral-900 p-5">
		<div class="flex items-center justify-between">
			<span class="text-xl font-semibold">BPM</span>
			<span class="font-mono text-4xl text-purple-400">
				{bpm.toFixed(0)}
			</span>
		</div>
		<p class="mt-2 text-sm text-neutral-500">Calculated from Arousal</p>
	</div>

	<Card class="space-y-6">
		<!-- Arousal -->
		<div>
			<div class="mb-3 flex justify-between">
				<label for="arousal" class="text-xl font-semibold">Arousal</label>
				<span class="font-mono text-lg text-purple-400">{localArousal.toFixed(2)}</span>
			</div>
			<Slider
				min={0}
				max={1}
				step={0.01}
				bind:value={localArousal}
				onValueChange={updateArousal}
				onpointerdown={() => onSliderStart('arousal')}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-red-600"
			/>
			<div class="mt-2 text-right text-sm text-neutral-500">Energy / Tempo</div>
		</div>

		<!-- Valence -->
		<div>
			<div class="mb-3 flex justify-between">
				<label for="valence" class="text-xl font-semibold">Valence</label>
				<span class="font-mono text-lg text-purple-400">{localValence.toFixed(2)}</span>
			</div>
			<Slider
				min={-1}
				max={1}
				step={0.01}
				bind:value={localValence}
				onValueChange={updateValence}
				onpointerdown={() => onSliderStart('valence')}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-green-600"
			/>
			<div class="mt-2 text-right text-sm text-neutral-500">Emotion / Harmony</div>
		</div>

		<!-- Density -->
		<div>
			<div class="mb-3 flex justify-between">
				<label for="density" class="text-xl font-semibold">Density</label>
				<span class="font-mono text-lg text-purple-400">{localDensity.toFixed(2)}</span>
			</div>
			<Slider
				min={0}
				max={1}
				step={0.01}
				bind:value={localDensity}
				onValueChange={updateDensity}
				onpointerdown={() => onSliderStart('density')}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-blue-600"
			/>
			<div class="mt-2 text-right text-sm text-neutral-500">Rhythm Complexity</div>
		</div>

		<!-- Tension -->
		<div>
			<div class="mb-3 flex justify-between">
				<label for="tension" class="text-xl font-semibold">Tension</label>
				<span class="font-mono text-lg text-purple-400">{localTension.toFixed(2)}</span>
			</div>
			<Slider
				min={0}
				max={1}
				step={0.01}
				bind:value={localTension}
				onValueChange={updateTension}
				onpointerdown={() => onSliderStart('tension')}
				onpointerup={onSliderEnd}
				onpointercancel={onSliderEnd}
				class="accent-yellow-600"
			/>
			<div class="mt-2 text-right text-sm text-neutral-500">Dissonance / Rotation</div>
		</div>
	</Card>
</div>
