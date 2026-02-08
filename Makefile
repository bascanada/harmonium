HF_CLI ?= hf
BUNDLE_DIR := target/bundled
DIST_DIR := dist
VST3_PATH := $(BUNDLE_DIR)/harmonium.vst3
CLAP_PATH := $(BUNDLE_DIR)/harmonium.clap
APP_PATH := $(BUNDLE_DIR)/harmonium.app
INSTALL_VST3 := ~/Library/Audio/Plug-Ins/VST3
INSTALL_CLAP := ~/Library/Audio/Plug-Ins/CLAP
PLUGINVAL := /Applications/pluginval.app/Contents/MacOS/pluginval

.PHONY: run test web/build web/build/watch web/build-vst web/serve web/install models/download vst vst/install vst/uninstall vst/clean vst/validate vst/run release release/cli release/plugins release/clean

# ════════════════════════════════════════════════════════════════════
# STANDALONE / CLI
# ════════════════════════════════════════════════════════════════════

run:
	cargo run -p harmonium --release -- $(ARGS)

run/debug:
	cargo run -p harmonium -- $(ARGS)

test:
	cargo test --lib

test/sheet:
	cargo test -p harmonium_core --test music_generation_tests generate_all_music_tests -- --ignored --nocapture

# ════════════════════════════════════════════════════════════════════
# VST / CLAP PLUGIN
# ════════════════════════════════════════════════════════════════════

## Build VST3 and CLAP plugins with GUI (release)
vst: web/build-vst
	@echo "Building VST3 & CLAP plugins..."
	cargo xtask bundle harmonium --release --no-default-features --features vst-gui
	@echo "Plugins built:"
	@ls -lh $(BUNDLE_DIR)/*.vst3 $(BUNDLE_DIR)/*.clap 2>/dev/null || true

## Build VST3 and CLAP plugins with GUI (debug)
vst/debug: web/build-vst
	@echo "Building VST3 & CLAP plugins (debug)..."
	cargo xtask bundle harmonium --no-default-features --features vst-gui

## Install plugins to system directories
vst/install: vst
	@echo "Installing plugins..."
	@mkdir -p $(INSTALL_VST3) $(INSTALL_CLAP)
	@cp -r $(VST3_PATH) $(INSTALL_VST3)/
	@cp -r $(CLAP_PATH) $(INSTALL_CLAP)/
	@echo "Removing quarantine (bypass Gatekeeper for unsigned plugins)..."
	@xattr -cr $(INSTALL_VST3)/harmonium.vst3
	@xattr -cr $(INSTALL_CLAP)/harmonium.clap
	@echo "Installed to:"
	@echo "   - $(INSTALL_VST3)/harmonium.vst3"
	@echo "   - $(INSTALL_CLAP)/harmonium.clap"
	@echo "Restart your DAW to detect the new plugins"

## Clean/uninstall manually installed plugins (run before using brew)
vst/clean:
	@echo "Removing manually installed plugins..."
	@rm -rf $(INSTALL_VST3)/harmonium.vst3
	@rm -rf $(INSTALL_CLAP)/harmonium.clap
	@echo "Plugins removed from:"
	@echo "   - $(INSTALL_VST3)/harmonium.vst3"
	@echo "   - $(INSTALL_CLAP)/harmonium.clap"
	@echo ""
	@echo "You can now install via brew:"
	@echo "   brew tap bascanada/tap"
	@echo "   brew install --cask harmonium-plugins"

## Alias for vst/clean
vst/uninstall: vst/clean

## Validate VST3 with pluginval
vst/validate: vst
	@echo "Validating VST3 plugin..."
	@if [ -f "$(PLUGINVAL)" ]; then \
		$(PLUGINVAL) --validate $(VST3_PATH); \
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
	RUSTFLAGS="--cfg=wasm_js" wasm-pack build harmonium_host --target web --out-dir ../pkg

web/build: wasm/build
	cd web && npm run build

## Watch and rebuild VST webview interface
web/build/watch:
	cd web && npm run build:vst:watch

## Alias for web/build/watch
build/web/watch: web/build/watch

## Build VST webview interface (Svelte -> embedded HTML)
web/build-vst: wasm/build
	@echo "Building VST webview interface..."
	cd web && npm run build:vst
	@echo "VST interface built: web/dist/vst/"

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

# ════════════════════════════════════════════════════════════════════
# QUALITY & CI
# ════════════════════════════════════════════════════════════════════

fmt:
	cargo fmt --all -- --check

lint: web/build-vst
	cargo clippy --workspace --all-targets --all-features -- -A clippy::all -W clippy::unwrap_used -W clippy::panic -W clippy::todo -W clippy::expect_used

# Override test to use workspace
test:
	cargo test --workspace

audit:
	# Checks for security vulnerabilities in dependencies
	cargo audit

quality: fmt lint test web/quality
	@echo "✨ All quality checks passed!"

web/check:
	cd web && npm run check

web/lint:
	cd web && npm run lint

web/quality: web/check web/lint

# ════════════════════════════════════════════════════════════════════
# RELEASE BUILDS (Universal macOS binaries)
# ════════════════════════════════════════════════════════════════════

## Clean dist directory
release/clean:
	@rm -rf $(DIST_DIR)
	@echo "Cleaned $(DIST_DIR)"

## Build universal CLI binary (ARM64 + x86_64)
release/cli:
	@echo "Building CLI for ARM64..."
	@cargo build --release --no-default-features --features standalone --target aarch64-apple-darwin
	@echo "Building CLI for x86_64..."
	@cargo build --release --no-default-features --features standalone --target x86_64-apple-darwin
	@echo "Creating universal binary..."
	@mkdir -p $(DIST_DIR)
	@lipo -create \
		target/aarch64-apple-darwin/release/harmonium \
		target/x86_64-apple-darwin/release/harmonium \
		-output $(DIST_DIR)/harmonium
	@chmod +x $(DIST_DIR)/harmonium
	@codesign --force --deep --sign - $(DIST_DIR)/harmonium
	@echo "CLI built: $(DIST_DIR)/harmonium"
	@file $(DIST_DIR)/harmonium

## Build universal VST3/CLAP plugins (ARM64 + x86_64)
release/plugins: web/build-vst
	@echo "Building plugins for ARM64..."
	@cargo xtask bundle harmonium --release --no-default-features --features vst-gui --target aarch64-apple-darwin
	@mkdir -p $(DIST_DIR)/arm64
	@cp -r $(BUNDLE_DIR)/harmonium.vst3 $(DIST_DIR)/arm64/
	@cp -r $(BUNDLE_DIR)/harmonium.clap $(DIST_DIR)/arm64/
	@echo "Building plugins for x86_64..."
	@cargo xtask bundle harmonium --release --no-default-features --features vst-gui --target x86_64-apple-darwin
	@mkdir -p $(DIST_DIR)/x86_64
	@cp -r $(BUNDLE_DIR)/harmonium.vst3 $(DIST_DIR)/x86_64/
	@cp -r $(BUNDLE_DIR)/harmonium.clap $(DIST_DIR)/x86_64/
	@echo "Creating universal plugin bundles..."
	@mkdir -p $(DIST_DIR)/harmonium.vst3/Contents/MacOS
	@mkdir -p $(DIST_DIR)/harmonium.clap/Contents/MacOS
	@cp -r $(DIST_DIR)/arm64/harmonium.vst3/Contents/Info.plist $(DIST_DIR)/harmonium.vst3/Contents/
	@cp -r $(DIST_DIR)/arm64/harmonium.clap/Contents/Info.plist $(DIST_DIR)/harmonium.clap/Contents/
	@lipo -create \
		$(DIST_DIR)/arm64/harmonium.vst3/Contents/MacOS/harmonium \
		$(DIST_DIR)/x86_64/harmonium.vst3/Contents/MacOS/harmonium \
		-output $(DIST_DIR)/harmonium.vst3/Contents/MacOS/harmonium
	@lipo -create \
		$(DIST_DIR)/arm64/harmonium.clap/Contents/MacOS/harmonium \
		$(DIST_DIR)/x86_64/harmonium.clap/Contents/MacOS/harmonium \
		-output $(DIST_DIR)/harmonium.clap/Contents/MacOS/harmonium
	@rm -rf $(DIST_DIR)/arm64 $(DIST_DIR)/x86_64
	@codesign --force --deep --sign - $(DIST_DIR)/harmonium.vst3
	@codesign --force --deep --sign - $(DIST_DIR)/harmonium.clap
	@echo "Plugins built:"
	@du -sh $(DIST_DIR)/harmonium.vst3 $(DIST_DIR)/harmonium.clap

## Build and package everything for release
release: release/clean release/cli release/plugins
	@echo "Packaging CLI..."
	@cd $(DIST_DIR) && tar -czvf harmonium-cli-macos-universal.tar.gz harmonium
	@echo "Packaging plugins..."
	@cd $(DIST_DIR) && zip -r harmonium-plugins-macos-universal.zip harmonium.vst3 harmonium.clap
	@echo ""
	@echo "Release artifacts:"
	@ls -lh $(DIST_DIR)/*.tar.gz $(DIST_DIR)/*.zip
	@echo ""
	@echo "Checksums:"
	@cd $(DIST_DIR) && shasum -a 256 *.tar.gz *.zip
