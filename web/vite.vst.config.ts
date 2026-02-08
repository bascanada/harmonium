import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

// VST build config - produces a single HTML file with inlined assets
export default defineConfig({
	plugins: [tailwindcss(), svelte()],
	root: 'src/vst',
	base: './',
	resolve: {
		alias: {
			$lib: resolve(__dirname, 'src/lib'),
			harmonium: resolve(__dirname, '../pkg')
		}
	},
	build: {
		outDir: '../../dist/vst',
		emptyOutDir: true,
		// Inline all assets into HTML
		assetsInlineLimit: 100000000,
		rollupOptions: {
			input: resolve(__dirname, 'src/vst/index.html'),
			output: {
				// Single chunk output
				inlineDynamicImports: true,
				// Ensure CSS is inlined
				assetFileNames: '[name][extname]',
				chunkFileNames: '[name].js',
				entryFileNames: '[name].js'
			}
		},
		// Minify for smaller bundle (using esbuild which is built-in)
		minify: 'esbuild'
	},
	worker: {
		// Use ES format for workers (compatible with inlineDynamicImports)
		format: 'es',
		rollupOptions: {
			output: {
				inlineDynamicImports: true
			}
		}
	},
	// No SvelteKit for VST build
	server: {
		fs: {
			allow: ['..']
		}
	}
});
