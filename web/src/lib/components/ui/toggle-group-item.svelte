<script lang="ts">
  import { ToggleGroup as BitsToggleGroup } from 'bits-ui';
  import { cn } from '$lib/utils';

  let { 
    value, 
    class: className,
    children,
    ...restProps
  } = $props();
</script>

<BitsToggleGroup.Item
  {value}
  class={cn(
    "inline-flex h-9 items-center justify-center whitespace-nowrap rounded-md px-3 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
    "data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:shadow-sm data-[state=on]:bg-neutral-600 data-[state=on]:text-white",
    "text-neutral-400 hover:text-neutral-200",
    className
  )}
  {...restProps}
>
  {@render children?.()}
</BitsToggleGroup.Item>
