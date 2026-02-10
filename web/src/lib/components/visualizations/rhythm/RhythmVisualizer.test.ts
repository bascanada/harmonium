import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';
import RhythmVisualizer from './RhythmVisualizer.svelte';

test('renders one circle in PerfectBalance mode', async () => {
	render(RhythmVisualizer, {
		rhythmMode: 1, // PerfectBalance
		primarySteps: 16,
		rhythmDensity: 0.5,
		rhythmTension: 0.3
	});

	// Header tag
	await expect.element(page.getByText('PerfectBalance')).toBeVisible();
	
	// Should have one background circle (1 EuclideanCircle)
	await expect.element(page.getByLabelText('background-circle')).toHaveLength(1);
	
	// Should show density and tension info
	await expect.element(page.getByText(/Density: 50%/)).toBeVisible();
	await expect.element(page.getByText(/Tension: 30%/)).toBeVisible();
});

test('renders two circles in Euclidean mode', async () => {
	render(RhythmVisualizer, {
		rhythmMode: 0, // Euclidean
		primarySteps: 16,
		primaryPulses: 4,
		secondarySteps: 12,
		secondaryPulses: 3
	});

	// Header tag
	await expect.element(page.getByText('Euclidean')).toBeVisible();

	// Should have two background circles (2 EuclideanCircles)
	await expect.element(page.getByLabelText('background-circle')).toHaveLength(2);
	
	// Should show polyrhythm info
	await expect.element(page.getByText(/16:12 polyrhythm/)).toBeVisible();
});
