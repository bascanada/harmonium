<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { HarmoniumBridge } from '$lib/bridge/base-bridge';
	import type { NoteEvent } from '$lib/bridge/types';

	export let bridge: HarmoniumBridge;

	let canvas: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D;
	let animationId: number;

	// Ring buffer for note history
	const maxNotes = 256; // ~8 measures at 32 steps
	let noteBuffer: NoteEvent[] = [];

	// Canvas dimensions
	const canvasWidth = 1200;
	const canvasHeight = 400;

	// MIDI range to display (3 octaves)
	const minMidi = 36; // C2
	const maxMidi = 84; // C6

	// Colors by instrument
	const instrumentColors = {
		0: '#ff6b6b', // Bass - red
		1: '#4ecdc4', // Lead - cyan
		2: '#ffe66d', // Snare - yellow
		3: '#a8dadc' // Hat - light blue
	};

	onMount(() => {
		ctx = canvas.getContext('2d')!;
		startRendering();

		// Subscribe to note events
		const unsubscribe = bridge.subscribeToEvents((newEvents) => {
			for (const event of newEvents) {
				noteBuffer.push(event);
				if (noteBuffer.length > maxNotes) {
					noteBuffer.shift(); // Remove oldest
				}
			}
		});

		return () => {
			unsubscribe();
			if (animationId) cancelAnimationFrame(animationId);
		};
	});

	onDestroy(() => {
		if (animationId) cancelAnimationFrame(animationId);
	});

	function startRendering() {
		const render = () => {
			if (!ctx) return;

			const now = performance.now();

			// Clear canvas
			ctx.fillStyle = '#0a0a0a';
			ctx.fillRect(0, 0, canvasWidth, canvasHeight);

			// Draw grid
			drawGrid();

			// Draw notes with age-based alpha and horizontal scrolling
			for (const note of noteBuffer) {
				const age = now - note.timestamp;
				const alpha = Math.max(0, 1.0 - age / 4000); // Fade over 4 seconds

				if (alpha > 0) {
					drawNote(note, alpha, age);
				}
			}

			animationId = requestAnimationFrame(render);
		};
		render();
	}

	function drawGrid() {
		ctx.strokeStyle = '#1a1a1a';
		ctx.lineWidth = 1;

		// Horizontal lines (octaves)
		for (let midi = minMidi; midi <= maxMidi; midi += 12) {
			const y = midiToY(midi);
			ctx.strokeStyle = midi % 12 === 0 ? '#2a2a2a' : '#1a1a1a';
			ctx.beginPath();
			ctx.moveTo(0, y);
			ctx.lineTo(canvasWidth, y);
			ctx.stroke();
		}

		// Vertical lines (beats) - these will scroll with time
		const beatsPerScreen = 8; // Show 2 measures (assuming 4 beats per measure)
		const pixelsPerBeat = canvasWidth / beatsPerScreen;

		for (let i = 0; i <= beatsPerScreen; i++) {
			const x = i * pixelsPerBeat;
			ctx.strokeStyle = i % 4 === 0 ? '#2a2a2a' : '#1a1a1a';
			ctx.beginPath();
			ctx.moveTo(x, 0);
			ctx.lineTo(x, canvasHeight);
			ctx.stroke();
		}
	}

	function drawNote(note: NoteEvent, alpha: number, age: number) {
		// Calculate position
		const y = midiToY(note.note);
		const height = 8;

		// Horizontal scrolling: notes move from right to left over 4 seconds
		const scrollDuration = 4000; // 4 seconds to traverse the screen
		const progress = age / scrollDuration; // 0 (just appeared) to 1 (leaving screen)
		const x = canvasWidth * (1 - progress); // Start at right edge, move left

		// Width based on duration (convert samples to approximate pixels)
		// Assuming 44100 Hz sample rate and 125 BPM
		const width = Math.max(5, (note.duration / 48000) * 100); // Adjust multiplier as needed

		// Get color by instrument
		const color = instrumentColors[note.instrument as keyof typeof instrumentColors] || '#666';

		// Draw note bar with glow
		ctx.fillStyle = color;
		ctx.globalAlpha = alpha;

		// Add glow effect
		ctx.shadowBlur = 10 * alpha;
		ctx.shadowColor = color;

		// Draw rounded rectangle
		drawRoundedRect(x, y - height / 2, width, height, 3);

		// Reset shadow
		ctx.shadowBlur = 0;
		ctx.globalAlpha = 1.0;
	}

	function drawRoundedRect(x: number, y: number, width: number, height: number, radius: number) {
		ctx.beginPath();
		ctx.moveTo(x + radius, y);
		ctx.lineTo(x + width - radius, y);
		ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
		ctx.lineTo(x + width, y + height - radius);
		ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
		ctx.lineTo(x + radius, y + height);
		ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
		ctx.lineTo(x, y + radius);
		ctx.quadraticCurveTo(x, y, x + radius, y);
		ctx.closePath();
		ctx.fill();
	}

	function midiToY(midi: number): number {
		// Map MIDI to canvas Y coordinate (inverted: lower notes at bottom)
		const range = maxMidi - minMidi;
		const normalized = (midi - minMidi) / range; // 0 (bottom) to 1 (top)
		return canvasHeight - normalized * canvasHeight;
	}
</script>

<div class="live-score-container">
	<h3 class="title">🎹 Live Score</h3>
	<div class="canvas-wrapper">
		<canvas bind:this={canvas} width={canvasWidth} height={canvasHeight}></canvas>
		<div class="legend">
			<div class="legend-item">
				<span class="dot" style="background-color: #ff6b6b;"></span>
				<span>Bass</span>
			</div>
			<div class="legend-item">
				<span class="dot" style="background-color: #4ecdc4;"></span>
				<span>Lead</span>
			</div>
			<div class="legend-item">
				<span class="dot" style="background-color: #ffe66d;"></span>
				<span>Snare</span>
			</div>
			<div class="legend-item">
				<span class="dot" style="background-color: #a8dadc;"></span>
				<span>Hat</span>
			</div>
		</div>
	</div>
</div>

<style>
	.live-score-container {
		background: rgba(0, 0, 0, 0.3);
		border-radius: 12px;
		padding: 1.5rem;
		margin: 1rem 0;
	}

	.title {
		margin: 0 0 1rem 0;
		color: #fff;
		font-size: 1.2rem;
		font-weight: 600;
	}

	.canvas-wrapper {
		position: relative;
		border-radius: 8px;
		overflow: hidden;
		border: 2px solid rgba(255, 255, 255, 0.1);
	}

	canvas {
		display: block;
		width: 100%;
		height: auto;
		background: #0a0a0a;
	}

	.legend {
		display: flex;
		gap: 1.5rem;
		padding: 0.75rem;
		background: rgba(0, 0, 0, 0.5);
		justify-content: center;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #fff;
		font-size: 0.875rem;
	}

	.dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		display: inline-block;
	}
</style>
