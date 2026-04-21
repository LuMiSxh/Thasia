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
        <span class="text-xs font-bold tracking-wider text-thasia-muted uppercase">{label}</span>
    {/if}
    <input
        bind:value
        class="
      w-full rounded-lg border border-thasia-border bg-thasia-bg px-3 py-1.5
      font-sans text-sm
      text-thasia-text transition-colors
      duration-150
      outline-none
      placeholder:text-thasia-muted hover:border-thasia-accent/40 focus:border-thasia-accent
      focus:ring-1 focus:ring-thasia-accent
      disabled:opacity-40
    "
        {...rest}
    />
    {#if hint}
        <span class="text-xs text-thasia-muted">{hint}</span>
    {/if}
</label>
