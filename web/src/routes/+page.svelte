<script lang="ts">
	// HarmoniumDemo requires WASM build — skip for design work
	// import HarmoniumDemo from '$lib/components/HarmoniumDemo.svelte';
</script>

<div class="min-h-screen p-8 space-y-8" style="background: var(--h-bg); color: var(--h-fg);">
	<h1 style="font-family: var(--h-font-display); font-size: 2rem;">Harmonium Web — Design Preview</h1>
	<p style="color: var(--h-fg-2);">This page previews the shared design tokens from <code style="font-family: var(--h-font-mono);">tokens.css</code>.</p>

	<section class="space-y-4">
		<h2 style="font-family: var(--h-font-display); font-size: 1.25rem;">Color Palette</h2>
		<div class="flex flex-wrap gap-3">
			{#each [
				['Background', 'var(--h-bg)'],
				['Background Alt', 'var(--h-bg-alt)'],
				['Card', 'var(--h-bg-card)'],
				['Elevated', 'var(--h-bg-elevated)'],
				['Amber', 'var(--h-amber)'],
				['Amber Light', 'var(--h-amber-light)'],
				['Copper', 'var(--h-copper)'],
				['Tonic', 'var(--h-tonic)'],
				['Subdominant', 'var(--h-subdominant)'],
				['Dominant', 'var(--h-dominant)'],
				['Chord Tone', 'var(--h-chord-tone)'],
			] as [label, color]}
				<div class="flex flex-col items-center gap-1">
					<div class="w-16 h-16 rounded-lg border border-white/10" style="background: {color};"></div>
					<span class="text-xs" style="color: var(--h-fg-2);">{label}</span>
				</div>
			{/each}
		</div>
	</section>

	<section class="space-y-4">
		<h2 style="font-family: var(--h-font-display); font-size: 1.25rem;">Typography</h2>
		<p style="font-family: var(--h-font-display); font-size: 1.5rem;">Space Grotesk — Display</p>
		<p style="font-family: var(--h-font-body);">Inter — Body text for readability</p>
		<p style="font-family: var(--h-font-mono); color: var(--h-amber);">JetBrains Mono — Code & data</p>
	</section>

	<section class="space-y-4">
		<h2 style="font-family: var(--h-font-display); font-size: 1.25rem;">Cards</h2>
		<div class="flex gap-4">
			{#each ['Card', 'Elevated'] as level}
				<div class="p-6 rounded-xl" style="background: var(--h-bg-{level.toLowerCase()}); border-radius: var(--h-radius);">
					<h3 style="color: var(--h-amber); font-family: var(--h-font-display);">{level}</h3>
					<p class="mt-2 text-sm" style="color: var(--h-fg-2);">A surface for grouping content.</p>
				</div>
			{/each}
		</div>
	</section>
</div>
