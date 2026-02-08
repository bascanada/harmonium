<script lang="ts">
	import { lookAheadBuffer } from '$lib/stores/engine-state';

	// Reactive debug log
	$: if ($lookAheadBuffer && $lookAheadBuffer.length > 0) {
		// console.log('TimelineVisualizer: Received buffer with', $lookAheadBuffer.length, 'steps');
	}

	// Constants for visualization
	const STEP_WIDTH = 12;
	const VISIBLE_STEPS = 48;

	// Trigger categories with colors
	const categories = [
		{ key: 'kick', color: 'bg-orange-500', label: 'KICK' },
		{ key: 'snare', color: 'bg-purple-500', label: 'SNARE' },
		{ key: 'hat', color: 'bg-green-400', label: 'HAT' },
		{ key: 'bass', color: 'bg-blue-500', label: 'BASS' },
		{ key: 'lead', color: 'bg-yellow-400', label: 'LEAD' }
	] as const;
</script>

<div class="rounded-lg border border-neutral-700 bg-neutral-800 p-4 shadow-xl">
	<div class="mb-3 flex items-center justify-between">
		<h3 class="text-xs font-bold uppercase tracking-wider text-neutral-400">Look-ahead Timeline</h3>
		<div class="flex gap-3">
			{#each categories as cat}
				<div class="flex items-center gap-1.5">
					<div class="h-1.5 w-1.5 rounded-full {cat.color}"></div>
					<span class="text-[9px] font-bold text-neutral-500">{cat.label}</span>
				</div>
			{/each}
		</div>
	</div>

	<div
		class="relative flex h-24 w-full items-center overflow-hidden rounded border border-neutral-700 bg-neutral-900/50 p-1 shadow-inner"
	>
		<!-- Playhead (Current moment is the left edge) -->
		<div
			class="absolute left-1 top-0 bottom-0 z-20 w-0.5 bg-white shadow-[0_0_8px_rgba(255,255,255,0.8)]"
		></div>

		<!-- Steps Grid -->
		<div class="flex h-full items-center gap-1 pl-1">
			{#each ($lookAheadBuffer || []).slice(0, VISIBLE_STEPS) as step, i (i)}
				<div
					class="relative flex h-full flex-col justify-center gap-1"
					style="width: {STEP_WIDTH}px;"
				>
					{#each categories as cat}
						{@const isActive = !!(step.trigger && step.trigger[cat.key])}
						<div
							class="w-full rounded-full transition-all duration-200 {isActive
								? cat.color
								: 'bg-neutral-800/30'}
              {isActive ? 'h-2 shadow-[0_0_4px_rgba(255,255,255,0.3)]' : 'h-1.5'}"
						></div>
					{/each}

					<!-- Measure marker -->
					{#if i % 16 === 0}
						<div class="absolute -left-0.5 top-0 bottom-0 w-px bg-neutral-600/50"></div>
					{/if}
				</div>
			{/each}
		</div>
	</div>

	<div class="mt-2 text-center">
		<p class="text-[9px] text-neutral-500">
			Visualizing next {VISIBLE_STEPS} steps (Procedural buffer)
		</p>
	</div>
</div>
