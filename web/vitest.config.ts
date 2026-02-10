import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	test: {
		browser: {
			enabled: true,
			instances: [
				{
					browser: 'chromium',
				}
			],
			provider: playwright(),
			headless: process.env.CI ? true : false
		},
		include: ['src/**/*.{test,spec}.{js,ts}'],
		setupFiles: ['./src/vitest-setup-client.ts']
	}
});
