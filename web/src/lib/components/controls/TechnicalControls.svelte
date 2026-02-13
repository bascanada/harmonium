<script lang="ts">
	import type { HarmoniumBridge, EngineState } from '$lib/bridge';
	import RhythmControls from './RhythmControls.svelte';
	import HarmonyControls from './HarmonyControls.svelte';
	import MelodyVoicingControls from './MelodyVoicingControls.svelte';
	import AudioBackendControls from './AudioBackendControls.svelte';

	// Svelte 5 Props Destructuring
	let {
		bridge,
		state,
		isAudioMode = true,
		audioBackend = 'odin2' as 'fundsp' | 'odin2',
		enableRhythm = true,
		enableHarmony = true,
		enableMelody = true,
		enableVoicing = false,
		fixedKick = false,
		bpm = 120,
		rhythmMode = 0,
		rhythmSteps = 16,
		rhythmPulses = 4,
		rhythmRotation = 0,
		rhythmDensity = 0.5,
		rhythmTension = 0.3,
		secondarySteps = 12,
		secondaryPulses = 3,
		secondaryRotation = 0,
		harmonyValence = 0.3,
		harmonyTension = 0.3,
		melodySmoothness = 0.7,
		voicingDensity = 0.5,
		filterCutoff = 0.7,
		filterResonance = 0.3,
		chorusMix = 0.0,
		delayMix = 0.0,
		reverbMix = 0.3,
		expression = 0.5
	} = $props<{
		bridge: HarmoniumBridge;
		state: EngineState;
		isAudioMode?: boolean;
		audioBackend?: 'fundsp' | 'odin2';
		enableRhythm?: boolean;
		enableHarmony?: boolean;
		enableMelody?: boolean;
		enableVoicing?: boolean;
		fixedKick?: boolean;
		bpm?: number;
		rhythmMode?: number;
		rhythmSteps?: number;
		rhythmPulses?: number;
		rhythmRotation?: number;
		rhythmDensity?: number;
		rhythmTension?: number;
		secondarySteps?: number;
		secondaryPulses?: number;
		secondaryRotation?: number;
		harmonyValence?: number;
		harmonyTension?: number;
		melodySmoothness?: number;
		voicingDensity?: number;
		filterCutoff?: number;
		filterResonance?: number;
		chorusMix?: number;
		delayMix?: number;
		reverbMix?: number;
		expression?: number;
	}>();

	// Svelte 5 State - Local state for controls (decoupled during active editing)
	let local = $state({
		enableRhythm,
		enableHarmony,
		enableMelody,
		enableVoicing,
		fixedKick,
		bpm,
		rhythmMode,
		rhythmSteps,
		rhythmPulses,
		rhythmRotation,
		rhythmDensity,
		rhythmTension,
		secondarySteps,
		secondaryPulses,
		secondaryRotation,
		harmonyValence,
		harmonyTension,
		melodySmoothness,
		voicingDensity
	});

	let isEditing = $state(false);

	// Svelte 5 Effect - Sync props to local ONLY when not editing
	$effect(() => {
		if (!isEditing) {
			local = {
				enableRhythm,
				enableHarmony,
				enableMelody,
				enableVoicing,
				fixedKick,
				bpm,
				rhythmMode,
				rhythmSteps,
				rhythmPulses,
				rhythmRotation,
				rhythmDensity,
				rhythmTension,
				secondarySteps,
				secondaryPulses,
				secondaryRotation,
				harmonyValence,
				harmonyTension,
				melodySmoothness,
				voicingDensity
			};
		}
	});

	function onSliderStart() {
		isEditing = true;
	}

	function onSliderEnd() {
		isEditing = false;
	}

	function update() {
		// Only send Direct updates, do not set isEditing here
		bridge.setDirectBpm(local.bpm);
		bridge.setDirectEnableRhythm(local.enableRhythm);
		bridge.setDirectEnableHarmony(local.enableHarmony);
		bridge.setDirectEnableMelody(local.enableMelody);
		bridge.setDirectEnableVoicing(local.enableVoicing);
		bridge.setDirectFixedKick(local.fixedKick);
		bridge.setDirectRhythmMode(local.rhythmMode);
		bridge.setDirectRhythmSteps(local.rhythmSteps);
		bridge.setDirectRhythmPulses(local.rhythmPulses);
		bridge.setDirectRhythmRotation(local.rhythmRotation);
		bridge.setDirectRhythmDensity(local.rhythmDensity);
		bridge.setDirectRhythmTension(local.rhythmTension);
		bridge.setDirectSecondarySteps(local.secondarySteps);
		bridge.setDirectSecondaryPulses(local.secondaryPulses);
		bridge.setDirectSecondaryRotation(local.secondaryRotation);
		bridge.setDirectHarmonyTension(local.harmonyTension);
		bridge.setDirectHarmonyValence(local.harmonyValence);
		bridge.setDirectMelodySmoothness(local.melodySmoothness);
		bridge.setDirectVoicingDensity(local.voicingDensity);
	}

	function toggleModule(module: 'rhythm' | 'harmony' | 'melody' | 'voicing') {
		// Buttons are instant (click), so we don't need pointer locking usually,
		// but setting isEditing=true briefly protects against instant rebound if needed.
		// simpler is direct update without locking:
		if (module === 'rhythm') local.enableRhythm = !local.enableRhythm;
		else if (module === 'harmony') local.enableHarmony = !local.enableHarmony;
		else if (module === 'melody') local.enableMelody = !local.enableMelody;
		else if (module === 'voicing') local.enableVoicing = !local.enableVoicing;
		update();
	}

	function toggleFixedKick() {
		local.fixedKick = !local.fixedKick;
		update();
	}
</script>

<div class="technical-controls space-y-6">
	<!-- Module Toggles -->
	<div class="rounded-lg bg-neutral-900 p-5">
		<h3 class="mb-4 text-base font-semibold text-neutral-400">Modules</h3>
		<div class="flex gap-3">
			<button
				onclick={() => toggleModule('rhythm')}
				class="flex-1 rounded-lg px-4 py-3 text-base font-medium transition-colors
          {local.enableRhythm ? 'bg-orange-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
			>
				Rhythm
			</button>
			<button
				onclick={() => toggleModule('harmony')}
				class="flex-1 rounded-lg px-4 py-3 text-base font-medium transition-colors
          {local.enableHarmony ? 'bg-green-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
			>
				Harmony
			</button>
			<button
				onclick={() => toggleModule('melody')}
				class="flex-1 rounded-lg px-4 py-3 text-base font-medium transition-colors
          {local.enableMelody ? 'bg-blue-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
			>
				Melody
			</button>
			<button
				onclick={() => toggleModule('voicing')}
				class="flex-1 rounded-lg px-4 py-3 text-base font-medium transition-colors
          {local.enableVoicing ? 'bg-purple-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
			>
				Voicing
			</button>
		</div>
	</div>

	<!-- Drum Mode Toggle -->
	<div class="rounded-lg bg-neutral-900 p-5">
		<h3 class="mb-4 text-base font-semibold text-neutral-400">Drum Mode</h3>
		<div class="flex gap-3">
			<button
				onclick={toggleFixedKick}
				class="flex-1 rounded-lg px-4 py-3 text-base font-medium transition-colors
          {local.fixedKick ? 'bg-amber-600 text-white' : 'bg-neutral-700 text-neutral-400'}"
				title="Mode Drum Kit : Kick fixe sur C1 (36) pour VST Drums"
			>
				{local.fixedKick ? 'Drum Kit (C1)' : 'Synth Bass'}
			</button>
		</div>
		<p class="mt-2 text-xs text-neutral-500">
			{local.fixedKick
				? 'Kick fixe sur C1 - ideal pour VST Drums'
				: 'Kick harmonise - ideal pour synth bass (Odin2)'}
		</p>
	</div>

	<!-- BPM Direct -->
	<div class="py-2">
		<div class="mb-3 flex justify-between">
			<span class="text-xl font-semibold">BPM</span>
			<span class="font-mono text-lg text-cyan-400">{local.bpm}</span>
		</div>
		<input
			type="range"
			min="30"
			max="200"
			step="1"
			bind:value={local.bpm}
			oninput={update}
			onpointerdown={onSliderStart}
			onpointerup={onSliderEnd}
			onpointercancel={onSliderEnd}
			class="h-3 w-full cursor-pointer appearance-none rounded-lg bg-neutral-700 accent-cyan-600"
		/>
	</div>

	<!-- Conditional child components -->
	{#if local.enableRhythm}
		<RhythmControls
			{bridge}
			rhythmMode={local.rhythmMode}
			rhythmSteps={local.rhythmSteps}
			rhythmPulses={local.rhythmPulses}
			rhythmRotation={local.rhythmRotation}
			rhythmDensity={local.rhythmDensity}
			rhythmTension={local.rhythmTension}
			secondarySteps={local.secondarySteps}
			secondaryPulses={local.secondaryPulses}
			secondaryRotation={local.secondaryRotation}
		/>
	{/if}

	{#if local.enableHarmony}
		<HarmonyControls
			{bridge}
			{state}
			harmonyValence={local.harmonyValence}
			harmonyTension={local.harmonyTension}
		/>
	{/if}

	{#if local.enableMelody || local.enableVoicing}
		<MelodyVoicingControls
			{bridge}
			melodySmoothness={local.melodySmoothness}
			voicingDensity={local.voicingDensity}
		/>
	{/if}

	<!-- Audio Backend Controls (only in audio mode, not VST MIDI-only) -->
	{#if isAudioMode}
		<AudioBackendControls
			{bridge}
			{audioBackend}
			{filterCutoff}
			{filterResonance}
			{chorusMix}
			{delayMix}
			{reverbMix}
			{expression}
		/>
	{/if}
</div>
