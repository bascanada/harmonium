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
    ...restProps
  } = $props();

  function handleValueChange(v: number) {
    // value is automatically updated via bind:value below, but we trigger the callback
    onValueChange?.(v);
  }
</script>

<div class={cn("grid gap-2", className)} {...restProps}>
  {#if label}
    <span class="text-sm font-medium text-neutral-400">{label}</span>
  {/if}
  
  <BitsSlider.Root
    type="single"
    bind:value={value}
    onValueChange={handleValueChange}
    {min}
    {max}
    {step}
    class="relative flex w-full touch-none select-none items-center"
  >
    <span class="relative h-2 w-full grow overflow-hidden rounded-full bg-neutral-700">
      <BitsSlider.Range class="absolute h-full bg-blue-500" />
    </span>
    <BitsSlider.Thumb
      index={0}
      class="block h-5 w-5 rounded-full border-2 border-primary bg-background ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
    />
  </BitsSlider.Root>
</div>
