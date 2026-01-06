// VST Bridge - Implementation for VST mode using nih-plug-webview IPC
import type { HarmoniumBridge, EngineState } from './types';
import { createEmptyState } from './types';

// Message types for VST <-> Webview communication
interface VstRequest {
  type: 'set' | 'get' | 'action';
  method: string;
  params?: Record<string, unknown>;
}

interface VstStateUpdate {
  type: 'state_update';
  data: EngineState;
}

// Declare the IPC interface that nih-plug-webview provides via script.js
declare global {
  interface Window {
    // Injected by nih-plug-webview
    ipc?: {
      postMessage(message: string): void;
    };
    sendToPlugin?: (msg: unknown) => void;
    onPluginMessage?: (msg: unknown) => void;
  }
}

export class VstBridge implements HarmoniumBridge {
  private subscribers: Set<(state: EngineState) => void> = new Set();
  private currentState: EngineState = createEmptyState();
  private connected = false;
  private previousOnPluginMessage: ((msg: unknown) => void) | undefined;

  async connect(): Promise<void> {
    // Store previous handler if any
    this.previousOnPluginMessage = window.onPluginMessage;

    // Set up message listener using nih-plug-webview's callback mechanism
    window.onPluginMessage = (msg: unknown) => {
      this.handleMessage(msg);
      // Chain to previous handler if it existed
      if (this.previousOnPluginMessage) {
        this.previousOnPluginMessage(msg);
      }
    };

    this.connected = true;

    // Notify the plugin that webview is ready
    this.postMessage({ type: 'action', method: 'init' });
  }

  disconnect(): void {
    // Restore previous handler
    window.onPluginMessage = this.previousOnPluginMessage;
    this.connected = false;
  }

  isConnected(): boolean {
    return this.connected;
  }

  private handleMessage = (msg: unknown) => {
    try {
      // Message is already parsed by nih-plug-webview
      const parsed = msg as { type?: string; data?: EngineState };

      if (parsed.type === 'state_update' && parsed.data) {
        const update = parsed as VstStateUpdate;
        this.currentState = { ...this.currentState, ...update.data };
        this.subscribers.forEach(cb => cb(this.currentState));
      }
    } catch (e) {
      console.error('[VstBridge] Error handling message:', e, msg);
    }
  };

  private postMessage(msg: VstRequest): void {
    // Use nih-plug-webview's sendToPlugin helper if available
    if (window.sendToPlugin) {
      window.sendToPlugin(msg);
    } else if (window.ipc?.postMessage) {
      window.ipc.postMessage(JSON.stringify(msg));
    } else {
      // Fallback for development/testing without actual VST
      console.warn('[VstBridge] No IPC available, message not sent:', msg);
    }
  }

  // === Mode Control ===
  useEmotionMode(): void {
    this.postMessage({ type: 'action', method: 'use_emotion_mode' });
    this.currentState.isEmotionMode = true;
  }

  useDirectMode(): void {
    this.postMessage({ type: 'action', method: 'use_direct_mode' });
    this.currentState.isEmotionMode = false;
  }

  // === Emotional Controls ===
  setArousal(value: number): void {
    this.postMessage({ type: 'set', method: 'set_arousal', params: { value } });
    this.currentState.arousal = value;
  }

  setValence(value: number): void {
    this.postMessage({ type: 'set', method: 'set_valence', params: { value } });
    this.currentState.valence = value;
  }

  setDensity(value: number): void {
    this.postMessage({ type: 'set', method: 'set_density', params: { value } });
    this.currentState.density = value;
  }

  setTension(value: number): void {
    this.postMessage({ type: 'set', method: 'set_tension', params: { value } });
    this.currentState.tension = value;
  }

  // === Algorithm & Harmony Mode ===
  setAlgorithm(mode: number): void {
    this.postMessage({ type: 'set', method: 'set_algorithm', params: { mode } });
  }

  setHarmonyMode(mode: number): void {
    this.postMessage({ type: 'set', method: 'set_harmony_mode', params: { mode } });
  }

  setPolySteps(steps: number): void {
    this.postMessage({ type: 'set', method: 'set_poly_steps', params: { steps } });
  }

  // === Direct Controls ===
  setDirectBpm(value: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_bpm', params: { value } });
  }

  setDirectEnableRhythm(enabled: boolean): void {
    this.postMessage({ type: 'set', method: 'set_direct_enable_rhythm', params: { enabled } });
  }

  setDirectEnableHarmony(enabled: boolean): void {
    this.postMessage({ type: 'set', method: 'set_direct_enable_harmony', params: { enabled } });
  }

  setDirectEnableMelody(enabled: boolean): void {
    this.postMessage({ type: 'set', method: 'set_direct_enable_melody', params: { enabled } });
  }

  setDirectEnableVoicing(enabled: boolean): void {
    this.postMessage({ type: 'set', method: 'set_direct_enable_voicing', params: { enabled } });
  }

  setDirectRhythmMode(mode: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_mode', params: { mode } });
  }

  setDirectRhythmSteps(steps: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_steps', params: { steps } });
  }

  setDirectRhythmPulses(pulses: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_pulses', params: { pulses } });
  }

  setDirectRhythmRotation(rotation: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_rotation', params: { rotation } });
  }

  setDirectRhythmDensity(density: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_density', params: { density } });
  }

  setDirectRhythmTension(tension: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_rhythm_tension', params: { tension } });
  }

  setDirectSecondarySteps(steps: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_secondary_steps', params: { steps } });
  }

  setDirectSecondaryPulses(pulses: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_secondary_pulses', params: { pulses } });
  }

  setDirectSecondaryRotation(rotation: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_secondary_rotation', params: { rotation } });
  }

  setDirectHarmonyTension(tension: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_harmony_tension', params: { tension } });
  }

  setDirectHarmonyValence(valence: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_harmony_valence', params: { valence } });
  }

  setDirectMelodySmoothness(smoothness: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_melody_smoothness', params: { smoothness } });
  }

  setDirectVoicingDensity(density: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_voicing_density', params: { density } });
  }

  setDirectVoicingTension(tension: number): void {
    this.postMessage({ type: 'set', method: 'set_direct_voicing_tension', params: { tension } });
  }

  // === Channel Controls ===
  setChannelMuted(channel: number, muted: boolean): void {
    this.postMessage({ type: 'set', method: 'set_channel_muted', params: { channel, muted } });
    this.currentState.channelMuted[channel] = muted;
  }

  setChannelRouting(channel: number, routing: number): void {
    this.postMessage({ type: 'set', method: 'set_channel_routing', params: { channel, routing } });
  }

  setChannelGain(channel: number, gain: number): void {
    this.postMessage({ type: 'set', method: 'set_channel_gain', params: { channel, gain } });
    this.currentState.channelGains[channel] = gain;
  }

  // === SoundFont (not supported in VST mode - SoundFonts loaded through DAW) ===
  // This method is optional in the interface

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
    return this.connected ? { ...this.currentState } : null;
  }
}
