<script lang="ts">
	type LogoVariant = "dark" | "light" | "mono";
	type LogoSize = "xs" | "sm" | "md" | "lg" | "xl";

	interface Props {
		variant?: LogoVariant;
		size?: LogoSize;
		class?: string;
	}

	let { variant = "dark", size = "md", class: className = "" }: Props = $props();

	const sizes: Record<LogoSize, string> = {
		xs: "w-4 h-4",
		sm: "w-6 h-6",
		md: "w-8 h-8",
		lg: "w-12 h-12",
		xl: "w-16 h-16",
	};

	const srcs: Record<LogoVariant, string> = {
		dark: "/logo/mark-dark.svg",
		light: "/logo/mark-light.svg",
		mono: "/logo/mark-mono.svg",
	};
</script>

<img
	src={srcs[variant]}
	alt="Harmonium"
	class="{sizes[size]} {className}"
	draggable="false"
/>
