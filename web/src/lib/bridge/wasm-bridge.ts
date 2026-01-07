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

      this.collectState();
      // Create a shallow copy to ensure Svelte detects the change
      this.subscribers.forEach(cb => cb({ ...this.currentState }));

      // Clear event queue
      this.handle.get_events();

      this.animationId = requestAnimationFrame(poll);
    };
    this.animationId = requestAnimationFrame(poll);
  }

  private collectState(): void {
    const h = this.handle!;

    // Harmony state
    this.currentState.currentChord = h.get_current_chord_name();
    this.currentState.currentMeasure = h.get_current_measure();
    this.currentState.currentStep = h.get_current_step();
    this.currentState.isMinorChord = h.is_current_chord_minor();
    this.currentState.progressionName = h.get_progression_name();
    this.currentState.progressionLength = h.get_progression_length();
    this.currentState.harmonyMode = h.get_harmony_mode();

    // Rhythm state - Primary
    this.currentState.primarySteps = h.get_primary_steps();
    this.currentState.primaryPulses = h.get_primary_pulses();
    this.currentState.primaryRotation = h.get_primary_rotation();
    const rawPrimary = h.get_primary_pattern();
    this.currentState.primaryPattern = Array.from(rawPrimary).map(v => v === 1);

    // Rhythm state - Secondary
    this.currentState.secondarySteps = h.get_secondary_steps();
    this.currentState.secondaryPulses = h.get_secondary_pulses();
    this.currentState.secondaryRotation = h.get_secondary_rotation();
    const rawSecondary = h.get_secondary_pattern();
    this.currentState.secondaryPattern = Array.from(rawSecondary).map(v => v === 1);

    // Control mode
    this.currentState.isEmotionMode = this._isEmotionMode;

    // Direct params (always sync from engine)
    this.currentState.bpm = h.get_direct_bpm();
    this.currentState.rhythmMode = h.get_direct_rhythm_mode();
    this.currentState.enableRhythm = h.get_direct_enable_rhythm();
    this.currentState.enableHarmony = h.get_direct_enable_harmony();
    this.currentState.enableMelody = h.get_direct_enable_melody();
    this.currentState.rhythmDensity = h.get_direct_rhythm_density();
    this.currentState.rhythmTension = h.get_direct_rhythm_tension();
    this.currentState.harmonyTension = h.get_direct_harmony_tension();
    this.currentState.harmonyValence = h.get_direct_harmony_valence();
    this.currentState.melodySmoothness = h.get_direct_melody_smoothness();
    this.currentState.voicingDensity = h.get_direct_voicing_density();
    this.currentState.voicingTension = h.get_direct_voicing_tension();
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
}
