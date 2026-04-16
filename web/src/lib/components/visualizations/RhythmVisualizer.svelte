<script lang="ts">
	/**
	 * RhythmVisualizer — circular step sequencer display.
	 *
	 * Euclidean mode: two circles side by side (primary + secondary polyrhythm)
	 * PerfectBalance / ClassicGroove: single circle with pattern polygon
	 */

	let {
		rhythmMode = 0,
		primarySteps = 16,
		primaryPulses = 4,
		primaryRotation = 0,
		primaryPattern = [] as boolean[],
		secondarySteps = 12,
		secondaryPulses = 3,
		secondaryRotation = 0,
		secondaryPattern = [] as boolean[],
		currentStep = 0,
		currentBar = 1,
		rhythmDensity = 0.5,
		rhythmTension = 0.3,
		size = 280,
	} = $props();

	const MODE_NAMES = ['Euclidean', 'PerfectBalance', 'ClassicGroove'];
	const PRIMARY_COLORS: Record<number, string> = {
		0: 'var(--h-amber, #e2a832)',
		1: 'var(--h-tonic, #5a9e6f)',
		2: 'var(--h-copper, #c67d4a)',
	};
	const SECONDARY_COLOR = 'var(--h-chord-tone, #5b8fd4)';

	// Bjorklund fallback
	function bjorklund(steps: number, pulses: number): boolean[] {
		const pat = new Array(steps).fill(false);
		if (steps === 0 || pulses === 0) return pat;
		let bucket = 0;
		for (let i = 0; i < steps; i++) {
			bucket += pulses;
			if (bucket >= steps) { bucket -= steps; pat[i] = true; }
		}
		return pat;
	}

	function rotate(pat: boolean[], rot: number): boolean[] {
		if (rot === 0 || pat.length === 0) return pat;
		const n = pat.length;
		const r = ((rot % n) + n) % n;
		return [...pat.slice(n - r), ...pat.slice(0, n - r)];
	}

	// ── Resolve patterns ──
	const primaryColor = $derived(PRIMARY_COLORS[rhythmMode] ?? PRIMARY_COLORS[0]);

	const primaryPat = $derived(
		primaryPattern.length >= primarySteps
			? primaryPattern.slice(0, primarySteps)
			: rotate(bjorklund(primarySteps, primaryPulses), primaryRotation)
	);

	const secondaryPat = $derived(
		secondaryPattern.length >= secondarySteps
			? secondaryPattern.slice(0, secondarySteps)
			: rotate(bjorklund(secondarySteps, secondaryPulses), secondaryRotation)
	);

	const showDual = $derived(rhythmMode === 0 && secondarySteps > 0);

	// ── Circle geometry helper ──
	function circlePoints(steps: number, pat: boolean[], step: number, r: number, cx: number, cy: number) {
		if (steps === 0) return { dots: [], polygon: '', cursor: null as { x: number; y: number } | null };
		const dots = Array.from({ length: steps }).map((_, i) => {
			const angle = (i * 2 * Math.PI) / steps - Math.PI / 2;
			return {
				x: cx + r * Math.cos(angle),
				y: cy + r * Math.sin(angle),
				active: pat[i] ?? false,
				isCurrent: (step % steps) === i,
			};
		});
		const polygon = dots.filter(d => d.active).map(d => `${d.x},${d.y}`).join(' ');
		const cursorIdx = step % steps;
		const cursor = dots[cursorIdx] ?? null;
		return { dots, polygon, cursor };
	}

	// ── Primary ──
	const primaryCx = $derived(showDual ? 27 : 50);
	const primaryR = $derived(showDual ? 22 : 40);
	const primary = $derived(circlePoints(primarySteps, primaryPat, currentStep, primaryR, primaryCx, 50));

	// ── Secondary (Euclidean only) ──
	// Secondary runs at its own cycle rate — compute step from bar position
	const secondaryCursorStep = $derived(
		secondarySteps > 0 && primarySteps > 0
			? Math.floor((currentStep / primarySteps) * secondarySteps)
			: 0
	);
	const secondary = $derived(
		showDual
			? circlePoints(secondarySteps, secondaryPat, secondaryCursorStep, 22, 73, 50)
			: { dots: [], polygon: '', cursor: null }
	);

	const activeCount = $derived(primaryPat.filter(Boolean).length);
	const secondaryActiveCount = $derived(secondaryPat.filter(Boolean).length);
</script>

<div class="rhythm-viz" style="width: {size}px; height: {size}px;">
	<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">

		<!-- ══ PRIMARY CIRCLE ══ -->
		<!-- Guide circle -->
		<circle cx={primaryCx} cy="50" r={primaryR} fill="none"
			stroke="var(--h-border, #2d2d4a)" stroke-width="0.4" />

		<!-- Polygon -->
		{#if primary.polygon}
			<polygon points={primary.polygon}
				fill={primaryColor} fill-opacity="0.08"
				stroke={primaryColor} stroke-width="1.2" />
		{/if}

		<!-- Dots -->
		{#each primary.dots as pt}
			<circle cx={pt.x} cy={pt.y}
				r={pt.isCurrent ? 2.8 : (pt.active ? 1.8 : 0.9)}
				fill={pt.active ? primaryColor : 'var(--h-fg-3, #5e5a70)'}
				opacity={pt.active ? 1 : 0.25} />
			{#if pt.isCurrent && pt.active}
				<circle cx={pt.x} cy={pt.y} r="4.5" fill="none"
					stroke={primaryColor} stroke-width="0.8" opacity="0.5" />
			{/if}
		{/each}

		<!-- Cursor -->
		{#if primary.cursor}
			<line x1={primaryCx} y1="50" x2={primary.cursor.x} y2={primary.cursor.y}
				stroke="var(--h-fg, #f0ece4)" stroke-width="0.5"
				stroke-dasharray="1.5,1.5" opacity="0.4" />
			<circle cx={primaryCx} cy="50" r="1.2"
				fill="var(--h-fg, #f0ece4)" opacity="0.3" />
		{/if}

		<!-- Primary label -->
		<text x={primaryCx} y={showDual ? 48 : 49} text-anchor="middle" dominant-baseline="central"
			font-size={showDual ? 3.5 : 4} font-weight="600"
			font-family="var(--h-font-mono, monospace)"
			fill={primaryColor} opacity="0.7">
			{#if rhythmMode === 0}
				E({primarySteps},{activeCount})
			{:else}
				{MODE_NAMES[rhythmMode]}
			{/if}
		</text>
		<text x={primaryCx} y={showDual ? 52 : 54} text-anchor="middle" dominant-baseline="central"
			font-size={showDual ? 2.5 : 3}
			font-family="var(--h-font-mono, monospace)"
			fill="var(--h-fg-3, #5e5a70)">
			{#if rhythmMode === 0}
				r{primaryRotation}
			{:else}
				d{(rhythmDensity * 100).toFixed(0)}% t{(rhythmTension * 100).toFixed(0)}%
			{/if}
		</text>

		<!-- ══ SECONDARY CIRCLE (Euclidean dual mode) ══ -->
		{#if showDual}
			<!-- Guide circle -->
			<circle cx="73" cy="50" r="22" fill="none"
				stroke="var(--h-border, #2d2d4a)" stroke-width="0.4" />

			<!-- Polygon -->
			{#if secondary.polygon}
				<polygon points={secondary.polygon}
					fill={SECONDARY_COLOR} fill-opacity="0.06"
					stroke={SECONDARY_COLOR} stroke-width="0.8" stroke-opacity="0.6" />
			{/if}

			<!-- Dots -->
			{#each secondary.dots as pt}
				<circle cx={pt.x} cy={pt.y}
					r={pt.isCurrent ? 2.4 : (pt.active ? 1.5 : 0.8)}
					fill={pt.active ? SECONDARY_COLOR : 'var(--h-fg-3, #5e5a70)'}
					opacity={pt.active ? 1 : 0.25} />
				{#if pt.isCurrent && pt.active}
					<circle cx={pt.x} cy={pt.y} r="4" fill="none"
						stroke={SECONDARY_COLOR} stroke-width="0.7" opacity="0.5" />
				{/if}
			{/each}

			<!-- Cursor -->
			{#if secondary.cursor}
				<line x1="73" y1="50" x2={secondary.cursor.x} y2={secondary.cursor.y}
					stroke="var(--h-fg, #f0ece4)" stroke-width="0.5"
					stroke-dasharray="1.5,1.5" opacity="0.4" />
				<circle cx="73" cy="50" r="1.2"
					fill="var(--h-fg, #f0ece4)" opacity="0.3" />
			{/if}

			<!-- Secondary label -->
			<text x="73" y="48" text-anchor="middle" dominant-baseline="central"
				font-size="3.5" font-weight="600"
				font-family="var(--h-font-mono, monospace)"
				fill={SECONDARY_COLOR} opacity="0.7">
				E({secondarySteps},{secondaryActiveCount})
			</text>
			<text x="73" y="52" text-anchor="middle" dominant-baseline="central"
				font-size="2.5"
				font-family="var(--h-font-mono, monospace)"
				fill="var(--h-fg-3, #5e5a70)">
				r{secondaryRotation}
			</text>

			<!-- Polyrhythm ratio -->
			<text x="50" y="96" text-anchor="middle"
				font-size="3"
				font-family="var(--h-font-mono, monospace)"
				fill="var(--h-fg-3, #5e5a70)">
				{primarySteps}:{secondarySteps} polyrhythm
			</text>
		{/if}

	</svg>
</div>

<style>
	.rhythm-viz {
		display: inline-block;
	}
	.rhythm-viz svg {
		width: 100%;
		height: 100%;
	}
</style>
