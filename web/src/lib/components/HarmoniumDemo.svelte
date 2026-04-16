<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { WasmBridge } from '$lib/bridge/wasm-bridge';
	import {
		type HarmoniumBridge,
		type EngineState,
		createEmptyState
	} from '$lib/bridge/types';
	import ControlPanel from '$lib/components/controls/ControlPanel.svelte';
	import RhythmVisualizer from '$lib/components/visualizations/RhythmVisualizer.svelte';
	import ChordProgression from '$lib/components/visualizations/ChordProgression.svelte';
	import MorphVisualization from '$lib/components/visualizations/MorphVisualization.svelte';
	import init from 'harmonium';

	// Default URL — overridable by the host page via window.__HARMONIUM_SF3_URL
	const SF3_URL = (typeof window !== 'undefined' && (window as any).__HARMONIUM_SF3_URL)
		|| '/soundfonts/musescore-general.sf3';

	let bridge: HarmoniumBridge | null = $state(null);
	let engineState: EngineState = $state(createEmptyState());
	let unsubscribe: (() => void) | null = null;
	let isPlaying = $state(false);
	let isLoadingSF = $state(false);
	let sfLoadProgress = $state('');
	let error = $state('');

	// Algorithm selection (before starting)
	let algorithm = $state(0);
	let polySteps = $state(48);

	// Harmony mode selection (before starting)
	let harmonyMode = $state(1);

	// Detect if we're in audio-rendering mode (web) or MIDI-only mode (VST)
	const isAudioMode = typeof window !== 'undefined' && !('ipc' in window);

	// Step tracking for visualizations
	let totalSteps = $state(0);
	let lastEngineStep = -1;
	let lastPrimarySteps = 16;
	let lastRhythmMode = 0;
	let lastIsEmotionMode = true;

	// Progression chords tracking
	let progressionChords: string[] = $state([]);

	// Recording State
	let isRecordingWav = $state(false);
	let isRecordingMidi = $state(false);
	let isRecordingMusicXml = $state(false);

	// Access WASM handle for recording
	function getHandle() {
		return bridge && (bridge as any).handle;
	}

	function toggleWavRecording() {
		const handle = getHandle();
		if (!handle) return;
		if (isRecordingWav) {
			handle.stop_recording_wav();
			isRecordingWav = false;
		} else {
			handle.start_recording_wav();
			isRecordingWav = true;
		}
	}

	function toggleMidiRecording() {
		const handle = getHandle();
		if (!handle) return;
		if (isRecordingMidi) {
			handle.stop_recording_midi();
			isRecordingMidi = false;
		} else {
			handle.start_recording_midi();
			isRecordingMidi = true;
		}
	}

	function toggleMusicXmlRecording() {
		const handle = getHandle();
		if (!handle) return;
		if (isRecordingMusicXml) {
			handle.stop_recording_musicxml();
			isRecordingMusicXml = false;
		} else {
			handle.start_recording_musicxml();
			isRecordingMusicXml = true;
		}
	}

	function checkRecordings() {
		const handle = getHandle();
		if (!handle) return;

		while (true) {
			const recording = handle.pop_finished_recording();
			if (!recording) break;

			const fmt = recording.format;
			const data = recording.data;

			const mimeType =
				fmt === 'wav' ? 'audio/wav' : fmt === 'midi' ? 'audio/midi' : 'application/xml';
			const ext = fmt === 'wav' ? 'wav' : fmt === 'midi' ? 'mid' : 'musicxml';

			const blob = new Blob([data], { type: mimeType });
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

	// ── IndexedDB cache for SoundFont bytes (persists across reloads) ──
	function openSfCache(): Promise<IDBDatabase> {
		return new Promise((resolve, reject) => {
			const req = indexedDB.open('harmonium-sf-cache', 1);
			req.onupgradeneeded = () => req.result.createObjectStore('sf');
			req.onsuccess = () => resolve(req.result);
			req.onerror = () => reject(req.error);
		});
	}

	async function getCachedSf(): Promise<Uint8Array | undefined> {
		try {
			const db = await openSfCache();
			return new Promise((resolve) => {
				const tx = db.transaction('sf', 'readonly');
				const req = tx.objectStore('sf').get('musescore-general');
				req.onsuccess = () => resolve(req.result ? new Uint8Array(req.result) : undefined);
				req.onerror = () => resolve(undefined);
			});
		} catch { return undefined; }
	}

	async function setCachedSf(bytes: Uint8Array): Promise<void> {
		try {
			const db = await openSfCache();
			const tx = db.transaction('sf', 'readwrite');
			tx.objectStore('sf').put(bytes.buffer, 'musescore-general');
		} catch { /* ignore cache write errors */ }
	}

	async function downloadSoundFont(): Promise<Uint8Array | undefined> {
		// 1. Memory cache (same session)
		if ((globalThis as any).__harmoniumSfCache) {
			console.log('[SF] Using memory cache');
			return (globalThis as any).__harmoniumSfCache;
		}

		// 2. IndexedDB cache (persists across reloads)
		isLoadingSF = true;
		sfLoadProgress = 'Loading SoundFont...';
		const cached = await getCachedSf();
		if (cached) {
			console.log('[SF] Using IndexedDB cache:', cached.length, 'bytes');
			(globalThis as any).__harmoniumSfCache = cached;
			sfLoadProgress = '';
			isLoadingSF = false;
			return cached;
		}

		// 3. Download from server
		try {
			sfLoadProgress = 'Downloading SoundFont...';
			const response = await fetch(SF3_URL);
			if (!response.ok) throw new Error(`HTTP ${response.status}`);
			const total = Number(response.headers.get('content-length') ?? 0);
			const reader = response.body?.getReader();
			if (!reader) throw new Error('No response body');
			const chunks: Uint8Array[] = [];
			let received = 0;
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;
				chunks.push(value);
				received += value.length;
				if (total > 0) {
					sfLoadProgress = `Downloading SoundFont... ${Math.round(received / 1024 / 1024)}/${Math.round(total / 1024 / 1024)}MB`;
				}
			}
			const bytes = new Uint8Array(received);
			let offset = 0;
			for (const chunk of chunks) {
				bytes.set(chunk, offset);
				offset += chunk.length;
			}
			(globalThis as any).__harmoniumSfCache = bytes;
			await setCachedSf(bytes);
			console.log('[SF] Downloaded, cached in IndexedDB:', bytes.length, 'bytes');
			sfLoadProgress = '';
			isLoadingSF = false;
			return bytes;
		} catch (e) {
			console.warn('SoundFont download failed, will use FundSP fallback:', e);
			sfLoadProgress = '';
			isLoadingSF = false;
			return undefined;
		}
	}

	onMount(() => {
		const interval = setInterval(checkRecordings, 1000);

		(async () => {
			try {
				await init();
			} catch (e) {
				console.warn('WASM init warning:', e);
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

			// Download SoundFont (cached after first download)
			const sfBytes = await downloadSoundFont();
			console.log('[SF] Connecting with SF bytes:', sfBytes ? sfBytes.length : 'none');

			// Show loading state while WASM decompresses the SF3 (blocks main thread)
			sfLoadProgress = 'Initializing audio engine...';
			isLoadingSF = true;

			// Yield to let the UI update before the blocking WASM call
			await new Promise(r => setTimeout(r, 50));

			bridge = new WasmBridge();
			await bridge.connect(sfBytes, 'fundsp');

			sfLoadProgress = '';
			isLoadingSF = false;

			bridge.setAlgorithm(algorithm);
			bridge.setHarmonyMode(harmonyMode);
			bridge.setPolySteps(polySteps);

			bridge.setArousal(0.5);
			bridge.setValence(0.3);
			bridge.setDensity(0.5);
			bridge.setTension(0.3);

			unsubscribe = bridge.subscribe((newState) => {
				const rhythmModeChanged = newState.rhythmMode !== lastRhythmMode;
				const stepsChanged = newState.primarySteps !== lastPrimarySteps;
				const emotionModeChanged = newState.isEmotionMode !== lastIsEmotionMode;

				if (rhythmModeChanged || stepsChanged || emotionModeChanged) {
					lastEngineStep = -1;
					lastPrimarySteps = newState.primarySteps;
					lastRhythmMode = newState.rhythmMode;
					lastIsEmotionMode = newState.isEmotionMode;
				}

				const rawStep = newState.currentStep;
				if (rawStep !== lastEngineStep) {
					let delta = rawStep - lastEngineStep;
					if (delta < 0) {
						delta += newState.primarySteps;
					}
					if (lastEngineStep === -1) {
						totalSteps = rawStep;
					} else {
						totalSteps += delta;
					}
					lastEngineStep = rawStep;
				}

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

				engineState = newState;
			});

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
	class="flex min-h-screen flex-col items-center justify-center p-8"
	style="background: var(--h-bg); color: var(--h-fg); font-family: var(--h-font-body);"
>
	<h1 class="mb-2 text-4xl font-bold" style="font-family: var(--h-font-display); color: var(--h-amber);">Harmonium</h1>
	<p class="mb-8" style="color: var(--h-fg-2);">Morphing Music Engine</p>

	{#if isLoadingSF}
		<div class="mb-6 text-center" style="color: var(--h-fg-2);">
			<p>{sfLoadProgress}</p>
		</div>
	{/if}

	{#if !isPlaying}
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
				disabled={isLoadingSF}
				class="cursor-pointer rounded-lg px-8 py-4 text-2xl font-semibold transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-50"
				style="background: {isPlaying ? 'var(--h-dominant)' : 'var(--h-amber)'}; color: var(--h-bg); border-radius: var(--h-radius-sm);"
			>
				{isLoadingSF ? 'Loading...' : isPlaying ? 'Stop Music' : 'Start Music'}
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

	{#if isPlaying && engineState.key && engineState.scale}
		<div class="mt-2 flex flex-col items-center gap-2">
			<div class="text-xl" style="font-family: var(--h-font-mono); color: var(--h-amber);">
				Global Key: {engineState.key}
				{engineState.scale}
			</div>
			<div class="flex items-center gap-2">
				<span class="rounded px-2 py-1 text-xs" style="background: color-mix(in srgb, var(--h-tonic) 20%, transparent); color: var(--h-tonic);">
					SoundFont
				</span>
				<span class="text-xs" style="color: var(--h-fg-3);">
					(The "home" tonality - stays constant during session)
				</span>
			</div>
			<!-- Live status bar -->
			<div class="mt-1 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs" style="font-family: var(--h-font-mono); color: var(--h-fg-2);">
				<span>Gen Bar <span style="color: var(--h-fg);">{engineState.currentMeasure}</span> | Step <span style="color: var(--h-fg);">{engineState.currentStep}/{engineState.primarySteps}</span></span>
				<span style="color: var(--h-fg-3);">|</span>
				<span>{engineState.bpm} bpm</span>
				<span style="color: var(--h-fg-3);">|</span>
				<span>A<span style="color: var(--h-amber);">{engineState.arousal.toFixed(2)}</span> V<span style="color: var(--h-amber);">{engineState.valence.toFixed(2)}</span> D<span style="color: var(--h-amber);">{engineState.density.toFixed(2)}</span> T<span style="color: var(--h-amber);">{engineState.tension.toFixed(2)}</span></span>
				<span style="color: var(--h-fg-3);">|</span>
				<span style="color: var(--h-fg-3);">{engineState.isEmotionMode ? 'EMO' : 'DIR'}</span>
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
					<RhythmVisualizer
						rhythmMode={engineState.rhythmMode}
						primarySteps={engineState.primarySteps}
						primaryPulses={engineState.primaryPulses}
						primaryRotation={engineState.primaryRotation}
						primaryPattern={engineState.primaryPattern}
						secondarySteps={engineState.secondarySteps}
						secondaryPulses={engineState.secondaryPulses}
						secondaryRotation={engineState.secondaryRotation}
						secondaryPattern={engineState.secondaryPattern}
						currentStep={engineState.currentStep}
						rhythmDensity={engineState.rhythmDensity}
						rhythmTension={engineState.rhythmTension}
					/>

					<ChordProgression
						currentChord={engineState.currentChord}
						currentMeasure={engineState.currentMeasure}
						isMinorChord={engineState.isMinorChord}
						progressionName={engineState.progressionName}
						{progressionChords}
						harmonyMode={engineState.harmonyMode}
					/>

					{#if bridge}
						{#key bridge}
							<MorphVisualization {bridge} state={engineState} />
						{/key}
					{/if}
				</div>

				<!-- Right: Controls -->
				{#if bridge}
					{#key bridge}
						<ControlPanel state={engineState} {bridge} />
					{/key}
				{/if}
			</div>
		</div>
	{/if}
</div>
