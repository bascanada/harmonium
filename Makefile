.PHONY: run web/build web/serve web/install models/download

run:
	cargo run

wasm/build:
	wasm-pack build --target web

web/build: wasm/build
	cd web && npm run build

web/serve:
	cd web && npm run dev

web/install:
	cd web && npm install

models/clear:
	rm -rf web/static/models/*

models/download: models/clear
	mkdir -p web/static/models
	# ~80MB - Best balance of size/performance for embeddings
	huggingface-cli download sentence-transformers/all-MiniLM-L6-v2 config.json model.safetensors tokenizer.json --local-dir web/static/models --local-dir-use-symlinks False

models/download-tiny: models/clear
	mkdir -p web/static/models
	# ~17MB - Extremely small, but lower quality embeddings
	huggingface-cli download prajjwal1/bert-tiny config.json model.safetensors tokenizer.json --local-dir web/static/models --local-dir-use-symlinks False

models/download-emotion: models/clear
	mkdir -p web/static/models
	# ~260MB - Specialized Emotion Detection (DistilBERT) - REQUIRES CODE CHANGE to DistilBertModel
	huggingface-cli download bhadresh-savani/distilbert-base-uncased-emotion config.json model.safetensors tokenizer.json --local-dir web/static/models --local-dir-use-symlinks False