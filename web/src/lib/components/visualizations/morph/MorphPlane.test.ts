import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';
import MorphPlane from './MorphPlane.svelte';

test('renders points at correct positions', async () => {
	render(MorphPlane, {
		title: 'Test Plane',
		xMin: 0,
		xMax: 100,
		yMin: 0,
		yMax: 100,
		points: [
			{ id: 'center', x: 50, y: 50, label: 'Center' },
			{ id: 'top-right', x: 100, y: 100, label: 'Top Right' }
		]
	});

	await expect.element(page.getByText('TEST PLANE')).toBeVisible();
	
	const centerPoint = page.getByLabelText('Center');
	await expect.element(centerPoint).toBeVisible();
	// Check style for positioning
	await expect.element(centerPoint).toHaveStyle({ left: '50%', top: '50%' });

	const topRightPoint = page.getByLabelText('Top Right');
	// y=100 in graph is top: 0% in CSS
	await expect.element(topRightPoint).toHaveStyle({ left: '100%', top: '0%' });
});
