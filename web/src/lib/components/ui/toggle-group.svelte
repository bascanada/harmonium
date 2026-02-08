<script lang="ts">
	import { ToggleGroup as BitsToggleGroup } from 'bits-ui';
	import { cn } from '$lib/utils';

	let {
		value = $bindable(),
		type = 'single' as 'single' | 'multiple',
		class: className,
		children,
		...restProps
	} = $props();
</script>

<BitsToggleGroup.Root
	bind:value
	{type}
	class={cn('flex items-center justify-center rounded-lg bg-neutral-800 p-1', className)}
	{...restProps}
>
	{@render children?.()}
</BitsToggleGroup.Root>
