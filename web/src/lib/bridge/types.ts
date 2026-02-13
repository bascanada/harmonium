// Types for the Harmonium Bridge abstraction layer
// Allows the same UI components to work with both WASM (web) and VST (postMessage) backends

export type AudioBackendType = 'fundsp' | 'odin2';

export interface EngineState {
	// Audio backend
	audioBackend: AudioBackendType;
	// Harmony state
	currentChord: string;
	currentMeasure: number;
	currentStep: number;
	isMinorChord: boolean;
	progressionName: string;
	progressionLength: number;
	harmonyMode: number; // 0 = Basic, 1 = Driver

	// Rhythm state - Primary
	primarySteps: number;
	primaryPulses: number;
	primaryRotation: number;
	primaryPattern: boolean[];

	// Rhythm state - Secondary
	secondarySteps: number;
	secondaryPulses: number;
	secondaryRotation: number;
	secondaryPattern: boolean[];

	// Control mode
	isEmotionMode: boolean;

	// Emotional params
	arousal: number;
	valence: number;
	density: number;
	tension: number;

	// Direct params
	bpm: number;
	rhythmMode: number; // 0 = Euclidean, 1 = PerfectBalance
	enableRhythm: boolean;
	enableHarmony: boolean;
	enableMelody: boolean;
	enableVoicing: boolean;
	fixedKick: boolean;

	// Direct rhythm params
	rhythmDensity: number;
	rhythmTension: number;

	// Direct harmony params
	harmonyTension: number;
	harmonyValence: number;

	// Direct melody/voicing params
	melodySmoothness: number;
	voicingDensity: number;
	voicingTension: number;

	// Channel state
	channelMuted: boolean[];
	channelGains: number[];

	// Session info
	key: string;
	scale: string;
}

export interface HarmoniumBridge {
	// === Lifecycle ===
	connect(sf2Data?: Uint8Array, backend?: AudioBackendType): Promise<void>;
	disconnect(): void;
	isConnected(): boolean;

	// === Backend ===
	getAvailableBackends(): AudioBackendType[];

	// === Mode Control ===
	useEmotionMode(): void;
	useDirectMode(): void;

	// === Emotional Controls ===
	setArousal(value: number): void;
	setValence(value: number): void;
	setDensity(value: number): void;
	setTension(value: number): void;

	// === Algorithm & Harmony Mode ===
	setAlgorithm(mode: number): void;
	setHarmonyMode(mode: number): void;
	setPolySteps(steps: number): void;

	// === Direct Controls ===
	setDirectBpm(value: number): void;
	setDirectEnableRhythm(enabled: boolean): void;
	setDirectEnableHarmony(enabled: boolean): void;
	setDirectEnableMelody(enabled: boolean): void;
	setDirectEnableVoicing(enabled: boolean): void;
	setDirectFixedKick(enabled: boolean): void;
	setDirectRhythmMode(mode: number): void;
	setDirectRhythmSteps(steps: number): void;
	setDirectRhythmPulses(pulses: number): void;
	setDirectRhythmRotation(rotation: number): void;
	setDirectRhythmDensity(density: number): void;
	setDirectRhythmTension(tension: number): void;
	setDirectSecondarySteps(steps: number): void;
	setDirectSecondaryPulses(pulses: number): void;
	setDirectSecondaryRotation(rotation: number): void;
	setDirectSecondaryRotation(rotation: number): void;
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
	): void;
	setDirectHarmonyTension(tension: number): void;
	setDirectHarmonyValence(valence: number): void;
	setDirectMelodySmoothness(smoothness: number): void;
	setDirectVoicingDensity(density: number): void;
	setDirectVoicingTension(tension: number): void;

	// === Channel Controls ===
	setChannelMuted(channel: number, muted: boolean): void;
	setChannelRouting(channel: number, routing: number): void;
	setChannelGain(channel: number, gain: number): void;

	// === SoundFont (WASM only, no-op in VST) ===
	addSoundFont?(bankId: number, data: Uint8Array): void;

	// === State Subscription ===
	subscribe(callback: (state: EngineState) => void): () => void;

	// === State Getters (for initial sync) ===
	getState(): EngineState | null;

	// === Look-ahead Simulation ===
	getLookaheadTruth(steps: number): string;
}

// Factory function type
export type BridgeFactory = (mode: 'wasm' | 'vst') => HarmoniumBridge;

// Default empty state
export function createEmptyState(): EngineState {
	return {
		audioBackend: 'odin2', // Odin2 par défaut
		currentChord: 'I',
		currentMeasure: 1,
		currentStep: 0,
		isMinorChord: false,
		progressionName: '',
		progressionLength: 4,
		harmonyMode: 1,

		primarySteps: 16,
		primaryPulses: 4,
		primaryRotation: 0,
		primaryPattern: [],

		secondarySteps: 12,
		secondaryPulses: 3,
		secondaryRotation: 0,
		secondaryPattern: [],

		isEmotionMode: true,

		arousal: 0.5,
		valence: 0.3,
		density: 0.5,
		tension: 0.3,

		bpm: 120,
		rhythmMode: 0,
		enableRhythm: true,
		enableHarmony: true,
		enableMelody: true,
		enableVoicing: false,
		fixedKick: false,

		rhythmDensity: 0.5,
		rhythmTension: 0.3,

		harmonyTension: 0.3,
		harmonyValence: 0.3,

		melodySmoothness: 0.7,
		voicingDensity: 0.5,
		voicingTension: 0.3,

		channelMuted: [false, false, false, false],
		channelGains: [0.6, 1.0, 0.5, 0.4],

		key: 'C',
		scale: 'major'
	};
}
