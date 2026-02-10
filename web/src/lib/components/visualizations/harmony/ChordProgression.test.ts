import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';
import ChordProgression from './ChordProgression.svelte';

test('renders ChordProgression with provided chords', async () => {
	render(ChordProgression, {
		progressionName: 'Jazz Standard',
		progressionChords: ['ii', 'V', 'I'],
		currentChord: 'ii',
		currentMeasure: 1,
		harmonyMode: 1
	});

	await expect.element(page.getByText('Jazz Standard')).toBeVisible();
	await expect.element(page.getByText('Driver')).toBeVisible();
	await expect.element(page.getByTestId('measure-display')).toHaveTextContent('Measure 1');
	await expect.element(page.getByTestId('current-chord-display')).toHaveTextContent('ii');
	
	// Check progression length
	await expect.element(page.getByTestId('progression-chord')).toHaveLength(3);
});

test('updates active chord state', async () => {
	const { rerender } = render(ChordProgression, {
		progressionChords: ['I', 'IV', 'V'],
		currentChord: 'I'
	});

	// Check that the first chord is active
	const chords = page.getByTestId('progression-chord');
	await expect.element(chords.first()).toHaveAttribute('data-active', 'true');
	await expect.element(chords.last()).toHaveAttribute('data-active', 'false');

	// Update current chord
	await rerender({ currentChord: 'V' });

	await expect.element(chords.first()).toHaveAttribute('data-active', 'false');
	await expect.element(chords.last()).toHaveAttribute('data-active', 'true');
	await expect.element(page.getByTestId('current-chord-display')).toHaveTextContent('V');
});

test('toggles minor chord styling', async () => {
	const { rerender } = render(ChordProgression, {
		currentChord: 'Am',
		isMinorChord: true
	});

	const display = page.getByTestId('current-chord-display');
	await expect.element(display).toHaveClass(/text-blue-400/);

	await rerender({ isMinorChord: false, currentChord: 'C' });
	await expect.element(display).toHaveClass(/text-yellow-400/);
});
