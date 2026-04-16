<script lang="ts">
	import Logo from "./Logo.svelte";

	interface Props {
		subtitle?: string;
		size?: "sm" | "md" | "lg";
		class?: string;
	}

	let { subtitle = "", size = "md", class: className = "" }: Props = $props();

	const textSizes = {
		sm: { title: "text-sm", sub: "text-[10px]", gap: "gap-1.5" },
		md: { title: "text-base", sub: "text-xs", gap: "gap-2" },
		lg: { title: "text-xl", sub: "text-sm", gap: "gap-3" },
	};

	const logoSizes = { sm: "xs" as const, md: "sm" as const, lg: "md" as const };
</script>

<div class="flex items-center {textSizes[size].gap} {className}">
	<Logo size={logoSizes[size]} />
	<div class="flex flex-col">
		<span class="{textSizes[size].title} font-semibold tracking-wider text-[var(--color-text,#f0ece4)]">HARMONIUM</span>
		{#if subtitle}
			<span class="{textSizes[size].sub} tracking-[0.3em] text-[var(--color-muted,#9490a8)] uppercase">{subtitle}</span>
		{/if}
	</div>
</div>
