<script lang="ts">
  import { cn } from '$lib/utils';

  let { 
    class: className, 
    children, 
    title = undefined,
    description = undefined
  } = $props();
</script>

<div class={cn("rounded-lg border bg-neutral-900/50 text-neutral-100 shadow-sm", className)}>
  {#if title || description}
    <div class="flex flex-col space-y-1.5 p-6 pb-2">
      {#if title}
        <h3 class="text-lg font-semibold leading-none tracking-tight">{title}</h3>
      {/if}
      {#if description}
        <p class="text-xs text-neutral-500">{description}</p>
      {/if}
    </div>
  {/if}
  <div class="p-6 pt-2">
    {@render children?.()}
  </div>
</div>
