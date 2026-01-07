<script lang="ts">
  import type { HarmoniumBridge, EngineState } from '$lib/bridge';

  // Props
  export let bridge: HarmoniumBridge;
  export let state: EngineState;

  const channelNames = ['Bass', 'Lead', 'Snare', 'Hat'];

  function toggleMute(channel: number) {
    bridge.setChannelMuted(channel, !state.channelMuted[channel]);
  }

  function updateGain(channel: number, value: number) {
    bridge.setChannelGain(channel, value);
  }
</script>

<div class="p-3 bg-neutral-800 rounded-lg border border-neutral-700">
  <h3 class="text-sm font-semibold mb-2">Mixer</h3>
  <div class="grid grid-cols-4 gap-2">
    {#each channelNames as name, i}
      <div class="p-2 bg-neutral-900 rounded border border-neutral-700">
        <!-- Header: Name + Mute -->
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs font-medium text-neutral-300">{name}</span>
          <button
            class="w-6 h-6 rounded flex items-center justify-center transition-colors {state.channelMuted[i]
              ? 'bg-red-500/20 text-red-400'
              : 'bg-neutral-800 text-neutral-500 hover:text-neutral-300'}"
            onclick={() => toggleMute(i)}
            title={state.channelMuted[i] ? 'Unmute' : 'Mute'}
          >
            {#if state.channelMuted[i]}
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
          value={state.channelGains[i]}
          oninput={(e) => updateGain(i, parseFloat(e.currentTarget.value))}
          class="w-full h-1.5 bg-neutral-700 rounded appearance-none cursor-pointer accent-emerald-500"
          disabled={state.channelMuted[i]}
        />
        <div class="text-center text-[10px] text-neutral-400 mt-0.5">
          {(state.channelGains[i] * 100).toFixed(0)}%
        </div>
      </div>
    {/each}
  </div>
</div>
