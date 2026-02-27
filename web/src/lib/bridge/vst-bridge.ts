// VST Bridge - Implementation for VST mode using nih-plug-webview IPC
import type { EngineState, AudioBackendType } from './types';
import { createEmptyState } from './types';
import { BaseBridge } from './base-bridge';

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

export class VstBridge extends BaseBridge {
	private connected = false;
	private previousOnPluginMessage: ((msg: unknown) => void) | undefined;

	constructor() {
		super(createEmptyState());
	}

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
		// VST backend is always native (controlled by plugin)
		this.currentState.audioBackend = 'fundsp';

		// Notify the plugin that webview is ready
		this.postMessage({ type: 'action', method: 'init' });
	}

	getAvailableBackends(): AudioBackendType[] {
		// VST mode uses plugin's native backend, cannot be changed
		return ['fundsp'];
	}

	disconnect(): void {
		// Restore previous handler
		window.onPluginMessage = this.previousOnPluginMessage;
		this.connected = false;
	}

	isConnected(): boolean {
		return this.connected;
	}

	/**
	 * Send command to VST plugin via IPC message passing.
	 */
	protected sendCommand(method: string, ...args: unknown[]): void {
		// Convert args array to params object
		const params: Record<string, unknown> = {};

		// Map common parameter names
		if (args.length === 1) {
			// Single argument - determine the parameter name from method
			if (method.includes('enable')) {
				params.enabled = args[0];
			} else if (method.includes('steps')) {
				params.steps = args[0];
			} else if (method.includes('pulses')) {
				params.pulses = args[0];
			} else if (method.includes('rotation')) {
				params.rotation = args[0];
			} else if (method.includes('mode')) {
				params.mode = args[0];
			} else if (method.includes('channel')) {
				params.channel = args[0];
			} else {
				params.value = args[0];
			}
		} else if (args.length === 2) {
			// Two arguments - typically channel operations
			params.channel = args[0];
			if (method.includes('muted')) {
				params.muted = args[1];
			} else if (method.includes('routing')) {
				params.routing = args[1];
			} else if (method.includes('gain')) {
				params.gain = args[1];
			} else {
				params.value = args[1];
			}
		} else {
			// Catch-all for multi-argument commands like setAllRhythmParams
			if (method === 'set_all_rhythm_params') {
				params.mode = args[0];
				params.steps = args[1];
				params.pulses = args[2];
				params.rotation = args[3];
				params.density = args[4];
				params.tension = args[5];
				params.secondarySteps = args[6];
				params.secondaryPulses = args[7];
				params.secondaryRotation = args[8];
			}
		}

		const message: VstRequest = {
			type: method.includes('use_') ? 'action' : 'set',
			method,
			params
		};
		this.postMessage(message);
	}

	// Atomic rhythm update proxy
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

	getState(): EngineState | null {
		return { ...this.currentState };
	}

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	getLookaheadTruth(_steps: number): string {
		// Lookahead IPC not yet implemented for VST
		return '{}';
	}

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	getLookaheadScore(_bars: number): string {
		// HarmoniumScore lookahead not yet implemented for VST
		return '{}';
	}

	private handleMessage = (msg: unknown) => {
		try {
			// Message is already parsed by nih-plug-webview
			const parsed = msg as { type?: string; data?: EngineState };

			if (parsed.type === 'state_update' && parsed.data) {
				const update = parsed as VstStateUpdate;
				this.currentState = { ...this.currentState, ...update.data };
				this.subscribers.forEach((cb) => cb(this.currentState));
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
}
