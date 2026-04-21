<script lang="ts">
    interface Props {
        value: number; // 0–1
        variant?: 'accent' | 'success' | 'warning' | 'danger';
        color?: string; // custom CSS color — overrides variant
        class?: string;
    }

    let { value, variant = 'accent', color = '', class: className = '' }: Props = $props();

    const trackColors: Record<string, string> = {
        accent: 'bg-thasia-accent',
        success: 'bg-emerald-500',
        warning: 'bg-amber-500',
        danger: 'bg-red-500',
    };

    let pct = $derived(Math.min(Math.max(value * 100, 0), 100));
</script>

<div
    class="h-1.5 w-full overflow-hidden rounded-full border border-thasia-border bg-thasia-bg {className}"
>
    <div
        class="h-full rounded-full transition-[width] duration-200 {color
            ? ''
            : trackColors[variant]}"
        style="width: {pct}%{color ? `; background: ${color}` : ''}"
    ></div>
</div>
