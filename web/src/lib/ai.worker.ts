// Dynamic import to avoid build-time errors when EmotionEngine is not available
let engine: any = null;

const MODEL_REPO = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main";

self.onmessage = async (e: MessageEvent) => {
    const { type, id, data } = e.data;

    switch (type) {
        case 'init':
            try {
                // Dynamic import to handle case where AI feature is not compiled
                const harmonium = await import('harmonium');
                await harmonium.default(); // Initialize WASM

                // Check if EmotionEngine exists in the module
                const EmotionEngine = (harmonium as any).EmotionEngine;

                if (!EmotionEngine) {
                    console.warn("EmotionEngine not available - AI feature not compiled into WASM");
                    self.postMessage({ type: 'error', error: 'AI feature not available in this build' });
                    return;
                }

                const [config, weights, tokenizer] = await Promise.all([
                    fetch(`${MODEL_REPO}/config.json`).then(r => {
                        if (!r.ok) throw new Error("Failed to load config.json");
                        return r.arrayBuffer();
                    }),
                    fetch(`${MODEL_REPO}/model.safetensors`).then(r => {
                        if (!r.ok) throw new Error("Failed to load model.safetensors");
                        return r.arrayBuffer();
                    }),
                    fetch(`${MODEL_REPO}/tokenizer.json`).then(r => {
                        if (!r.ok) throw new Error("Failed to load tokenizer.json");
                        return r.arrayBuffer();
                    })
                ]);

                engine = EmotionEngine.new(
                    new Uint8Array(config),
                    new Uint8Array(weights),
                    new Uint8Array(tokenizer)
                );

                self.postMessage({ type: 'ready' });
            } catch (error: any) {
                console.error("AI Worker initialization error:", error);
                self.postMessage({ type: 'error', error: error.message });
            }
            break;

        case 'predict':
            if (!engine) {
                self.postMessage({ type: 'prediction', id, result: null });
                return;
            }
            try {
                const result = engine.predict(data);
                self.postMessage({ type: 'prediction', id, result });
            } catch (error: any) {
                console.error("Worker prediction error:", error);
                self.postMessage({ type: 'prediction', id, result: null });
            }
            break;
    }
};
