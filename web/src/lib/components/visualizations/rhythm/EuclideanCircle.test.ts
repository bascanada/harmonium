import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';
import EuclideanCircle from './EuclideanCircle.svelte';

test('renders EuclideanCircle with correct number of pulses', async () => {
	// 1. Render component
	const { rerender } = render(EuclideanCircle, {
		steps: 16,
		pulses: 4,
		label: 'Kick'
	});

	// 2. Check for label text
	await expect.element(page.getByText('Kick')).toBeVisible();

	// 3. Check for the dots (active points)
	await expect.element(page.getByLabelText('pulse-dot')).toHaveLength(4);
	await expect.element(page.getByLabelText('background-circle')).toBeVisible();

	// 4. Update pulses
	await rerender({ pulses: 8 });
	
	await expect.element(page.getByLabelText('pulse-dot')).toHaveLength(8);
});

test('highlights the current step', async () => {
	render(EuclideanCircle, {
		steps: 4,
		pulses: 4,
		currentStep: 0
	});
	
	await expect.element(page.getByTestId('current-step')).toBeVisible();
});