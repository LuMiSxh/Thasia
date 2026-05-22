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
        <span class="text-xs font-bold tracking-wider text-anasthasia-muted uppercase">{label}</span>
    {/if}
    <input
        bind:value
        class="
      w-full rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 py-1.5
      font-sans text-sm
      text-anasthasia-text transition-colors
      duration-150
      outline-none
      placeholder:text-anasthasia-muted hover:border-anasthasia-accent/40 focus:border-anasthasia-accent
      focus:ring-1 focus:ring-anasthasia-accent
      disabled:opacity-40
    "
        {...rest}
    />
    {#if hint}
        <span class="text-xs text-anasthasia-muted">{hint}</span>
    {/if}
</label>
