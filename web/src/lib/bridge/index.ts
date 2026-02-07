// Bridge module - Abstraction layer for WASM and VST backends
export * from './types';
export { WasmBridge } from './wasm-bridge';
export { VstBridge } from './vst-bridge';

import type { HarmoniumBridge } from './types';
import { WasmBridge } from './wasm-bridge';
import { VstBridge } from './vst-bridge';

/**
 * Detect if we're running inside a VST webview
 * nih-plug-webview provides window.ipc
 */
export function isVstMode(): boolean {
	return typeof window !== 'undefined' && 'ipc' in window;
}

/**
 * Create the appropriate bridge based on the runtime environment
 */
export function createBridge(mode?: 'wasm' | 'vst'): HarmoniumBridge {
	const resolvedMode = mode ?? (isVstMode() ? 'vst' : 'wasm');

	if (resolvedMode === 'vst') {
		return new VstBridge();
	} else {
		return new WasmBridge();
	}
}

/**
 * Create a bridge and automatically connect
 */
export async function createAndConnectBridge(
	mode?: 'wasm' | 'vst',
	sf2Data?: Uint8Array
): Promise<HarmoniumBridge> {
	const bridge = createBridge(mode);
	await bridge.connect(sf2Data);
	return bridge;
}
