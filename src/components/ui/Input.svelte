<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    label?: string;
    hint?: string;
    class?: string;
  }

  let { label, hint, class: className = '', value = $bindable(), ...rest }: Props = $props();
</script>

<label class="flex flex-col gap-1.5 {className}">
  {#if label}
    <span class="text-xs font-bold tracking-wider uppercase text-thasia-muted">{label}</span>
  {/if}
  <input
    bind:value
    class="
      w-full rounded-lg border px-3 py-1.5 text-sm font-sans
      bg-zinc-50 dark:bg-thasia-bg
      border-zinc-300 dark:border-thasia-border
      text-thasia-text placeholder:text-thasia-muted
      shadow-inner
      outline-none focus:ring-1 focus:ring-thasia-accent
      transition-shadow duration-150
      disabled:opacity-40
    "
    {...rest}
  />
  {#if hint}
    <span class="text-xs text-thasia-muted">{hint}</span>
  {/if}
</label>
