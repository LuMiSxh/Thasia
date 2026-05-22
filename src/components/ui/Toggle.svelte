<script lang="ts">
    interface Props {
        checked: boolean;
        onchange?: (checked: boolean) => void;
        disabled?: boolean;
        label?: string;
        class?: string;
    }

    let {
        checked = $bindable(),
        onchange,
        disabled = false,
        label,
        class: className = '',
    }: Props = $props();

    function toggle() {
        if (disabled) return;
        checked = !checked;
        onchange?.(checked);
    }
</script>

<button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={label}
    {disabled}
    onclick={toggle}
    class="
    group inline-flex cursor-pointer items-center gap-2.5 select-none
    focus-visible:ring-2 focus-visible:ring-anasthasia-accent
    focus-visible:ring-offset-2 focus-visible:ring-offset-anasthasia-surface focus-visible:outline-none disabled:pointer-events-none disabled:opacity-40
    {className}
  "
>
    <!-- Track -->
    <span
        class="
      relative inline-flex h-5 w-9 shrink-0 items-center rounded-md border transition-colors duration-200
      {checked
            ? 'border-anasthasia-accent/50 bg-anasthasia-accent'
            : 'border-anasthasia-border bg-anasthasia-bg group-hover:border-anasthasia-accent/40'}
    "
    >
        <!-- Thumb -->
        <span
            class="
        absolute h-3.5 w-3.5 rounded-sm border shadow-sm transition-all duration-200
        {checked
                ? 'translate-x-[18px] border-black/20 bg-black dark:bg-zinc-900'
                : 'translate-x-0.5 border-anasthasia-border bg-anasthasia-muted group-hover:border-anasthasia-accent/40 group-hover:bg-anasthasia-accent/30'}
      "
        ></span>
    </span>
    {#if label}
        <span class="text-sm text-anasthasia-text">{label}</span>
    {/if}
</button>
