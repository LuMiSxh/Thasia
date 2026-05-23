<script lang="ts">
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { duration, Input, SegmentedControl, Toggle } from 'anasthasia';
    import { IconPhoto, IconRuler } from '@tabler/icons-svelte';

    interface Props {
        format: 'avif' | 'webp' | 'original';
        maxWidth: number | null;
        enableMaxWidth: boolean;
    }

    let {
        format = $bindable(),
        maxWidth = $bindable(),
        enableMaxWidth = $bindable(),
    }: Props = $props();

    const collapse = { duration: duration.base, easing: cubicInOut };

    const formatHint: Record<Props['format'], string> = {
        avif: 'Best compression, slower — ideal for archiving',
        webp: 'Good compression, widely supported',
        original: 'No re-encoding — fastest, preserves originals',
    };
</script>

<div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
    <!-- Format -->
    <div class="flex flex-col gap-2.5 px-4 py-4">
        <div class="flex items-center gap-2">
            <IconPhoto size={14} class="flex-shrink-0 text-anasthasia-muted" />
            <span class="text-sm font-medium">Format</span>
        </div>
        <SegmentedControl
            options={[
                { value: 'avif', label: 'AVIF' },
                { value: 'webp', label: 'WebP' },
                { value: 'original', label: 'Original' },
            ]}
            bind:value={format}
        />
        <p class="text-xs text-anasthasia-muted">{formatHint[format]}</p>
    </div>

    <div class="mx-4 border-t border-anasthasia-border"></div>

    <!-- Max width -->
    <div class="flex flex-col gap-2.5 px-4 py-4">
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
                <IconRuler size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <div>
                    <div class="text-sm font-medium">Max Width</div>
                    <div class="text-xs text-anasthasia-muted">Downscale wider images (px)</div>
                </div>
            </div>
            <Toggle
                bind:checked={enableMaxWidth}
                onchange={(v) => {
                    maxWidth = v ? 1920 : null;
                }}
            />
        </div>
        {#if enableMaxWidth}
            <div transition:slide={collapse}>
                <Input
                    type="number"
                    min="400"
                    max="4000"
                    step="100"
                    bind:value={maxWidth as number}
                    hint="Common values: 1200, 1440, 1920"
                />
            </div>
        {/if}
    </div>
</div>
