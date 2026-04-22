<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { STEPS, activeSteps } from '$lib/wizard/steps';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

    onMount(() => {
        sidebar.enterWizard();
        if (!wizard.currentStepId) wizard.currentStepId = 'source';

        const raw = localStorage.getItem('thasia:settings');
        if (raw && !wizard.sourcePath) {
            try {
                const d = JSON.parse(raw);
                if (d.imageFormat) wizard.imageFormat = d.imageFormat;
                if (d.container) wizard.container = d.container;
                if (d.direction) wizard.direction = d.direction;
                if (d.bundle) wizard.bundle = d.bundle;
                if (d.volumeSeparator !== undefined) wizard.volumeSeparator = d.volumeSeparator;
                if (d.hideSingleVolume !== undefined) wizard.hideSingleVolume = d.hideSingleVolume;
                if (d.createDirectory !== undefined) wizard.createDirectory = d.createDirectory;
                if (d.maxWidth !== undefined) wizard.maxWidth = d.maxWidth;
            } catch {}
        }

        document.addEventListener('wizard:goto', handleGoto);

        const cleanup = keyboard.smartRegister([
            ['alt+arrowright', (e) => { e.preventDefault(); goNext(); return true; }],
            ['alt+arrowleft', (e) => { e.preventDefault(); goBack(); return true; }],
        ]);

        return () => {
            cleanup();
            document.removeEventListener('wizard:goto', handleGoto);
        };
    });

    onDestroy(() => {
        sidebar.exitWizard();
    });

    function handleGoto(e: Event) {
        const id = (e as CustomEvent<string>).detail;
        if (wizard.completedStepIds.has(id)) wizard.currentStepId = id;
    }

    let active = $derived(activeSteps(wizard));
    let currentStep = $derived(active.find((s) => s.id === wizard.currentStepId) ?? active[0]);
    let currentIndex = $derived(active.findIndex((s) => s.id === wizard.currentStepId));

    function goNext() {
        if (!currentStep) return;
        wizard.markComplete(currentStep.id);
        const nextIndex = currentIndex + 1;
        if (nextIndex < active.length) wizard.currentStepId = active[nextIndex].id;
        if (active[nextIndex]?.id === 'convert') sidebar.collapseForConversion();
    }

    function goBack() {
        if (currentIndex > 0) wizard.currentStepId = active[currentIndex - 1].id;
    }
</script>

<div
    class="flex h-full flex-col"
    use:mountedHint={[
        ['alt+arrowright', 'Next step'],
        ['alt+arrowleft', 'Back'],
    ]}
>
    {#if currentStep}
        {#key currentStep.id}
            <currentStep.component onNext={goNext} onBack={goBack} />
        {/key}
    {/if}
</div>
