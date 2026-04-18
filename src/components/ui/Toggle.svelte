<script lang="ts">
  interface Props {
    checked: boolean;
    onchange?: (checked: boolean) => void;
    disabled?: boolean;
    label?: string;
    class?: string;
  }

  let { checked = $bindable(), onchange, disabled = false, label, class: className = '' }: Props = $props();

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
    inline-flex items-center gap-2.5 cursor-pointer select-none
    disabled:opacity-40 disabled:pointer-events-none
    focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-thasia-accent focus-visible:ring-offset-2 focus-visible:ring-offset-thasia-surface
    {className}
  "
>
  <!-- Track -->
  <span
    class="
      relative inline-flex h-5 w-9 shrink-0 items-center rounded-md border transition-colors duration-200
      {checked
        ? 'bg-thasia-accent border-thasia-accent/50'
        : 'bg-thasia-bg border-thasia-border'}
    "
  >
    <!-- Thumb -->
    <span
      class="
        absolute h-3.5 w-3.5 rounded-sm border transition-all duration-200 shadow-sm
        {checked
          ? 'translate-x-[18px] bg-black dark:bg-zinc-900 border-black/20'
          : 'translate-x-0.5 bg-thasia-muted border-thasia-border'}
      "
    ></span>
  </span>
  {#if label}
    <span class="text-sm text-thasia-text">{label}</span>
  {/if}
</button>
