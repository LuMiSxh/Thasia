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
        <span class="text-xs font-bold tracking-wider text-anasthasia-muted uppercase">{label}</span>
    {/if}
    <select
        bind:value
        {disabled}
        onchange={() => onchange?.(value)}
        class="
      w-full cursor-pointer rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3
      py-1.5 font-sans text-sm
      text-anasthasia-text
      transition-colors
      duration-150 outline-none hover:border-anasthasia-accent/40
      focus:border-anasthasia-accent focus:ring-1
      focus:ring-anasthasia-accent disabled:opacity-40
    "
    >
        {#each options as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
        {/each}
    </select>
    {#if hint}
        <span class="text-xs text-anasthasia-muted">{hint}</span>
    {/if}
</label>
