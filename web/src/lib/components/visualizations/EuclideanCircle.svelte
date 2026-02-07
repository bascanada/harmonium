<script lang="ts">
	export let steps = 16;
	export let pulses = 4;
	export let rotation = 0;
	export let color = '#ff3e00';
	export let label = '';
	export let currentStep = 0;
	export let radius = 150;
	// Pattern optionnel fourni par le moteur (prioritaire sur le calcul local)
	export let externalPattern: boolean[] | null = null;

	// Fonction utilitaire pour générer le pattern de Bjorklund (Euclidien)
	// Utilisé seulement si externalPattern n'est pas fourni
	function getPattern(steps: number, pulses: number): boolean[] {
		let pattern = new Array(steps).fill(false);
		if (steps === 0) return pattern;
		let bucket = 0;
		for (let i = 0; i < steps; i++) {
			bucket += pulses;
			if (bucket >= steps) {
				bucket -= steps;
				pattern[i] = true;
			}
		}
		return pattern;
	}

	// Utiliser le pattern externe s'il est fourni, sinon calculer localement
	$: pattern = externalPattern ?? getPattern(steps, pulses);
	// Note: Le pattern externe est déjà rotaté par le moteur, pas besoin de re-rotater
	$: rotated = externalPattern
		? pattern
		: [...pattern.slice(steps - rotation), ...pattern.slice(0, steps - rotation)];

	$: points = Array.from({ length: steps }).map((_, i) => {
		const angle = (i * 2 * Math.PI) / steps - Math.PI / 2;
		const r = 45;
		return {
			x: 50 + r * Math.cos(angle),
			y: 50 + r * Math.sin(angle),
			active: rotated[i],
			isCurrent: currentStep % steps === i
		};
	});

	$: path = points
		.filter((p) => p.active)
		.map((p) => `${p.x},${p.y}`)
		.join(' ');
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
