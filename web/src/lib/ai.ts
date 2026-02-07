import { writable } from 'svelte/store';

export const aiStatus = writable<'idle' | 'loading' | 'ready' | 'error'>('idle');
export const aiError = writable<string | null>(null);

export interface EngineParams {
	arousal: number;
	valence: number;
	tension: number;
	density: number;
	smoothness: number;
}

export type Glossary = Record<string, number>;

export function glossaryToString(glossary: Glossary): string {
	const MAX_REPEATS = 5; // Au-delà de 5x, le modèle a compris, pas besoin de spammer
	const promptParts: string[] = [];

	for (const [word, count] of Object.entries(glossary)) {
		// On limite le nombre de répétitions pour garder le prompt concis
		// ex: count = 50 -> on répète juste 5 fois
		const repeats = Math.min(count, MAX_REPEATS);

		for (let i = 0; i < repeats; i++) {
			promptParts.push(word);
		}
	}

	// Résultat : "darkness darkness battle battle battle..."
	return promptParts.join(' ');
}

export class AIController {
	private worker: Worker | null = null;
	isReady: boolean = false;
	private pendingRequests = new Map<number, (result: EngineParams | null) => void>();
	private requestId = 0;

	async init() {
		if (this.isReady || this.worker) return;

		aiStatus.set('loading');
		aiError.set(null);

		try {
			// Initialize worker
			this.worker = new Worker(new URL('./ai.worker.ts', import.meta.url), {
				type: 'module'
			});

			this.worker.onmessage = (e) => {
				const { type, id, result, error } = e.data;

				if (type === 'ready') {
					this.isReady = true;
					aiStatus.set('ready');
					console.log('AI Engine Ready (Worker).');
				} else if (type === 'error') {
					console.error('AI Worker Error:', error);
					aiStatus.set('error');
					aiError.set(error);
				} else if (type === 'prediction') {
					const resolve = this.pendingRequests.get(id);
					if (resolve) {
						resolve(result);
						this.pendingRequests.delete(id);
					}
				}
			};

			this.worker.postMessage({ type: 'init' });
		} catch (e: unknown) {
			console.error('Failed to initialize AI:', e);
			aiStatus.set('error');
			const errorMessage = e instanceof Error ? e.message : 'Unknown error';
			aiError.set(errorMessage);
		}
	}

	/**
	 * Analyse le texte et retourne les paramètres moteur suggérés
	 * La logique est maintenant gérée par le Web Worker.
	 */
	predictParameters(input: string | Glossary): Promise<EngineParams | null> {
		if (!this.worker || !this.isReady) {
			console.warn('AI Engine not ready');
			return Promise.resolve(null);
		}

		let text = '';
		if (typeof input === 'string') {
			text = input;
		} else {
			text = glossaryToString(input);
		}

		const id = this.requestId++;
		return new Promise((resolve) => {
			this.pendingRequests.set(id, resolve);
			this.worker!.postMessage({ type: 'predict', id, data: text });
		});
	}
}

export const ai = new AIController();
