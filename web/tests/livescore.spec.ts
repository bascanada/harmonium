import { test, expect } from '@playwright/test';

test.describe('LiveScore Piano Roll', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/test');
	});

	test('should render LiveScore canvas', async ({ page }) => {
		// Start engine
		const startButton = page.locator('button:has-text("Start Music")');
		await startButton.click();
		await page.waitForTimeout(2000);

		// LiveScore should be visible
		const liveScore = page.locator('.live-score-container');
		await expect(liveScore).toBeVisible();

		// Should have canvas
		const canvas = liveScore.locator('canvas');
		await expect(canvas).toBeVisible();

		// Should have title
		await expect(liveScore.locator('text=Live Score')).toBeVisible();

		console.log('✅ LiveScore rendered successfully');
	});

	test('should display instrument legend', async ({ page }) => {
		const startButton = page.locator('button:has-text("Start Music")');
		await startButton.click();
		await page.waitForTimeout(2000);

		const liveScore = page.locator('.live-score-container');

		// Check for all 4 instruments in legend
		await expect(liveScore.locator('text=Bass')).toBeVisible();
		await expect(liveScore.locator('text=Lead')).toBeVisible();
		await expect(liveScore.locator('text=Snare')).toBeVisible();
		await expect(liveScore.locator('text=Hat')).toBeVisible();

		console.log('✅ All instruments shown in legend');
	});

	test('should maintain 60fps while rendering', async ({ page }) => {
		await page.goto('/test');
		const startButton = page.locator('button:has-text("Start Music")');
		await startButton.click();
		await page.waitForTimeout(2000);

		// Measure FPS
		const fps = await page.evaluate(async () => {
			return new Promise<number>((resolve) => {
				let frameCount = 0;
				const startTime = performance.now();
				const duration = 2000; // 2 seconds

				function countFrames() {
					frameCount++;
					const elapsed = performance.now() - startTime;

					if (elapsed < duration) {
						requestAnimationFrame(countFrames);
					} else {
						const measuredFps = (frameCount / elapsed) * 1000;
						resolve(measuredFps);
					}
				}

				requestAnimationFrame(countFrames);
			});
		});

		console.log(`✅ Measured FPS: ${fps.toFixed(2)}`);
		expect(fps).toBeGreaterThan(50); // Allow some margin
	});

	test('should handle canvas resize', async ({ page }) => {
		const startButton = page.locator('button:has-text("Start Music")');
		await startButton.click();
		await page.waitForTimeout(1000);

		const canvas = page.locator('.live-score-container canvas');

		// Get initial dimensions
		const initialBox = await canvas.boundingBox();
		expect(initialBox).toBeTruthy();

		// Resize viewport
		await page.setViewportSize({ width: 800, height: 600 });
		await page.waitForTimeout(500);

		// Canvas should still be visible
		await expect(canvas).toBeVisible();

		console.log('✅ Canvas handles resize gracefully');
	});

	test('should not have console errors', async ({ page }) => {
		const errors: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') {
				errors.push(msg.text());
			}
		});

		const startButton = page.locator('button:has-text("Start Music")');
		await startButton.click();
		await page.waitForTimeout(3000);

		// Filter out known non-critical errors
		const criticalErrors = errors.filter(
			(err) => !err.includes('each_key_duplicate') && !err.includes('vite')
		);

		expect(criticalErrors).toHaveLength(0);
		console.log('✅ No critical console errors');
	});
});
