// Base Bridge - Abstract class with shared functionality for Harmonium bridges
// Eliminates code duplication between WASM and VST implementations
import type { HarmoniumBridge, EngineState, AudioBackendType } from './types';

/**
 * Abstract base class providing shared functionality for Harmonium bridges.
 * Subclasses must implement communication-specific methods.
 */
export abstract class BaseBridge implements HarmoniumBridge {
	protected currentState: EngineState;
	protected subscribers: Set<(state: EngineState) => void> = new Set();

	constructor(initialState: EngineState) {
		this.currentState = initialState;
	}

	// Abstract methods - must be implemented by subclasses
	abstract connect(sf2Data?: Uint8Array, backend?: AudioBackendType): Promise<void>;
	abstract disconnect(): void;
	abstract isConnected(): boolean;
	abstract getAvailableBackends(): AudioBackendType[];
	abstract getLookaheadTruth(steps: number): string;

	/**
	 * Send a command to the engine backend.
	 * Subclasses implement this to handle their specific communication mechanism.
	 */
	protected abstract sendCommand(method: string, ...args: unknown[]): void;

	/**
	 * Update the current state and notify all subscribers.
	 */
	protected updateState(updates: Partial<EngineState>): void {
		this.currentState = { ...this.currentState, ...updates };
		this.notifySubscribers();
	}

	/**
	 * Notify all subscribers of state changes.
	 */
	protected notifySubscribers(): void {
		this.subscribers.forEach((cb) => cb(this.currentState));
	}

	// === Mode Control ===
	useEmotionMode(): void {
		this.sendCommand('use_emotion_mode');
		this.updateState({ isEmotionMode: true });
	}

	useDirectMode(): void {
		this.sendCommand('use_direct_mode');
		this.updateState({ isEmotionMode: false });
	}

	// === Emotional Controls ===
	setArousal(value: number): void {
		this.sendCommand('set_arousal', value);
		this.currentState.arousal = value;
	}

	setValence(value: number): void {
		this.sendCommand('set_valence', value);
		this.currentState.valence = value;
	}

	setDensity(value: number): void {
		this.sendCommand('set_density', value);
		this.currentState.density = value;
	}

	setTension(value: number): void {
		this.sendCommand('set_tension', value);
		this.currentState.tension = value;
	}

	// === Algorithm & Harmony Mode ===
	setAlgorithm(mode: number): void {
		this.sendCommand('set_algorithm', mode);
	}

	setHarmonyMode(mode: number): void {
		this.sendCommand('set_harmony_mode', mode);
	}

	setPolySteps(steps: number): void {
		this.sendCommand('set_poly_steps', steps);
	}

	// === Direct Controls ===
	setDirectBpm(value: number): void {
		this.sendCommand('set_direct_bpm', value);
	}

	setDirectEnableRhythm(enabled: boolean): void {
		this.sendCommand('set_direct_enable_rhythm', enabled);
	}

	setDirectEnableHarmony(enabled: boolean): void {
		this.sendCommand('set_direct_enable_harmony', enabled);
	}

	setDirectEnableMelody(enabled: boolean): void {
		this.sendCommand('set_direct_enable_melody', enabled);
	}

	setDirectEnableVoicing(enabled: boolean): void {
		this.sendCommand('set_direct_enable_voicing', enabled);
	}

	setDirectFixedKick(enabled: boolean): void {
		this.sendCommand('set_direct_fixed_kick', enabled);
	}

	setDirectRhythmMode(mode: number): void {
		this.sendCommand('set_direct_rhythm_mode', mode);
	}

	setDirectRhythmSteps(steps: number): void {
		this.sendCommand('set_direct_rhythm_steps', steps);
	}

	setDirectRhythmPulses(pulses: number): void {
		this.sendCommand('set_direct_rhythm_pulses', pulses);
	}

	setDirectRhythmRotation(rotation: number): void {
		this.sendCommand('set_direct_rhythm_rotation', rotation);
	}

	setDirectRhythmDensity(density: number): void {
		this.sendCommand('set_direct_rhythm_density', density);
	}

	setDirectRhythmTension(tension: number): void {
		this.sendCommand('set_direct_rhythm_tension', tension);
	}

	setDirectSecondarySteps(steps: number): void {
		this.sendCommand('set_direct_secondary_steps', steps);
	}

	setDirectSecondaryPulses(pulses: number): void {
		this.sendCommand('set_direct_secondary_pulses', pulses);
	}

	setDirectSecondaryRotation(rotation: number): void {
		this.sendCommand('set_direct_secondary_rotation', rotation);
	}

	// Set all rhythm parameters at once (avoids read-modify-write race)
	setAllRhythmParams(
		mode: number,
		steps: number,
		pulses: number,
		rotation: number,
		density: number,
		tension: number,
		secondarySteps: number,
		secondaryPulses: number,
		secondaryRotation: number
	): void {
		this.sendCommand(
			'set_all_rhythm_params',
			mode,
			steps,
			pulses,
			rotation,
			density,
			tension,
			secondarySteps,
			secondaryPulses,
			secondaryRotation
		);
	}

	setDirectHarmonyTension(tension: number): void {
		this.sendCommand('set_direct_harmony_tension', tension);
	}

	setDirectHarmonyValence(valence: number): void {
		this.sendCommand('set_direct_harmony_valence', valence);
	}

	setDirectMelodySmoothness(smoothness: number): void {
		this.sendCommand('set_direct_melody_smoothness', smoothness);
	}

	setDirectVoicingDensity(density: number): void {
		this.sendCommand('set_direct_voicing_density', density);
	}

	setDirectVoicingTension(tension: number): void {
		this.sendCommand('set_direct_voicing_tension', tension);
	}

	// === Channel Controls ===
	setChannelMuted(channel: number, muted: boolean): void {
		this.sendCommand('set_channel_muted', channel, muted);
		this.currentState.channelMuted[channel] = muted;
	}

	setChannelRouting(channel: number, routing: number): void {
		this.sendCommand('set_channel_routing', channel, routing);
	}

	setChannelGain(channel: number, gain: number): void {
		this.currentState.channelGains[channel] = gain;
		this.sendCommand('set_channel_gain', channel, gain);
	}

	// === State Subscription ===
	subscribe(callback: (state: EngineState) => void): () => void {
		this.subscribers.add(callback);
		// Immediately call with current state
		callback(this.currentState);
		return () => {
			this.subscribers.delete(callback);
		};
	}

	getState(): EngineState | null {
		return { ...this.currentState };
	}
}
