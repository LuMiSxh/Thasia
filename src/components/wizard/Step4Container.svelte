<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { Button, SegmentedControl } from '$components/ui/index';
    import { IconArrowLeft, IconArrowRight, IconPackage } from '@tabler/icons-svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

    const containerHint: Record<string, string> = {
        cbz: 'Comic Book ZIP — widely supported by all readers',
        epub: 'EPUB 3 fixed-layout — best for e-readers',
        raw: 'Flat image folder — no packaging',
    };
</script>

<div class="flex h-full flex-col">
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Output Container</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">
            The file format used to package the converted images.
        </p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconPackage size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Container format</span>
                </div>
                <SegmentedControl
                    options={[
                        { value: 'cbz', label: 'CBZ' },
                        { value: 'epub', label: 'EPUB' },
                        { value: 'raw', label: 'Raw' },
                    ]}
                    bind:value={wizard.container}
                />
                <p class="text-xs text-thasia-muted">{containerHint[wizard.container]}</p>
            </div>
        </div>
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={onNext} class="ml-auto">Next <IconArrowRight size={15} /></Button>
    </div>
</div>
