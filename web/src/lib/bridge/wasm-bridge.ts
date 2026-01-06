// WASM Bridge - Implementation for web mode using harmonium.js
import init, { start, start_with_backend, get_available_backends, type Handle } from 'harmonium';
import type { HarmoniumBridge, EngineState, AudioBackendType } from './types';
import { createEmptyState } from './types';

export class WasmBridge implements HarmoniumBridge {
  private handle: Handle | null = null;
  private animationId: number | null = null;
  private subscribers: Set<(state: EngineState) => void> = new Set();
  private currentState: EngineState = createEmptyState();
  private _isEmotionMode = true;
  private _currentBackend: AudioBackendType = 'fundsp';
  private _availableBackends: AudioBackendType[] = ['fundsp'];

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

  private startPolling(): void {
    const poll = () => {
      if (!this.handle) return;

      this.collectState();
      this.subscribers.forEach(cb => cb(this.currentState));

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

  // === Mode Control ===
  useEmotionMode(): void {
    this._isEmotionMode = true;
    this.handle?.use_emotion_mode();
  }

  useDirectMode(): void {
    this._isEmotionMode = false;
    this.handle?.use_direct_mode();
  }

  // === Emotional Controls ===
  setArousal(value: number): void {
    this.handle?.set_arousal(value);
    this.currentState.arousal = value;
  }

  setValence(value: number): void {
    this.handle?.set_valence(value);
    this.currentState.valence = value;
  }

  setDensity(value: number): void {
    this.handle?.set_density(value);
    this.currentState.density = value;
  }

  setTension(value: number): void {
    this.handle?.set_tension(value);
    this.currentState.tension = value;
  }

  // === Algorithm & Harmony Mode ===
  setAlgorithm(mode: number): void {
    this.handle?.set_algorithm(mode);
  }

  setHarmonyMode(mode: number): void {
    this.handle?.set_harmony_mode(mode);
  }

  setPolySteps(steps: number): void {
    this.handle?.set_poly_steps(steps);
  }

  // === Direct Controls ===
  setDirectBpm(value: number): void {
    this.handle?.set_direct_bpm(value);
  }

  setDirectEnableRhythm(enabled: boolean): void {
    this.handle?.set_direct_enable_rhythm(enabled);
  }

  setDirectEnableHarmony(enabled: boolean): void {
    this.handle?.set_direct_enable_harmony(enabled);
  }

  setDirectEnableMelody(enabled: boolean): void {
    this.handle?.set_direct_enable_melody(enabled);
  }

  setDirectEnableVoicing(enabled: boolean): void {
    this.handle?.set_direct_enable_voicing(enabled);
  }

  setDirectRhythmMode(mode: number): void {
    this.handle?.set_direct_rhythm_mode(mode);
  }

  setDirectRhythmSteps(steps: number): void {
    this.handle?.set_direct_rhythm_steps(steps);
  }

  setDirectRhythmPulses(pulses: number): void {
    this.handle?.set_direct_rhythm_pulses(pulses);
  }

  setDirectRhythmRotation(rotation: number): void {
    this.handle?.set_direct_rhythm_rotation(rotation);
  }

  setDirectRhythmDensity(density: number): void {
    this.handle?.set_direct_rhythm_density(density);
  }

  setDirectRhythmTension(tension: number): void {
    this.handle?.set_direct_rhythm_tension(tension);
  }

  setDirectSecondarySteps(steps: number): void {
    this.handle?.set_direct_secondary_steps(steps);
  }

  setDirectSecondaryPulses(pulses: number): void {
    this.handle?.set_direct_secondary_pulses(pulses);
  }

  setDirectSecondaryRotation(rotation: number): void {
    this.handle?.set_direct_secondary_rotation(rotation);
  }

  setDirectHarmonyTension(tension: number): void {
    this.handle?.set_direct_harmony_tension(tension);
  }

  setDirectHarmonyValence(valence: number): void {
    this.handle?.set_direct_harmony_valence(valence);
  }

  setDirectMelodySmoothness(smoothness: number): void {
    this.handle?.set_direct_melody_smoothness(smoothness);
  }

  setDirectVoicingDensity(density: number): void {
    this.handle?.set_direct_voicing_density(density);
  }

  setDirectVoicingTension(tension: number): void {
    this.handle?.set_direct_voicing_tension(tension);
  }

  // === Channel Controls ===
  setChannelMuted(channel: number, muted: boolean): void {
    this.handle?.set_channel_muted(channel, muted);
    this.currentState.channelMuted[channel] = muted;
  }

  setChannelRouting(channel: number, routing: number): void {
    this.handle?.set_channel_routing(channel, routing);
  }

  setChannelGain(channel: number, gain: number): void {
    this.currentState.channelGains[channel] = gain;
    // Map channel index to specific gain setter
    if (channel === 0) this.handle?.set_gain_bass(gain);
    else if (channel === 1) this.handle?.set_gain_lead(gain);
    else if (channel === 2) this.handle?.set_gain_snare(gain);
    else if (channel === 3) this.handle?.set_gain_hat(gain);
  }

  // === SoundFont ===
  addSoundFont(bankId: number, data: Uint8Array): void {
    this.handle?.add_soundfont(bankId, data);
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
    return this.handle ? { ...this.currentState } : null;
  }
}
