import { describe, it, expect, beforeEach } from 'vitest';
import { WasmBridge } from './wasm-bridge';
import type { NoteEvent } from './types';

describe('Event Parsing', () => {
	let bridge: WasmBridge;

	beforeEach(() => {
		bridge = new WasmBridge();
	});

	it('should parse flat event array correctly', () => {
		const flatEvents = [60, 1, 10, 48000, 64, 0, 11, 24000];
		// Access private method via type assertion for testing
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed).toHaveLength(2);
		expect(parsed[0]).toMatchObject({
			note: 60,
			instrument: 1,
			step: 10,
			duration: 48000
		});
		expect(parsed[1]).toMatchObject({
			note: 64,
			instrument: 0,
			step: 11,
			duration: 24000
		});
	});

	it('should handle empty event array', () => {
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents([]);

		expect(parsed).toHaveLength(0);
	});

	it('should add timestamp to each event', () => {
		const before = performance.now();
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents([60, 1, 10, 48000]);
		const after = performance.now();

		expect(parsed).toHaveLength(1);
		expect(parsed[0].timestamp).toBeGreaterThanOrEqual(before);
		expect(parsed[0].timestamp).toBeLessThanOrEqual(after);
	});

	it('should parse multiple events with same timestamp', () => {
		const flatEvents = [60, 1, 0, 1000, 64, 1, 0, 1000, 67, 1, 0, 1000];
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed).toHaveLength(3);
		// All should have same timestamp (within same parsing call)
		expect(parsed[0].timestamp).toBe(parsed[1].timestamp);
		expect(parsed[1].timestamp).toBe(parsed[2].timestamp);
	});

	it('should validate instrument channel values', () => {
		const flatEvents = [60, 0, 10, 1000, 64, 3, 11, 2000];
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed[0].instrument).toBe(0); // Bass
		expect(parsed[1].instrument).toBe(3); // Hat
		expect(parsed[0].instrument).toBeGreaterThanOrEqual(0);
		expect(parsed[1].instrument).toBeLessThanOrEqual(3);
	});
});

describe('Event Subscription', () => {
	let bridge: WasmBridge;

	beforeEach(() => {
		bridge = new WasmBridge();
	});

	it('should notify subscribers on new events', () => {
		const receivedEvents: NoteEvent[] = [];

		const unsubscribe = bridge.subscribeToEvents((newEvents) => {
			receivedEvents.push(...newEvents);
		});

		// Simulate event emission by accessing private method
		const testEvents: NoteEvent[] = [
			{ note: 60, instrument: 1, step: 0, duration: 1000, timestamp: performance.now() }
		];

		// Access private eventSubscribers to simulate event dispatch
		const eventSubscribers = (bridge as unknown as { eventSubscribers: ((events: NoteEvent[]) => void)[] })
			.eventSubscribers;
		eventSubscribers.forEach((cb) => cb(testEvents));

		expect(receivedEvents).toHaveLength(1);
		expect(receivedEvents[0].note).toBe(60);

		unsubscribe();
	});

	it('should unsubscribe correctly', () => {
		let callCount = 0;

		const unsubscribe = bridge.subscribeToEvents(() => callCount++);

		// Simulate event emission
		const eventSubscribers = (bridge as unknown as { eventSubscribers: ((events: NoteEvent[]) => void)[] })
			.eventSubscribers;

		eventSubscribers.forEach((cb) => cb([]));
		expect(callCount).toBe(1);

		unsubscribe();
		eventSubscribers.forEach((cb) => cb([]));
		expect(callCount).toBe(1); // Should not increment after unsubscribe
	});

	it('should support multiple subscribers', () => {
		const received1: NoteEvent[] = [];
		const received2: NoteEvent[] = [];

		const unsub1 = bridge.subscribeToEvents((events) => received1.push(...events));
		const unsub2 = bridge.subscribeToEvents((events) => received2.push(...events));

		const testEvents: NoteEvent[] = [
			{ note: 60, instrument: 1, step: 0, duration: 1000, timestamp: 0 }
		];

		const eventSubscribers = (bridge as unknown as { eventSubscribers: ((events: NoteEvent[]) => void)[] })
			.eventSubscribers;
		eventSubscribers.forEach((cb) => cb(testEvents));

		expect(received1).toHaveLength(1);
		expect(received2).toHaveLength(1);

		unsub1();
		unsub2();
	});

	it('should handle unsubscribe when not subscribed', () => {
		const unsubscribe = bridge.subscribeToEvents(() => {});

		// Call unsubscribe multiple times - should not throw
		expect(() => {
			unsubscribe();
			unsubscribe();
		}).not.toThrow();
	});

	it('should return different unsubscribe functions for different subscriptions', () => {
		let count1 = 0;
		let count2 = 0;

		const unsub1 = bridge.subscribeToEvents(() => count1++);
		const unsub2 = bridge.subscribeToEvents(() => count2++);

		expect(unsub1).not.toBe(unsub2);

		const eventSubscribers = (bridge as unknown as { eventSubscribers: ((events: NoteEvent[]) => void)[] })
			.eventSubscribers;

		eventSubscribers.forEach((cb) => cb([]));
		expect(count1).toBe(1);
		expect(count2).toBe(1);

		unsub1();
		eventSubscribers.forEach((cb) => cb([]));
		expect(count1).toBe(1); // Should not increment
		expect(count2).toBe(2); // Should increment

		unsub2();
	});
});

describe('Event Parsing Edge Cases', () => {
	let bridge: WasmBridge;

	beforeEach(() => {
		bridge = new WasmBridge();
	});

	it('should handle MIDI note range boundaries', () => {
		const flatEvents = [0, 0, 0, 1000, 127, 3, 1, 2000];
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed[0].note).toBe(0); // Lowest MIDI note
		expect(parsed[1].note).toBe(127); // Highest MIDI note
	});

	it('should handle large step numbers', () => {
		const flatEvents = [60, 1, 999999, 1000];
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed[0].step).toBe(999999);
	});

	it('should handle zero duration', () => {
		const flatEvents = [60, 1, 10, 0];
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed[0].duration).toBe(0);
	});

	it('should ignore incomplete event data (not multiple of 4)', () => {
		// This should only parse complete events (each event is 4 values)
		const flatEvents = [60, 1, 10, 1000, 64]; // Last event incomplete
		const parsed = (bridge as unknown as { parseEvents: (flat: number[]) => NoteEvent[] })
			.parseEvents(flatEvents);

		expect(parsed).toHaveLength(1); // Only first complete event
	});
});
