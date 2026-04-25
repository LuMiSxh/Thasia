<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { Button, SegmentedControl } from '$components/ui/index';
    import { IconArrowLeft, IconArrowRight, IconDirection } from '@tabler/icons-svelte';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

    let { onNext, onBack, nextDisabled = false, backDisabled = false }: {
        onNext: () => void;
        onBack: () => void;
        nextDisabled?: boolean;
        backDisabled?: boolean;
    } = $props();

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['keyl', () => { wizard.direction = 'ltr'; return true; }],
            ['keyr', () => { wizard.direction = 'rtl'; return true; }],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<div class="flex h-full flex-col" use:mountedHint={[['keyl', 'LTR'], ['keyr', 'RTL']]}>
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Reading Direction</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">Controls page order in the EPUB output.</p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconDirection size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Direction</span>
                </div>
                <SegmentedControl
                    options={[
                        { value: 'ltr', label: 'Left to Right' },
                        { value: 'rtl', label: 'Right to Left' },
                    ]}
                    bind:value={wizard.direction}
                />
                <p class="text-xs text-thasia-muted">
                    {wizard.direction === 'rtl'
                        ? 'Right-to-left — standard for manga and manhua'
                        : 'Left-to-right — standard for Western comics and manhwa'}
                </p>
            </div>
        </div>
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack} disabled={backDisabled}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={onNext} disabled={nextDisabled} class="ml-auto">Next <IconArrowRight size={15} /></Button>
    </div>
</div>
