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
    <span class="text-xs font-bold tracking-wider uppercase text-thasia-muted">{label}</span>
  {/if}
  <select
    bind:value
    {disabled}
    onchange={() => onchange?.(value)}
    class="
      w-full rounded-lg border px-3 py-1.5 text-sm font-sans
      bg-zinc-50 dark:bg-thasia-bg
      border-zinc-300 dark:border-thasia-border
      text-thasia-text
      shadow-inner
      outline-none focus:ring-1 focus:ring-thasia-accent
      transition-shadow duration-150
      disabled:opacity-40 cursor-pointer
    "
  >
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
  </select>
  {#if hint}
    <span class="text-xs text-thasia-muted">{hint}</span>
  {/if}
</label>
