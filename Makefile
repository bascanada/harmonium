.PHONY: run web/build web/serve web/install

run:
	cargo run

wasm/build:
	wasm-pack build --target web

web/serve:
	cd web && npm run dev

web/install:
	cd web && npm install