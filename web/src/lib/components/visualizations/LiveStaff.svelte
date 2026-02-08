<script lang="ts">
	import { onMount, afterUpdate } from 'svelte';
	import { Renderer, RendererBackends, Stave, StaveNote, Voice, Formatter, Accidental } from 'vexflow';
	import { lookAheadBuffer, engineState } from '$lib/stores/engine-state';
	import type { ScheduledStep } from '$lib/bridge/types';

	const MOCK_MODE = false;

	let container: HTMLDivElement;
	let renderer: Renderer;
	let context: any;

	const WIDTH = 600;
	const HEIGHT = 150;

	// Map MIDI note to VexFlow keys (e.g., 60 -> 'c/4')
	function midiToVex(midi: number): string {
		const notes = ['c', 'c#', 'd', 'd#', 'e', 'f', 'f#', 'g', 'g#', 'a', 'a#', 'b'];
		const octave = Math.floor(midi / 12) - 1;
		const name = notes[midi % 12];
		return `${name}/${octave}`;
	}

	function renderStaff() {
		if (!context) return;

		let steps: ScheduledStep[] = $lookAheadBuffer.slice(0, 16);

		if (MOCK_MODE) {
			steps = Array(16)
				.fill(null)
				.map((_, i) => ({
					absoluteStep: i,
					trigger: {
						kick: false,
						snare: false,
						hat: false,
						bass: false,
						lead: i % 4 === 0,
						velocity: 1
					},
					pitches: [null, 60 + (i % 12), null, null, null]
				}));
		}

		if (steps.length === 0) {
			context.clear();
			return;
		}

		context.clear();

		// Create a stave of width WIDTH-20 at position 10, 40 on the canvas.
		const stave = new Stave(10, 40, WIDTH - 20);

		// Add a clef and time signature.
		stave.addClef('treble').addTimeSignature('4/4');

		// Connect it to the rendering context and draw!
		stave.setContext(context).draw();

		// Convert buffer steps to VexFlow notes
		// We'll take the first 16 steps (one 4/4 measure)
		const notes: StaveNote[] = [];

		for (let i = 0; i < steps.length; i++) {
			const step = steps[i];
			const leadPitch = step.pitches[1]; // Lead is index 1

			if (leadPitch !== null && leadPitch !== undefined) {
				const vexKey = midiToVex(leadPitch);
				const note = new StaveNote({
					keys: [vexKey],
					duration: '16',
					clef: 'treble'
				});

				// Add accidental if needed
				if (vexKey.includes('#')) {
					note.addModifier(new Accidental('#'), 0);
				}

				notes.push(note);
			} else {
				// Add a rest
				notes.push(new StaveNote({ keys: ['b/4'], duration: '16r', clef: 'treble' }));
			}
		}

		if (notes.length > 0) {
			// Create a voice in 4/4 and add the notes from above
			const voice = new Voice({ num_beats: 4, beat_value: 4 });
			voice.addTickables(notes);

			// Format and justify the notes to 500 pixels.
			new Formatter().joinVoices([voice]).format([voice], WIDTH - 100);

			// Render voice
			voice.draw(context, stave);
		}
	}

	onMount(() => {
		renderer = new Renderer(container, RendererBackends.SVG);
		renderer.resize(WIDTH, HEIGHT);
		context = renderer.getContext();
		renderStaff();
	});

	// Reactive update when buffer changes
	$: if ($lookAheadBuffer) {
		renderStaff();
	}
</script>

<div class="rounded-lg border border-neutral-700 bg-neutral-800 p-4 shadow-xl">
	<div class="mb-2 flex items-center justify-between">
		<h3 class="text-xs font-bold uppercase tracking-wider text-neutral-400">Live Staff (Lead)</h3>
		<span class="font-mono text-[10px] text-neutral-500">
			{$engineState.key} {$engineState.scale}
		</span>
	</div>

	<div
		bind:this={container}
		class="staff-container flex justify-center overflow-hidden rounded bg-white/90 p-2"
	></div>

	<p class="mt-2 text-center text-[9px] text-neutral-500">
		Real-time VexFlow rendering from procedural look-ahead buffer
	</p>
</div>

<style>
	.staff-container :global(svg) {
		max-width: 100%;
		height: auto;
	}
</style>
