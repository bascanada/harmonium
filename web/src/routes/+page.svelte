<script lang="ts">
	import mermaid from 'mermaid';
	import { onMount } from 'svelte';

	let { data } = $props();

	onMount(() => {
		mermaid.initialize({
			startOnLoad: true,
			theme: 'dark',
			securityLevel: 'loose'
		});
		mermaid.run();
	});
</script>

<!-- Floating Action Button -->
<a
	href="/test"
	class="group fixed top-6 left-6 z-50 flex items-center gap-2 rounded-full
           bg-purple-600/90 px-6 py-3 font-semibold
           text-white shadow-lg backdrop-blur-sm transition-all duration-300 hover:bg-purple-500 hover:shadow-purple-500/20"
>
	<span>Try Harmonium</span>
	<svg
		xmlns="http://www.w3.org/2000/svg"
		fill="none"
		viewBox="0 0 24 24"
		stroke-width="2"
		stroke="currentColor"
		class="h-4 w-4 transition-transform group-hover:translate-x-1"
	>
		<path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
	</svg>
</a>

<div
	class="flex min-h-screen justify-center bg-neutral-950 px-4 py-24 font-sans text-neutral-100 sm:px-8"
>
	<!-- Readme Container -->
	<article
		class="w-full max-w-4xl rounded-2xl border border-neutral-800 bg-neutral-900/50 p-8 shadow-2xl backdrop-blur-sm md:p-12"
	>
		<div
			class="prose prose-lg max-w-none prose-invert
            prose-headings:font-bold prose-headings:text-purple-100
            prose-h1:mb-8 prose-h1:border-b prose-h1:border-neutral-800 prose-h1:pb-4 prose-h1:text-4xl prose-h1:text-purple-400
            prose-h2:mt-12 prose-h2:text-2xl prose-h2:text-purple-300
            prose-a:text-purple-400 prose-a:no-underline hover:prose-a:text-purple-300 hover:prose-a:underline
            prose-strong:text-white
            prose-code:rounded prose-code:bg-purple-900/30 prose-code:px-1 prose-code:py-0.5 prose-code:text-purple-200 prose-code:before:content-none prose-code:after:content-none
            prose-li:marker:text-neutral-500"
		>
			{@html data.readme}
		</div>
	</article>
</div>
