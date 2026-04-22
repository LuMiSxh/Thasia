<script lang="ts">
    interface Option<T extends string = string> {
        value: T;
        label: string;
    }

    interface Props<T extends string = string> {
        options: Option<T>[];
        value: T;
        label?: string;
        hint?: string;
        disabled?: boolean;
        class?: string;
        onchange?: (value: T) => void;
    }

    let {
        options,
        value = $bindable(),
        label,
        hint,
        disabled = false,
        class: className = '',
        onchange,
    }: Props = $props();
</script>

<label class="flex flex-col gap-1.5 {className}">
    {#if label}
        <span class="text-xs font-bold tracking-wider text-thasia-muted uppercase">{label}</span>
    {/if}
    <select
        bind:value
        {disabled}
        onchange={() => onchange?.(value)}
        class="
      w-full cursor-pointer rounded-lg border border-thasia-border bg-thasia-bg px-3
      py-1.5 font-sans text-sm
      text-thasia-text
      transition-colors
      duration-150 outline-none hover:border-thasia-accent/40
      focus:border-thasia-accent focus:ring-1
      focus:ring-thasia-accent disabled:opacity-40
    "
    >
        {#each options as opt (opt)}
            <option value={opt.value}>{opt.label}</option>
        {/each}
    </select>
    {#if hint}
        <span class="text-xs text-thasia-muted">{hint}</span>
    {/if}
</label>
