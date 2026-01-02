import init, { EmotionEngine } from 'harmonium';

let engine: EmotionEngine | null = null;


const MODEL_REPO = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main";

self.onmessage = async (e: MessageEvent) => {
    const { type, id, data } = e.data;

    switch (type) {
        case 'init':
            try {
                await init();
                
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
