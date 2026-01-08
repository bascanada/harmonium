// WASM Bridge - Implementation for web mode using harmonium.js
import init, { start, start_with_backend, get_available_backends, type Handle } from 'harmonium';
import type { EngineState, AudioBackendType } from './types';
import { createEmptyState } from './types';
import { BaseBridge } from './base-bridge';

export class WasmBridge extends BaseBridge {
  private handle: Handle | null = null;
  private animationId: number | null = null;
  private _isEmotionMode = true;
  private _currentBackend: AudioBackendType = 'fundsp';
  private _availableBackends: AudioBackendType[] = ['fundsp'];

  constructor() {
    super(createEmptyState());
  }

  async connect(sf2Data?: Uint8Array, backend: AudioBackendType = 'fundsp'): Promise<void> {
    await init();

    // Get available backends from WASM
    try {
      const backends = get_available_backends();
      this._availableBackends = backends.map((b: string) => b as AudioBackendType);
    } catch {
      this._availableBackends = ['fundsp'];
    }

    // Start with selected backend
    this._currentBackend = backend;
    this.currentState.audioBackend = backend;
    this.handle = start_with_backend(sf2Data, backend);

    // Get initial key/scale
    this.currentState.key = this.handle.get_key();
    this.currentState.scale = this.handle.get_scale();

    // Collect initial state immediately before starting polling
    this.collectState();
    // Create a shallow copy to ensure Svelte detects the change
    this.subscribers.forEach(cb => cb({ ...this.currentState }));

    this.startPolling();
  }

  getAvailableBackends(): AudioBackendType[] {
    return this._availableBackends;
  }

  disconnect(): void {
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
    if (this.handle) {
      this.handle.free();
      this.handle = null;
    }
  }

  isConnected(): boolean {
    return this.handle !== null;
  }

  /**
   * Send command to WASM by calling the appropriate method on the Handle.
   */
  protected sendCommand(method: string, ...args: any[]): void {
    if (this.handle && typeof (this.handle as any)[method] === 'function') {
      (this.handle as any)[method](...args);
    }
  }

  private startPolling(): void {
    const poll = () => {
      if (!this.handle) return;

      const hasChanges = this.collectState();

      // Only notify if something actually changed
      if (hasChanges) {
        this.subscribers.forEach(cb => cb({ ...this.currentState }));
      }

      // Clear event queue
      this.handle.get_events();

      this.animationId = requestAnimationFrame(poll);
    };
    this.animationId = requestAnimationFrame(poll);
  }

  // Utility to compare arrays efficiently
  private arraysEqual<T>(a: T[], b: T[]): boolean {
    if (a === b) return true;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }

  private collectState(): boolean {
    const h = this.handle!;
    let changed = false;

    // Helper to update primitive fields
    const update = <K extends keyof EngineState>(key: K, value: EngineState[K]) => {
      if (this.currentState[key] !== value) {
        this.currentState[key] = value;
        changed = true;
      }
    };

    // Helper to update array fields (only changes ref if content differs)
    const updateArray = <K extends keyof EngineState>(key: K, newValue: BigInt64Array | Uint8Array | Float32Array | boolean[] | number[]) => {
      // Convert typed arrays/iterables to standard arrays for comparison if needed
      // ensuring type compatibility
      let newArray: any[] = [];

      if (newValue instanceof BigInt64Array || newValue instanceof Uint8Array || newValue instanceof Float32Array) {
        newArray = Array.from(newValue as any);
      } else {
        newArray = newValue as any[];
      }

      // For boolean arrays mapped from WASM (0/1)
      if (key === 'primaryPattern' || key === 'secondaryPattern') {
        newArray = newArray.map((v: any) => v === 1 || v === true);
      }

      const currentArray = this.currentState[key] as any[];
      if (!this.arraysEqual(currentArray, newArray)) {
        (this.currentState as any)[key] = newArray;
        changed = true;
      }
    };


    // Harmony state
    update('currentChord', h.get_current_chord_name());
    update('currentMeasure', h.get_current_measure());
    update('currentStep', h.get_current_step());
    update('isMinorChord', h.is_current_chord_minor());
    update('progressionName', h.get_progression_name());
    update('progressionLength', h.get_progression_length());
    update('harmonyMode', h.get_harmony_mode());

    // Rhythm state - Primary
    update('primarySteps', h.get_primary_steps());
    update('primaryPulses', h.get_primary_pulses());
    update('primaryRotation', h.get_primary_rotation());

    // Pattern handling
    const rawPrimary = h.get_primary_pattern(); // Int32Array (0 or 1)
    updateArray('primaryPattern', Array.from(rawPrimary));

    // Rhythm state - Secondary
    update('secondarySteps', h.get_secondary_steps());
    update('secondaryPulses', h.get_secondary_pulses());
    update('secondaryRotation', h.get_secondary_rotation());

    const rawSecondary = h.get_secondary_pattern();
    updateArray('secondaryPattern', Array.from(rawSecondary));


    // Control mode
    update('isEmotionMode', this._isEmotionMode);

    // Direct params (always sync from engine)
    update('bpm', h.get_direct_bpm());
    update('rhythmMode', h.get_direct_rhythm_mode());
    update('enableRhythm', h.get_direct_enable_rhythm());
    update('enableHarmony', h.get_direct_enable_harmony());
    update('enableMelody', h.get_direct_enable_melody());
    update('rhythmDensity', h.get_direct_rhythm_density());
    update('rhythmTension', h.get_direct_rhythm_tension());
    update('harmonyTension', h.get_direct_harmony_tension());
    update('harmonyValence', h.get_direct_harmony_valence());
    update('melodySmoothness', h.get_direct_melody_smoothness());
    update('voicingDensity', h.get_direct_voicing_density());
    update('voicingTension', h.get_direct_voicing_tension());

    return changed;
  }

  // Override mode control to track emotion mode locally
  override useEmotionMode(): void {
    this._isEmotionMode = true;
    super.useEmotionMode();
  }

  override useDirectMode(): void {
    this._isEmotionMode = false;
    super.useDirectMode();
  }

  // Override channel gain to handle WASM-specific channel mapping
  override setChannelGain(channel: number, gain: number): void {
    this.currentState.channelGains[channel] = gain;
    // Map channel index to specific gain setter (WASM-specific)
    if (channel === 0) this.handle?.set_gain_bass(gain);
    else if (channel === 1) this.handle?.set_gain_lead(gain);
    else if (channel === 2) this.handle?.set_gain_snare(gain);
    else if (channel === 3) this.handle?.set_gain_hat(gain);
  }

  // === SoundFont (WASM only) ===
  addSoundFont(bankId: number, data: Uint8Array): void {
    this.handle?.add_soundfont(bankId, data);
  }

  // Atomic rhythm update
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
    // WASM implementation: we can set them one by one or add a batch method if performance is critical.
    // Given the WASM speed, setting them individually is likely fast enough, 
    // but the key is they are all set within one JS event loop tick.
    if (!this.handle) return;

    // Direct setters
    this.handle.set_direct_rhythm_mode(mode);
    this.handle.set_direct_rhythm_steps(steps);
    this.handle.set_direct_rhythm_pulses(pulses);
    this.handle.set_direct_rhythm_rotation(rotation);
    this.handle.set_direct_rhythm_density(density);
    this.handle.set_direct_rhythm_tension(tension);
    this.handle.set_direct_secondary_steps(secondarySteps);
    this.handle.set_direct_secondary_pulses(secondaryPulses);
    this.handle.set_direct_secondary_rotation(secondaryRotation);

    // Update local state to reflect changes immediately
    this.currentState.rhythmMode = mode;
    this.currentState.primarySteps = steps;
    this.currentState.primaryPulses = pulses;
    this.currentState.primaryRotation = rotation;
    this.currentState.rhythmDensity = density;
    this.currentState.rhythmTension = tension;
    this.currentState.secondarySteps = secondarySteps;
    this.currentState.secondaryPulses = secondaryPulses;
    this.currentState.secondaryRotation = secondaryRotation;
  }
}
