<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { WasmBridge } from '$lib/bridge/wasm-bridge';
	import {
		type HarmoniumBridge,
		type EngineState,
		type AudioBackendType,
		createEmptyState
	} from '$lib/bridge/types';
	import ControlPanel from '$lib/components/controls/ControlPanel.svelte';
	import RhythmVisualizer from '$lib/components/visualizations/rhythm/RhythmVisualizer.svelte';
	import ChordProgression from '$lib/components/visualizations/harmony/ChordProgression.svelte';
	import MorphVisualization from '$lib/components/visualizations/morph/MorphVisualization.svelte';
	import MusicSheet from '$lib/components/visualizations/MusicSheet.svelte';
	import init, { get_available_backends } from 'harmonium';

	let bridge: HarmoniumBridge | null = null;
	let state: EngineState = createEmptyState();
	let unsubscribe: (() => void) | null = null;
	let isPlaying = false;
	let error = '';

	// Audio Backend Selection (before starting)
	let selectedBackend: AudioBackendType = 'odin2';
	let availableBackends: AudioBackendType[] = ['fundsp'];

	// Algorithm selection (before starting)
	let algorithm = 0;
	let polySteps = 48;

	// Harmony mode selection (before starting)
	let harmonyMode = 1;

	// Detect if we're in audio-rendering mode (web) or MIDI-only mode (VST)
	const isAudioMode = typeof window !== 'undefined' && !('ipc' in window);

	// Step tracking for visualizations
	let totalSteps = 0;
	let lastEngineStep = -1;
	let lastPrimarySteps = 16;
	let lastRhythmMode = 0;
	let lastIsEmotionMode = true;

	// Progression chords tracking
	let progressionChords: string[] = [];

	// Recording State
	let isRecordingWav = false;
	let isRecordingMidi = false;
	let isRecordingMusicXml = false;

	// Recording functions using bridge API
	function toggleWavRecording() {
		if (!bridge) return;
		if (isRecordingWav) {
			bridge.stopRecordingWav?.();
			isRecordingWav = false;
		} else {
			bridge.startRecordingWav?.();
			isRecordingWav = true;
		}
	}

	function toggleMidiRecording() {
		if (!bridge) return;
		if (isRecordingMidi) {
			bridge.stopRecordingMidi?.();
			isRecordingMidi = false;
		} else {
			bridge.startRecordingMidi?.();
			isRecordingMidi = true;
		}
	}

	function toggleMusicXmlRecording() {
		if (!bridge) return;
		if (isRecordingMusicXml) {
			bridge.stopRecordingMusicXml?.();
			isRecordingMusicXml = false;
		} else {
			bridge.startRecordingMusicXml?.();
			isRecordingMusicXml = true;
		}
	}

	function checkRecordings() {
		if (!bridge || !bridge.popFinishedRecording) return;

		// Loop to get all finished recordings
		while (true) {
			const recording = bridge.popFinishedRecording();
			if (!recording) break;

			const fmt = recording.format;
			const data = recording.data;

			const mimeType =
				fmt === 'wav' ? 'audio/wav' : fmt === 'midi' ? 'audio/midi' : 'application/xml';
			const ext = fmt === 'wav' ? 'wav' : fmt === 'midi' ? 'mid' : 'musicxml';

			const blob = new Blob([data as any], { type: mimeType });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `recording_${new Date().toISOString()}.${ext}`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		}
	}

	onMount(() => {
		const interval = setInterval(checkRecordings, 1000);

		// Fetch available backends
		(async () => {
			try {
				await init();
				const backends = get_available_backends();
				availableBackends = backends.map((b: string) => b as AudioBackendType);
			} catch (e) {
				console.warn('Could not fetch available backends:', e);
				availableBackends = ['fundsp'];
			}
		})();

		return () => clearInterval(interval);
	});

	onDestroy(() => {
		unsubscribe?.();
		bridge?.disconnect();
	});

	async function togglePlay() {
		if (isPlaying) {
			// Stop
			unsubscribe?.();
			bridge?.disconnect();
			bridge = null;
			isPlaying = false;
			return;
		}

		try {
			const AudioContext = window.AudioContext || (window as any).webkitAudioContext;
			if (!AudioContext) {
				throw new Error('Web Audio API is not supported in this browser');
			}

			// Create and connect bridge
			bridge = new WasmBridge();
			await bridge.connect(undefined, selectedBackend);

			// Set algorithm and harmony mode (before starting main loop)
			bridge.setAlgorithm(algorithm);
			bridge.setHarmonyMode(harmonyMode);
			bridge.setPolySteps(polySteps);

			// Set initial emotional parameters to ensure proper visualization state
			bridge.setArousal(0.5);
			bridge.setValence(0.3);
			bridge.setDensity(0.5);
			bridge.setTension(0.3);

						// Subscribe to state updates
						unsubscribe = bridge.subscribe((newState) => {
							// Reset step tracking when mode or steps change
							const rhythmModeChanged = newState.rhythmMode !== lastRhythmMode;
							const stepsChanged = newState.primarySteps !== lastPrimarySteps;
							const emotionModeChanged = newState.isEmotionMode !== lastIsEmotionMode;
			
							if (rhythmModeChanged || stepsChanged || emotionModeChanged) {
								lastEngineStep = -1;
								lastPrimarySteps = newState.primarySteps;
								lastRhythmMode = newState.rhythmMode;
								lastIsEmotionMode = newState.isEmotionMode;
								// Synchronize totalSteps with currentStep on reset
								totalSteps = newState.currentStep;
							}
			
							// Track continuous step counter
							const rawStep = newState.currentStep;
							if (rawStep !== lastEngineStep) {
								if (lastEngineStep === -1) {
									totalSteps = rawStep;
								} else {
									let delta = rawStep - lastEngineStep;
									if (delta < 0) {
										// Wrapped around or engine reset
										delta = rawStep;
										totalSteps = rawStep;
									} else {
										totalSteps += delta;
									}
								}
								lastEngineStep = rawStep;
							}
				// Update progression chords
				if (newState.progressionLength !== progressionChords.length) {
					progressionChords = Array(newState.progressionLength).fill('?');
				}
				const chordIndex =
					newState.currentMeasure > 0
						? (newState.currentMeasure - 1) % progressionChords.length
						: 0;
				if (chordIndex < progressionChords.length) {
					progressionChords[chordIndex] = newState.currentChord;
					progressionChords = [...progressionChords];
				}

				state = newState;
			});

			// Reset counters
			totalSteps = 0;
			lastEngineStep = -1;

			isPlaying = true;
			error = '';
		} catch (e) {
			console.error(e);
			error = String(e);
		}
	}
</script>

<div
	class="flex min-h-screen flex-col items-center justify-center bg-neutral-900 p-8 font-sans text-neutral-100"
>
	<h1 class="mb-2 text-4xl font-bold">Harmonium</h1>
	<p class="mb-8 text-neutral-400">Morphing Music Engine</p>

	{#if !isPlaying}
		<!-- Audio Backend Selection (only when stopped) -->
		{#if isAudioMode && availableBackends.length > 1}
			<div class="mb-6 w-80 rounded-xl border border-neutral-700 bg-neutral-800 p-4">
				<h3 class="mb-3 text-center text-sm font-semibold text-neutral-400">Audio Backend</h3>
				<div class="flex flex-col gap-2">
					<label
						class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                        {selectedBackend === 'fundsp'
							? 'border border-emerald-500 bg-emerald-500/20'
							: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
					>
						<input
							type="radio"
							name="audioBackend"
							value="fundsp"
							bind:group={selectedBackend}
							class="h-4 w-4 accent-emerald-500"
						/>
						<div>
							<span
								class="font-semibold {selectedBackend === 'fundsp'
									? 'text-emerald-400'
									: 'text-neutral-300'}">FundSP</span
							>
							<p class="text-xs text-neutral-500">FM synthesis + SoundFont support</p>
						</div>
					</label>

					{#if availableBackends.includes('odin2')}
						<label
							class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                            {selectedBackend === 'odin2'
								? 'border border-pink-500 bg-pink-500/20'
								: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
						>
							<input
								type="radio"
								name="audioBackend"
								value="odin2"
								bind:group={selectedBackend}
								class="h-4 w-4 accent-pink-500"
							/>
							<div>
								<span
									class="font-semibold {selectedBackend === 'odin2'
										? 'text-pink-400'
										: 'text-neutral-300'}">Odin2</span
								>
								<p class="text-xs text-neutral-500">Analog modeling synth with rich sound</p>
							</div>
						</label>
					{/if}
				</div>
			</div>
		{/if}

		<!-- Algorithm Selection (only when stopped) -->
		<div class="mb-6 w-80 rounded-xl border border-neutral-700 bg-neutral-800 p-4">
			<h3 class="mb-3 text-center text-sm font-semibold text-neutral-400">Rhythm Algorithm</h3>
			<div class="flex flex-col gap-2">
				<label
					class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                    {algorithm === 0
						? 'border border-orange-500 bg-orange-500/20'
						: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
				>
					<input
						type="radio"
						name="algorithm"
						value={0}
						bind:group={algorithm}
						class="h-4 w-4 accent-orange-500"
					/>
					<div>
						<span class="font-semibold {algorithm === 0 ? 'text-orange-400' : 'text-neutral-300'}"
							>Euclidean</span
						>
						<p class="text-xs text-neutral-500">16 steps - Classic Bjorklund grooves</p>
					</div>
				</label>

				<label
					class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                    {algorithm === 1
						? 'border border-purple-500 bg-purple-500/20'
						: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
				>
					<input
						type="radio"
						name="algorithm"
						value={1}
						bind:group={algorithm}
						class="h-4 w-4 accent-purple-500"
					/>
					<div>
						<span class="font-semibold {algorithm === 1 ? 'text-purple-400' : 'text-neutral-300'}"
							>PerfectBalance</span
						>
						<p class="text-xs text-neutral-500">Geometric polygons - XronoMorph polyrhythms</p>
					</div>
				</label>

				<label
					class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                    {algorithm === 2
						? 'border border-cyan-500 bg-cyan-500/20'
						: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
				>
					<input
						type="radio"
						name="algorithm"
						value={2}
						bind:group={algorithm}
						class="h-4 w-4 accent-cyan-500"
					/>
					<div>
						<span class="font-semibold {algorithm === 2 ? 'text-cyan-400' : 'text-neutral-300'}"
							>ClassicGroove</span
						>
						<p class="text-xs text-neutral-500">Real drum patterns - Ghost notes & grooves</p>
					</div>
				</label>

				{#if algorithm === 1 || algorithm === 2}
					<div class="mt-3 rounded-lg border border-purple-500/30 bg-purple-900/20 p-3">
						<span class="mb-2 block text-xs text-purple-300">Resolution (steps per measure)</span>
						<div class="flex gap-2">
							{#each [48, 96, 192] as steps}
								<button
									class="flex-1 rounded px-3 py-2 font-mono text-sm transition-colors
                                        {polySteps === steps
										? 'bg-purple-600 text-white'
										: 'bg-neutral-800 text-neutral-400 hover:bg-neutral-700'}"
									onclick={() => (polySteps = steps)}
								>
									{steps}
								</button>
							{/each}
						</div>
						<p class="mt-2 text-xs text-neutral-500">
							{#if polySteps === 48}
								Standard - Good for most polyrhythms
							{:else if polySteps === 96}
								High - Finer subdivisions (32nd notes)
							{:else}
								Ultra - Maximum precision (64th notes)
							{/if}
						</p>
					</div>
				{/if}
			</div>
		</div>

		<!-- Harmony Mode Selection (only when stopped) -->
		<div class="mb-6 w-80 rounded-xl border border-neutral-700 bg-neutral-800 p-4">
			<h3 class="mb-3 text-center text-sm font-semibold text-neutral-400">Harmony Engine</h3>
			<div class="flex flex-col gap-2">
				<label
					class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                    {harmonyMode === 0
						? 'border border-green-500 bg-green-500/20'
						: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
				>
					<input
						type="radio"
						name="harmonyMode"
						value={0}
						bind:group={harmonyMode}
						class="h-4 w-4 accent-green-500"
					/>
					<div>
						<span class="font-semibold {harmonyMode === 0 ? 'text-green-400' : 'text-neutral-300'}"
							>Basic</span
						>
						<p class="text-xs text-neutral-500">Russell Circumplex (I-IV-vi-V)</p>
					</div>
				</label>

				<label
					class="flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-colors
                    {harmonyMode === 1
						? 'border border-cyan-500 bg-cyan-500/20'
						: 'border border-transparent bg-neutral-700/50 hover:bg-neutral-700'}"
				>
					<input
						type="radio"
						name="harmonyMode"
						value={1}
						bind:group={harmonyMode}
						class="h-4 w-4 accent-cyan-500"
					/>
					<div>
						<span class="font-semibold {harmonyMode === 1 ? 'text-cyan-400' : 'text-neutral-300'}"
							>Driver</span
						>
						<p class="text-xs text-neutral-500">Steedman + Neo-Riemannian + LCC</p>
					</div>
				</label>
			</div>
		</div>
	{/if}

	<div class="flex flex-col items-center gap-4">
		<div class="flex gap-4">
			<button
				onclick={togglePlay}
				class="cursor-pointer rounded-lg px-8 py-4 text-2xl font-semibold transition-colors duration-200
                    {isPlaying
					? 'bg-red-600 hover:bg-red-700'
					: 'bg-purple-700 hover:bg-purple-800'}
                    disabled:cursor-not-allowed disabled:opacity-50"
			>
				{isPlaying ? 'Stop Music' : 'Start Music'}
			</button>
		</div>

		{#if isPlaying}
			<div class="flex gap-4">
				<button
					onclick={toggleWavRecording}
					class="flex cursor-pointer items-center gap-2 rounded-lg px-4 py-2 font-semibold transition-colors duration-200
                        {isRecordingWav
						? 'animate-pulse bg-red-500 text-white hover:bg-red-600'
						: 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'}"
				>
					<div class="h-3 w-3 rounded-full {isRecordingWav ? 'bg-white' : 'bg-red-500'}"></div>
					{isRecordingWav ? 'Stop WAV' : 'Record WAV'}
				</button>

				<button
					onclick={toggleMidiRecording}
					class="flex cursor-pointer items-center gap-2 rounded-lg px-4 py-2 font-semibold transition-colors duration-200
                        {isRecordingMidi
						? 'animate-pulse bg-red-500 text-white hover:bg-red-600'
						: 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'}"
				>
					<div class="h-3 w-3 rounded-full {isRecordingMidi ? 'bg-white' : 'bg-red-500'}"></div>
					{isRecordingMidi ? 'Stop MIDI' : 'Record MIDI'}
				</button>

				<button
					onclick={toggleMusicXmlRecording}
					class="flex cursor-pointer items-center gap-2 rounded-lg px-4 py-2 font-semibold transition-colors duration-200
                        {isRecordingMusicXml
						? 'animate-pulse bg-red-500 text-white hover:bg-red-600'
						: 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'}"
				>
					<div class="h-3 w-3 rounded-full {isRecordingMusicXml ? 'bg-white' : 'bg-red-500'}"></div>
					{isRecordingMusicXml ? 'Stop MusicXML' : 'Record MusicXML'}
				</button>
			</div>
		{/if}
	</div>

	{#if isPlaying && state.key && state.scale}
		<div class="mt-2 flex flex-col items-center gap-2">
			<div class="font-mono text-xl text-purple-300">
				Global Key: {state.key}
				{state.scale}
			</div>
			<div class="flex items-center gap-2">
				<span
					class="rounded px-2 py-1 text-xs {selectedBackend === 'odin2'
						? 'bg-pink-500/30 text-pink-300'
						: 'bg-emerald-500/30 text-emerald-300'}"
				>
					{selectedBackend === 'odin2' ? 'Odin2' : 'FundSP'}
				</span>
				<span class="text-xs text-neutral-500">
					(The "home" tonality - stays constant during session)
				</span>
			</div>
		</div>
	{/if}

	{#if error}
		<div class="mt-4 max-w-md text-center text-red-400">
			{error}
		</div>
	{/if}

	{#if isPlaying}
		<div class="mt-8 w-full max-w-6xl">
			<div class="grid w-full grid-cols-1 gap-8 lg:grid-cols-2">
				<!-- Left: Visualizations -->
				<div class="flex flex-col gap-6">
					{#if bridge}
						<MusicSheet {bridge} steps={32} />
					{/if}

					<RhythmVisualizer
						rhythmMode={state.rhythmMode}
						primarySteps={state.primarySteps}
						primaryPulses={state.primaryPulses}
						primaryRotation={state.primaryRotation}
						primaryPattern={state.primaryPattern}
						secondarySteps={state.secondarySteps}
						secondaryPulses={state.secondaryPulses}
						secondaryRotation={state.secondaryRotation}
						secondaryPattern={state.secondaryPattern}
						currentStep={totalSteps}
						rhythmDensity={state.rhythmDensity}
						rhythmTension={state.rhythmTension}
					/>

					<ChordProgression
						currentChord={state.currentChord}
						currentMeasure={state.currentMeasure}
						isMinorChord={state.isMinorChord}
						progressionName={state.progressionName}
						{progressionChords}
						harmonyMode={state.harmonyMode}
					/>

					{#if bridge}
						{#key bridge}
							<MorphVisualization {bridge} {state} />
						{/key}
					{/if}
				</div>

				<!-- Right: Controls -->
				{#if bridge}
					{#key bridge}
						<ControlPanel {state} {bridge} {isAudioMode} />
					{/key}
				{/if}
			</div>
		</div>
	{/if}
</div>
