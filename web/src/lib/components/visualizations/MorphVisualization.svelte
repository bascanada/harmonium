<script lang="ts">
	import type { HarmoniumBridge, EngineState } from '$lib/bridge';
	import MorphPlane from './MorphPlane.svelte';

	let { bridge, state: engineState }: { bridge: HarmoniumBridge; state: EngineState } = $props();

	// Placeholder Presets (User can update these with real data)
	const PRESET_LOCATIONS = [
		// Emotional Plane (Valence vs Arousal)
		{ id: 'p_warm_pad', name: 'Warm Pad', x: 0.8, y: 0.2, plane: 'emotional', color: '#4ade8050' },
		{
			id: 'p_dark_drone',
			name: 'Dark Drone',
			x: -0.7,
			y: 0.1,
			plane: 'emotional',
			color: '#60a5fa50'
		},
		{
			id: 'p_aggro_bass',
			name: 'Aggro Bass',
			x: -0.5,
			y: 0.9,
			plane: 'emotional',
			color: '#f8717150'
		},
		{
			id: 'p_bright_lead',
			name: 'Bright Lead',
			x: 0.6,
			y: 0.8,
			plane: 'emotional',
			color: '#facc1550'
		},

		// Musical Plane (Density vs Tension)
		{ id: 'p_minimal', name: 'Minimal', x: 0.1, y: 0.1, plane: 'musical', color: '#94a3b850' },
		{ id: 'p_complex', name: 'Complex', x: 0.9, y: 0.8, plane: 'musical', color: '#a78bfa50' }
	];

	// Local state for smooth dragging
	let isEditing = $state(false);
	let editTimeout: ReturnType<typeof setTimeout> | null = null;

	let localState: EngineState = $state({} as EngineState);

	$effect(() => {
		if (!isEditing) {
			localState = { ...engineState };
		}
	});

	function startEditing() {
		isEditing = true;
		if (editTimeout) clearTimeout(editTimeout);
		editTimeout = setTimeout(() => {
			isEditing = false;
		}, 500);
	}

	// --- Plane 1: Valence (X) vs Arousal (Y) ---
	function handleValenceArousalChange(id: string, x: number, y: number) {
		startEditing();
		if (id === 'global') {
			localState.valence = x;
			localState.arousal = y;
			bridge.setValence(x);
			bridge.setArousal(y);
		} else if (id === 'harmony') {
			localState.harmonyValence = x;
			bridge.setDirectHarmonyValence(x);
		}
	}

	// --- Plane 2: Density (X) vs Tension (Y) ---
	function handleDensityTensionChange(id: string, x: number, y: number) {
		startEditing();
		if (id === 'global') {
			localState.density = x;
			localState.tension = y;
			bridge.setDensity(x);
			bridge.setTension(y);
		} else if (id === 'rhythm') {
			localState.rhythmDensity = x;
			localState.rhythmTension = y;
			bridge.setDirectRhythmDensity(x);
			bridge.setDirectRhythmTension(y);
		} else if (id === 'voicing') {
			localState.voicingDensity = x;
			localState.voicingTension = y;
			bridge.setDirectVoicingDensity(x);
			bridge.setDirectVoicingTension(y);
		} else if (id === 'harmony') {
			localState.harmonyTension = y;
			bridge.setDirectHarmonyTension(y);
		}
	}

	// COLORS
	const C_GLOBAL = '#ffffff';
	const C_HARMONY = '#16a34a'; // green-600
	const C_RHYTHM = '#ea580c'; // orange-600
	const C_VOICING = '#9333ea'; // purple-600
	const C_GHOST = '#525252'; // neutral-600

	// Derived Points
	let isTechnical = $derived(!engineState.isEmotionMode);

	let pointsVA = $derived((() => {
		const pts: { id: string; x: number; y: number; label?: string; color: string; size: number; editable: boolean }[] = [];

		PRESET_LOCATIONS.filter((p) => p.plane === 'emotional').forEach((p) => {
			pts.push({
				id: p.id,
				x: p.x,
				y: p.y,
				label: p.name,
				color: p.color,
				size: 6,
				editable: false
			});
		});

		pts.push({
			id: 'global',
			x: localState.valence,
			y: localState.arousal,
			label: isTechnical ? undefined : 'Global',
			color: isTechnical ? C_GHOST : C_GLOBAL,
			size: isTechnical ? 8 : 14,
			editable: !isTechnical
		});

		if (isTechnical) {
			if (engineState.enableHarmony) {
				pts.push({
					id: 'harmony',
					x: localState.harmonyValence,
					y: localState.arousal,
					label: 'Harmony',
					color: C_HARMONY,
					size: 12,
					editable: true
				});
			}
		}

		return pts;
	})());

	let pointsDT = $derived((() => {
		const pts: { id: string; x: number; y: number; label?: string; color: string; size: number; editable: boolean }[] = [];

		PRESET_LOCATIONS.filter((p) => p.plane === 'musical').forEach((p) => {
			pts.push({
				id: p.id,
				x: p.x,
				y: p.y,
				label: p.name,
				color: p.color,
				size: 6,
				editable: false
			});
		});

		pts.push({
			id: 'global',
			x: localState.density,
			y: localState.tension,
			label: isTechnical ? undefined : 'Global',
			color: isTechnical ? C_GHOST : C_GLOBAL,
			size: isTechnical ? 8 : 14,
			editable: !isTechnical
		});

		if (isTechnical) {
			if (engineState.enableRhythm) {
				pts.push({
					id: 'rhythm',
					x: localState.rhythmDensity,
					y: localState.rhythmTension,
					label: 'Rhythm',
					color: C_RHYTHM,
					size: 12,
					editable: true
				});
			}
			if (engineState.enableVoicing) {
				pts.push({
					id: 'voicing',
					x: localState.voicingDensity,
					y: localState.voicingTension,
					label: 'Voicing',
					color: C_VOICING,
					size: 12,
					editable: true
				});
			}
			if (engineState.enableHarmony) {
				pts.push({
					id: 'harmony',
					x: localState.density,
					y: localState.harmonyTension,
					label: 'Harmony',
					color: C_HARMONY,
					size: 12,
					editable: true
				});
			}
		}

		return pts;
	})());
</script>

<div class="grid grid-cols-2 gap-4">
	<MorphPlane
		title="Emotional (V-A)"
		xAxisLabel="Valence"
		yAxisLabel="Arousal"
		xMin={-1}
		xMax={1}
		yMin={0}
		yMax={1}
		points={pointsVA}
		onPointChange={handleValenceArousalChange}
	/>

	<MorphPlane
		title="Musical (D-T)"
		xAxisLabel="Density"
		yAxisLabel="Tension"
		xMin={0}
		xMax={1}
		yMin={0}
		yMax={1}
		points={pointsDT}
		onPointChange={handleDensityTensionChange}
	/>
</div>
