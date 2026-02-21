<script lang="ts">
	import type { HarmoniumBridge, EngineState } from '$lib/bridge';
	import EmotionalControls from './EmotionalControls.svelte';
	import TechnicalControls from './TechnicalControls.svelte';
	import ChannelMixer from './ChannelMixer.svelte';

	// Svelte 5 Props Destructuring
	let {
		bridge,
		state: engineState,
		isAudioMode = true
	} = $props<{
		bridge: HarmoniumBridge;
		state: EngineState;
		isAudioMode?: boolean;
	}>();

	// Svelte 5 State
	let localIsEmotionMode = $state(true);
	let isEditing = $state(false);
	let editTimeout: ReturnType<typeof setTimeout> | null = $state(null);

	// Svelte 5 Effect (replaces $: )
	$effect(() => {
		if (!isEditing) {
			localIsEmotionMode = engineState.isEmotionMode;
		}
	});

	function toggleControlMode() {
		isEditing = true;
		if (editTimeout) clearTimeout(editTimeout);
		editTimeout = setTimeout(() => {
			isEditing = false;
		}, 500);

		localIsEmotionMode = !localIsEmotionMode;
		if (localIsEmotionMode) {
			bridge.useEmotionMode();
		} else {
			bridge.useDirectMode();
		}
	}
</script>

<div class="sticky top-8 h-fit rounded-lg bg-neutral-800 p-4 shadow-xl">
	<!-- MODE TOGGLE -->
	<div class="mb-4">
		<div class="flex rounded-lg bg-neutral-900 p-1.5">
			<button
				onclick={() => {
					if (!localIsEmotionMode) toggleControlMode();
				}}
				class="flex-1 rounded-md px-4 py-2 text-sm font-semibold transition-all duration-200
          {localIsEmotionMode
					? 'bg-purple-600 text-white shadow-lg'
					: 'text-neutral-400 hover:text-neutral-200'}"
			>
				Emotional
			</button>
			<button
				onclick={() => {
					if (localIsEmotionMode) toggleControlMode();
				}}
				class="flex-1 rounded-md px-4 py-2 text-sm font-semibold transition-all duration-200
          {!localIsEmotionMode
					? 'bg-cyan-600 text-white shadow-lg'
					: 'text-neutral-400 hover:text-neutral-200'}"
			>
				Technical
			</button>
		</div>
		<p class="mt-2 text-center text-xs text-neutral-500">
			{localIsEmotionMode ? "Russell's Circumplex Model" : 'Direct Musical Parameters'}
		</p>
	</div>

	<!-- CHANNEL MIXER (always visible) -->
	<div class="mb-4">
		<ChannelMixer {bridge} state={engineState} />
	</div>

	{#if localIsEmotionMode}
		<EmotionalControls
			{bridge}
			arousal={engineState.arousal}
			valence={engineState.valence}
			density={engineState.density}
			tension={engineState.tension}
		/>
	{:else}
		<TechnicalControls
			{bridge}
			state={engineState}
			{isAudioMode}
			audioBackend={engineState.audioBackend}
			enableRhythm={engineState.enableRhythm}
			enableHarmony={engineState.enableHarmony}
			enableMelody={engineState.enableMelody}
			enableVoicing={engineState.enableVoicing}
			fixedKick={engineState.fixedKick}
			bpm={engineState.bpm}
			rhythmMode={engineState.rhythmMode}
			rhythmSteps={engineState.primarySteps}
			rhythmPulses={engineState.primaryPulses}
			rhythmRotation={engineState.primaryRotation}
			rhythmDensity={engineState.rhythmDensity}
			rhythmTension={engineState.rhythmTension}
			secondarySteps={engineState.secondarySteps}
			secondaryPulses={engineState.secondaryPulses}
			secondaryRotation={engineState.secondaryRotation}
			harmonyValence={engineState.harmonyValence}
			harmonyTension={engineState.harmonyTension}
			melodySmoothness={engineState.melodySmoothness}
			voicingDensity={engineState.voicingDensity}
			filterCutoff={engineState.voicingTension}
			filterResonance={0.3}
			chorusMix={0.0}
			delayMix={0.0}
			reverbMix={0.3}
			expression={0.5}
		/>
	{/if}
</div>
