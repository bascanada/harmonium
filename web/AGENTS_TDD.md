# Svelte 5 TDD with Vitest Browser Mode

This project uses **Vitest Browser Mode** (powered by Playwright) for testing Svelte 5 components. This provides a real browser environment, which is more accurate than JSDOM.

## Commands

- `npm run test`: Runs all tests once in headless mode (chromium).
- `npm run test:browser`: Runs tests in watch mode.

## Testing Strategy

1. **Think in Roles**: Use `page.getByRole()` for accessible and robust element selection.
2. **Real Events**: Use `await element.click()` and other Playwright-powered actions.
3. **Browser Matchers**: Use `expect.element(locator)` for browser-specific assertions.

## Example Test (`src/lib/components/Counter.test.ts`)

```typescript
import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';
import Counter from './Counter.svelte';

test('increments the count when button is clicked', async () => {
	// 1. Render component with props
	render(Counter, { initialCount: 5 });

	// 2. Locate element by its accessible role
	const button = page.getByRole('button', { name: /count is 5/i });

	// 3. Trigger action
	await button.click();

	// 4. Assert updated state (using a new locator if the role/text changed)
	const updatedButton = page.getByRole('button', { name: /count is 6/i });
	await expect.element(updatedButton).toBeVisible();
});
```

## Svelte 5 Component (`src/lib/components/Counter.svelte`)

```svelte
<script lang="ts">
	let { initialCount = 0 } = $props();
	let count = $state(initialCount);
</script>

<button onclick={() => count++}>
	count is {count}
</button>
```

## Troubleshooting

- If `expect.element().toHaveTextContent()` fails, ensure you aren't using an outdated locator if the element's text (and thus its accessible name) has changed.
- Use `npm run test:browser -- --ui` locally to see the tests running in a real browser for debugging.
