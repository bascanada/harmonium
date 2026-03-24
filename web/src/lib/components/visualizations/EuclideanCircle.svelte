<script lang="ts">
	let {
		steps = 16,
		pulses = 4,
		rotation = 0,
		color = '#ff3e00',
		label = '',
		currentStep = 0,
		radius = 150,
		externalPattern = null as boolean[] | null
	} = $props();

	// Fonction utilitaire pour générer le pattern de Bjorklund (Euclidien)
	// Utilisé seulement si externalPattern n'est pas fourni
	function getPattern(s: number, p: number): boolean[] {
		let pat = new Array(s).fill(false);
		if (s === 0) return pat;
		let bucket = 0;
		for (let i = 0; i < s; i++) {
			bucket += p;
			if (bucket >= s) {
				bucket -= s;
				pat[i] = true;
			}
		}
		return pat;
	}

	// Utiliser le pattern externe s'il est fourni, sinon calculer localement
	let pattern = $derived(externalPattern ?? getPattern(steps, pulses));
	// Note: Le pattern externe est déjà rotaté par le moteur, pas besoin de re-rotater
	let rotated = $derived(
		externalPattern
			? pattern
			: [...pattern.slice(steps - rotation), ...pattern.slice(0, steps - rotation)]
	);

	let points = $derived(
		Array.from({ length: steps }).map((_, i) => {
			const angle = (i * 2 * Math.PI) / steps - Math.PI / 2;
			const r = 45;
			return {
				x: 50 + r * Math.cos(angle),
				y: 50 + r * Math.sin(angle),
				active: rotated[i],
				isCurrent: currentStep % steps === i
			};
		})
	);

	let path = $derived(
		points
			.filter((p) => p.active)
			.map((p) => `${p.x},${p.y}`)
			.join(' ')
	);
</script>

<div class="circle-container" style="width: {radius * 2}px; height: {radius * 2}px;">
	<svg viewBox="0 0 100 100">
		<!-- Background Circle -->
		<circle cx="50" cy="50" r="45" stroke="#333" stroke-width="0.5" fill="none" />

		<!-- Polygon -->
		<polygon points={path} fill={color} fill-opacity="0.1" stroke={color} stroke-width="1.5" />

		<!-- Dots -->
		{#each points as point}
			{#if point.active}
				<circle
					cx={point.x}
					cy={point.y}
					r={point.isCurrent ? 3 : 2}
					fill={color}
					class:current={point.isCurrent}
				/>
			{/if}
		{/each}

		<!-- Cursor Line -->
		{#if points.length > 0}
			<line
				x1="50"
				y1="50"
				x2={points[currentStep % steps].x}
				y2={points[currentStep % steps].y}
				stroke="white"
				stroke-width="1"
				stroke-dasharray="2,2"
				opacity="0.8"
			/>
		{/if}

		{#if label}
			<text x="50" y="52" text-anchor="middle" font-size="6" fill={color} font-weight="bold"
				>{label}</text
			>
		{/if}
	</svg>
</div>

<style>
	.circle-container {
		display: inline-block;
		position: relative;
	}

	.current {
		stroke: white;
		stroke-width: 1.5px;
		transition: all 0.1s ease;
	}
</style>
