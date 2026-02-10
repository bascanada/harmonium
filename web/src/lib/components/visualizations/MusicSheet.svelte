<script lang="ts">
	import { onMount } from 'svelte';
	import type { HarmoniumBridge } from '$lib/bridge';

	interface Props {
		bridge: HarmoniumBridge;
		steps?: number;
		width?: number;
		height?: number;
	}

	let { bridge, steps = 16, width = 600, height = 150 }: Props = $props();

	let canvas: HTMLCanvasElement | null = $state(null);
	let lookaheadData: any = $state(null);

	// Update lookahead data periodically
	async function updateLookahead() {
		const json = bridge.getLookaheadTruth(steps);
		try {
			lookaheadData = JSON.parse(json);
		} catch (e) {
			console.error('Failed to parse lookahead data', e);
		}
	}

	onMount(() => {
		const interval = setInterval(updateLookahead, 100); // 10fps update
		return () => clearInterval(interval);
	});

	// Draw logic
	$effect(() => {
		if (!canvas || !lookaheadData?.events) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		// Clear
		ctx.clearRect(0, 0, width, height);

		// Grid
		ctx.strokeStyle = '#333';
		ctx.lineWidth = 1;
		for (let i = 0; i <= steps; i++) {
			const x = (i / steps) * width;
			ctx.beginPath();
			ctx.moveTo(x, 0);
			ctx.lineTo(x, height);
			ctx.stroke();
		}

		// Draw Note events
		const events = lookaheadData.events;
		const minNote = 36;
		const maxNote = 84;
		const noteRange = maxNote - minNote;

		events.forEach(([offset, event]: [number, any]) => {
			if (event.NoteOn) {
				const { note, channel, velocity } = event.NoteOn;
				const x = (offset / steps) * width;
				const y = height - ((note - minNote) / noteRange) * height;
				
				// Color by channel
				const colors = ['#f87171', '#a78bfa', '#4ade80', '#fbbf24'];
				ctx.fillStyle = colors[channel % colors.length];
				
				// Draw a small block
				const noteWidth = (width / steps) * 0.8;
				ctx.fillRect(x, y - 4, noteWidth, 8);
				
				// Glow for velocity
				ctx.shadowBlur = velocity / 10;
				ctx.shadowColor = ctx.fillStyle;
			}
		});
	});
</script>

<div class="music-sheet-container rounded-xl border border-neutral-700 bg-neutral-900 p-2 shadow-inner">
	<div class="mb-1 flex items-center justify-between px-2">
		<span class="text-[10px] font-bold tracking-widest text-neutral-500 uppercase">Look-ahead Visualization</span>
		<span class="text-[10px] text-neutral-600">Next {steps} steps</span>
	</div>
	<canvas
		bind:this={canvas}
		{width}
		{height}
		class="block w-full"
	></canvas>
</div>

<style>
	.music-sheet-container {
		width: 100%;
	}
	canvas {
		image-rendering: pixelated;
	}
</style>
