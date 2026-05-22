<script lang="ts">
    import { receivePill, sendPill } from 'anasthasia';

    interface Option<T extends string = string> {
        value: T;
        label: string;
    }

    interface Props<T extends string = string> {
        options: Option<T>[];
        value: T;
        label?: string;
        disabled?: boolean;
        class?: string;
        onchange?: (value: T) => void;
    }

    let {
        options,
        value = $bindable(),
        label,
        disabled = false,
        class: className = '',
        onchange,
    }: Props = $props();
</script>

<div class="flex flex-col gap-1.5 {className}">
    {#if label}
        <span class="text-xs font-bold tracking-wider text-anasthasia-muted uppercase">{label}</span
        >
    {/if}

    <div
        class="inline-flex gap-0 rounded-lg border border-anasthasia-border bg-anasthasia-bg p-0.5
              transition-colors duration-150 hover:border-anasthasia-accent/40"
    >
        {#each options as opt (opt.value)}
            <button
                type="button"
                {disabled}
                onclick={() => {
                    value = opt.value;
                    onchange?.(opt.value);
                }}
                class="
          relative flex-1 rounded-md px-3 py-1 text-sm font-bold
          transition-colors duration-150
          focus-visible:ring-1 focus-visible:ring-anasthasia-accent focus-visible:outline-none
          disabled:pointer-events-none disabled:opacity-40
          {value === opt.value
                    ? 'text-anasthasia-accent'
                    : 'text-anasthasia-muted hover:text-anasthasia-text'}
        "
            >
                {#if value === opt.value}
                    <span
                        class="absolute inset-0 rounded-md border border-anasthasia-accent/50 bg-anasthasia-surface shadow-sm"
                        in:receivePill={{ key: 'pill' }}
                        out:sendPill={{ key: 'pill' }}
                    ></span>
                {/if}
                <span class="relative z-10">{opt.label}</span>
            </button>
        {/each}
    </div>
</div>
