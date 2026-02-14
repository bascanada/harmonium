<script lang="ts">
	import type { HarmoniumBridge, EngineState } from '$lib/bridge';

	// Svelte 5 Props Destructuring
	let { bridge, engineState } = $props<{
		bridge: HarmoniumBridge;
		engineState: EngineState;
	}>();

	const channelNames = ['Bass', 'Lead', 'Snare', 'Hat'];

	function toggleMute(channel: number) {
		bridge.setChannelMuted(channel, !engineState.channelMuted[channel]);
	}

	function updateGain(channel: number, value: number) {
		bridge.setChannelGain(channel, value);
	}
</script>

<div class="rounded-lg border border-neutral-700 bg-neutral-800 p-3">
	<h3 class="mb-2 text-sm font-semibold">Mixer</h3>
	<div class="grid grid-cols-4 gap-2">
		{#each channelNames as name, i}
			<div class="rounded border border-neutral-700 bg-neutral-900 p-2">
				<!-- Header: Name + Mute -->
				<div class="mb-1 flex items-center justify-between">
					<span class="text-xs font-medium text-neutral-300">{name}</span>
					<button
						class="flex h-6 w-6 items-center justify-center rounded transition-colors {engineState
							.channelMuted[i]
							? 'bg-red-500/20 text-red-400'
							: 'bg-neutral-800 text-neutral-500 hover:text-neutral-300'}"
						onclick={() => toggleMute(i)}
						title={engineState.channelMuted[i] ? 'Unmute' : 'Mute'}
					>
						{#if engineState.channelMuted[i]}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path d="M11 5L6 9H2v6h4l5 4V5z" />
								<line x1="23" y1="9" x2="17" y2="15" />
								<line x1="17" y1="9" x2="23" y2="15" />
							</svg>
						{:else}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
							</svg>
						{/if}
					</button>
				</div>
				<!-- Volume Slider -->
				<input
					type="range"
					min="0"
					max="1"
					step="0.01"
					value={engineState.channelGains[i]}
					oninput={(e) => updateGain(i, parseFloat(e.currentTarget.value))}
					class="h-1.5 w-full cursor-pointer appearance-none rounded bg-neutral-700 accent-emerald-500"
					disabled={engineState.channelMuted[i]}
				/>
				<div class="mt-0.5 text-center text-[10px] text-neutral-400">
					{(engineState.channelGains[i] * 100).toFixed(0)}%
				</div>
			</div>
		{/each}
	</div>
</div>
