<script lang="ts">
    let { data } = $props();
</script>

<!-- Floating Action Button -->
<a 
    href="/test"
    class="fixed top-6 left-6 z-50 px-6 py-3 font-semibold text-white rounded-full 
           bg-purple-600/90 hover:bg-purple-500 shadow-lg hover:shadow-purple-500/20 
           backdrop-blur-sm transition-all duration-300 flex items-center gap-2 group"
>
    <span>Try Harmonium</span>
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 group-hover:translate-x-1 transition-transform">
        <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
    </svg>
</a>

<div class="min-h-screen bg-neutral-950 text-neutral-100 font-sans py-24 px-4 sm:px-8 flex justify-center">
    <!-- Readme Container -->
    <article class="w-full max-w-4xl bg-neutral-900/50 backdrop-blur-sm border border-neutral-800 rounded-2xl p-8 md:p-12 shadow-2xl">
        <div class="prose prose-invert prose-lg max-w-none 
            prose-headings:text-purple-100 prose-headings:font-bold
            prose-h1:text-4xl prose-h1:text-purple-400 prose-h1:mb-8 prose-h1:pb-4 prose-h1:border-b prose-h1:border-neutral-800
            prose-h2:text-2xl prose-h2:text-purple-300 prose-h2:mt-12
            prose-a:text-purple-400 prose-a:no-underline hover:prose-a:text-purple-300 hover:prose-a:underline
            prose-strong:text-white
            prose-code:text-purple-200 prose-code:bg-purple-900/30 prose-code:px-1 prose-code:py-0.5 prose-code:rounded prose-code:before:content-none prose-code:after:content-none
            prose-li:marker:text-neutral-500">
            {@html data.readme}
        </div>
    </article>
</div>