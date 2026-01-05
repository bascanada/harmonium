HF_CLI ?= hf
BUNDLE_DIR := target/bundled
VST3_PATH := $(BUNDLE_DIR)/harmonium.vst3
CLAP_PATH := $(BUNDLE_DIR)/harmonium.clap
APP_PATH := $(BUNDLE_DIR)/harmonium.app
INSTALL_VST3 := ~/Library/Audio/Plug-Ins/VST3
INSTALL_CLAP := ~/Library/Audio/Plug-Ins/CLAP
PLUGINVAL := /Applications/pluginval.app/Contents/MacOS/pluginval

.PHONY: run test web/build web/serve web/install models/download vst vst/install vst/uninstall vst/validate vst/run

# ════════════════════════════════════════════════════════════════════
# STANDALONE / CLI
# ════════════════════════════════════════════════════════════════════

run:
	cargo run -- $(ARGS)

test:
	cargo test --lib

# ════════════════════════════════════════════════════════════════════
# VST / CLAP PLUGIN
# ════════════════════════════════════════════════════════════════════

## Build VST3 and CLAP plugins (release)
vst:
	@echo "Building VST3 & CLAP plugins..."
	cargo xtask bundle harmonium --release --no-default-features --features vst
	@echo "Plugins built:"
	@ls -lh $(BUNDLE_DIR)/*.vst3 $(BUNDLE_DIR)/*.clap 2>/dev/null || true

## Build VST3 and CLAP plugins (debug)
vst/debug:
	@echo "Building VST3 & CLAP plugins (debug)..."
	cargo xtask bundle harmonium --no-default-features --features vst

## Install plugins to system directories
vst/install: vst
	@echo "Installing plugins..."
	@mkdir -p $(INSTALL_VST3) $(INSTALL_CLAP)
	@cp -r $(VST3_PATH) $(INSTALL_VST3)/
	@cp -r $(CLAP_PATH) $(INSTALL_CLAP)/
	@echo "Installed to:"
	@echo "   - $(INSTALL_VST3)/harmonium.vst3"
	@echo "   - $(INSTALL_CLAP)/harmonium.clap"
	@echo "Restart your DAW to detect the new plugins"

## Uninstall plugins from system directories
vst/uninstall:
	@echo "Uninstalling plugins..."
	@rm -rf $(INSTALL_VST3)/harmonium.vst3
	@rm -rf $(INSTALL_CLAP)/harmonium.clap
	@echo "Plugins removed"

## Validate VST3 with pluginval
vst/validate: vst
	@echo "Validating VST3 plugin..."
	@if [ -f "$(PLUGINVAL)" ]; then \
		$(PLUGINVAL) --validate $(VST3_PATH) --strictness-level 1; \
	else \
		echo "pluginval not found. Install with: brew install --cask pluginval"; \
	fi

## Run the standalone VST app (GUI)
vst/run: vst
	@echo "🎹 Launching standalone app..."
	@open $(APP_PATH)

## Show plugin sizes
vst/info:
	@echo "Plugin Info:"
	@if [ -d "$(VST3_PATH)" ]; then \
		echo "   VST3: $$(du -sh $(VST3_PATH) | cut -f1)"; \
		echo "   CLAP: $$(du -sh $(CLAP_PATH) | cut -f1)"; \
		echo "   App:  $$(du -sh $(APP_PATH) | cut -f1)"; \
	else \
		echo "   No plugins built. Run 'make vst' first."; \
	fi

# ════════════════════════════════════════════════════════════════════
# WASM / WEB
# ════════════════════════════════════════════════════════════════════

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
	$(HF_CLI) download sentence-transformers/all-MiniLM-L6-v2 config.json model.safetensors tokenizer.json --local-dir web/static/models

models/download-tiny: models/clear
	mkdir -p web/static/models
	# ~17MB - Extremely small, but lower quality embeddings
	$(HF_CLI) download prajjwal1/bert-tiny config.json model.safetensors tokenizer.json --local-dir web/static/models

models/download-emotion: models/clear
	mkdir -p web/static/models
	# ~260MB - Specialized Emotion Detection (DistilBERT) - REQUIRES CODE CHANGE to DistilBertModel
	$(HF_CLI) download bhadresh-savani/distilbert-base-uncased-emotion config.json model.safetensors tokenizer.json --local-dir web/static/models

python/venv:
	python3 -m venv .venv

python/install: python/venv
	. .venv/bin/activate && pip install -r scripts/requirements.txt

python/run:
	. .venv/bin/activate && python3 scripts/video_to_osc.py --video "$(VIDEO)"
