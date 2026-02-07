<script lang="ts">
	import { ToggleGroup as BitsToggleGroup } from 'bits-ui';
	import { cn } from '$lib/utils';

	let { value, class: className, children, ...restProps } = $props();
</script>

<BitsToggleGroup.Item
	{value}
	class={cn(
		'focus-visible:ring-ring inline-flex h-9 items-center justify-center rounded-md px-3 text-sm font-medium whitespace-nowrap transition-all focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
		'data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:bg-neutral-600 data-[state=on]:text-white data-[state=on]:shadow-sm',
		'text-neutral-400 hover:text-neutral-200',
		className
	)}
	{...restProps}
>
	{@render children?.()}
</BitsToggleGroup.Item>
