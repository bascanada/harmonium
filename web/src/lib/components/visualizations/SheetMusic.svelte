<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Renderer,
		Stave,
		StaveNote,
		Voice,
		Formatter,
		Accidental,
		StaveConnector
	} from 'vexflow';
	import type { HarmoniumBridge } from '$lib/bridge';
	import type { HarmoniumScore, KeySignature, ScoreNoteEvent } from '$lib/types/notation';
	import { pitchesToVexFlow, durationToVexFlow } from '$lib/utils/notation';
	import {
		currentScore,
		playingNoteIds,
		scoreLoading,
		scoreError,
		loadScore
	} from '$lib/stores/playback';
	import { engineState } from '$lib/stores/engine-state';

	interface Props {
		bridge: HarmoniumBridge;
		/** Number of bars to pre-generate for the score. */
		bars?: number;
	}

	let { bridge, bars = 8 }: Props = $props();

	let scrollContainer: HTMLDivElement | undefined = $state(undefined);

	// Layout constants
	const STAVE_HEIGHT = 90;
	const FIRST_STAVE_EXTRA = 80; // space for clef + key sig + time sig
	const MEASURE_WIDTH = 220;
	const LEAD_Y = 20;
	const BASS_Y = 150;
	const SVG_HEIGHT = 270;
	const MARGIN = 10;
	const PLAYHEAD_OFFSET_X = 120; // fixed playhead position from left

	// Map from ScoreNoteEvent.id → SVG <g> element, populated after each VexFlow render.
	// Plain (non-reactive) variable: the highlight $effect re-runs at ~60fps via $playingNoteIds,
	// so fine-grained Map tracking is not needed here.
	let noteElementMap = new Map<number, SVGElement>();

	// ─────────────────────────────────────────────────────────
	// Lifecycle
	// ─────────────────────────────────────────────────────────

	onMount(async () => {
		await loadScore(bridge, bars);
	});

	// ─────────────────────────────────────────────────────────
	// Reactive: update highlights at ~60fps (no VexFlow re-draw)
	// Direct inline-style fill is used instead of CSS classes to
	// reliably override VexFlow's SVG presentation attributes.
	// ─────────────────────────────────────────────────────────

	$effect(() => {
		const playing = $playingNoteIds;
		for (const [id, el] of noteElementMap) {
			const active = playing.has(id);
			el.querySelectorAll<SVGElement>('path, rect').forEach((shape) => {
				shape.style.fill = active ? '#a78bfa' : '';
			});
		}
	});

	// ─────────────────────────────────────────────────────────
	// Reactive: Auto-scroll to keep current position visible
	// ─────────────────────────────────────────────────────────

	$effect(() => {
		const score = $currentScore;
		const state = $engineState;
		if (!score || !scrollContainer || !state.currentMeasure) return;

		const totalMeasures = score.parts[0]?.measures.length ?? 0;
		if (totalMeasures === 0) return;

		// Normalized measure for looping (0..totalMeasures-1)
		const m = (state.currentMeasure - 1) % totalMeasures;
		const beatsPerMeasure = score.time_signature[0];

		// Beat progress within measure (0..1)
		const beatProgress = state.currentStep / (beatsPerMeasure * 4); // 4 steps per beat

		// Calculate current X in pixels
		const currentPx = MARGIN + FIRST_STAVE_EXTRA + (m + beatProgress) * MEASURE_WIDTH;

		// Smooth scroll update
		scrollContainer.scrollTo({
			left: currentPx - PLAYHEAD_OFFSET_X,
			behavior: 'auto'
		});
	});

	// ─────────────────────────────────────────────────────────
	// VexFlow Action
	// ─────────────────────────────────────────────────────────

	function renderScoreAction(node: HTMLElement) {
		$effect(() => {
			const score = $currentScore;
			if (!score) return;

			// Clear previous render and element map
			node.innerHTML = '';
			noteElementMap = new Map<number, SVGElement>();

			const leadPart = score.parts.find((p) => p.id === 'lead');
			const bassPart = score.parts.find((p) => p.id === 'bass');

			if (!leadPart && !bassPart) return;

			const numMeasures = Math.max(leadPart?.measures.length ?? 0, bassPart?.measures.length ?? 0);
			if (numMeasures === 0) return;

			const hasBoth = !!leadPart && !!bassPart;
			const totalHeight = hasBoth ? SVG_HEIGHT : STAVE_HEIGHT + 60;
			const totalWidth = MARGIN + FIRST_STAVE_EXTRA + numMeasures * MEASURE_WIDTH + MARGIN;

			const renderer = new Renderer(node, Renderer.Backends.SVG);
			renderer.resize(totalWidth, totalHeight);
			const ctx = renderer.getContext();

			const [beats, beatValue] = score.time_signature;
			const keyName = vexFlowKey(score.key_signature);

			for (let i = 0; i < numMeasures; i++) {
				const isFirst = i === 0;
				const measureNum = i + 1;

				const x = isFirst ? MARGIN : MARGIN + FIRST_STAVE_EXTRA + i * MEASURE_WIDTH;
				const staveWidth = isFirst ? MEASURE_WIDTH + FIRST_STAVE_EXTRA : MEASURE_WIDTH;

				let leadStave: Stave | undefined;
				if (leadPart) {
					leadStave = new Stave(x, LEAD_Y, staveWidth);
					if (isFirst) {
						leadStave
							.addClef('treble')
							.addKeySignature(keyName)
							.addTimeSignature(`${beats}/${beatValue}`);
					}
					leadStave.setContext(ctx).draw();
				}

				let bassStave: Stave | undefined;
				if (bassPart) {
					bassStave = new Stave(x, BASS_Y, staveWidth);
					if (isFirst) {
						bassStave
							.addClef('bass')
							.addKeySignature(keyName)
							.addTimeSignature(`${beats}/${beatValue}`);
					}
					bassStave.setContext(ctx).draw();
				}

				if (isFirst && leadStave && bassStave) {
					new StaveConnector(leadStave, bassStave).setType('brace').setContext(ctx).draw();
					new StaveConnector(leadStave, bassStave).setType('single').setContext(ctx).draw();
				}

				if (leadPart && leadStave) {
					const measure = leadPart.measures.find((m) => m.number === measureNum);
					const events = (measure?.events ?? []).sort((a, b) => a.beat - b.beat);

					if (events.length > 0) {
						const tickables = events.map((e) => createStaveNote(e, 'treble'));

						const voice = new Voice({ numBeats: beats, beatValue: beatValue });
						voice.setMode(Voice.Mode.SOFT);
						voice.addTickables(tickables);

						new Formatter().joinVoices([voice]).format([voice], staveWidth - 30);
						voice.draw(ctx, leadStave);

						events.forEach((event) => {
							const el = node.querySelector<SVGElement>(`[id="vf-hn-${event.id}"]`);
							if (el) noteElementMap.set(event.id, el);
						});
					}
				}

				if (bassPart && bassStave) {
					const measure = bassPart.measures.find((m) => m.number === measureNum);
					const events = (measure?.events ?? []).sort((a, b) => a.beat - b.beat);

					if (events.length > 0) {
						const tickables = events.map((e) => createStaveNote(e, 'bass'));

						const voice = new Voice({ numBeats: beats, beatValue: beatValue });
						voice.setMode(Voice.Mode.SOFT);
						voice.addTickables(tickables);

						new Formatter().joinVoices([voice]).format([voice], staveWidth - 30);
						voice.draw(ctx, bassStave);

						events.forEach((event) => {
							const el = node.querySelector<SVGElement>(`[id="vf-hn-${event.id}"]`);
							if (el) noteElementMap.set(event.id, el);
						});
					}
				}
			}
		});
	}

	// ─────────────────────────────────────────────────────────
	// VexFlow helpers
	// ─────────────────────────────────────────────────────────

	/** Map HarmoniumScore alter value to a VexFlow accidental string. */
	function alterToVexFlow(alter: number): string {
		if (alter === 2) return '##';
		if (alter === 1) return '#';
		if (alter === -1) return 'b';
		if (alter === -2) return 'bb';
		return 'n';
	}

	/** Build a VexFlow key name from a KeySignature. */
	function vexFlowKey(keySig: KeySignature): string {
		return keySig.mode === 'minor' ? `${keySig.root}m` : keySig.root;
	}

	/**
	 * Create a VexFlow StaveNote from a ScoreNoteEvent.
	 * The note is tagged with `id="hn-{event.id}"` so it can be queried
	 * from the DOM after rendering for highlight updates.
	 */
	function createStaveNote(event: ScoreNoteEvent, clef: string): StaveNote {
		const dur = durationToVexFlow(event.duration);
		const isRest = event.type === 'rest' || !event.pitches || event.pitches.length === 0;

		let note: StaveNote;
		if (isRest) {
			// VexFlow rest notes: 'b/4' is the canonical pitch for treble rests.
			const restKey = clef === 'bass' ? 'd/3' : 'b/4';
			note = new StaveNote({ keys: [restKey], duration: `${dur}r`, clef });
		} else {
			const keys = pitchesToVexFlow(event.pitches!);
			note = new StaveNote({ keys, duration: dur, clef });
			// Add accidental modifiers for altered pitches.
			event.pitches!.forEach((pitch, i) => {
				const alter = pitch.alter ?? 0;
				if (alter !== 0) {
					note.addModifier(new Accidental(alterToVexFlow(alter)), i);
				}
			});
		}

		// Tag with a stable DOM id for highlight lookup after rendering.
		// VexFlow's SVGContext prefixes all ids with "vf-", so the rendered
		// element will have id="vf-hn-{event.id}" in the DOM.
		note.setAttribute('id', `hn-${event.id}`);
		return note;
	}
</script>

<div class="sheet-music-wrapper">
	<div class="sheet-music-header">
		<span class="sheet-music-title">Sheet Music</span>
		{#if $scoreLoading}
			<span class="status-badge loading">Loading...</span>
		{:else if $scoreError}
			<span class="status-badge error">Error</span>
		{:else if $currentScore}
			<span class="status-badge ok">
				{$currentScore.key_signature.root}
				{$currentScore.key_signature.mode} · {$currentScore.tempo} BPM
			</span>
		{/if}
	</div>

	{#if $scoreError}
		<p class="error-message">{$scoreError}</p>
	{/if}

	<div class="vexflow-viewport">
		<div bind:this={scrollContainer} class="vexflow-scroll">
			<div use:renderScoreAction class="vexflow-canvas"></div>
		</div>
		<div class="playhead" style="left: {PLAYHEAD_OFFSET_X}px;"></div>
	</div>
</div>

<style>
	.sheet-music-wrapper {
		background: white;
		border-radius: 8px;
		overflow: hidden;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
		position: relative;
	}

	.sheet-music-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: #f8f8f8;
		border-bottom: 1px solid #e0e0e0;
	}

	.sheet-music-title {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #555;
	}

	.status-badge {
		font-size: 0.7rem;
		padding: 2px 8px;
		border-radius: 999px;
	}

	.status-badge.loading {
		background: #e2e8f0;
		color: #4a5568;
	}

	.status-badge.error {
		background: #fed7d7;
		color: #c53030;
	}

	.status-badge.ok {
		background: #c6f6d5;
		color: #276749;
	}

	.error-message {
		padding: 8px 12px;
		font-size: 0.75rem;
		color: #c53030;
	}

	.vexflow-viewport {
		position: relative;
		overflow: hidden;
	}

	.vexflow-scroll {
		overflow-x: auto;
		scrollbar-width: none; /* Hide scrollbar Firefox */
	}

	.vexflow-scroll::-webkit-scrollbar {
		display: none; /* Hide scrollbar Chrome/Safari */
	}

	.vexflow-canvas {
		min-height: 120px;
	}

	.playhead {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 2px;
		background: rgba(167, 139, 250, 0.5); /* Semi-transparent violet */
		pointer-events: none;
		z-index: 10;
		box-shadow: 0 0 4px rgba(167, 139, 250, 0.8);
	}
</style>
