<script lang="ts">
	import { Slider as BitsSlider } from 'bits-ui';
	import { cn } from '$lib/utils';

	let {
		value = $bindable(0),
		min = 0,
		max = 1,
		step = 0.01,
		label = undefined,
		class: className = undefined,
		onValueChange, // Optional callback
		onValueCommit, // Optional callback for drag end
		...restProps
	} = $props();

	// Extract pointer events to pass them directly to the Root
	const { onpointerdown, onpointerup, onpointercancel, ...others } = restProps;

	function handleValueChange(v: number) {
		// value is automatically updated via bind:value below, but we trigger the callback
		onValueChange?.(v);
	}

	function handleValueCommit(v: number) {
		onValueCommit?.(v);
	}

	// Determine range color from class or default to blue
	const rangeClass = className?.includes('accent-') 
		? className.split(' ').find(c => c.startsWith('accent-'))?.replace('accent-', 'bg-')
		: 'bg-blue-500';
</script>

<div class={cn('grid gap-2', className)} {...others}>
	{#if label}
		<span class="text-sm font-medium text-neutral-400">{label}</span>
	{/if}

	<BitsSlider.Root
		type="single"
		bind:value
		onValueChange={handleValueChange}
		onValueCommit={handleValueCommit}
		{min}
		{max}
		{step}
		{onpointerdown}
		{onpointerup}
		{onpointercancel}
		class="relative flex w-full touch-none items-center select-none"
	>
		<span class="relative h-2 w-full grow overflow-hidden rounded-full bg-neutral-700">
			<BitsSlider.Range class={cn("absolute h-full", rangeClass)} />
		</span>
		<BitsSlider.Thumb
			index={0}
			class="border-primary bg-background ring-offset-background focus-visible:ring-ring block h-5 w-5 rounded-full border-2 transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
		/>
	</BitsSlider.Root>
</div>
