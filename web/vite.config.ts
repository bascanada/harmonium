import devtoolsJson from 'vite-plugin-devtools-json';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), devtoolsJson()],
	server: {
		fs: {
			allow: ['..']
		}
	},
	worker: {
		format: 'es'
	},
	build: {
		rollupOptions: {
			// Suppress eval warning from wasm-bindgen generated code (harmonium.js)
			// This is expected behavior from wasm-bindgen and cannot be avoided
			onwarn(warning, warn) {
				if (warning.code === 'EVAL' && warning.id?.includes('harmonium.js')) {
					return;
				}
				warn(warning);
			}
		}
	}
});
