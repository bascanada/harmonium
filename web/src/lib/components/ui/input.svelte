<script lang="ts">
  import { cn } from '$lib/utils';

  let { 
    value = $bindable(),
    class: className = undefined, 
    type = "text",
    ...restProps 
  } = $props();
</script>

<input
  bind:value
  {type}
  class={cn(
    "flex h-9 w-full rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-neutral-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50",
    className
  )}
  {...restProps}
/>
