<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { HarmoniumBridge } from '$lib/bridge/base-bridge';
	import type { NoteEvent } from '$lib/bridge/types';

	export let bridge: HarmoniumBridge;

	let events: NoteEvent[] = [];
	let eventCount = 0;
	let lastEventTime: number | null = null;
	let averageLatency: number | null = null;
	let unsubscribe: (() => void) | null = null;

	const maxDisplayEvents = 10; // Only show last 10 events

	onMount(() => {
		console.log('✨ EventLogger: Subscribing to note events...');

		unsubscribe = bridge.subscribeToEvents((newEvents) => {
			const now = performance.now();

			// Calculate latency (time since event was timestamped)
			if (newEvents.length > 0) {
				const latencies = newEvents.map((e) => now - e.timestamp);
				const avgLatency = latencies.reduce((sum, l) => sum + l, 0) / latencies.length;

				if (averageLatency === null) {
					averageLatency = avgLatency;
				} else {
					// Exponential moving average
					averageLatency = averageLatency * 0.9 + avgLatency * 0.1;
				}
			}

			// Update event list
			events = [...newEvents, ...events].slice(0, maxDisplayEvents);
			eventCount += newEvents.length;
			lastEventTime = now;

			// Log to console
			newEvents.forEach((event) => {
				console.log(`🎵 NoteEvent:`, {
					note: event.note,
					instrument: ['Bass', 'Lead', 'Snare', 'Hat'][event.instrument],
					step: event.step,
					duration: `${(event.duration / 48000 * 1000).toFixed(1)}ms`,
					latency: `${(now - event.timestamp).toFixed(2)}ms`
				});
			});
		});

		return () => {
			if (unsubscribe) {
				console.log('👋 EventLogger: Unsubscribing from events');
				unsubscribe();
			}
		};
	});

	onDestroy(() => {
		if (unsubscribe) unsubscribe();
	});

	function getNoteName(midiNote: number): string {
		const notes = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
		const octave = Math.floor(midiNote / 12) - 1;
		const noteName = notes[midiNote % 12];
		return `${noteName}${octave}`;
	}

	function getInstrumentName(instrument: number): string {
		const names = ['Bass', 'Lead', 'Snare', 'Hat'];
		return names[instrument] || 'Unknown';
	}

	function getInstrumentColor(instrument: number): string {
		const colors = ['#ff6b6b', '#4ecdc4', '#ffe66d', '#a8dadc'];
		return colors[instrument] || '#666';
	}
</script>

<div class="event-logger">
	<div class="stats">
		<h3>🎹 Event Stream Monitor</h3>
		<div class="stat-row">
			<span class="label">Total Events:</span>
			<span class="value">{eventCount}</span>
		</div>
		<div class="stat-row">
			<span class="label">Average Latency:</span>
			<span class="value" class:good-latency={averageLatency !== null && averageLatency < 10}>
				{averageLatency !== null ? `${averageLatency.toFixed(2)}ms` : 'N/A'}
			</span>
		</div>
		<div class="stat-row">
			<span class="label">Last Event:</span>
			<span class="value">
				{lastEventTime !== null
					? `${((performance.now() - lastEventTime) / 1000).toFixed(1)}s ago`
					: 'None'}
			</span>
		</div>
	</div>

	<div class="events-list">
		<h4>Recent Events (Last {maxDisplayEvents})</h4>
		{#if events.length === 0}
			<p class="no-events">No events yet. Enable harmony/melody and play!</p>
		{:else}
			<div class="event-items">
				{#each events as event, index (`${event.timestamp}-${index}`)}
					<div class="event-item" style="border-left-color: {getInstrumentColor(event.instrument)}">
						<div class="event-info">
							<span class="instrument" style="color: {getInstrumentColor(event.instrument)}">
								{getInstrumentName(event.instrument)}
							</span>
							<span class="note">{getNoteName(event.note)}</span>
							<span class="midi-note">MIDI {event.note}</span>
						</div>
						<div class="event-meta">
							<span class="step">Step: {event.step}</span>
							<span class="duration">{(event.duration / 48000 * 1000).toFixed(0)}ms</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.event-logger {
		background: rgba(0, 0, 0, 0.3);
		border-radius: 8px;
		padding: 1rem;
		margin: 1rem 0;
		font-family: 'Courier New', monospace;
	}

	.stats h3 {
		margin: 0 0 1rem 0;
		color: #fff;
		font-size: 1.2rem;
	}

	.stat-row {
		display: flex;
		justify-content: space-between;
		margin: 0.5rem 0;
		padding: 0.5rem;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 4px;
	}

	.label {
		color: #aaa;
	}

	.value {
		color: #fff;
		font-weight: bold;
	}

	.good-latency {
		color: #4ecdc4;
	}

	.events-list {
		margin-top: 1.5rem;
	}

	.events-list h4 {
		color: #fff;
		font-size: 1rem;
		margin-bottom: 0.75rem;
	}

	.no-events {
		color: #888;
		font-style: italic;
		padding: 1rem;
		text-align: center;
	}

	.event-items {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.event-item {
		background: rgba(255, 255, 255, 0.05);
		border-left: 3px solid;
		border-radius: 4px;
		padding: 0.75rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		transition: background 0.2s;
	}

	.event-item:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.event-info {
		display: flex;
		gap: 1rem;
		align-items: center;
	}

	.instrument {
		font-weight: bold;
		min-width: 60px;
	}

	.note {
		color: #fff;
		font-size: 1.1rem;
		font-weight: bold;
	}

	.midi-note {
		color: #888;
		font-size: 0.9rem;
	}

	.event-meta {
		display: flex;
		gap: 1rem;
		color: #aaa;
		font-size: 0.9rem;
	}

	.step {
		color: #4ecdc4;
	}

	.duration {
		color: #ffe66d;
	}
</style>
